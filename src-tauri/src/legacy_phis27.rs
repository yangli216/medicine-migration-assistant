use crate::local_store::LocalStore;
use crate::model::{
    ConnectionProfile, SourceColumnMetadata, SourceDictionaryItem, SourceDictionaryMetadata,
    SourcePreview,
};
use crate::odbc;
use crate::target_contract::validate_schema_identifier;
use odbc_api::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

const REQUIRED_TABLES: &[&str] = &["YK_TYPK", "YK_YPCD", "YK_CDDZ", "YK_CDXX"];
const INSPECTED_TABLES: &[&str] = &[
    "YK_TYPK",
    "YK_YPBM",
    "YK_YPCD",
    "YK_CDDZ",
    "YK_YPXX",
    "YK_CDXX",
    "YK_KCMX",
    "YK_YKLB",
    "YF_YPXX",
    "YF_KCMX",
    "YF_YFLB",
    "SYS_ORGANIZATION",
    "YK_YPSX",
    "ZY_YPYF",
    "GY_SYPC",
    "ZY_FYFS",
];

const DYNAMIC_DICTIONARY_TABLES: &[(&str, &str)] = &[
    ("YK_YPSX", "剂型"),
    ("ZY_YPYF", "给药方法"),
    ("GY_SYPC", "使用频次"),
    ("ZY_FYFS", "发药方式"),
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

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectPhis27InventoryRequest {
    pub connection: ConnectionProfile,
    pub source_name: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27InventoryLocation {
    pub source_kind: String,
    pub source_location_key: String,
    pub source_location_name: String,
    pub organization_id: String,
    pub source_option_count: usize,
    pub stock_row_count: usize,
    pub stock_group_count: usize,
    pub medicine_count: usize,
    pub mapping_status: String,
    pub mapping_message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27InventoryReadiness {
    pub ready_for_location_mapping: bool,
    pub schema: String,
    pub source_name: String,
    pub stock_row_count: usize,
    pub stock_group_count: usize,
    pub medicine_count: usize,
    pub mapped_medicine_count: usize,
    pub unresolved_medicine_count: usize,
    pub locations: Vec<Phis27InventoryLocation>,
    pub unresolved_source_keys: Vec<String>,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27InventoryStockItem {
    pub source_kind: String,
    pub source_record_id: String,
    pub source_location_key: String,
    pub source_location_name: String,
    pub source_organization_id: String,
    pub source_product_key: String,
    pub drug_name: String,
    pub specification: String,
    pub dosage_form: String,
    pub minimum_unit: String,
    pub sale_unit: String,
    pub sale_specification: String,
    pub unit_sale_factor: String,
    pub product_sale_unit: String,
    pub product_unit_sale_factor: String,
    pub factory_name: String,
    pub product_name: String,
    pub amount: String,
    pub price_pur: String,
    pub price_sale: String,
    pub purchase_total: String,
    pub retail_total: String,
    pub batch_code: String,
    pub effective_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27Organization {
    pub id: String,
    pub name: String,
    pub parent_id: String,
    pub organization_type: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27InventoryLocationDefinition {
    pub source_kind: String,
    pub source_location_key: String,
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub category: String,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Phis27InventoryReferenceCatalog {
    pub organizations: Vec<Phis27Organization>,
    pub locations: Vec<Phis27InventoryLocationDefinition>,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone)]
struct InventoryStockGroup {
    source_kind: String,
    source_location_key: String,
    source_location_name: String,
    organization_id: String,
    source_option_count: usize,
    source_key: String,
    stock_rows: usize,
}

#[derive(Debug, Clone)]
struct LegacyPhysicalColumn {
    table: String,
    column: String,
    data_type: String,
    comment: String,
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
    let physical_columns = load_physical_columns(&request.connection, &schema);
    let query = medicine_query_with_physical_columns(&schema, scope, &physical_columns);
    let mut preview =
        odbc::preview_source(&request.connection, &query, request.limit.clamp(1, 10_000))?;
    preview.column_metadata =
        phis27_column_metadata(&request.connection, &schema, &physical_columns);
    Ok(preview)
}

pub fn inspect_inventory(
    store: &LocalStore,
    tenant_id: &str,
    request: &InspectPhis27InventoryRequest,
) -> Result<Phis27InventoryReadiness, String> {
    ensure_oracle(&request.connection)?;
    let source_name = request.source_name.trim();
    if source_name.is_empty() {
        return Err("缺少二系列phis数据库身份，无法核对基础药品迁移台账".into());
    }
    let schema = source_schema(&request.connection)?;
    let inspection = inspect(&request.connection)?;
    let has_warehouse_stock = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YK_KCMX");
    let has_pharmacy_stock = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YF_KCMX");
    if !has_warehouse_stock && !has_pharmacy_stock {
        return Err("当前老库无法读取 YK_KCMX 或 YF_KCMX，不能检查机构库存".into());
    }
    let query = inventory_group_query(
        &schema,
        has_warehouse_stock,
        has_pharmacy_stock,
        inspection
            .checked_tables
            .iter()
            .any(|item| item == "YK_YKLB"),
        inspection
            .checked_tables
            .iter()
            .any(|item| item == "YF_YFLB"),
    );
    let preview = odbc::preview_source(&request.connection, &query, 10_000)
        .map_err(|error| format!("读取二系列phis非零库存失败：{error}"))?;
    if preview.truncated {
        return Err("库存分组超过 10,000 条，请先按机构拆分后再执行库存初始化".into());
    }
    let groups = preview
        .rows
        .iter()
        .map(inventory_group_from_row)
        .collect::<Result<Vec<_>, _>>()?;
    summarize_inventory_readiness(store, tenant_id, source_name, &schema, groups)
}

pub fn load_inventory_reference_catalog(
    profile: &ConnectionProfile,
) -> Result<Phis27InventoryReferenceCatalog, String> {
    ensure_oracle(profile)?;
    let schema = source_schema(profile)?;
    let inspection = inspect(profile)?;
    if !inspection
        .checked_tables
        .iter()
        .any(|table| table == "SYS_ORGANIZATION")
    {
        return Err("老系统无法读取 SYS_ORGANIZATION，不能建立机构对应关系".into());
    }
    let organization_sql = format!(
        "SELECT CAST(ORGANIZCODE AS NVARCHAR2(128)) AS ORGANIZATION_ID,\
         CAST(ORGANIZNAME AS NVARCHAR2(200)) AS ORGANIZATION_NAME,\
         CAST(PARENTID AS NVARCHAR2(128)) AS PARENT_ID,\
         CAST(ORGANIZTYPE AS NVARCHAR2(64)) AS ORGANIZATION_TYPE,\
         CASE WHEN NVL(CAST(LOGOFF AS NVARCHAR2(8)),N'0') IN (N'0',N'false',N'FALSE') THEN N'1' ELSE N'0' END AS ACTIVE_FLAG \
         FROM {} ORDER BY ORGANIZCODE",
        table_name(&schema, "SYS_ORGANIZATION")
    );
    let organization_rows = odbc::preview_source(profile, &organization_sql, 5_000)
        .map_err(|error| format!("读取老系统机构 SYS_ORGANIZATION 失败：{error}"))?;
    if organization_rows.truncated {
        return Err("老系统机构超过 5,000 条，请先缩小数据库范围".into());
    }
    let text = |row: &Map<String, Value>, key: &str| {
        row.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let mut organizations = organization_rows
        .rows
        .iter()
        .filter_map(|row| {
            let id = text(row, "ORGANIZATION_ID");
            (!id.is_empty()).then(|| Phis27Organization {
                id,
                name: text(row, "ORGANIZATION_NAME"),
                parent_id: text(row, "PARENT_ID"),
                organization_type: text(row, "ORGANIZATION_TYPE"),
                active: text(row, "ACTIVE_FLAG") == "1",
            })
        })
        .collect::<Vec<_>>();
    let mut locations = Vec::new();
    let mut warnings = Vec::new();
    if inspection
        .checked_tables
        .iter()
        .any(|table| table == "YK_YKLB")
    {
        let sql = format!(
            "SELECT N'WAREHOUSE' AS SOURCE_KIND,N'YK:'||TO_NCHAR(YKSB) AS LOCATION_KEY,\
             TO_NCHAR(YKSB) AS LOCATION_ID,CAST(YKMC AS NVARCHAR2(200)) AS LOCATION_NAME,\
             CAST(JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,\
             NVL(TO_NCHAR(YKLB),N'') AS CATEGORY,N'1' AS ACTIVE_FLAG \
             FROM {} ORDER BY JGID,YKSB",
            table_name(&schema, "YK_YKLB")
        );
        let rows = odbc::preview_source(profile, &sql, 5_000)
            .map_err(|error| format!("读取老系统药库 YK_YKLB 失败：{error}"))?;
        if rows.truncated {
            return Err("老系统药库超过 5,000 条，请先缩小数据库范围".into());
        }
        locations.extend(
            rows.rows
                .iter()
                .map(|row| Phis27InventoryLocationDefinition {
                    source_kind: text(row, "SOURCE_KIND"),
                    source_location_key: text(row, "LOCATION_KEY"),
                    id: text(row, "LOCATION_ID"),
                    name: text(row, "LOCATION_NAME"),
                    organization_id: text(row, "ORGANIZATION_ID"),
                    category: text(row, "CATEGORY"),
                    active: text(row, "ACTIVE_FLAG") == "1",
                }),
        );
    } else {
        warnings.push("老系统未读取到 YK_YKLB，药库只能按库存所属机构人工确认".into());
    }
    if inspection
        .checked_tables
        .iter()
        .any(|table| table == "YF_YFLB")
    {
        let sql = format!(
            "SELECT N'PHARMACY' AS SOURCE_KIND,N'YF:'||TO_NCHAR(YFSB) AS LOCATION_KEY,\
             TO_NCHAR(YFSB) AS LOCATION_ID,CAST(YFMC AS NVARCHAR2(200)) AS LOCATION_NAME,\
             CAST(JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,N'' AS CATEGORY,\
             CASE WHEN NVL(TO_NCHAR(ZXBZ),N'0')=N'0' THEN N'1' ELSE N'0' END AS ACTIVE_FLAG \
             FROM {} ORDER BY JGID,YFSB",
            table_name(&schema, "YF_YFLB")
        );
        let rows = odbc::preview_source(profile, &sql, 5_000)
            .map_err(|error| format!("读取老系统药房 YF_YFLB 失败：{error}"))?;
        if rows.truncated {
            return Err("老系统药房超过 5,000 条，请先缩小数据库范围".into());
        }
        locations.extend(
            rows.rows
                .iter()
                .map(|row| Phis27InventoryLocationDefinition {
                    source_kind: text(row, "SOURCE_KIND"),
                    source_location_key: text(row, "LOCATION_KEY"),
                    id: text(row, "LOCATION_ID"),
                    name: text(row, "LOCATION_NAME"),
                    organization_id: text(row, "ORGANIZATION_ID"),
                    category: text(row, "CATEGORY"),
                    active: text(row, "ACTIVE_FLAG") == "1",
                }),
        );
    } else {
        warnings.push("老系统未读取到 YF_YFLB，无法建立药房对应关系".into());
    }
    locations.sort_by(|left, right| {
        left.organization_id
            .cmp(&right.organization_id)
            .then(left.source_kind.cmp(&right.source_kind))
            .then(left.name.cmp(&right.name))
    });
    let known_organization_ids = organizations
        .iter()
        .map(|organization| organization.id.clone())
        .collect::<HashSet<_>>();
    let missing_organization_ids = locations
        .iter()
        .map(|location| location.organization_id.clone())
        .filter(|id| !id.is_empty() && !known_organization_ids.contains(id))
        .collect::<BTreeSet<_>>();
    if !missing_organization_ids.is_empty() {
        warnings.push(format!(
            "SYS_ORGANIZATION 缺少 {} 个药库/药房所属机构，已保留机构编码供人工对应",
            missing_organization_ids.len()
        ));
        organizations.extend(
            missing_organization_ids
                .into_iter()
                .map(|id| Phis27Organization {
                    name: format!("机构 {id}"),
                    id,
                    parent_id: String::new(),
                    organization_type: String::new(),
                    active: true,
                }),
        );
    }
    organizations.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(Phis27InventoryReferenceCatalog {
        message: format!(
            "已读取 {} 个老系统机构、{} 个药库/药房",
            organizations.len(),
            locations.len()
        ),
        organizations,
        locations,
        warnings,
    })
}

pub fn load_inventory_stock_items(
    profile: &ConnectionProfile,
) -> Result<Vec<Phis27InventoryStockItem>, String> {
    ensure_oracle(profile)?;
    let schema = source_schema(profile)?;
    let inspection = inspect(profile)?;
    let has_warehouse_stock = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YK_KCMX");
    let has_pharmacy_stock = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YF_KCMX");
    if !has_warehouse_stock && !has_pharmacy_stock {
        return Err("老系统未发现可读取的 YK_KCMX 或 YF_KCMX 库存明细表".into());
    }
    let query = inventory_detail_query(
        &schema,
        has_warehouse_stock,
        has_pharmacy_stock,
        inspection
            .checked_tables
            .iter()
            .any(|item| item == "YK_YKLB"),
        inspection
            .checked_tables
            .iter()
            .any(|item| item == "YF_YFLB"),
    );
    let preview = odbc::preview_source(profile, &query, 10_000)
        .map_err(|error| format!("读取二系列phis库存明细失败：{error}"))?;
    if preview.truncated {
        return Err("非零库存明细超过 10,000 条，请先按机构拆分后再迁移".into());
    }
    preview
        .rows
        .iter()
        .map(inventory_stock_item_from_row)
        .collect()
}

fn inventory_detail_query(
    schema: &str,
    has_warehouse_stock: bool,
    has_pharmacy_stock: bool,
    has_warehouse_list: bool,
    has_pharmacy_list: bool,
) -> String {
    let mut queries = Vec::new();
    if has_warehouse_stock {
        let location_join = if has_warehouse_list {
            format!(
                "LEFT JOIN (SELECT JGID,COUNT(*) AS OPTION_COUNT,MAX(TO_NCHAR(YKSB)) AS LOCATION_ID,MAX(CAST(YKMC AS NVARCHAR2(200))) AS LOCATION_NAME FROM {} GROUP BY JGID) l ON l.JGID=k.JGID",
                table_name(schema, "YK_YKLB")
            )
        } else {
            String::new()
        };
        let location_name = if has_warehouse_list {
            "CASE WHEN l.OPTION_COUNT=1 THEN l.LOCATION_NAME ELSE N'药库库存总账' END"
        } else {
            "N'药库库存总账'"
        };
        let location_key = if has_warehouse_list {
            "CASE WHEN l.OPTION_COUNT=1 THEN N'YK:'||l.LOCATION_ID ELSE N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128)) END"
        } else {
            "N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128))"
        };
        queries.push(format!(
            "SELECT N'WAREHOUSE' AS SOURCE_KIND,CAST(k.SBXH AS NVARCHAR2(64)) AS SOURCE_RECORD_ID,\
             {location_key} AS LOCATION_KEY,{location_name} AS LOCATION_NAME,\
             CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,\
             CAST(t.YPMC AS NVARCHAR2(200)) AS DRUG_NAME,CAST(t.YPGG AS NVARCHAR2(200)) AS SPECIFICATION,\
             CAST(t.YPSX AS NVARCHAR2(80)) AS DOSAGE_FORM,CAST(t.ZXDW AS NVARCHAR2(80)) AS MINIMUM_UNIT,\
             CAST(t.YPDW AS NVARCHAR2(80)) AS SALE_UNIT,CAST(t.YPGG AS NVARCHAR2(200)) AS SALE_SPECIFICATION,\
             CAST(t.ZXBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR,CAST(t.YFDW AS NVARCHAR2(80)) AS PRODUCT_SALE_UNIT,\
             CAST(t.YFBZ AS NVARCHAR2(80)) AS PRODUCT_UNIT_SALE_FACTOR,CAST(COALESCE(f.CDQC,f.CDMC) AS NVARCHAR2(300)) AS FACTORY_NAME,\
             CAST(NVL(p.YBSPMC,t.YPMC) AS NVARCHAR2(200)) AS PRODUCT_NAME,\
             CAST(k.KCSL AS NVARCHAR2(80)) AS AMOUNT,CAST(k.JHJG AS NVARCHAR2(80)) AS PRICE_PUR,CAST(k.LSJG AS NVARCHAR2(80)) AS PRICE_SALE,\
             CAST(k.JHJE AS NVARCHAR2(80)) AS PURCHASE_TOTAL,CAST(k.LSJE AS NVARCHAR2(80)) AS RETAIL_TOTAL,\
             CAST(k.YPPH AS NVARCHAR2(200)) AS BATCH_CODE,TO_NCHAR(k.YPXQ,'YYYY-MM-DD') AS EFFECTIVE_DATE \
             FROM {} k INNER JOIN {} t ON t.YPXH=k.YPXH \
             LEFT JOIN {} p ON p.YPXH=k.YPXH AND p.YPCD=k.YPCD \
             LEFT JOIN {} f ON f.YPCD=k.YPCD {location_join} WHERE NVL(k.KCSL,0)<>0",
            table_name(schema, "YK_KCMX"),
            table_name(schema, "YK_TYPK"),
            table_name(schema, "YK_YPCD"),
            table_name(schema, "YK_CDDZ")
        ));
    }
    if has_pharmacy_stock {
        let location_join = if has_pharmacy_list {
            format!(
                "LEFT JOIN {} l ON l.YFSB=k.YFSB",
                table_name(schema, "YF_YFLB")
            )
        } else {
            String::new()
        };
        let location_name = if has_pharmacy_list {
            "NVL(CAST(l.YFMC AS NVARCHAR2(200)),N'未命名药房')"
        } else {
            "N'未命名药房'"
        };
        queries.push(format!(
            "SELECT N'PHARMACY' AS SOURCE_KIND,CAST(k.SBXH AS NVARCHAR2(64)) AS SOURCE_RECORD_ID,\
             N'YF:'||CAST(k.YFSB AS NVARCHAR2(128)) AS LOCATION_KEY,{location_name} AS LOCATION_NAME,\
             CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,\
             CAST(t.YPMC AS NVARCHAR2(200)) AS DRUG_NAME,CAST(t.YPGG AS NVARCHAR2(200)) AS SPECIFICATION,\
             CAST(t.YPSX AS NVARCHAR2(80)) AS DOSAGE_FORM,CAST(t.ZXDW AS NVARCHAR2(80)) AS MINIMUM_UNIT,\
             CAST(y.YFDW AS NVARCHAR2(80)) AS SALE_UNIT,CAST(y.YFGG AS NVARCHAR2(200)) AS SALE_SPECIFICATION,\
             CAST(y.YFBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR,CAST(t.YFDW AS NVARCHAR2(80)) AS PRODUCT_SALE_UNIT,\
             CAST(t.YFBZ AS NVARCHAR2(80)) AS PRODUCT_UNIT_SALE_FACTOR,CAST(COALESCE(f.CDQC,f.CDMC) AS NVARCHAR2(300)) AS FACTORY_NAME,\
             CAST(NVL(p.YBSPMC,t.YPMC) AS NVARCHAR2(200)) AS PRODUCT_NAME,\
             CAST(k.YPSL AS NVARCHAR2(80)) AS AMOUNT,CAST(k.JHJG AS NVARCHAR2(80)) AS PRICE_PUR,CAST(k.LSJG AS NVARCHAR2(80)) AS PRICE_SALE,\
             CAST(k.JHJE AS NVARCHAR2(80)) AS PURCHASE_TOTAL,CAST(k.LSJE AS NVARCHAR2(80)) AS RETAIL_TOTAL,\
             CAST(k.YPPH AS NVARCHAR2(200)) AS BATCH_CODE,TO_NCHAR(k.YPXQ,'YYYY-MM-DD') AS EFFECTIVE_DATE \
             FROM {} k INNER JOIN {} t ON t.YPXH=k.YPXH \
             LEFT JOIN {} y ON y.JGID=k.JGID AND y.YFSB=k.YFSB AND y.YPXH=k.YPXH \
             LEFT JOIN {} p ON p.YPXH=k.YPXH AND p.YPCD=k.YPCD \
             LEFT JOIN {} f ON f.YPCD=k.YPCD {location_join} WHERE NVL(k.YPSL,0)<>0",
            table_name(schema, "YF_KCMX"),
            table_name(schema, "YK_TYPK"),
            table_name(schema, "YF_YPXX"),
            table_name(schema, "YK_YPCD"),
            table_name(schema, "YK_CDDZ")
        ));
    }
    format!(
        "SELECT SOURCE_KIND,SOURCE_RECORD_ID,LOCATION_KEY,LOCATION_NAME,ORGANIZATION_ID,SOURCE_KEY,\
         DRUG_NAME,SPECIFICATION,DOSAGE_FORM,MINIMUM_UNIT,SALE_UNIT,SALE_SPECIFICATION,UNIT_SALE_FACTOR,\
         PRODUCT_SALE_UNIT,PRODUCT_UNIT_SALE_FACTOR,FACTORY_NAME,PRODUCT_NAME,\
         AMOUNT,PRICE_PUR,PRICE_SALE,PURCHASE_TOTAL,RETAIL_TOTAL,BATCH_CODE,EFFECTIVE_DATE FROM ({}) \
         ORDER BY SOURCE_KIND,LOCATION_KEY,SOURCE_KEY,SOURCE_RECORD_ID",
        queries.join(" UNION ALL ")
    )
}

fn inventory_stock_item_from_row(
    row: &Map<String, Value>,
) -> Result<Phis27InventoryStockItem, String> {
    let text = |key: &str| {
        row.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let item = Phis27InventoryStockItem {
        source_kind: text("SOURCE_KIND"),
        source_record_id: text("SOURCE_RECORD_ID"),
        source_location_key: text("LOCATION_KEY"),
        source_location_name: text("LOCATION_NAME"),
        source_organization_id: text("ORGANIZATION_ID"),
        source_product_key: text("SOURCE_KEY"),
        drug_name: text("DRUG_NAME"),
        specification: text("SPECIFICATION"),
        dosage_form: text("DOSAGE_FORM"),
        minimum_unit: text("MINIMUM_UNIT"),
        sale_unit: text("SALE_UNIT"),
        sale_specification: text("SALE_SPECIFICATION"),
        unit_sale_factor: text("UNIT_SALE_FACTOR"),
        product_sale_unit: text("PRODUCT_SALE_UNIT"),
        product_unit_sale_factor: text("PRODUCT_UNIT_SALE_FACTOR"),
        factory_name: text("FACTORY_NAME"),
        product_name: text("PRODUCT_NAME"),
        amount: text("AMOUNT"),
        price_pur: text("PRICE_PUR"),
        price_sale: text("PRICE_SALE"),
        purchase_total: text("PURCHASE_TOTAL"),
        retail_total: text("RETAIL_TOTAL"),
        batch_code: text("BATCH_CODE"),
        effective_date: text("EFFECTIVE_DATE"),
    };
    if item.source_record_id.is_empty()
        || item.source_location_key.is_empty()
        || item.source_product_key.is_empty()
    {
        return Err("库存明细缺少 SBXH、库房或 YPXH:YPCD 来源键".into());
    }
    Ok(item)
}

fn inventory_group_query(
    schema: &str,
    has_warehouse_stock: bool,
    has_pharmacy_stock: bool,
    has_warehouse_list: bool,
    has_pharmacy_list: bool,
) -> String {
    let mut queries = Vec::new();
    if has_warehouse_stock {
        let location_join = if has_warehouse_list {
            format!(
                "LEFT JOIN (SELECT JGID,COUNT(*) AS OPTION_COUNT,MAX(TO_NCHAR(YKSB)) AS LOCATION_ID,MAX(CAST(YKMC AS NVARCHAR2(200))) AS LOCATION_NAME FROM {} GROUP BY JGID) l ON l.JGID=k.JGID",
                table_name(schema, "YK_YKLB")
            )
        } else {
            String::new()
        };
        let (option_count, location_name) = if has_warehouse_list {
            (
                "NVL(l.OPTION_COUNT,0)",
                "CASE WHEN l.OPTION_COUNT=1 THEN l.LOCATION_NAME ELSE N'药库库存总账' END",
            )
        } else {
            ("0", "N'药库库存总账'")
        };
        let location_key = if has_warehouse_list {
            "CASE WHEN l.OPTION_COUNT=1 THEN N'YK:'||l.LOCATION_ID ELSE N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128)) END"
        } else {
            "N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128))"
        };
        queries.push(format!(
            "SELECT N'WAREHOUSE' AS SOURCE_KIND,{location_key} AS LOCATION_KEY,\
             {location_name} AS LOCATION_NAME,CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,\
             {option_count} AS OPTION_COUNT,CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,\
             COUNT(*) AS STOCK_ROWS FROM {} k {location_join} \
             WHERE NVL(k.KCSL,0)<>0 GROUP BY k.JGID,{location_key},{location_name},{option_count},k.YPXH,k.YPCD",
            table_name(schema, "YK_KCMX")
        ));
    }
    if has_pharmacy_stock {
        let location_join = if has_pharmacy_list {
            format!(
                "LEFT JOIN {} l ON l.YFSB=k.YFSB",
                table_name(schema, "YF_YFLB")
            )
        } else {
            String::new()
        };
        let location_name = if has_pharmacy_list {
            "NVL(CAST(l.YFMC AS NVARCHAR2(200)),N'未命名药房')"
        } else {
            "N'未命名药房'"
        };
        queries.push(format!(
            "SELECT N'PHARMACY' AS SOURCE_KIND,N'YF:'||CAST(k.YFSB AS NVARCHAR2(128)) AS LOCATION_KEY,\
             {location_name} AS LOCATION_NAME,CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,1 AS OPTION_COUNT,\
             CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,COUNT(*) AS STOCK_ROWS \
             FROM {} k {location_join} WHERE NVL(k.YPSL,0)<>0 \
             GROUP BY k.YFSB,{location_name},k.JGID,k.YPXH,k.YPCD",
            table_name(schema, "YF_KCMX")
        ));
    }
    format!(
        "SELECT SOURCE_KIND,LOCATION_KEY,LOCATION_NAME,ORGANIZATION_ID,OPTION_COUNT,SOURCE_KEY,STOCK_ROWS \
         FROM ({}) ORDER BY SOURCE_KIND,LOCATION_KEY,SOURCE_KEY",
        queries.join(" UNION ALL ")
    )
}

fn inventory_group_from_row(row: &Map<String, Value>) -> Result<InventoryStockGroup, String> {
    let text = |key: &str| {
        row.get(key)
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let number = |key: &str| {
        text(key)
            .parse::<usize>()
            .map_err(|_| format!("库存统计字段 {key} 无法识别"))
    };
    let source_key = text("SOURCE_KEY");
    if source_key.is_empty() {
        return Err("库存记录缺少 YPXH:YPCD 药品来源键".into());
    }
    Ok(InventoryStockGroup {
        source_kind: text("SOURCE_KIND"),
        source_location_key: text("LOCATION_KEY"),
        source_location_name: text("LOCATION_NAME"),
        organization_id: text("ORGANIZATION_ID"),
        source_option_count: number("OPTION_COUNT")?,
        source_key,
        stock_rows: number("STOCK_ROWS")?,
    })
}

fn summarize_inventory_readiness(
    store: &LocalStore,
    tenant_id: &str,
    source_name: &str,
    schema: &str,
    groups: Vec<InventoryStockGroup>,
) -> Result<Phis27InventoryReadiness, String> {
    let medicine_keys = groups
        .iter()
        .map(|group| group.source_key.clone())
        .collect::<HashSet<_>>();
    let mut unresolved = Vec::new();
    for source_key in &medicine_keys {
        let linked = store
            .find_source_link(tenant_id, "PHIS27", source_name, source_key)?
            .is_some_and(|link| !link.id_med.is_empty() && !link.id_med_pro.is_empty());
        if !linked {
            unresolved.push(source_key.clone());
        }
    }
    unresolved.sort();
    let mut locations = BTreeMap::<String, Vec<&InventoryStockGroup>>::new();
    for group in &groups {
        locations
            .entry(group.source_location_key.clone())
            .or_default()
            .push(group);
    }
    let locations = locations
        .into_values()
        .filter_map(|items| {
            let first = items.first().copied()?;
            let keys = items
                .iter()
                .map(|item| item.source_key.as_str())
                .collect::<HashSet<_>>();
            let (mapping_status, mapping_message) =
                if first.source_kind == "WAREHOUSE" && first.source_option_count > 1 {
                    (
                        "SOURCE_LOCATION_AMBIGUOUS",
                        format!(
                            "YK_KCMX 未保存药库主键；该机构有 {} 个药库，需人工指定目标库房",
                            first.source_option_count
                        ),
                    )
                } else if first.source_option_count == 0 {
                    (
                        "SOURCE_LOCATION_MISSING",
                        "未读取到对应的老系统库房定义，需人工指定目标库房".into(),
                    )
                } else {
                    (
                        "PENDING_TARGET_MAPPING",
                        "老系统位置已识别，下一步选择对应的新系统库房".into(),
                    )
                };
            Some(Phis27InventoryLocation {
                source_kind: first.source_kind.clone(),
                source_location_key: first.source_location_key.clone(),
                source_location_name: first.source_location_name.clone(),
                organization_id: first.organization_id.clone(),
                source_option_count: first.source_option_count,
                stock_row_count: items.iter().map(|item| item.stock_rows).sum(),
                stock_group_count: items.len(),
                medicine_count: keys.len(),
                mapping_status: mapping_status.into(),
                mapping_message,
            })
        })
        .collect::<Vec<_>>();
    let mut warnings = Vec::new();
    if !unresolved.is_empty() {
        warnings.push(format!(
            "{} 个库存药品没有找到完整的 id_med/id_med_pro 基础数据台账；需先补迁药品基础数据",
            unresolved.len()
        ));
    }
    let ambiguous_locations = locations
        .iter()
        .filter(|location| location.mapping_status == "SOURCE_LOCATION_AMBIGUOUS")
        .count();
    if ambiguous_locations > 0 {
        warnings.push(format!(
            "{ambiguous_locations} 个药库库存范围无法从 YK_KCMX 直接还原药库主键，配置目标库房时需人工确认"
        ));
    }
    if groups.is_empty() {
        warnings.push("YK_KCMX/YF_KCMX 中没有非零库存，无需初始化".into());
    }
    let medicine_count = medicine_keys.len();
    let unresolved_medicine_count = unresolved.len();
    let ready_for_location_mapping = !groups.is_empty() && unresolved_medicine_count == 0;
    Ok(Phis27InventoryReadiness {
        ready_for_location_mapping,
        schema: schema.into(),
        source_name: source_name.into(),
        stock_row_count: groups.iter().map(|group| group.stock_rows).sum(),
        stock_group_count: groups.len(),
        medicine_count,
        mapped_medicine_count: medicine_count.saturating_sub(unresolved_medicine_count),
        unresolved_medicine_count,
        locations,
        unresolved_source_keys: unresolved.into_iter().take(20).collect(),
        warnings,
        message: if ready_for_location_mapping {
            "库存药品已全部关联到本次核实的基础数据台账，可以进入库房映射配置".into()
        } else if groups.is_empty() {
            "没有需要初始化的非零库存记录".into()
        } else {
            "库存读取完成，但仍有药品未关联到基础数据台账".into()
        },
    })
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
            title: "二系列phis药品数据".into(),
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
            message: "未识别为完整的二系列phis药品数据源".into(),
        });
    }

    let typk = table_name(schema, "YK_TYPK");
    let ypcd = table_name(schema, "YK_YPCD");
    let cdxx = table_name(schema, "YK_CDXX");
    let yk_kcmx = table_name(schema, "YK_KCMX");
    let yf_kcmx = table_name(schema, "YF_KCMX");

    let total_medicines = count(connection, &format!("SELECT COUNT(*) FROM {typk}"))?;
    let configured_medicines = count(
        connection,
        &format!(
            "SELECT COUNT(DISTINCT t.YPXH) FROM {typk} t WHERE EXISTS (SELECT 1 FROM {cdxx} c WHERE c.YPXH=t.YPXH)"
        ),
    )?;
    let active_configured_medicines = count(
        connection,
        &format!(
            "SELECT COUNT(DISTINCT t.YPXH) FROM {typk} t WHERE EXISTS (SELECT 1 FROM {cdxx} c WHERE c.YPXH=t.YPXH AND NVL(c.ZFPB,0)=0)"
        ),
    )?;
    let product_rows = count(connection, &format!("SELECT COUNT(*) FROM {ypcd}"))?;
    let active_rows = count(connection, &scope_count_query(schema, Scope::UsedActive))?;
    let configured_rows = count(connection, &scope_count_query(schema, Scope::UsedAll))?;
    let all_medicine_rows = count(connection, &scope_count_query(schema, Scope::AllMedicines))?;
    let duplicate_business_groups = count(
        connection,
        &format!(
            "SELECT COUNT(*) FROM (SELECT t.YPMC,t.YPGG,t.ZXDW FROM {typk} t WHERE EXISTS (SELECT 1 FROM {cdxx} c WHERE c.YPXH=t.YPXH) GROUP BY t.YPMC,t.YPGG,t.ZXDW HAVING COUNT(*)>1)"
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
    let orphan_pharmacy_configs = 0;

    let mut warnings = Vec::new();
    if duplicate_business_groups > 0 {
        warnings.push(format!(
            "YK_CDXX 机构配置范围发现{duplicate_business_groups}组名称、规格、单位一致的药品；迁移时会自动复用同一新药品，并分别保留每个 YPXH:YPCD 的来源映射"
        ));
    }
    let unavailable_dictionaries = DYNAMIC_DICTIONARY_TABLES
        .iter()
        .filter(|(table, _)| !checked_tables.iter().any(|checked| checked == table))
        .map(|(table, label)| format!("{label}（{table}）"))
        .collect::<Vec<_>>();
    if !unavailable_dictionaries.is_empty() {
        warnings.push(format!(
            "以下动态字典表当前不可读：{}；相关编码仍可迁移，但需人工确认字典含义",
            unavailable_dictionaries.join("、")
        ));
    }
    warnings.push("费用归并主键和剂型编码属于新系统字典，需在校验阶段设置默认值或值映射".into());

    Ok(Phis27Inspection {
        detected: true,
        adapter_id: "PHIS27".into(),
        title: "二系列phis药品数据".into(),
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
                description:
                    "仅以 YK_CDXX 中未作废的机构产地配置判断在用；不关联药库或药房药品配置表".into(),
                estimated_rows: active_rows,
                medicine_count: active_configured_medicines,
                recommended: true,
            },
            Phis27Scope {
                id: "USED_ALL".into(),
                label: "机构全部配置药品".into(),
                description: "包含 YK_CDXX 中曾配置的全部药品及产地，不关联 YK_YPXX、YF_YPXX"
                    .into(),
                estimated_rows: configured_rows,
                medicine_count: configured_medicines,
                recommended: false,
            },
            Phis27Scope {
                id: "ALL_MEDICINES".into(),
                label: "全部通用药品".into(),
                description: "读取药品主表中的全部通用药品，不受机构配置和作废状态限制".into(),
                estimated_rows: all_medicine_rows,
                medicine_count: total_medicines,
                recommended: false,
            },
        ],
        warnings,
        message: "已识别二系列phis药品主数据结构，可使用内置安全查询模板".into(),
    })
}

#[derive(Debug, Clone, Copy)]
enum Scope {
    UsedActive,
    UsedAll,
    AllMedicines,
}

fn normalize_scope(scope: &str) -> Result<Scope, String> {
    match scope.trim().to_ascii_uppercase().as_str() {
        "USED_ACTIVE" => Ok(Scope::UsedActive),
        "USED_ALL" => Ok(Scope::UsedAll),
        "ALL_MEDICINES" => Ok(Scope::AllMedicines),
        _ => Err("不支持的二系列phis药品同步范围".into()),
    }
}

fn scope_predicate(scope: Scope) -> &'static str {
    match scope {
        Scope::UsedActive => {
            "EXISTS (SELECT 1 FROM {YK_CDXX} c WHERE c.YPXH=t.YPXH AND NVL(c.ZFPB,0)=0)"
        }
        Scope::UsedAll => "EXISTS (SELECT 1 FROM {YK_CDXX} c WHERE c.YPXH=t.YPXH)",
        Scope::AllMedicines => "1=1",
    }
}

fn expanded_scope_predicate(schema: &str, scope: Scope) -> String {
    scope_predicate(scope).replace("{YK_CDXX}", &table_name(schema, "YK_CDXX"))
}

fn scope_count_query(schema: &str, scope: Scope) -> String {
    let predicate = expanded_scope_predicate(schema, scope);
    format!(
        "SELECT COUNT(*) FROM {} t LEFT JOIN {} p ON p.YPXH=t.YPXH WHERE {predicate}",
        table_name(schema, "YK_TYPK"),
        table_name(schema, "YK_YPCD")
    )
}

#[cfg(test)]
fn medicine_query(schema: &str, scope: Scope) -> String {
    medicine_query_with_physical_columns(schema, scope, &[])
}

fn medicine_query_with_physical_columns(
    schema: &str,
    scope: Scope,
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    let typk = table_name(schema, "YK_TYPK");
    let ypcd = table_name(schema, "YK_YPCD");
    let cddz = table_name(schema, "YK_CDDZ");
    let predicate = expanded_scope_predicate(schema, scope);
    let additional_columns = additional_physical_columns(physical_columns)
        .into_iter()
        .map(|column| {
            format!(
                ",\n            {}.{} AS {}",
                table_query_alias(&column.table),
                column.column,
                physical_result_alias(&column.table, &column.column)
            )
        })
        .collect::<String>();
    format!(
        r#"WITH selected_med AS (
            SELECT t.*,
                   COUNT(*) OVER (PARTITION BY t.YPMC,t.YPGG,t.ZXDW) AS SOURCE_DUPLICATE_COUNT
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
            t.ZXDW AS PRE_UNIT,
            t.YPJL AS DOSE,
            t.JLDW AS DOSE_UNIT,
            t.YPGG AS SPEC,
            TO_CHAR(t.GYFF) AS USAGE_CODE,
            TO_CHAR(t.MRYF) AS FREQ_CODE,
            t.YCJL AS DOSE_ONCE,
            TO_CHAR(t.QZCL) AS ROUND_CODE,
            TO_CHAR(t.FYFS) AS DISPENSE_CODE,
            TO_CHAR(p.YPCD) AS SOURCE_FACTORY_ID,
            TO_CHAR(p.YPLSH) AS SOURCE_PRODUCT_ID,
            CASE WHEN p.YPCD IS NOT NULL THEN TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS SOURCE_MED_PRO_KEY,
            CASE WHEN p.YPCD IS NOT NULL THEN COALESCE(f.CDQC,f.CDMC) END AS FACTORY_NAME,
            CASE WHEN p.YPCD IS NOT NULL THEN f.CDMC END AS FACTORY_SHORT_NAME,
            CASE WHEN p.YPCD IS NOT NULL THEN f.PYDM END AS FACTORY_PINYIN,
            CASE WHEN p.YPCD IS NOT NULL THEN NVL(p.YBSPMC,t.YPMC) END AS PRODUCT_NAME,
            CASE WHEN p.YPCD IS NOT NULL THEN t.YFDW END AS SALE_UNIT,
            CASE WHEN p.YPCD IS NOT NULL THEN t.YFBZ END AS PACK_FACTOR,
            CASE WHEN p.YPCD IS NOT NULL THEN NVL(t.YFGG,t.YPGG) END AS SALE_SPEC,
            p.JHJG AS BUY_PRICE,
            p.LSJG AS RETAIL_PRICE,
            p.PZWH AS APPROVAL_NO,
            p.YPTM AS BARCODE,
            CASE WHEN p.YPCD IS NOT NULL THEN TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS CD_MED_PRO,
            TO_CHAR(t.CFYP) AS RX_FLAG,
            TO_CHAR(t.JYLX) AS BASIC_DRUG_TYPE,
            TO_CHAR(t.YBFL) AS INSURANCE_LEVEL,
            TO_CHAR(t.YPDC) AS ORIGIN_TYPE,
            TO_CHAR(t.YPZC) AS STORAGE_CODE,
            TO_CHAR(t.TSYP) AS SPECIAL_DRUG_TYPE,
            TO_CHAR(t.GMYWLB) AS ALLERGY_CODE,
            TO_CHAR(t.SFSP) AS ANTI_APPROVAL,
            TO_CHAR(t.KSBZ) AS ANTIBIOTIC_FLAG,
            t.YCYL AS DAILY_LIMIT,
            TO_CHAR(t.ZFPB) AS SOURCE_STOP_FLAG{additional_columns}
        FROM selected_med t
        LEFT JOIN {ypcd} p ON p.YPXH=t.YPXH
        LEFT JOIN {cddz} f ON f.YPCD=p.YPCD
        ORDER BY t.YPXH,p.YPCD"#
    )
}

fn load_physical_columns(profile: &ConnectionProfile, schema: &str) -> Vec<LegacyPhysicalColumn> {
    let comment_query = format!(
        "SELECT c.TABLE_NAME,c.COLUMN_NAME,c.DATA_TYPE,m.COMMENTS \
         FROM ALL_TAB_COLUMNS c \
         LEFT JOIN ALL_COL_COMMENTS m ON m.OWNER=c.OWNER AND m.TABLE_NAME=c.TABLE_NAME AND m.COLUMN_NAME=c.COLUMN_NAME \
         WHERE c.OWNER='{schema}' AND c.TABLE_NAME IN ('YK_TYPK','YK_YPCD','YK_CDDZ') \
         ORDER BY DECODE(c.TABLE_NAME,'YK_TYPK',1,'YK_YPCD',2,3),c.COLUMN_ID"
    );
    odbc::preview_source(profile, &comment_query, 1_000)
        .ok()
        .map(|preview| {
            preview
                .rows
                .into_iter()
                .filter_map(|row| {
                    let table = row.get("TABLE_NAME")?.as_str()?.trim().to_string();
                    let column = row.get("COLUMN_NAME")?.as_str()?.trim().to_string();
                    let data_type = row.get("DATA_TYPE")?.as_str()?.trim().to_string();
                    let comment = row
                        .get("COMMENTS")
                        .and_then(|value| value.as_str())
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    Some(LegacyPhysicalColumn {
                        table,
                        column,
                        data_type,
                        comment,
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn phis27_column_metadata(
    profile: &ConnectionProfile,
    schema: &str,
    physical_columns: &[LegacyPhysicalColumn],
) -> Vec<SourceColumnMetadata> {
    let comments = physical_columns
        .iter()
        .map(|column| {
            (
                (column.table.clone(), column.column.clone()),
                column.comment.clone(),
            )
        })
        .collect::<HashMap<_, _>>();

    let mut metadata = phis27_column_definitions()
        .into_iter()
        .map(
            |(name, table, column, fallback_comment, mapping_eligible)| {
                let database_comment = comments
                    .get(&(table.to_string(), column.to_string()))
                    .filter(|comment| !comment.is_empty())
                    .cloned();
                SourceColumnMetadata {
                    name: name.into(),
                    comment: database_comment.unwrap_or_else(|| fallback_comment.into()),
                    source_table: table.into(),
                    source_column: column.into(),
                    mapping_eligible,
                    source_dictionary: phis27_source_dictionary(name, profile, schema),
                }
            },
        )
        .collect::<Vec<_>>();
    metadata.extend(
        additional_physical_columns(physical_columns)
            .into_iter()
            .map(|column| SourceColumnMetadata {
                name: physical_result_alias(&column.table, &column.column),
                comment: if column.comment.is_empty() {
                    column.column.clone()
                } else {
                    column.comment.clone()
                },
                source_table: column.table.clone(),
                source_column: column.column.clone(),
                mapping_eligible: true,
                source_dictionary: None,
            }),
    );
    metadata
}

fn additional_physical_columns(
    physical_columns: &[LegacyPhysicalColumn],
) -> Vec<&LegacyPhysicalColumn> {
    let represented = phis27_column_definitions()
        .into_iter()
        .filter(|(_, table, column, _, _)| !table.is_empty() && !column.is_empty())
        .map(|(_, table, column, _, _)| (table, column))
        .collect::<HashSet<_>>();
    physical_columns
        .iter()
        .filter(|column| {
            !represented.contains(&(column.table.as_str(), column.column.as_str()))
                && selectable_physical_column(column)
        })
        .collect()
}

fn selectable_physical_column(column: &LegacyPhysicalColumn) -> bool {
    let identifier_is_safe = !column.column.is_empty()
        && column.column.len() <= 24
        && column.column.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        });
    let scalar_type = !matches!(
        column.data_type.trim().to_ascii_uppercase().as_str(),
        "BLOB" | "CLOB" | "NCLOB" | "LONG" | "LONG RAW" | "RAW" | "BFILE" | "XMLTYPE"
    );
    let technical_tracking = matches!(
        column.column.as_str(),
        "YPXH" | "YPCD" | "YPLSH" | "XZSJ" | "XGSJ" | "CJSJ" | "ZHXGR" | "CJR" | "ZFPB"
    );
    identifier_is_safe
        && scalar_type
        && !technical_tracking
        && matches!(column.table.as_str(), "YK_TYPK" | "YK_YPCD" | "YK_CDDZ")
}

fn table_query_alias(table: &str) -> &'static str {
    match table {
        "YK_TYPK" => "t",
        "YK_YPCD" => "p",
        "YK_CDDZ" => "f",
        _ => unreachable!("physical column table is filtered before query construction"),
    }
}

fn physical_result_alias(table: &str, column: &str) -> String {
    let prefix = match table {
        "YK_TYPK" => "T",
        "YK_YPCD" => "P",
        "YK_CDDZ" => "F",
        _ => "S",
    };
    format!("{prefix}__{column}")
}

fn phis27_column_definitions() -> Vec<(&'static str, &'static str, &'static str, &'static str, bool)>
{
    vec![
        ("SOURCE_KEY", "", "", "内部追溯键（YPXH:YPCD）", false),
        (
            "SOURCE_MED_ID",
            "YK_TYPK",
            "YPXH",
            "通用药品来源序号",
            false,
        ),
        ("SOURCE_DUPLICATE_COUNT", "", "", "目标判重分组数量", false),
        ("DRUG_NAME", "YK_TYPK", "YPMC", "药品名称", true),
        ("DRUG_TYPE", "YK_TYPK", "TYPE", "药品类型", true),
        ("FORM_CODE", "YK_TYPK", "YPSX", "剂型编码", true),
        ("PRE_UNIT", "YK_TYPK", "ZXDW", "最小单位（制剂单位）", true),
        ("DOSE", "YK_TYPK", "YPJL", "制剂剂量", true),
        ("DOSE_UNIT", "YK_TYPK", "JLDW", "剂量单位", true),
        ("SPEC", "YK_TYPK", "YPGG", "制剂规格", true),
        ("USAGE_CODE", "YK_TYPK", "GYFF", "默认给药方法", true),
        ("FREQ_CODE", "YK_TYPK", "MRYF", "默认频次", true),
        ("DOSE_ONCE", "YK_TYPK", "YCJL", "默认一次剂量", true),
        ("ROUND_CODE", "YK_TYPK", "QZCL", "取整策略", true),
        ("DISPENSE_CODE", "YK_TYPK", "FYFS", "发药方式", true),
        (
            "SOURCE_FACTORY_ID",
            "YK_YPCD",
            "YPCD",
            "生产厂家来源序号",
            false,
        ),
        (
            "SOURCE_PRODUCT_ID",
            "YK_YPCD",
            "YPLSH",
            "商品流水号（仅审计）",
            false,
        ),
        (
            "SOURCE_MED_PRO_KEY",
            "",
            "",
            "商品追溯键（YPXH:YPCD）",
            false,
        ),
        (
            "FACTORY_NAME",
            "YK_CDDZ",
            "CDQC",
            "生产厂家全称（为空时使用 CDMC）",
            true,
        ),
        (
            "FACTORY_SHORT_NAME",
            "YK_CDDZ",
            "CDMC",
            "生产厂家简称",
            true,
        ),
        (
            "FACTORY_PINYIN",
            "YK_CDDZ",
            "PYDM",
            "生产厂家拼音代码",
            true,
        ),
        ("PRODUCT_NAME", "YK_YPCD", "YBSPMC", "商品名", true),
        ("SALE_UNIT", "YK_TYPK", "YFDW", "零售包装单位", true),
        ("PACK_FACTOR", "YK_TYPK", "YFBZ", "包装系数", true),
        ("SALE_SPEC", "YK_TYPK", "YFGG", "零售包装规格", true),
        ("BUY_PRICE", "YK_YPCD", "JHJG", "进货价格", true),
        ("RETAIL_PRICE", "YK_YPCD", "LSJG", "零售价格", true),
        ("APPROVAL_NO", "YK_YPCD", "PZWH", "批准文号", true),
        ("BARCODE", "YK_YPCD", "YPTM", "商品条形码", true),
        ("CD_MED_PRO", "", "YPXH:YPCD", "商品来源组合码", true),
        ("RX_FLAG", "YK_TYPK", "CFYP", "处方药标志", true),
        ("BASIC_DRUG_TYPE", "YK_TYPK", "JYLX", "基药类型", true),
        ("INSURANCE_LEVEL", "YK_TYPK", "YBFL", "医保分类", true),
        ("ORIGIN_TYPE", "YK_TYPK", "YPDC", "产地档次", true),
        ("STORAGE_CODE", "YK_TYPK", "YPZC", "药品贮藏", true),
        ("SPECIAL_DRUG_TYPE", "YK_TYPK", "TSYP", "特殊药品类型", true),
        ("ALLERGY_CODE", "YK_TYPK", "GMYWLB", "过敏药物类别", true),
        ("ANTI_APPROVAL", "YK_TYPK", "SFSP", "抗菌药物是否审批", true),
        ("ANTIBIOTIC_FLAG", "YK_TYPK", "KSBZ", "是否抗生素", true),
        ("DAILY_LIMIT", "YK_TYPK", "YCYL", "一日限量", true),
        ("SOURCE_STOP_FLAG", "YK_TYPK", "ZFPB", "来源停用标志", false),
    ]
}

fn phis27_source_dictionary(
    column: &str,
    profile: &ConnectionProfile,
    schema: &str,
) -> Option<SourceDictionaryMetadata> {
    match column {
        "DRUG_TYPE" => Some(static_dictionary(
            "phis.dictionary.prescriptionType",
            "处方类型",
            "prescriptionType.dic",
            &[("1", "西药"), ("2", "中药"), ("3", "草药"), ("9", "疫苗")],
        )),
        "FORM_CODE" => Some(table_dictionary(
            profile,
            schema,
            "phis.dictionary.dosageForm",
            "剂型",
            "YK_YPSX",
            "YPSX",
            "SXMC",
            &[],
        )),
        "USAGE_CODE" => Some(table_dictionary(
            profile,
            schema,
            "phis.dictionary.drugWay",
            "使用途径",
            "ZY_YPYF",
            "YPYF",
            "XMMC",
            &["PYDM", "FYXH", "SYFW", "BZYF"],
        )),
        "FREQ_CODE" => Some(table_dictionary(
            profile,
            schema,
            "phis.dictionary.useRate",
            "用药频次",
            "GY_SYPC",
            "PCBM",
            "PCMC",
            &["MRCS", "ZXSJ", "ZXZQ", "RZXZQ"],
        )),
        "ROUND_CODE" => Some(static_dictionary(
            "phis.schema.YK_TYPK.QZCL",
            "取整策略",
            "YK_TYPK.sc 内嵌字典",
            &[
                ("0", "每次发药数量取整"),
                ("1", "每天发药数量取整"),
                ("2", "不取整"),
            ],
        )),
        "DISPENSE_CODE" => Some(table_dictionary(
            profile,
            schema,
            "phis.dictionary.hairMedicineWay",
            "发药方式",
            "ZY_FYFS",
            "FYFS",
            "FSMC",
            &[],
        )),
        "RX_FLAG" => Some(static_dictionary(
            "phis.dictionary.prescriptionDrugs",
            "处方药",
            "prescriptionDrugs.dic",
            &[("1", "处方药品（RX）"), ("2", "非处方药品（OTC）")],
        )),
        "BASIC_DRUG_TYPE" => Some(static_dictionary(
            "phis.dictionary.jylx",
            "基药类型",
            "jylx.dic",
            &[
                ("1", "非基本药物"),
                ("2", "国家基本药物"),
                ("3", "省基本药物"),
                ("4", "区自选"),
            ],
        )),
        "INSURANCE_LEVEL" => Some(static_dictionary(
            "phis.dictionary.medicalInsuranceClassification",
            "医保分类",
            "medicalInsuranceClassification.dic",
            &[("1", "甲类"), ("2", "乙类"), ("3", "丙类")],
        )),
        "ORIGIN_TYPE" => Some(static_dictionary(
            "phis.dictionary.grade",
            "产地档次",
            "grade.dic",
            &[("1", "国产"), ("2", "合资"), ("3", "进口")],
        )),
        "STORAGE_CODE" => Some(static_dictionary(
            "phis.dictionary.drugStore",
            "药品贮藏",
            "drugStore.dic",
            &[("1", "常温"), ("2", "阴凉"), ("3", "低温")],
        )),
        "SPECIAL_DRUG_TYPE" => Some(static_dictionary(
            "phis.dictionary.pecialMedicines",
            "特殊药品",
            "pecialMedicines.dic",
            &[
                ("1", "麻醉"),
                ("3", "贵重"),
                ("4", "毒性"),
                ("5", "放射"),
                ("6", "一般"),
                ("7", "一类精神"),
                ("8", "二类精神"),
            ],
        )),
        "ALLERGY_CODE" => Some(static_dictionary(
            "phis.dictionary.gmywlb",
            "过敏药物类别",
            "gmywlb.dic",
            &[
                ("1", "青霉素"),
                ("2", "磺胺"),
                ("3", "喹诺酮"),
                ("4", "头孢"),
                ("5", "四环素"),
                ("6", "抗生素"),
                ("7", "血清制剂"),
                ("8", "链霉素"),
            ],
        )),
        "ANTI_APPROVAL" => Some(static_dictionary(
            "phis.schema.YK_TYPK.SFSP",
            "是否审批",
            "YK_TYPK.sc 内嵌字典",
            &[("1", "需要"), ("2", "不需要")],
        )),
        "ANTIBIOTIC_FLAG" | "SOURCE_STOP_FLAG" => Some(static_dictionary(
            "phis.dictionary.confirm",
            "确认",
            "confirm.dic",
            &[("0", "否"), ("1", "是")],
        )),
        _ => None,
    }
}

fn static_dictionary(
    id: &str,
    name: &str,
    source: &str,
    items: &[(&str, &str)],
) -> SourceDictionaryMetadata {
    SourceDictionaryMetadata {
        id: id.into(),
        name: name.into(),
        source: source.into(),
        items: items
            .iter()
            .map(|(key, text)| SourceDictionaryItem {
                key: (*key).into(),
                text: (*text).into(),
                properties: Map::new(),
            })
            .collect(),
        entry: String::new(),
        key_field: String::new(),
        text_field: String::new(),
        property_fields: Vec::new(),
        load_status: "bundled".into(),
        load_message: "来自二系列字典配置文件".into(),
    }
}

#[allow(clippy::too_many_arguments)]
fn table_dictionary(
    profile: &ConnectionProfile,
    schema: &str,
    id: &str,
    name: &str,
    table: &str,
    key_column: &str,
    text_column: &str,
    property_columns: &[&str],
) -> SourceDictionaryMetadata {
    let property_columns = readable_dictionary_properties(profile, schema, table, property_columns);
    let query = table_dictionary_query(schema, table, key_column, text_column, &property_columns);
    let (items, load_status, load_message) = match odbc::preview_source(profile, &query, 5_000) {
        Ok(preview) => {
            let items = preview
                .rows
                .into_iter()
                .filter_map(|row| {
                    let key = row.get("DICT_KEY")?.as_str()?.trim().to_string();
                    let text = row
                        .get("DICT_TEXT")
                        .and_then(json_value_text)
                        .unwrap_or_default()
                        .trim()
                        .to_string();
                    let properties = property_columns
                        .iter()
                        .enumerate()
                        .filter_map(|(index, column)| {
                            let value = row
                                .get(&format!("DICT_PROP_{index}"))
                                .and_then(json_value_text)
                                .unwrap_or_default()
                                .trim()
                                .to_string();
                            (!value.is_empty()).then_some(((*column).into(), Value::String(value)))
                        })
                        .collect::<Map<String, Value>>();
                    (!key.is_empty()).then_some(SourceDictionaryItem {
                        key,
                        text,
                        properties,
                    })
                })
                .collect::<Vec<_>>();
            let status = if items.is_empty() { "empty" } else { "loaded" };
            let message = if items.is_empty() {
                format!("已读取 {table}，但没有可用字典记录")
            } else {
                format!("已从 {table} 读取 {} 个字典项", items.len())
            };
            (items, status.to_string(), message)
        }
        Err(error) => (
            Vec::new(),
            "unavailable".into(),
            format!("读取 {table} 失败：{error}"),
        ),
    };
    SourceDictionaryMetadata {
        id: id.into(),
        name: name.into(),
        source: format!("{table}.{key_column} → {text_column}"),
        entry: table.into(),
        key_field: key_column.into(),
        text_field: text_column.into(),
        property_fields: property_columns
            .iter()
            .map(|column| (*column).into())
            .collect(),
        load_status,
        load_message,
        items,
    }
}

fn readable_dictionary_properties<'a>(
    profile: &ConnectionProfile,
    schema: &str,
    table: &str,
    property_columns: &'a [&'a str],
) -> Vec<&'a str> {
    if property_columns.is_empty() {
        return Vec::new();
    }
    let query = format!(
        "SELECT COLUMN_NAME FROM ALL_TAB_COLUMNS WHERE OWNER='{schema}' AND TABLE_NAME='{table}'"
    );
    let available = odbc::preview_source(profile, &query, 500)
        .ok()
        .map(|preview| {
            preview
                .rows
                .into_iter()
                .filter_map(|row| row.get("COLUMN_NAME")?.as_str().map(str::to_string))
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    property_columns
        .iter()
        .copied()
        .filter(|column| available.contains(*column))
        .collect()
}

fn table_dictionary_query(
    schema: &str,
    table: &str,
    key_column: &str,
    text_column: &str,
    property_columns: &[&str],
) -> String {
    let property_select = property_columns
        .iter()
        .enumerate()
        .map(|(index, column)| format!(", {column} AS DICT_PROP_{index}"))
        .collect::<String>();
    format!(
        "SELECT TO_CHAR({key_column}) AS DICT_KEY, {text_column} AS DICT_TEXT{property_select} FROM {} \
         WHERE {key_column} IS NOT NULL ORDER BY {key_column}",
        table_name(schema, table)
    )
}

fn json_value_text(value: &Value) -> Option<&str> {
    value.as_str()
}

fn count(connection: &Connection<'_>, query: &str) -> Result<usize, String> {
    let value = odbc::query_optional_string(connection, query, Vec::new())?
        .ok_or_else(|| "二系列phis数据统计没有返回结果".to_string())?;
    value
        .trim()
        .parse::<usize>()
        .map_err(|_| format!("二系列phis数据统计结果无法识别：{value}"))
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
        Err("二系列phis自动识别当前仅支持 Oracle 老库".into())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        additional_physical_columns, inventory_detail_query, inventory_group_query, medicine_query,
        medicine_query_with_physical_columns, normalize_scope, phis27_column_definitions,
        phis27_source_dictionary, source_schema, table_dictionary_query, LegacyPhysicalColumn,
        Scope,
    };
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
        assert!(query.contains("SELECT 1 FROM PHIS27.YK_CDXX c"));
        assert!(query.contains("NVL(c.ZFPB,0)=0"));
        assert!(!query.contains("YK_YPXX"));
        assert!(!query.contains("YF_YPXX"));
        assert!(query.contains("SOURCE_DUPLICATE_COUNT"));
        assert!(query.contains("PARTITION BY t.YPMC,t.YPGG,t.ZXDW"));
        assert!(query.contains("TO_CHAR(t.MRYF) AS FREQ_CODE"));
        assert!(query.contains("t.YCJL AS DOSE_ONCE"));
        assert!(query.contains("TO_CHAR(t.YBFL) AS INSURANCE_LEVEL"));
        assert!(query.contains("TO_CHAR(t.YPDC) AS ORIGIN_TYPE"));
        assert!(query.contains("COALESCE(f.CDQC,f.CDMC) END AS FACTORY_NAME"));
        assert!(query.contains("f.CDMC END AS FACTORY_SHORT_NAME"));
        assert!(query.contains("f.PYDM END AS FACTORY_PINYIN"));
        assert!(!query.contains(";"));
    }

    #[test]
    fn additional_scalar_physical_fields_are_available_without_duplicate_projections() {
        let physical_columns = vec![
            LegacyPhysicalColumn {
                table: "YK_TYPK".into(),
                column: "ZXDW".into(),
                data_type: "NVARCHAR2".into(),
                comment: "最小单位".into(),
            },
            LegacyPhysicalColumn {
                table: "YK_TYPK".into(),
                column: "YPDW".into(),
                data_type: "NVARCHAR2".into(),
                comment: "药品单位".into(),
            },
            LegacyPhysicalColumn {
                table: "YK_TYPK".into(),
                column: "MESS".into(),
                data_type: "CLOB".into(),
                comment: "药品说明".into(),
            },
            LegacyPhysicalColumn {
                table: "YK_YPCD".into(),
                column: "JHDW".into(),
                data_type: "VARCHAR2".into(),
                comment: "进货单位".into(),
            },
        ];
        let additional = additional_physical_columns(&physical_columns);
        assert!(!additional.iter().any(|column| column.column == "ZXDW"));
        assert!(additional.iter().any(|column| column.column == "JHDW"));
        assert!(additional.iter().any(|column| column.column == "YPDW"));
        assert!(!additional.iter().any(|column| column.column == "MESS"));

        let query =
            medicine_query_with_physical_columns("PHIS27", Scope::UsedActive, &physical_columns);
        assert!(query.contains("t.ZXDW AS PRE_UNIT"));
        assert!(query.contains("t.YPDW AS T__YPDW"));
        assert!(query.contains("p.JHDW AS P__JHDW"));
        assert!(!query.contains("t.MESS AS T__MESS"));
        assert!(!query.contains("t.ZXDW AS T__ZXDW"));
    }

    #[test]
    fn only_known_scopes_are_accepted() {
        assert!(normalize_scope("USED_ACTIVE").is_ok());
        assert!(normalize_scope("USED_ALL").is_ok());
        assert!(normalize_scope("ALL_MEDICINES").is_ok());
        assert!(normalize_scope("ALL_TABLES").is_err());
    }

    #[test]
    fn all_medicines_scope_does_not_require_an_institution_configuration() {
        let query = medicine_query("PHIS27", Scope::AllMedicines);
        assert!(query.contains("WHERE 1=1"));
        assert!(!query.contains("YK_CDXX"));
        assert!(!query.contains("YK_YPXX"));
        assert!(!query.contains("YF_YPXX"));
    }

    #[test]
    fn configured_scope_uses_only_cdxx_as_the_membership_filter() {
        let query = medicine_query("PHIS27", Scope::UsedAll);
        assert!(query.contains("SELECT 1 FROM PHIS27.YK_CDXX c"));
        assert!(!query.contains("YK_YPXX"));
        assert!(!query.contains("YF_YPXX"));
        assert!(query.contains("FROM selected_med t"));
        assert!(query.contains("LEFT JOIN PHIS27.YK_YPCD p"));
    }

    #[test]
    fn inventory_preflight_reads_both_stock_ledgers_by_composite_product_key() {
        let query = inventory_group_query("PHIS27", true, true, true, true);
        assert!(query.contains("PHIS27.YK_KCMX"));
        assert!(query.contains("PHIS27.YF_KCMX"));
        assert!(query.contains("NVL(k.KCSL,0)<>0"));
        assert!(query.contains("NVL(k.YPSL,0)<>0"));
        assert!(query.contains(
            "CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY"
        ));
        assert!(query.contains("MAX(TO_NCHAR(YKSB)) AS LOCATION_ID"));
        assert!(query.contains("N'YK:'||l.LOCATION_ID"));
        assert!(query.contains("N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128))"));
        assert!(query.contains("N'YF:'||CAST(k.YFSB AS NVARCHAR2(128)) AS LOCATION_KEY"));
        assert!(query.contains("NVL(CAST(l.YFMC AS NVARCHAR2(200)),N'未命名药房')"));
        assert!(!query.contains("NVL(l.YFMC,'未命名药房')"));
        assert!(!query.contains("YK_YPXX"));
        assert!(!query.contains("YF_YPXX"));
        assert!(!query.contains(';'));

        let detail = inventory_detail_query("PHIS27", true, true, true, true);
        assert!(detail.contains("N'YK:'||l.LOCATION_ID"));
        assert!(detail.contains("N'YKORG:'||CAST(k.JGID AS NVARCHAR2(128))"));
        assert!(detail.contains("CAST(k.YPPH AS NVARCHAR2(200)) AS BATCH_CODE"));
        assert!(detail.contains("TO_NCHAR(k.YPXQ,'YYYY-MM-DD') AS EFFECTIVE_DATE"));
        assert!(detail.contains("CAST(t.ZXBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR"));
        assert!(detail.contains("CAST(t.YPDW AS NVARCHAR2(80)) AS SALE_UNIT"));
        assert!(detail.contains("LEFT JOIN PHIS27.YF_YPXX y"));
        assert!(detail.contains("CAST(y.YFBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR"));
        assert!(detail.contains("CAST(y.YFDW AS NVARCHAR2(80)) AS SALE_UNIT"));
        assert!(detail.contains("CAST(k.JHJE AS NVARCHAR2(80)) AS PURCHASE_TOTAL"));
        assert!(detail.contains("CAST(k.LSJE AS NVARCHAR2(80)) AS RETAIL_TOTAL"));
        assert!(!detail.contains("NVL(k.YPPH,'')"));
    }

    #[test]
    fn product_identity_is_always_ypxh_plus_ypcd() {
        let query = medicine_query("PHIS27", Scope::UsedActive);
        assert!(query.contains("TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS SOURCE_MED_PRO_KEY"));
        assert!(query.contains("TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS CD_MED_PRO"));
        assert!(query.contains("TO_CHAR(p.YPLSH) AS SOURCE_PRODUCT_ID"));
        assert!(!query.contains("NVL(p.YPLSH"));
    }

    #[test]
    fn technical_tracking_columns_are_not_mapping_candidates() {
        let columns = phis27_column_definitions();
        assert!(
            !columns
                .iter()
                .find(|column| column.0 == "SOURCE_PRODUCT_ID")
                .unwrap()
                .4
        );
        assert!(
            !columns
                .iter()
                .find(|column| column.0 == "SOURCE_MED_PRO_KEY")
                .unwrap()
                .4
        );
        assert!(
            columns
                .iter()
                .find(|column| column.0 == "DRUG_NAME")
                .unwrap()
                .4
        );
        assert!(
            columns
                .iter()
                .find(|column| column.0 == "CD_MED_PRO")
                .unwrap()
                .4
        );
    }

    #[test]
    fn static_legacy_dictionaries_follow_the_phis_schema_configuration() {
        let drug_type =
            phis27_source_dictionary("DRUG_TYPE", &profile("PHIS27"), "PHIS27").unwrap();
        assert_eq!(drug_type.id, "phis.dictionary.prescriptionType");
        assert!(drug_type
            .items
            .iter()
            .any(|item| item.key == "9" && item.text == "疫苗"));

        let insurance =
            phis27_source_dictionary("INSURANCE_LEVEL", &profile("PHIS27"), "PHIS27").unwrap();
        assert!(insurance
            .items
            .iter()
            .any(|item| item.key == "2" && item.text == "乙类"));

        let allergy =
            phis27_source_dictionary("ALLERGY_CODE", &profile("PHIS27"), "PHIS27").unwrap();
        assert!(allergy
            .items
            .iter()
            .any(|item| item.key == "3" && item.text == "喹诺酮"));
    }

    #[test]
    fn frequency_dictionary_query_includes_declared_properties() {
        let query = table_dictionary_query(
            "PHIS27",
            "GY_SYPC",
            "PCBM",
            "PCMC",
            &["MRCS", "ZXSJ", "ZXZQ", "RZXZQ"],
        );
        assert!(query.contains("PHIS27.GY_SYPC"));
        assert!(query.contains("TO_CHAR(PCBM) AS DICT_KEY"));
        assert!(query.contains("PCMC AS DICT_TEXT"));
        assert!(query.contains("MRCS AS DICT_PROP_0"));
        assert!(query.contains("ZXSJ AS DICT_PROP_1"));
        assert!(query.contains("ZXZQ AS DICT_PROP_2"));
        assert!(query.contains("RZXZQ AS DICT_PROP_3"));
        assert!(!query.contains(';'));
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
        let reference_catalog = super::load_inventory_reference_catalog(&live)
            .expect("PHIS27 organization/location catalog");
        assert!(!reference_catalog.organizations.is_empty());
        assert!(!reference_catalog.locations.is_empty());
        let schema = source_schema(&live).expect("PHIS27 schema");
        let inventory_query = inventory_group_query(
            &schema,
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YK_KCMX"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YF_KCMX"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YK_YKLB"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YF_YFLB"),
        );
        let inventory_preview = crate::odbc::preview_source(&live, &inventory_query, 10_000)
            .expect("PHIS27 inventory preflight");
        assert!(!inventory_preview.truncated);
        assert!(inventory_preview
            .columns
            .contains(&"LOCATION_NAME".to_string()));
        let inventory_detail = inventory_detail_query(
            &schema,
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YK_KCMX"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YF_KCMX"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YK_YKLB"),
            inspection
                .checked_tables
                .iter()
                .any(|table| table == "YF_YFLB"),
        );
        let inventory_detail_preview = crate::odbc::preview_source(&live, &inventory_detail, 1)
            .expect("PHIS27 inventory detail");
        assert!(inventory_detail_preview
            .columns
            .contains(&"BATCH_CODE".to_string()));
        let preview = super::load(&super::LoadPhis27Request {
            connection: live.clone(),
            scope: "USED_ACTIVE".into(),
            limit: 10_000,
        })
        .expect("PHIS27 load");
        eprintln!(
            "PHIS27 summary: total={}, configured={}, active={}, products={}, used_preview={}",
            inspection.total_medicines,
            inspection.configured_medicines,
            inspection.active_configured_medicines,
            inspection.product_rows,
            preview.rows.len()
        );
        eprintln!(
            "PHIS27 inventory preflight groups={}, organizations={}, locations={}",
            inventory_preview.rows.len(),
            reference_catalog.organizations.len(),
            reference_catalog.locations.len()
        );
        for metadata in preview
            .column_metadata
            .iter()
            .filter_map(|metadata| metadata.source_dictionary.as_ref())
            .filter(|dictionary| !dictionary.entry.is_empty())
        {
            let samples = metadata
                .items
                .iter()
                .take(3)
                .map(|item| format!("{}={}", item.key, item.text))
                .collect::<Vec<_>>()
                .join("、");
            eprintln!(
                "PHIS27 dictionary: {} {}.{} -> {}, status={}, items={}, samples=[{}]",
                metadata.name,
                metadata.entry,
                metadata.key_field,
                metadata.text_field,
                metadata.load_status,
                metadata.items.len(),
                samples
            );
        }
        assert!(!preview.truncated);
        assert!(preview.columns.contains(&"SOURCE_KEY".to_string()));
        assert!(preview.columns.contains(&"PRE_UNIT".to_string()));
        let minimum_unit = preview
            .column_metadata
            .iter()
            .find(|metadata| metadata.name == "PRE_UNIT")
            .expect("YK_TYPK.ZXDW mapping metadata");
        assert_eq!(minimum_unit.source_table, "YK_TYPK");
        assert_eq!(minimum_unit.source_column, "ZXDW");
        assert!(minimum_unit.mapping_eligible);
        assert_eq!(preview.rows.len(), inspection.scopes[0].estimated_rows);
        let frequency = preview
            .column_metadata
            .iter()
            .find(|metadata| metadata.name == "FREQ_CODE")
            .and_then(|metadata| metadata.source_dictionary.as_ref())
            .expect("frequency dictionary metadata");
        assert_eq!(frequency.entry, "GY_SYPC");
        assert_eq!(frequency.load_status, "loaded");
        assert!(!frequency.items.is_empty());
        assert_eq!(
            frequency.property_fields,
            vec!["MRCS", "ZXSJ", "ZXZQ", "RZXZQ"]
        );
    }
}
