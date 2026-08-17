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
    let physical_columns = load_physical_columns(&request.connection, &schema)?;
    validate_medicine_source_columns(&physical_columns, scope)?;
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
    let physical_columns = load_physical_columns(&request.connection, &schema)?;
    validate_inventory_source_columns(&physical_columns, has_warehouse_stock, has_pharmacy_stock)?;
    let has_warehouse_list = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YK_YKLB")
        && has_physical_column(&physical_columns, "YK_YKLB", "YKSB")
        && has_physical_column(&physical_columns, "YK_YKLB", "JGID");
    let has_pharmacy_list = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YF_YFLB")
        && has_physical_column(&physical_columns, "YF_YFLB", "YFSB")
        && has_physical_column(&physical_columns, "YF_YFLB", "JGID");
    let query = inventory_group_query(
        &schema,
        has_warehouse_stock,
        has_pharmacy_stock,
        has_warehouse_list,
        has_pharmacy_list,
        &physical_columns,
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
    let physical_columns = load_physical_columns(profile, &schema)?;
    if !inspection
        .checked_tables
        .iter()
        .any(|table| table == "SYS_ORGANIZATION")
    {
        return Err("老系统无法读取 SYS_ORGANIZATION，不能建立机构对应关系".into());
    }
    validate_source_columns(
        &physical_columns,
        &[
            ("SYS_ORGANIZATION", "ORGANIZCODE"),
            ("SYS_ORGANIZATION", "ORGANIZNAME"),
        ],
        "机构映射",
    )?;
    let parent_id = optional_source_expression(
        &physical_columns,
        "SYS_ORGANIZATION",
        "PARENTID",
        "CAST(PARENTID AS NVARCHAR2(128))",
        "CAST(NULL AS NVARCHAR2(128))",
    );
    let organization_type = optional_source_expression(
        &physical_columns,
        "SYS_ORGANIZATION",
        "ORGANIZTYPE",
        "CAST(ORGANIZTYPE AS NVARCHAR2(64))",
        "CAST(NULL AS NVARCHAR2(64))",
    );
    let active_flag = optional_source_expression(
        &physical_columns,
        "SYS_ORGANIZATION",
        "LOGOFF",
        "CASE WHEN NVL(CAST(LOGOFF AS NVARCHAR2(8)),N'0') IN (N'0',N'false',N'FALSE') THEN N'1' ELSE N'0' END",
        "N'1'",
    );
    let organization_sql = format!(
        "SELECT CAST(ORGANIZCODE AS NVARCHAR2(128)) AS ORGANIZATION_ID,\
         CAST(ORGANIZNAME AS NVARCHAR2(200)) AS ORGANIZATION_NAME,\
         {parent_id} AS PARENT_ID,{organization_type} AS ORGANIZATION_TYPE,\
         {active_flag} AS ACTIVE_FLAG \
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
        validate_source_columns(
            &physical_columns,
            &[("YK_YKLB", "YKSB"), ("YK_YKLB", "JGID")],
            "药库映射",
        )?;
        let warehouse_name = optional_source_expression(
            &physical_columns,
            "YK_YKLB",
            "YKMC",
            "CAST(YKMC AS NVARCHAR2(200))",
            "N'药库 '||TO_NCHAR(YKSB)",
        );
        let warehouse_category = optional_source_expression(
            &physical_columns,
            "YK_YKLB",
            "YKLB",
            "NVL(TO_NCHAR(YKLB),N'')",
            "N''",
        );
        let sql = format!(
            "SELECT N'WAREHOUSE' AS SOURCE_KIND,N'YK:'||TO_NCHAR(YKSB) AS LOCATION_KEY,\
             TO_NCHAR(YKSB) AS LOCATION_ID,{warehouse_name} AS LOCATION_NAME,\
             CAST(JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,\
             {warehouse_category} AS CATEGORY,N'1' AS ACTIVE_FLAG \
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
        validate_source_columns(
            &physical_columns,
            &[("YF_YFLB", "YFSB"), ("YF_YFLB", "JGID")],
            "药房映射",
        )?;
        let pharmacy_name = optional_source_expression(
            &physical_columns,
            "YF_YFLB",
            "YFMC",
            "CAST(YFMC AS NVARCHAR2(200))",
            "N'药房 '||TO_NCHAR(YFSB)",
        );
        let pharmacy_active = optional_source_expression(
            &physical_columns,
            "YF_YFLB",
            "ZXBZ",
            "CASE WHEN NVL(TO_NCHAR(ZXBZ),N'0')=N'0' THEN N'1' ELSE N'0' END",
            "N'1'",
        );
        let sql = format!(
            "SELECT N'PHARMACY' AS SOURCE_KIND,N'YF:'||TO_NCHAR(YFSB) AS LOCATION_KEY,\
             TO_NCHAR(YFSB) AS LOCATION_ID,{pharmacy_name} AS LOCATION_NAME,\
             CAST(JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,N'' AS CATEGORY,\
             {pharmacy_active} AS ACTIVE_FLAG \
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
    let physical_columns = load_physical_columns(profile, &schema)?;
    validate_inventory_source_columns(&physical_columns, has_warehouse_stock, has_pharmacy_stock)?;
    let has_warehouse_list = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YK_YKLB")
        && has_physical_column(&physical_columns, "YK_YKLB", "YKSB")
        && has_physical_column(&physical_columns, "YK_YKLB", "JGID");
    let has_pharmacy_list = inspection
        .checked_tables
        .iter()
        .any(|item| item == "YF_YFLB")
        && has_physical_column(&physical_columns, "YF_YFLB", "YFSB")
        && has_physical_column(&physical_columns, "YF_YFLB", "JGID");
    let query = inventory_detail_query(
        &schema,
        has_warehouse_stock,
        has_pharmacy_stock,
        has_warehouse_list,
        has_pharmacy_list,
        &physical_columns,
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
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    let mut queries = Vec::new();
    let specification = optional_source_expression(
        physical_columns,
        "YK_TYPK",
        "YPGG",
        "CAST(t.YPGG AS NVARCHAR2(200))",
        "CAST(NULL AS NVARCHAR2(200))",
    );
    let dosage_form = optional_source_expression(
        physical_columns,
        "YK_TYPK",
        "YPSX",
        "CAST(t.YPSX AS NVARCHAR2(80))",
        "CAST(NULL AS NVARCHAR2(80))",
    );
    let product_sale_unit = optional_source_expression(
        physical_columns,
        "YK_TYPK",
        "YFDW",
        "CAST(t.YFDW AS NVARCHAR2(80))",
        "CAST(NULL AS NVARCHAR2(80))",
    );
    let product_unit_sale_factor = optional_source_expression(
        physical_columns,
        "YK_TYPK",
        "YFBZ",
        "CAST(t.YFBZ AS NVARCHAR2(80))",
        "CAST(NULL AS NVARCHAR2(80))",
    );
    let factory_name = match (
        has_physical_column(physical_columns, "YK_CDDZ", "CDQC"),
        has_physical_column(physical_columns, "YK_CDDZ", "CDMC"),
    ) {
        (true, true) => "COALESCE(CAST(f.CDQC AS NVARCHAR2(300)),CAST(f.CDMC AS NVARCHAR2(300)))",
        (true, false) => "CAST(f.CDQC AS NVARCHAR2(300))",
        (false, true) => "CAST(f.CDMC AS NVARCHAR2(300))",
        (false, false) => "CAST(NULL AS NVARCHAR2(300))",
    };
    let product_name = optional_source_expression(
        physical_columns,
        "YK_YPCD",
        "YBSPMC",
        "NVL(CAST(p.YBSPMC AS NVARCHAR2(200)),CAST(t.YPMC AS NVARCHAR2(200)))",
        "CAST(t.YPMC AS NVARCHAR2(200))",
    );
    if has_warehouse_stock {
        let warehouse_location_name = optional_source_expression(
            physical_columns,
            "YK_YKLB",
            "YKMC",
            "MAX(CAST(YKMC AS NVARCHAR2(200)))",
            "N'药库 '||MAX(TO_NCHAR(YKSB))",
        );
        let location_join = if has_warehouse_list {
            format!(
                "LEFT JOIN (SELECT JGID,COUNT(*) AS OPTION_COUNT,MAX(TO_NCHAR(YKSB)) AS LOCATION_ID,{warehouse_location_name} AS LOCATION_NAME FROM {} GROUP BY JGID) l ON l.JGID=k.JGID",
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
        let purchase_total = optional_source_expression(
            physical_columns,
            "YK_KCMX",
            "JHJE",
            "CAST(k.JHJE AS NVARCHAR2(80))",
            "CAST(k.KCSL*k.JHJG AS NVARCHAR2(80))",
        );
        let retail_total = optional_source_expression(
            physical_columns,
            "YK_KCMX",
            "LSJE",
            "CAST(k.LSJE AS NVARCHAR2(80))",
            "CAST(k.KCSL*k.LSJG AS NVARCHAR2(80))",
        );
        let batch_code = optional_source_expression(
            physical_columns,
            "YK_KCMX",
            "YPPH",
            "CAST(k.YPPH AS NVARCHAR2(200))",
            "CAST(NULL AS NVARCHAR2(200))",
        );
        let effective_date =
            optional_oracle_date_text_expression(physical_columns, "YK_KCMX", "YPXQ", "k");
        queries.push(format!(
            "SELECT N'WAREHOUSE' AS SOURCE_KIND,CAST(k.SBXH AS NVARCHAR2(64)) AS SOURCE_RECORD_ID,\
             {location_key} AS LOCATION_KEY,{location_name} AS LOCATION_NAME,\
             CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,\
             CAST(t.YPMC AS NVARCHAR2(200)) AS DRUG_NAME,{specification} AS SPECIFICATION,\
             {dosage_form} AS DOSAGE_FORM,CAST(t.ZXDW AS NVARCHAR2(80)) AS MINIMUM_UNIT,\
             CAST(t.YPDW AS NVARCHAR2(80)) AS SALE_UNIT,{specification} AS SALE_SPECIFICATION,\
             CAST(t.ZXBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR,{product_sale_unit} AS PRODUCT_SALE_UNIT,\
             {product_unit_sale_factor} AS PRODUCT_UNIT_SALE_FACTOR,{factory_name} AS FACTORY_NAME,\
             {product_name} AS PRODUCT_NAME,\
             CAST(k.KCSL AS NVARCHAR2(80)) AS AMOUNT,CAST(k.JHJG AS NVARCHAR2(80)) AS PRICE_PUR,CAST(k.LSJG AS NVARCHAR2(80)) AS PRICE_SALE,\
             {purchase_total} AS PURCHASE_TOTAL,{retail_total} AS RETAIL_TOTAL,\
             {batch_code} AS BATCH_CODE,{effective_date} AS EFFECTIVE_DATE \
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
        let pharmacy_location_name = optional_source_expression(
            physical_columns,
            "YF_YFLB",
            "YFMC",
            "NVL(CAST(l.YFMC AS NVARCHAR2(200)),N'未命名药房')",
            "N'药房 '||CAST(k.YFSB AS NVARCHAR2(128))",
        );
        let location_join = if has_pharmacy_list {
            format!(
                "LEFT JOIN {} l ON l.YFSB=k.YFSB",
                table_name(schema, "YF_YFLB")
            )
        } else {
            String::new()
        };
        let location_name = if has_pharmacy_list {
            pharmacy_location_name
        } else {
            "N'未命名药房'"
        };
        let pharmacy_specification = optional_source_expression(
            physical_columns,
            "YF_YPXX",
            "YFGG",
            "CAST(y.YFGG AS NVARCHAR2(200))",
            specification,
        );
        let purchase_total = optional_source_expression(
            physical_columns,
            "YF_KCMX",
            "JHJE",
            "CAST(k.JHJE AS NVARCHAR2(80))",
            "CAST(k.YPSL*k.JHJG AS NVARCHAR2(80))",
        );
        let retail_total = optional_source_expression(
            physical_columns,
            "YF_KCMX",
            "LSJE",
            "CAST(k.LSJE AS NVARCHAR2(80))",
            "CAST(k.YPSL*k.LSJG AS NVARCHAR2(80))",
        );
        let batch_code = optional_source_expression(
            physical_columns,
            "YF_KCMX",
            "YPPH",
            "CAST(k.YPPH AS NVARCHAR2(200))",
            "CAST(NULL AS NVARCHAR2(200))",
        );
        let effective_date =
            optional_oracle_date_text_expression(physical_columns, "YF_KCMX", "YPXQ", "k");
        queries.push(format!(
            "SELECT N'PHARMACY' AS SOURCE_KIND,CAST(k.SBXH AS NVARCHAR2(64)) AS SOURCE_RECORD_ID,\
             N'YF:'||CAST(k.YFSB AS NVARCHAR2(128)) AS LOCATION_KEY,{location_name} AS LOCATION_NAME,\
             CAST(k.JGID AS NVARCHAR2(128)) AS ORGANIZATION_ID,CAST(k.YPXH AS NVARCHAR2(64))||N':'||CAST(k.YPCD AS NVARCHAR2(64)) AS SOURCE_KEY,\
             CAST(t.YPMC AS NVARCHAR2(200)) AS DRUG_NAME,{specification} AS SPECIFICATION,\
             {dosage_form} AS DOSAGE_FORM,CAST(t.ZXDW AS NVARCHAR2(80)) AS MINIMUM_UNIT,\
             CAST(y.YFDW AS NVARCHAR2(80)) AS SALE_UNIT,{pharmacy_specification} AS SALE_SPECIFICATION,\
             CAST(y.YFBZ AS NVARCHAR2(80)) AS UNIT_SALE_FACTOR,{product_sale_unit} AS PRODUCT_SALE_UNIT,\
             {product_unit_sale_factor} AS PRODUCT_UNIT_SALE_FACTOR,{factory_name} AS FACTORY_NAME,\
             {product_name} AS PRODUCT_NAME,\
             CAST(k.YPSL AS NVARCHAR2(80)) AS AMOUNT,CAST(k.JHJG AS NVARCHAR2(80)) AS PRICE_PUR,CAST(k.LSJG AS NVARCHAR2(80)) AS PRICE_SALE,\
             {purchase_total} AS PURCHASE_TOTAL,{retail_total} AS RETAIL_TOTAL,\
             {batch_code} AS BATCH_CODE,{effective_date} AS EFFECTIVE_DATE \
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
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    let mut queries = Vec::new();
    if has_warehouse_stock {
        let warehouse_location_name = optional_source_expression(
            physical_columns,
            "YK_YKLB",
            "YKMC",
            "MAX(CAST(YKMC AS NVARCHAR2(200)))",
            "N'药库 '||MAX(TO_NCHAR(YKSB))",
        );
        let location_join = if has_warehouse_list {
            format!(
                "LEFT JOIN (SELECT JGID,COUNT(*) AS OPTION_COUNT,MAX(TO_NCHAR(YKSB)) AS LOCATION_ID,{warehouse_location_name} AS LOCATION_NAME FROM {} GROUP BY JGID) l ON l.JGID=k.JGID",
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
        let pharmacy_location_name = optional_source_expression(
            physical_columns,
            "YF_YFLB",
            "YFMC",
            "NVL(CAST(l.YFMC AS NVARCHAR2(200)),N'未命名药房')",
            "N'药房 '||CAST(k.YFSB AS NVARCHAR2(128))",
        );
        let location_join = if has_pharmacy_list {
            format!(
                "LEFT JOIN {} l ON l.YFSB=k.YFSB",
                table_name(schema, "YF_YFLB")
            )
        } else {
            String::new()
        };
        let location_name = if has_pharmacy_list {
            pharmacy_location_name
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

    let physical_columns = load_physical_columns_from_connection(connection, schema)?;
    validate_medicine_source_columns(&physical_columns, Scope::UsedAll)?;

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
    let active_predicate =
        expanded_scope_predicate_with_columns(schema, Scope::UsedActive, &physical_columns);
    let active_configured_medicines = count(
        connection,
        &format!("SELECT COUNT(DISTINCT t.YPXH) FROM {typk} t WHERE {active_predicate}"),
    )?;
    let product_rows = count(connection, &format!("SELECT COUNT(*) FROM {ypcd}"))?;
    let active_rows = count(
        connection,
        &scope_count_query_with_columns(schema, Scope::UsedActive, &physical_columns),
    )?;
    let configured_rows = count(
        connection,
        &scope_count_query_with_columns(schema, Scope::UsedAll, &physical_columns),
    )?;
    let all_medicine_rows = count(
        connection,
        &scope_count_query_with_columns(schema, Scope::AllMedicines, &physical_columns),
    )?;
    let can_merge_by_business_key = ["YPMC", "YPGG", "ZXDW"]
        .iter()
        .all(|column| has_physical_column(&physical_columns, "YK_TYPK", column));
    let duplicate_business_groups = if can_merge_by_business_key {
        count(
            connection,
            &format!(
                "SELECT COUNT(*) FROM (SELECT t.YPMC,t.YPGG,t.ZXDW FROM {typk} t WHERE EXISTS (SELECT 1 FROM {cdxx} c WHERE c.YPXH=t.YPXH) GROUP BY t.YPMC,t.YPGG,t.ZXDW HAVING COUNT(*)>1)"
            ),
        )?
    } else {
        0
    };

    let stock_medicines = if checked_tables.contains(&"YK_KCMX".to_string())
        && checked_tables.contains(&"YF_KCMX".to_string())
        && has_physical_column(&physical_columns, "YK_KCMX", "YPXH")
        && has_physical_column(&physical_columns, "YF_KCMX", "YPXH")
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
    let missing_optional_columns = phis27_column_definitions()
        .into_iter()
        .filter(|(_, table, column, _, _)| {
            !table.is_empty()
                && !column.contains(':')
                && !has_physical_column(&physical_columns, table, column)
        })
        .map(|(_, table, column, _, _)| format!("{table}.{column}"))
        .collect::<Vec<_>>();
    if !missing_optional_columns.is_empty() {
        warnings.push(format!(
            "检测到项目化字段差异：{}。标准读取会对缺失的可选字段返回空值，并继续提供表中其他实际字段供人工映射",
            missing_optional_columns.join("、")
        ));
    }
    if !can_merge_by_business_key {
        warnings.push(
            "YK_TYPK 缺少 YPMC/YPGG/ZXDW 中的部分字段，本批不会按名称、规格、最小单位自动合并；请先核对实际替代字段"
                .into(),
        );
    }
    if !has_physical_column(&physical_columns, "YK_CDXX", "ZFPB") {
        warnings.push(
            "YK_CDXX 不含 ZFPB，“机构在用药品”暂按全部机构配置药品读取，请在迁移范围确认时人工复核"
                .into(),
        );
    }
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

fn expanded_scope_predicate_with_columns(
    schema: &str,
    scope: Scope,
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    if matches!(scope, Scope::UsedActive)
        && !has_physical_column(physical_columns, "YK_CDXX", "ZFPB")
    {
        format!(
            "EXISTS (SELECT 1 FROM {} c WHERE c.YPXH=t.YPXH)",
            table_name(schema, "YK_CDXX")
        )
    } else if matches!(scope, Scope::UsedActive)
        && physical_column(physical_columns, "YK_CDXX", "ZFPB")
            .is_some_and(|column| !oracle_numeric_type(&column.data_type))
    {
        format!(
            "EXISTS (SELECT 1 FROM {} c WHERE c.YPXH=t.YPXH AND NVL(TRIM(TO_NCHAR(c.ZFPB)),N'0')=N'0')",
            table_name(schema, "YK_CDXX")
        )
    } else {
        expanded_scope_predicate(schema, scope)
    }
}

fn scope_count_query_with_columns(
    schema: &str,
    scope: Scope,
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    let predicate = expanded_scope_predicate_with_columns(schema, scope, physical_columns);
    format!(
        "SELECT COUNT(*) FROM {} t LEFT JOIN {} p ON p.YPXH=t.YPXH WHERE {predicate}",
        table_name(schema, "YK_TYPK"),
        table_name(schema, "YK_YPCD")
    )
}

#[cfg(test)]
fn medicine_query(schema: &str, scope: Scope) -> String {
    let physical_columns = phis27_column_definitions()
        .into_iter()
        .filter(|(_, table, column, _, _)| !table.is_empty() && !column.contains(':'))
        .map(|(_, table, column, _, _)| LegacyPhysicalColumn {
            table: table.into(),
            column: column.into(),
            data_type: "NVARCHAR2".into(),
            comment: String::new(),
        })
        .chain(
            [
                ("YK_YPCD", "YPXH"),
                ("YK_CDDZ", "YPCD"),
                ("YK_CDXX", "YPXH"),
                ("YK_CDXX", "ZFPB"),
            ]
            .into_iter()
            .map(|(table, column)| LegacyPhysicalColumn {
                table: table.into(),
                column: column.into(),
                data_type: "NUMBER".into(),
                comment: String::new(),
            }),
        )
        .collect::<Vec<_>>();
    medicine_query_with_physical_columns(schema, scope, &physical_columns)
}

fn has_physical_column(
    physical_columns: &[LegacyPhysicalColumn],
    table: &str,
    column: &str,
) -> bool {
    physical_column(physical_columns, table, column).is_some()
}

fn physical_column<'a>(
    physical_columns: &'a [LegacyPhysicalColumn],
    table: &str,
    column: &str,
) -> Option<&'a LegacyPhysicalColumn> {
    physical_columns
        .iter()
        .find(|item| item.table == table && item.column == column)
}

fn oracle_numeric_type(data_type: &str) -> bool {
    matches!(
        data_type.trim().to_ascii_uppercase().as_str(),
        "NUMBER" | "FLOAT" | "BINARY_FLOAT" | "BINARY_DOUBLE" | "INTEGER" | "DECIMAL"
    )
}

fn optional_oracle_date_text_expression(
    physical_columns: &[LegacyPhysicalColumn],
    table: &str,
    column: &str,
    query_alias: &str,
) -> String {
    let Some(physical) = physical_column(physical_columns, table, column) else {
        return "CAST(NULL AS NVARCHAR2(10))".into();
    };
    let data_type = physical.data_type.trim().to_ascii_uppercase();
    if data_type == "DATE" || data_type.starts_with("TIMESTAMP") {
        format!("TO_NCHAR({query_alias}.{column},'YYYY-MM-DD')")
    } else {
        format!("SUBSTR(CAST({query_alias}.{column} AS NVARCHAR2(200)),1,10)")
    }
}

fn optional_source_expression<'a>(
    physical_columns: &[LegacyPhysicalColumn],
    table: &str,
    column: &str,
    available: &'a str,
    missing: &'a str,
) -> &'a str {
    if has_physical_column(physical_columns, table, column) {
        available
    } else {
        missing
    }
}

fn validate_source_columns(
    physical_columns: &[LegacyPhysicalColumn],
    required: &[(&str, &str)],
    stage: &str,
) -> Result<(), String> {
    let missing = required
        .iter()
        .filter(|(table, column)| !has_physical_column(physical_columns, table, column))
        .map(|(table, column)| format!("{table}.{column}"))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "二系列phis{stage}缺少核心字段或当前账号不可见：{}。请核对实际表结构或字段权限",
            missing.join("、")
        ))
    }
}

fn validate_inventory_source_columns(
    physical_columns: &[LegacyPhysicalColumn],
    has_warehouse_stock: bool,
    has_pharmacy_stock: bool,
) -> Result<(), String> {
    let mut required = vec![
        ("YK_TYPK", "YPXH"),
        ("YK_TYPK", "YPMC"),
        ("YK_TYPK", "ZXDW"),
    ];
    if has_warehouse_stock {
        required.extend([
            ("YK_TYPK", "YPDW"),
            ("YK_TYPK", "ZXBZ"),
            ("YK_KCMX", "SBXH"),
            ("YK_KCMX", "JGID"),
            ("YK_KCMX", "YPXH"),
            ("YK_KCMX", "YPCD"),
            ("YK_KCMX", "KCSL"),
            ("YK_KCMX", "JHJG"),
            ("YK_KCMX", "LSJG"),
        ]);
    }
    if has_pharmacy_stock {
        required.extend([
            ("YF_KCMX", "SBXH"),
            ("YF_KCMX", "YFSB"),
            ("YF_KCMX", "JGID"),
            ("YF_KCMX", "YPXH"),
            ("YF_KCMX", "YPCD"),
            ("YF_KCMX", "YPSL"),
            ("YF_KCMX", "JHJG"),
            ("YF_KCMX", "LSJG"),
            ("YF_YPXX", "JGID"),
            ("YF_YPXX", "YFSB"),
            ("YF_YPXX", "YPXH"),
            ("YF_YPXX", "YFBZ"),
            ("YF_YPXX", "YFDW"),
        ]);
    }
    required.sort_unstable();
    required.dedup();
    validate_source_columns(physical_columns, &required, "库存读取")
}

fn validate_medicine_source_columns(
    physical_columns: &[LegacyPhysicalColumn],
    scope: Scope,
) -> Result<(), String> {
    let mut required = vec![
        ("YK_TYPK", "YPXH"),
        ("YK_TYPK", "YPMC"),
        ("YK_YPCD", "YPXH"),
        ("YK_YPCD", "YPCD"),
        ("YK_CDDZ", "YPCD"),
    ];
    if !matches!(scope, Scope::AllMedicines) {
        required.push(("YK_CDXX", "YPXH"));
    }
    let missing = required
        .into_iter()
        .filter(|(table, column)| !has_physical_column(physical_columns, table, column))
        .map(|(table, column)| format!("{table}.{column}"))
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "二系列phis核心关联字段缺失或当前账号不可见：{}。请核对实际表结构或为读取账号补充字段权限",
            missing.join("、")
        ))
    }
}

fn optional_projection(
    physical_columns: &[LegacyPhysicalColumn],
    table: &str,
    column: &str,
    expression: &str,
    result_alias: &str,
) -> String {
    if has_physical_column(physical_columns, table, column) {
        format!("{expression} AS {result_alias}")
    } else {
        format!("CAST(NULL AS NVARCHAR2(1)) AS {result_alias}")
    }
}

fn optional_product_projection(
    physical_columns: &[LegacyPhysicalColumn],
    table: &str,
    column: &str,
    expression: &str,
    result_alias: &str,
) -> String {
    if has_physical_column(physical_columns, table, column) {
        format!("CASE WHEN p.YPCD IS NOT NULL THEN {expression} END AS {result_alias}")
    } else {
        format!("CAST(NULL AS NVARCHAR2(1)) AS {result_alias}")
    }
}

fn medicine_query_with_physical_columns(
    schema: &str,
    scope: Scope,
    physical_columns: &[LegacyPhysicalColumn],
) -> String {
    let typk = table_name(schema, "YK_TYPK");
    let ypcd = table_name(schema, "YK_YPCD");
    let cddz = table_name(schema, "YK_CDDZ");
    let predicate = expanded_scope_predicate_with_columns(schema, scope, physical_columns);
    let duplicate_spec = if has_physical_column(physical_columns, "YK_TYPK", "YPGG") {
        "t.YPGG"
    } else {
        "CAST(NULL AS NVARCHAR2(1))"
    };
    let duplicate_unit = if has_physical_column(physical_columns, "YK_TYPK", "ZXDW") {
        "t.ZXDW"
    } else {
        "CAST(NULL AS NVARCHAR2(1))"
    };
    let projection = |table, column, expression, alias| {
        optional_projection(physical_columns, table, column, expression, alias)
    };
    let product_projection = |table, column, expression, alias| {
        optional_product_projection(physical_columns, table, column, expression, alias)
    };
    let drug_type = projection("YK_TYPK", "TYPE", "TO_CHAR(t.TYPE)", "DRUG_TYPE");
    let form_code = projection("YK_TYPK", "YPSX", "TO_CHAR(t.YPSX)", "FORM_CODE");
    let pre_unit = projection("YK_TYPK", "ZXDW", "t.ZXDW", "PRE_UNIT");
    let dose = projection("YK_TYPK", "YPJL", "t.YPJL", "DOSE");
    let dose_unit = projection("YK_TYPK", "JLDW", "t.JLDW", "DOSE_UNIT");
    let spec = projection("YK_TYPK", "YPGG", "t.YPGG", "SPEC");
    let usage_code = projection("YK_TYPK", "GYFF", "TO_CHAR(t.GYFF)", "USAGE_CODE");
    let freq_code = projection("YK_TYPK", "MRYF", "TO_CHAR(t.MRYF)", "FREQ_CODE");
    let dose_once = projection("YK_TYPK", "YCJL", "t.YCJL", "DOSE_ONCE");
    let round_code = projection("YK_TYPK", "QZCL", "TO_CHAR(t.QZCL)", "ROUND_CODE");
    let dispense_code = projection("YK_TYPK", "FYFS", "TO_CHAR(t.FYFS)", "DISPENSE_CODE");
    let source_product_id = projection("YK_YPCD", "YPLSH", "TO_CHAR(p.YPLSH)", "SOURCE_PRODUCT_ID");
    let factory_name_expression = match (
        has_physical_column(physical_columns, "YK_CDDZ", "CDQC"),
        has_physical_column(physical_columns, "YK_CDDZ", "CDMC"),
    ) {
        (true, true) => "COALESCE(CAST(f.CDQC AS NVARCHAR2(300)),CAST(f.CDMC AS NVARCHAR2(300)))",
        (true, false) => "CAST(f.CDQC AS NVARCHAR2(300))",
        (false, true) => "CAST(f.CDMC AS NVARCHAR2(300))",
        (false, false) => "CAST(NULL AS NVARCHAR2(1))",
    };
    let factory_name =
        format!("CASE WHEN p.YPCD IS NOT NULL THEN {factory_name_expression} END AS FACTORY_NAME");
    let factory_short_name = product_projection("YK_CDDZ", "CDMC", "f.CDMC", "FACTORY_SHORT_NAME");
    let factory_pinyin = product_projection("YK_CDDZ", "PYDM", "f.PYDM", "FACTORY_PINYIN");
    let product_name_expression = if has_physical_column(physical_columns, "YK_YPCD", "YBSPMC") {
        "NVL(CAST(p.YBSPMC AS NVARCHAR2(200)),CAST(t.YPMC AS NVARCHAR2(200)))"
    } else {
        "CAST(t.YPMC AS NVARCHAR2(200))"
    };
    let product_name =
        format!("CASE WHEN p.YPCD IS NOT NULL THEN {product_name_expression} END AS PRODUCT_NAME");
    let sale_unit = product_projection("YK_TYPK", "YFDW", "t.YFDW", "SALE_UNIT");
    let pack_factor = product_projection("YK_TYPK", "YFBZ", "t.YFBZ", "PACK_FACTOR");
    let sale_spec_expression = match (
        has_physical_column(physical_columns, "YK_TYPK", "YFGG"),
        has_physical_column(physical_columns, "YK_TYPK", "YPGG"),
    ) {
        (true, true) => "NVL(CAST(t.YFGG AS NVARCHAR2(200)),CAST(t.YPGG AS NVARCHAR2(200)))",
        (true, false) => "CAST(t.YFGG AS NVARCHAR2(200))",
        (false, true) => "CAST(t.YPGG AS NVARCHAR2(200))",
        (false, false) => "CAST(NULL AS NVARCHAR2(1))",
    };
    let sale_spec =
        format!("CASE WHEN p.YPCD IS NOT NULL THEN {sale_spec_expression} END AS SALE_SPEC");
    let buy_price = projection("YK_YPCD", "JHJG", "p.JHJG", "BUY_PRICE");
    let retail_price = projection("YK_YPCD", "LSJG", "p.LSJG", "RETAIL_PRICE");
    let approval_no = projection("YK_YPCD", "PZWH", "p.PZWH", "APPROVAL_NO");
    let barcode = projection("YK_YPCD", "YPTM", "p.YPTM", "BARCODE");
    let rx_flag = projection("YK_TYPK", "CFYP", "TO_CHAR(t.CFYP)", "RX_FLAG");
    let basic_drug_type = projection("YK_TYPK", "JYLX", "TO_CHAR(t.JYLX)", "BASIC_DRUG_TYPE");
    let insurance_level = projection("YK_TYPK", "YBFL", "TO_CHAR(t.YBFL)", "INSURANCE_LEVEL");
    let origin_type = projection("YK_TYPK", "YPDC", "TO_CHAR(t.YPDC)", "ORIGIN_TYPE");
    let storage_code = projection("YK_TYPK", "YPZC", "TO_CHAR(t.YPZC)", "STORAGE_CODE");
    let special_drug_type = projection("YK_TYPK", "TSYP", "TO_CHAR(t.TSYP)", "SPECIAL_DRUG_TYPE");
    let allergy_code = projection("YK_TYPK", "GMYWLB", "TO_CHAR(t.GMYWLB)", "ALLERGY_CODE");
    let anti_approval = projection("YK_TYPK", "SFSP", "TO_CHAR(t.SFSP)", "ANTI_APPROVAL");
    let antibiotic_flag = projection("YK_TYPK", "KSBZ", "TO_CHAR(t.KSBZ)", "ANTIBIOTIC_FLAG");
    let daily_limit = projection("YK_TYPK", "YCYL", "t.YCYL", "DAILY_LIMIT");
    let source_stop_flag = projection("YK_TYPK", "ZFPB", "TO_CHAR(t.ZFPB)", "SOURCE_STOP_FLAG");
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
                   COUNT(*) OVER (PARTITION BY t.YPMC,{duplicate_spec},{duplicate_unit}) AS SOURCE_DUPLICATE_COUNT
            FROM {typk} t
            WHERE {predicate}
        )
        SELECT
            TO_CHAR(t.YPXH)||':'||NVL(TO_CHAR(p.YPCD),'BASE') AS SOURCE_KEY,
            TO_CHAR(t.YPXH) AS SOURCE_MED_ID,
            t.SOURCE_DUPLICATE_COUNT,
            t.YPMC AS DRUG_NAME,
            {drug_type},
            {form_code},
            {pre_unit},
            {dose},
            {dose_unit},
            {spec},
            {usage_code},
            {freq_code},
            {dose_once},
            {round_code},
            {dispense_code},
            TO_CHAR(p.YPCD) AS SOURCE_FACTORY_ID,
            {source_product_id},
            CASE WHEN p.YPCD IS NOT NULL THEN TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS SOURCE_MED_PRO_KEY,
            {factory_name},
            {factory_short_name},
            {factory_pinyin},
            {product_name},
            {sale_unit},
            {pack_factor},
            {sale_spec},
            {buy_price},
            {retail_price},
            {approval_no},
            {barcode},
            CASE WHEN p.YPCD IS NOT NULL THEN TO_CHAR(t.YPXH)||':'||TO_CHAR(p.YPCD) END AS CD_MED_PRO,
            {rx_flag},
            {basic_drug_type},
            {insurance_level},
            {origin_type},
            {storage_code},
            {special_drug_type},
            {allergy_code},
            {anti_approval},
            {antibiotic_flag},
            {daily_limit},
            {source_stop_flag}{additional_columns}
        FROM selected_med t
        LEFT JOIN {ypcd} p ON p.YPXH=t.YPXH
        LEFT JOIN {cddz} f ON f.YPCD=p.YPCD
        ORDER BY t.YPXH,p.YPCD"#
    )
}

fn load_physical_columns(
    profile: &ConnectionProfile,
    schema: &str,
) -> Result<Vec<LegacyPhysicalColumn>, String> {
    odbc::with_connection(profile, |connection| {
        load_physical_columns_from_connection(connection, schema)
    })
}

fn load_physical_columns_from_connection(
    connection: &Connection<'_>,
    schema: &str,
) -> Result<Vec<LegacyPhysicalColumn>, String> {
    let comment_query = format!(
        "SELECT c.TABLE_NAME,c.COLUMN_NAME,c.DATA_TYPE,m.COMMENTS \
         FROM ALL_TAB_COLUMNS c \
         LEFT JOIN ALL_COL_COMMENTS m ON m.OWNER=c.OWNER AND m.TABLE_NAME=c.TABLE_NAME AND m.COLUMN_NAME=c.COLUMN_NAME \
         WHERE c.OWNER='{schema}' AND c.TABLE_NAME IN (\
         'YK_TYPK','YK_YPCD','YK_CDDZ','YK_CDXX','YK_KCMX','YK_YKLB',\
         'YF_YPXX','YF_KCMX','YF_YFLB','SYS_ORGANIZATION') \
         ORDER BY DECODE(c.TABLE_NAME,'YK_TYPK',1,'YK_YPCD',2,3),c.COLUMN_ID"
    );
    let rows = odbc::query_rows_strings(connection, &comment_query, Vec::new(), 5_000)
        .map_err(|error| format!("读取 {schema} 实际表字段失败：{error}"))?;
    Ok(rows
        .into_iter()
        .filter_map(|row| {
            let table = row.first()?.as_deref()?.trim().to_ascii_uppercase();
            let column = row.get(1)?.as_deref()?.trim().to_ascii_uppercase();
            let data_type = row.get(2)?.as_deref()?.trim().to_string();
            let comment = row
                .get(3)
                .and_then(|value| value.as_deref())
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
        .collect())
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
                let physical_field_exists = table.is_empty()
                    || column.contains(':')
                    || has_physical_column(physical_columns, table, column);
                let database_comment = comments
                    .get(&(table.to_string(), column.to_string()))
                    .filter(|comment| !comment.is_empty())
                    .cloned();
                SourceColumnMetadata {
                    name: name.into(),
                    comment: if physical_field_exists {
                        database_comment.unwrap_or_else(|| fallback_comment.into())
                    } else {
                        format!("{fallback_comment}（当前实际表无此字段，标准查询以空值占位）")
                    },
                    source_table: table.into(),
                    source_column: column.into(),
                    mapping_eligible: mapping_eligible && physical_field_exists,
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
        phis27_source_dictionary, source_schema, table_dictionary_query,
        validate_inventory_source_columns, validate_medicine_source_columns, LegacyPhysicalColumn,
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

    fn inventory_physical_columns() -> Vec<LegacyPhysicalColumn> {
        [
            ("YK_TYPK", "YPXH"),
            ("YK_TYPK", "YPMC"),
            ("YK_TYPK", "YPGG"),
            ("YK_TYPK", "YPSX"),
            ("YK_TYPK", "ZXDW"),
            ("YK_TYPK", "YPDW"),
            ("YK_TYPK", "ZXBZ"),
            ("YK_TYPK", "YFDW"),
            ("YK_TYPK", "YFBZ"),
            ("YK_YPCD", "YPXH"),
            ("YK_YPCD", "YPCD"),
            ("YK_YPCD", "YBSPMC"),
            ("YK_CDDZ", "YPCD"),
            ("YK_CDDZ", "CDQC"),
            ("YK_CDDZ", "CDMC"),
            ("YK_KCMX", "SBXH"),
            ("YK_KCMX", "JGID"),
            ("YK_KCMX", "YPXH"),
            ("YK_KCMX", "YPCD"),
            ("YK_KCMX", "KCSL"),
            ("YK_KCMX", "JHJG"),
            ("YK_KCMX", "LSJG"),
            ("YK_KCMX", "JHJE"),
            ("YK_KCMX", "LSJE"),
            ("YK_KCMX", "YPPH"),
            ("YK_KCMX", "YPXQ"),
            ("YK_YKLB", "YKSB"),
            ("YK_YKLB", "JGID"),
            ("YK_YKLB", "YKMC"),
            ("YF_KCMX", "SBXH"),
            ("YF_KCMX", "YFSB"),
            ("YF_KCMX", "JGID"),
            ("YF_KCMX", "YPXH"),
            ("YF_KCMX", "YPCD"),
            ("YF_KCMX", "YPSL"),
            ("YF_KCMX", "JHJG"),
            ("YF_KCMX", "LSJG"),
            ("YF_KCMX", "JHJE"),
            ("YF_KCMX", "LSJE"),
            ("YF_KCMX", "YPPH"),
            ("YF_KCMX", "YPXQ"),
            ("YF_YPXX", "JGID"),
            ("YF_YPXX", "YFSB"),
            ("YF_YPXX", "YPXH"),
            ("YF_YPXX", "YFBZ"),
            ("YF_YPXX", "YFDW"),
            ("YF_YPXX", "YFGG"),
            ("YF_YFLB", "YFSB"),
            ("YF_YFLB", "JGID"),
            ("YF_YFLB", "YFMC"),
        ]
        .into_iter()
        .map(|(table, column)| LegacyPhysicalColumn {
            table: table.into(),
            column: column.into(),
            data_type: if column == "YPXQ" {
                "DATE".into()
            } else {
                "NVARCHAR2".into()
            },
            comment: String::new(),
        })
        .collect()
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
        assert!(query.contains(
            "COALESCE(CAST(f.CDQC AS NVARCHAR2(300)),CAST(f.CDMC AS NVARCHAR2(300))) END AS FACTORY_NAME"
        ));
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
    fn project_specific_missing_optional_columns_use_null_projections_instead_of_invalid_sql() {
        let physical_columns = [
            ("YK_TYPK", "YPXH"),
            ("YK_TYPK", "YPMC"),
            ("YK_YPCD", "YPXH"),
            ("YK_YPCD", "YPCD"),
            ("YK_CDDZ", "YPCD"),
            ("YK_CDXX", "YPXH"),
        ]
        .into_iter()
        .map(|(table, column)| LegacyPhysicalColumn {
            table: table.into(),
            column: column.into(),
            data_type: "NVARCHAR2".into(),
            comment: String::new(),
        })
        .collect::<Vec<_>>();

        validate_medicine_source_columns(&physical_columns, Scope::UsedActive).unwrap();
        let query =
            medicine_query_with_physical_columns("PHIS27", Scope::UsedActive, &physical_columns);
        assert!(!query.contains("t.MRYF"));
        assert!(query.contains("CAST(NULL AS NVARCHAR2(1)) AS FREQ_CODE"));
        assert!(!query.contains(&format!("NVARCHAR2({})", 4_000)));
        assert!(!query.contains("c.ZFPB"));
        assert!(query.contains("EXISTS (SELECT 1 FROM PHIS27.YK_CDXX c WHERE c.YPXH=t.YPXH)"));
    }

    #[test]
    fn character_stop_flag_avoids_oracle_implicit_number_conversion() {
        let mut physical_columns = phis27_column_definitions()
            .into_iter()
            .filter(|(_, table, column, _, _)| !table.is_empty() && !column.contains(':'))
            .map(|(_, table, column, _, _)| LegacyPhysicalColumn {
                table: table.into(),
                column: column.into(),
                data_type: "NVARCHAR2".into(),
                comment: String::new(),
            })
            .collect::<Vec<_>>();
        physical_columns.extend(
            [
                ("YK_YPCD", "YPXH", "NUMBER"),
                ("YK_CDDZ", "YPCD", "NUMBER"),
                ("YK_CDXX", "YPXH", "NUMBER"),
                ("YK_CDXX", "ZFPB", "VARCHAR2"),
            ]
            .into_iter()
            .map(|(table, column, data_type)| LegacyPhysicalColumn {
                table: table.into(),
                column: column.into(),
                data_type: data_type.into(),
                comment: String::new(),
            }),
        );

        let query =
            medicine_query_with_physical_columns("PHIS27", Scope::UsedActive, &physical_columns);
        assert!(query.contains("NVL(TRIM(TO_NCHAR(c.ZFPB)),N'0')=N'0'"));
        assert!(!query.contains("NVL(c.ZFPB,0)=0"));
    }

    #[test]
    fn generated_oracle_sql_stays_within_11g_character_type_limits() {
        let source = include_str!("legacy_phis27.rs");
        for (marker, maximum) in [("NVARCHAR2(", 2_000_u32), ("VARCHAR2(", 4_000_u32)] {
            for suffix in source.split(marker).skip(1) {
                let digits = suffix
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>();
                if let Ok(length) = digits.parse::<u32>() {
                    assert!(
                        length <= maximum,
                        "Oracle 11g 不允许 {marker}{length})，上限为 {maximum}"
                    );
                }
            }
        }
    }

    #[test]
    fn missing_core_relation_columns_are_reported_before_query_execution() {
        let error = validate_medicine_source_columns(&[], Scope::UsedActive).unwrap_err();
        assert!(error.contains("YK_TYPK.YPXH"));
        assert!(error.contains("YK_YPCD.YPCD"));
        assert!(error.contains("YK_CDXX.YPXH"));
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
        let physical_columns = inventory_physical_columns();
        let query = inventory_group_query("PHIS27", true, true, true, true, &physical_columns);
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

        let detail = inventory_detail_query("PHIS27", true, true, true, true, &physical_columns);
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
    fn character_expiry_is_read_as_text_without_implicit_date_conversion() {
        let mut physical_columns = inventory_physical_columns();
        for column in &mut physical_columns {
            if column.table == "YK_KCMX" && column.column == "YPXQ" {
                column.data_type = "VARCHAR2".into();
            }
        }
        let detail = inventory_detail_query("PHIS27", true, false, true, false, &physical_columns);
        assert!(detail.contains("SUBSTR(CAST(k.YPXQ AS NVARCHAR2(200)),1,10) AS EFFECTIVE_DATE"));
        assert!(!detail.contains("TO_NCHAR(k.YPXQ,'YYYY-MM-DD')"));
    }

    #[test]
    fn inventory_optional_columns_do_not_generate_invalid_identifiers() {
        let physical_columns = inventory_physical_columns()
            .into_iter()
            .filter(|column| {
                !matches!(
                    (column.table.as_str(), column.column.as_str()),
                    ("YK_KCMX", "JHJE")
                        | ("YK_KCMX", "LSJE")
                        | ("YK_KCMX", "YPPH")
                        | ("YK_KCMX", "YPXQ")
                        | ("YF_KCMX", "JHJE")
                        | ("YF_KCMX", "LSJE")
                        | ("YF_KCMX", "YPPH")
                        | ("YF_KCMX", "YPXQ")
                        | ("YF_YPXX", "YFGG")
                        | ("YK_CDDZ", "CDQC")
                        | ("YK_CDDZ", "CDMC")
                        | ("YK_YPCD", "YBSPMC")
                )
            })
            .collect::<Vec<_>>();
        validate_inventory_source_columns(&physical_columns, true, true).unwrap();
        let detail = inventory_detail_query("PHIS27", true, true, true, true, &physical_columns);
        assert!(!detail.contains("k.JHJE"));
        assert!(!detail.contains("k.LSJE"));
        assert!(!detail.contains("k.YPPH"));
        assert!(!detail.contains("k.YPXQ"));
        assert!(!detail.contains("y.YFGG"));
        assert!(!detail.contains("f.CDQC"));
        assert!(!detail.contains("f.CDMC"));
        assert!(!detail.contains("p.YBSPMC"));
        assert!(detail.contains("k.KCSL*k.JHJG"));
        assert!(detail.contains("k.YPSL*k.LSJG"));
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
        let physical_columns =
            super::load_physical_columns(&live, &schema).expect("PHIS27 physical columns");
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
            &physical_columns,
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
            &physical_columns,
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
