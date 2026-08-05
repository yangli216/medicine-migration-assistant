use crate::model::{ConnectionProfile, SourcePreview};
use crate::odbc;
use crate::target_contract::validate_schema_identifier;
use odbc_api::Connection;
use serde::{Deserialize, Serialize};

const REQUIRED_TABLES: &[&str] = &["YK_TYPK", "YK_YPCD", "YK_CDDZ", "YF_YPXX"];
const INSPECTED_TABLES: &[&str] = &[
    "YK_TYPK", "YK_YPBM", "YK_YPCD", "YK_CDDZ", "YK_YPXX", "YK_CDXX", "YK_KCMX", "YK_YKLB",
    "YF_YPXX", "YF_KCMX", "YF_YFLB", "YK_YPSX",
];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27Scope {
    pub id: String,
    pub label: String,
    pub description: String,
    pub estimated_rows: usize,
    pub medicine_count: usize,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27Inspection {
    pub detected: bool,
    pub adapter_id: String,
    pub title: String,
    pub schema: String,
    pub checked_tables: Vec<String>,
    pub missing_tables: Vec<String>,
    pub total_medicines: usize,
    pub configured_medicines: usize,
    pub active_configured_medicines: usize,
    pub product_rows: usize,
    pub stock_medicines: usize,
    pub duplicate_business_groups: usize,
    pub orphan_pharmacy_configs: usize,
    pub scopes: Vec<Phis27Scope>,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadPhis27Request {
    pub connection: ConnectionProfile,
    pub scope: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

fn default_limit() -> u32 {
    10_000
}

pub fn inspect(profile: &ConnectionProfile) -> Result<Phis27Inspection, String> {
    ensure_oracle(profile)?;
    let schema = source_schema(profile)?;
    odbc::with_connection(profile, |connection| {
        inspect_connection(connection, &schema)
    })
}

pub fn load(request: &LoadPhis27Request) -> Result<SourcePreview, String> {
    ensure_oracle(&request.connection)?;
    let schema = source_schema(&request.connection)?;
    let scope = normalize_scope(&request.scope)?;
    let query = medicine_query(&schema, scope);
    odbc::preview_source(&request.connection, &query, request.limit.clamp(1, 10_000))
}

fn inspect_connection(
    connection: &Connection<'_>,
    schema: &str,
) -> Result<Phis27Inspection, String> {
    let mut checked_tables = Vec::new();
    let mut missing_tables = Vec::new();
    for table in INSPECTED_TABLES {
        let statement = format!("SELECT 1 FROM {} WHERE 1=0", table_name(schema, table));
        if connection.execute(&statement, (), Some(30)).is_ok() {
            checked_tables.push((*table).to_string());
        } else if REQUIRED_TABLES.contains(table) {
            missing_tables.push((*table).to_string());
        }
    }

    if !missing_tables.is_empty() {
        return Ok(Phis27Inspection {
            detected: false,
            adapter_id: "PHIS27".into(),
            title: "Bsoft PHIS27 药品数据".into(),
            schema: schema.into(),
            checked_tables,
            missing_tables: missing_tables.clone(),
            total_medicines: 0,
            configured_medicines: 0,
            active_configured_medicines: 0,
            product_rows: 0,
            stock_medicines: 0,
            duplicate_business_groups: 0,
            orphan_pharmacy_configs: 0,
            scopes: Vec::new(),
            warnings: vec![format!(
                "缺少核心表：{}；请检查 Schema 或查询授权",
                missing_tables.join("、")
            )],
            message: "未识别为完整的 PHIS27 药品数据源".into(),
        });
    }

    let typk = table_name(schema, "YK_TYPK");
    let ypcd = table_name(schema, "YK_YPCD");
    let yf_ypxx = table_name(schema, "YF_YPXX");
    let yf_yflb = table_name(schema, "YF_YFLB");
    let yk_kcmx = table_name(schema, "YK_KCMX");
    let yf_kcmx = table_name(schema, "YF_KCMX");

    let total_medicines = count(connection, &format!("SELECT COUNT(*) FROM {typk}"))?;
    let configured_medicines = count(
        connection,
        &format!(
            "SELECT COUNT(DISTINCT t.YPXH) FROM {typk} t WHERE EXISTS (SELECT 1 FROM {yf_ypxx} x WHERE x.YPXH=t.YPXH)"
        ),
    )?;
    let active_configured_medicines = count(
        connection,
        &format!(
            "SELECT COUNT(DISTINCT t.YPXH) FROM {typk} t WHERE NVL(t.ZFPB,0)=0 AND EXISTS (SELECT 1 FROM {yf_ypxx} x WHERE x.YPXH=t.YPXH)"
        ),
    )?;
    let product_rows = count(connection, &format!("SELECT COUNT(*) FROM {ypcd}"))?;
    let active_rows = count(connection, &scope_count_query(schema, Scope::UsedActive))?;
    let configured_rows = count(connection, &scope_count_query(schema, Scope::UsedAll))?;
    let duplicate_business_groups = count(
        connection,
        &format!(
            "SELECT COUNT(*) FROM (SELECT t.YPMC,t.TYPE,t.YPGG FROM {typk} t WHERE EXISTS (SELECT 1 FROM {yf_ypxx} x WHERE x.YPXH=t.YPXH) GROUP BY t.YPMC,t.TYPE,t.YPGG HAVING COUNT(*)>1)"
        ),
    )?;

    let stock_medicines = if checked_tables.contains(&"YK_KCMX".to_string())
        && checked_tables.contains(&"YF_KCMX".to_string())
    {
        count(
            connection,
            &format!(
                "SELECT COUNT(*) FROM (SELECT YPXH FROM {yk_kcmx} UNION SELECT YPXH FROM {yf_kcmx})"
            ),
        )?
    } else {
        0
    };
    let orphan_pharmacy_configs = if checked_tables.contains(&"YF_YFLB".to_string()) {
        count(
            connection,
            &format!(
                "SELECT COUNT(*) FROM {yf_ypxx} x WHERE NOT EXISTS (SELECT 1 FROM {yf_yflb} f WHERE f.YFSB=x.YFSB)"
            ),
        )?
    } else {
        0
    };

    let mut warnings = vec![format!(
        "机构配置范围发现{duplicate_business_groups}组目标判重键重复药品；迁移将保留 YPXH 并在预校验阶段阻止自动合并"
    )];
    if orphan_pharmacy_configs > 0 {
        warnings.push(format!(
            "发现{orphan_pharmacy_configs}条药房药品配置无法关联药房，基础同步可继续，库存初始化前必须处理"
        ));
    }
    warnings.push("费用归并主键和剂型编码属于新系统字典，需在校验阶段设置默认值或值映射".into());

    Ok(Phis27Inspection {
        detected: true,
        adapter_id: "PHIS27".into(),
        title: "Bsoft PHIS27 药品数据".into(),
        schema: schema.into(),
        checked_tables,
        missing_tables,
        total_medicines,
        configured_medicines,
        active_configured_medicines,
        product_rows,
        stock_medicines,
        duplicate_business_groups,
        orphan_pharmacy_configs,
        scopes: vec![
            Phis27Scope {
                id: "USED_ACTIVE".into(),
                label: "机构在用药品".into(),
                description: "同步药房已配置且未作废的药品；包含关联厂家商品，推荐首次迁移使用"
                    .into(),
                estimated_rows: active_rows,
                medicine_count: active_configured_medicines,
                recommended: true,
            },
            Phis27Scope {
                id: "USED_ALL".into(),
                label: "机构全部配置药品".into(),
                description: "包含已作废药品，用于需要保留完整机构历史字典的场景".into(),
                estimated_rows: configured_rows,
                medicine_count: configured_medicines,
                recommended: false,
            },
        ],
        warnings,
        message: "已识别 PHIS27 药品主数据结构，可使用内置安全查询模板".into(),
    })
}

#[derive(Debug, Clone, Copy)]
enum Scope {
    UsedActive,
    UsedAll,
}

fn normalize_scope(scope: &str) -> Result<Scope, String> {
    match scope.trim().to_ascii_uppercase().as_str() {
        "USED_ACTIVE" => Ok(Scope::UsedActive),
        "USED_ALL" => Ok(Scope::UsedAll),
        _ => Err("不支持的 PHIS27 药品同步范围".into()),
    }
}

fn scope_predicate(scope: Scope) -> &'static str {
    match scope {
        Scope::UsedActive => {
            "NVL(t.ZFPB,0)=0 AND EXISTS (SELECT 1 FROM {YF_YPXX} x WHERE x.YPXH=t.YPXH)"
        }
        Scope::UsedAll => "EXISTS (SELECT 1 FROM {YF_YPXX} x WHERE x.YPXH=t.YPXH)",
    }
}

fn scope_count_query(schema: &str, scope: Scope) -> String {
    let predicate = scope_predicate(scope).replace("{YF_YPXX}", &table_name(schema, "YF_YPXX"));
    format!(
        "SELECT COUNT(*) FROM {} t LEFT JOIN {} p ON p.YPXH=t.YPXH WHERE {predicate}",
        table_name(schema, "YK_TYPK"),
        table_name(schema, "YK_YPCD")
    )
}

fn medicine_query(schema: &str, scope: Scope) -> String {
    let typk = table_name(schema, "YK_TYPK");
    let ypcd = table_name(schema, "YK_YPCD");
    let cddz = table_name(schema, "YK_CDDZ");
    let predicate = scope_predicate(scope).replace("{YF_YPXX}", &table_name(schema, "YF_YPXX"));
    format!(
        r#"WITH selected_med AS (
            SELECT t.*,
                   COUNT(*) OVER (PARTITION BY t.YPMC,t.TYPE,t.YPGG) AS SOURCE_DUPLICATE_COUNT
            FROM {typk} t
            WHERE {predicate}
        )
        SELECT
            TO_CHAR(t.YPXH)||':'||NVL(TO_CHAR(p.YPCD),'BASE') AS SOURCE_KEY,
            TO_CHAR(t.YPXH) AS SOURCE_MED_ID,
            t.SOURCE_DUPLICATE_COUNT,
            t.YPMC AS DRUG_NAME,
            TO_CHAR(t.TYPE) AS DRUG_TYPE,
            TO_CHAR(t.YPSX) AS FORM_CODE,
            t.YPDW AS PRE_UNIT,
            t.YPJL AS DOSE,
            t.JLDW AS DOSE_UNIT,
            t.YPGG AS SPEC,
            TO_CHAR(t.GYFF) AS USAGE_CODE,
            CAST(NULL AS NVARCHAR2(20)) AS FREQ_CODE,
            TO_CHAR(p.YPCD) AS SOURCE_FACTORY_ID,
            CASE WHEN p.YPCD IS NOT NULL THEN f.CDMC END AS FACTORY_NAME,
            CASE WHEN p.YPCD IS NOT NULL THEN NVL(p.YBSPMC,t.YPMC) END AS PRODUCT_NAME,
            CASE WHEN p.YPCD IS NOT NULL THEN t.YFDW END AS SALE_UNIT,
            CASE WHEN p.YPCD IS NOT NULL THEN t.YFBZ END AS PACK_FACTOR,
            CASE WHEN p.YPCD IS NOT NULL THEN NVL(t.YFGG,t.YPGG) END AS SALE_SPEC,
            p.JHJG AS BUY_PRICE,
            p.LSJG AS RETAIL_PRICE,
            p.PZWH AS APPROVAL_NO,
            p.YPTM AS BARCODE,
            CASE WHEN p.YPCD IS NOT NULL THEN NVL(p.YPLSH,TO_CHAR(t.YPXH)||'-'||TO_CHAR(p.YPCD)) END AS CD_MED_PRO,
            TO_CHAR(t.CFYP) AS RX_FLAG,
            TO_CHAR(t.JYLX) AS BASIC_DRUG_TYPE,
            TO_CHAR(t.TSYP) AS SPECIAL_DRUG_TYPE,
            TO_CHAR(t.ZFPB) AS SOURCE_STOP_FLAG
        FROM selected_med t
        LEFT JOIN {ypcd} p ON p.YPXH=t.YPXH
        LEFT JOIN {cddz} f ON f.YPCD=p.YPCD
        ORDER BY t.YPXH,p.YPCD"#
    )
}

fn count(connection: &Connection<'_>, query: &str) -> Result<usize, String> {
    let value = odbc::query_optional_string(connection, query, Vec::new())?
        .ok_or_else(|| "PHIS27 数据统计没有返回结果".to_string())?;
    value
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("PHIS27 数据统计结果无法识别：{value}"))
}

fn source_schema(profile: &ConnectionProfile) -> Result<String, String> {
    let configured = if profile.schema.trim().is_empty() {
        profile.username.trim()
    } else {
        profile.schema.trim()
    };
    if configured.is_empty() {
        return Err("请填写老库 Schema；通常与 Oracle 用户名一致".into());
    }
    validate_schema_identifier(configured).map(|schema| schema.to_ascii_uppercase())
}

fn table_name(schema: &str, table: &str) -> String {
    format!("{schema}.{table}")
}

fn ensure_oracle(profile: &ConnectionProfile) -> Result<(), String> {
    if profile.kind.eq_ignore_ascii_case("oracle") {
        Ok(())
    } else {
        Err("PHIS27 自动识别当前仅支持 Oracle 老库".into())
    }
}

#[cfg(test)]
mod tests {
    use super::{medicine_query, normalize_scope, source_schema, Scope};
    use crate::model::ConnectionProfile;

    fn profile(schema: &str) -> ConnectionProfile {
        ConnectionProfile {
            kind: "oracle".into(),
            host: "127.0.0.1".into(),
            port: 1521,
            database: "phis".into(),
            username: "PHIS27".into(),
            password: String::new(),
            schema: schema.into(),
            service_name: "phis".into(),
            driver: "Oracle 19 ODBC driver".into(),
            connection_string: String::new(),
        }
    }

    #[test]
    fn uses_validated_schema_and_read_only_query() {
        assert_eq!(source_schema(&profile("phis27")).unwrap(), "PHIS27");
        assert!(source_schema(&profile("PHIS27;DROP TABLE X")).is_err());
        let query = medicine_query("PHIS27", Scope::UsedActive);
        assert!(query.starts_with("WITH"));
        assert!(query.contains("PHIS27.YK_TYPK"));
        assert!(query.contains("NVL(t.ZFPB,0)=0"));
        assert!(query.contains("SOURCE_DUPLICATE_COUNT"));
        assert!(!query.contains(";"));
    }

    #[test]
    fn only_known_scopes_are_accepted() {
        assert!(normalize_scope("USED_ACTIVE").is_ok());
        assert!(normalize_scope("USED_ALL").is_ok());
        assert!(normalize_scope("ALL_TABLES").is_err());
    }

    #[test]
    #[ignore = "需要通过环境变量提供可访问的 PHIS27 Oracle 测试库"]
    fn live_phis27_adapter_reads_expected_structure() {
        let service = std::env::var("PHIS27_TEST_SERVICE").expect("PHIS27_TEST_SERVICE");
        let username = std::env::var("PHIS27_TEST_USER").expect("PHIS27_TEST_USER");
        let live = ConnectionProfile {
            kind: "oracle".into(),
            host: std::env::var("PHIS27_TEST_HOST").expect("PHIS27_TEST_HOST"),
            port: std::env::var("PHIS27_TEST_PORT")
                .unwrap_or_else(|_| "1521".into())
                .parse()
                .expect("PHIS27_TEST_PORT"),
            database: service.clone(),
            username: username.clone(),
            password: std::env::var("PHIS27_TEST_PASSWORD").expect("PHIS27_TEST_PASSWORD"),
            schema: std::env::var("PHIS27_TEST_SCHEMA").unwrap_or(username),
            service_name: service,
            driver: std::env::var("PHIS27_TEST_DRIVER")
                .unwrap_or_else(|_| "Oracle 19 ODBC driver".into()),
            connection_string: String::new(),
        };
        let inspection = super::inspect(&live).expect("PHIS27 inspection");
        assert!(inspection.detected);
        assert!(inspection.configured_medicines > 0);
        let preview = super::load(&super::LoadPhis27Request {
            connection: live,
            scope: "USED_ACTIVE".into(),
            limit: 10_000,
        })
        .expect("PHIS27 load");
        assert!(!preview.truncated);
        assert!(preview.columns.contains(&"SOURCE_KEY".to_string()));
        assert_eq!(preview.rows.len(), inspection.scopes[0].estimated_rows);
    }
}
