use super::sdk::*;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

const ADAPTER_ID: &str = "STANDARD_HIS";
const ADAPTER_NAME: &str = "标准教学 HIS（示例）";
const ADAPTER_VERSION: u32 = 1;
const MEDICINE_OBJECTS: [&str; 2] = ["HIS_DRUG", "HIS_PRODUCT"];
const INVENTORY_OBJECTS: [&str; 5] = [
    "HIS_DRUG",
    "HIS_PRODUCT",
    "HIS_ORGANIZATION",
    "HIS_STORAGE",
    "HIS_INVENTORY",
];

struct StandardHisExampleAdapter;

#[derive(Default)]
struct MedicineShape {
    objects: HashMap<String, HashSet<String>>,
}

impl MedicineShape {
    fn from_structures(structures: &[SourceObjectStructure]) -> Self {
        let objects = structures
            .iter()
            .map(|structure| {
                (
                    structure.name.trim().to_ascii_uppercase(),
                    structure
                        .columns
                        .iter()
                        .map(|column| column.name.trim().to_ascii_uppercase())
                        .collect(),
                )
            })
            .collect();
        Self { objects }
    }

    fn has_object(&self, name: &str) -> bool {
        self.objects.contains_key(name)
    }

    fn has_column(&self, object: &str, column: &str) -> bool {
        self.objects
            .get(object)
            .is_some_and(|columns| columns.contains(column))
    }

    fn has_columns(&self, object: &str, columns: &[&str]) -> bool {
        columns.iter().all(|column| self.has_column(object, column))
    }
}

impl MedicineSourceAdapter for StandardHisExampleAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn inspect(&self, profile: &ConnectionProfile) -> Result<MedicineSourceInspection, String> {
        let schema = source_schema(profile)?;
        let preview =
            read_source_select(profile, structure_query(&schema, &MEDICINE_OBJECTS), 200)?;
        Ok(inspection_from_structures(
            schema,
            structures_from_preview(&preview, &MEDICINE_OBJECTS),
        ))
    }

    fn load(&self, request: &LoadMedicineSourceAdapterRequest) -> Result<SourcePreview, String> {
        if request.scope != "ALL_MEDICINES" {
            return Err(format!("不支持的药品读取范围：{}", request.scope));
        }
        let schema = source_schema(&request.connection)?;
        let structure_preview = read_source_select(
            &request.connection,
            structure_query(&schema, &MEDICINE_OBJECTS),
            200,
        )?;
        let structures = structures_from_preview(&structure_preview, &MEDICINE_OBJECTS);
        let inspection = inspection_from_structures(schema.clone(), structures.clone());
        if !inspection.detected {
            return Err(inspection.message);
        }
        let shape = MedicineShape::from_structures(&structures);
        read_source_select(
            &request.connection,
            medicine_query(&schema, &shape),
            request.limit,
        )
    }
}

impl InventorySourceAdapter for StandardHisExampleAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn load_catalog(&self, profile: &ConnectionProfile) -> Result<InventorySourceCatalog, String> {
        let (schema, structures) = load_inventory_structures(profile)?;
        let shape = MedicineShape::from_structures(&structures);
        ensure_inventory_core(&shape)?;
        let organizations =
            read_source_select(profile, organization_catalog_query(&schema, &shape), 5_000)?;
        let locations = read_source_select(profile, storage_catalog_query(&schema, &shape), 5_000)?;
        inventory_catalog_from_previews(&organizations, &locations)
    }

    fn inspect(
        &self,
        _store: &LocalStore,
        _tenant_id: &str,
        request: &InspectInventorySourceAdapterRequest,
    ) -> Result<InventorySourceReadiness, String> {
        let source_name = request.source_name.trim();
        if source_name.is_empty() {
            return Err("缺少来源数据库身份，无法隔离药品台账和库存批次".into());
        }
        let (schema, structures) = load_inventory_structures(&request.connection)?;
        let shape = MedicineShape::from_structures(&structures);
        let checked_objects = INVENTORY_OBJECTS
            .iter()
            .filter(|name| shape.has_object(name))
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        let missing_objects = INVENTORY_OBJECTS
            .iter()
            .filter(|name| !shape.has_object(name))
            .map(|name| (*name).to_string())
            .collect::<Vec<_>>();
        if let Err(error) = ensure_inventory_core(&shape) {
            return Ok(
                InventorySourceReadiness::new(schema, source_name, Vec::new())
                    .with_ready_for_location_mapping(false)
                    .with_source_structure(checked_objects, missing_objects, structures)
                    .with_warnings(vec![error])
                    .with_message("教学库存结构存在阻断项，请先核对来源字段"),
            );
        }
        let preview = read_source_select(
            &request.connection,
            inventory_summary_query(&schema),
            10_000,
        )?;
        if preview.truncated {
            return Err("库存机构/库房范围超过 10,000 项，请检查来源库房配置".into());
        }
        let locations = inventory_readiness_from_preview(&preview)?;
        let ready = !locations.is_empty();
        Ok(InventorySourceReadiness::new(
            schema,
            source_name,
            locations,
        )
        .with_ready_for_location_mapping(ready)
        .with_medicine_ledger(0, 0, Vec::new())
        .with_source_structure(checked_objects, missing_objects, structures)
        .with_guidance(vec![InventorySourceGuidance::info(
            "STANDARD_INVENTORY_RELATION",
            "教学库存关系",
            "HIS_INVENTORY 通过 STORAGE_ID 定位库房，通过 PRODUCT_ID 关联厂家商品；药品台账由公共预检在选定本批库房后逐行核对。",
        )])
        .with_warnings(vec![
            "轻量范围只统计非零库存；正式明细在选择机构和库房后读取".into(),
        ])
        .with_message(if ready {
            "已读取教学库存范围，请选择本批机构和库房"
        } else {
            "没有需要初始化的非零库存记录"
        }))
    }

    fn load_stock_items(
        &self,
        profile: &ConnectionProfile,
        selected_organization_ids: &HashSet<String>,
        selected_location_keys: &HashSet<String>,
    ) -> Result<Vec<InventorySourceStockItem>, String> {
        let (schema, structures) = load_inventory_structures(profile)?;
        let shape = MedicineShape::from_structures(&structures);
        ensure_inventory_core(&shape)?;
        let query = inventory_detail_query(
            &schema,
            &shape,
            selected_organization_ids,
            selected_location_keys,
        )?;
        let preview = read_source_select(profile, query, 10_000)?;
        if preview.truncated {
            return Err("本批库存明细超过 10,000 条，请减少本批机构或库房数量后重试".into());
        }
        preview.rows.iter().map(stock_item_from_row).collect()
    }
}

fn load_inventory_structures(
    profile: &ConnectionProfile,
) -> Result<(String, Vec<SourceObjectStructure>), String> {
    let schema = source_schema(profile)?;
    let preview = read_source_select(profile, structure_query(&schema, &INVENTORY_OBJECTS), 500)?;
    let structures = structures_from_preview(&preview, &INVENTORY_OBJECTS);
    Ok((schema, structures))
}

fn ensure_inventory_core(shape: &MedicineShape) -> Result<(), String> {
    let requirements = [
        ("HIS_DRUG", &["DRUG_ID", "DRUG_NAME", "MINIMUM_UNIT"][..]),
        ("HIS_PRODUCT", &["PRODUCT_ID", "DRUG_ID"][..]),
        ("HIS_ORGANIZATION", &["ORG_ID", "ORG_NAME"][..]),
        (
            "HIS_STORAGE",
            &["STORAGE_ID", "ORG_ID", "STORAGE_NAME", "STORAGE_KIND"][..],
        ),
        (
            "HIS_INVENTORY",
            &[
                "INVENTORY_ID",
                "STORAGE_ID",
                "PRODUCT_ID",
                "QUANTITY",
                "SALE_UNIT",
                "UNIT_SALE_FACTOR",
                "PURCHASE_PRICE",
                "RETAIL_PRICE",
            ][..],
        ),
    ];
    let missing = requirements
        .iter()
        .flat_map(|(object, columns)| {
            if !shape.has_object(object) {
                vec![(*object).to_string()]
            } else {
                columns
                    .iter()
                    .filter(|column| !shape.has_column(object, column))
                    .map(|column| format!("{object}.{column}"))
                    .collect()
            }
        })
        .collect::<Vec<_>>();
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("教学库存缺少核心来源结构：{}", missing.join("、")))
    }
}

fn source_schema(profile: &ConnectionProfile) -> Result<String, String> {
    let schema = if profile.schema.trim().is_empty() {
        "public"
    } else {
        profile.schema.trim()
    };
    let valid = schema.len() <= 63
        && schema
            .chars()
            .next()
            .is_some_and(|character| character == '_' || character.is_ascii_alphabetic())
        && schema
            .chars()
            .all(|character| character == '_' || character.is_ascii_alphanumeric());
    if !valid {
        return Err("Schema 只能包含字母、数字和下划线，且不能以数字开头".into());
    }
    Ok(schema.to_ascii_lowercase())
}

fn teaching_table(schema: &str, object: &str) -> String {
    source_qualified_object("postgresql", schema, object)
        .expect("teaching schema and object names were validated")
}

fn structure_query(schema: &str, expected_objects: &[&str]) -> String {
    let object_names = expected_objects
        .iter()
        .map(|name| format!("'{}'", name.to_ascii_lowercase()))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "SELECT UPPER(table_name) AS \"OBJECT_NAME\", \
                UPPER(column_name) AS \"COLUMN_NAME\", \
                UPPER(data_type) AS \"DATA_TYPE\" \
         FROM information_schema.columns \
         WHERE table_schema = '{schema}' \
           AND table_name IN ({object_names}) \
         ORDER BY table_name, ordinal_position"
    )
}

fn structures_from_preview(
    preview: &SourcePreview,
    expected_objects: &[&str],
) -> Vec<SourceObjectStructure> {
    let mut by_object = HashMap::<String, Vec<SourceObjectColumnStructure>>::new();
    for row in &preview.rows {
        let object = row_text(row, "OBJECT_NAME").to_ascii_uppercase();
        let column = row_text(row, "COLUMN_NAME").to_ascii_uppercase();
        if object.is_empty() || column.is_empty() || !expected_objects.contains(&object.as_str()) {
            continue;
        }
        by_object
            .entry(object)
            .or_default()
            .push(SourceObjectColumnStructure {
                name: column,
                data_type: row_text(row, "DATA_TYPE"),
            });
    }
    expected_objects
        .iter()
        .filter_map(|name| {
            by_object
                .remove(*name)
                .map(|columns| SourceObjectStructure {
                    name: (*name).into(),
                    columns,
                })
        })
        .collect()
}

fn inspection_from_structures(
    schema: String,
    structures: Vec<SourceObjectStructure>,
) -> MedicineSourceInspection {
    let shape = MedicineShape::from_structures(&structures);
    let checked_objects = MEDICINE_OBJECTS
        .iter()
        .filter(|name| shape.has_object(name))
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    let missing_objects = MEDICINE_OBJECTS
        .iter()
        .filter(|name| !shape.has_object(name))
        .map(|name| (*name).to_string())
        .collect::<Vec<_>>();
    let detected = shape.has_columns("HIS_DRUG", &["DRUG_ID", "DRUG_NAME", "MINIMUM_UNIT"]);
    let mut warnings = Vec::new();
    if !shape.has_object("HIS_PRODUCT") {
        warnings.push("未发现 HIS_PRODUCT，将只读取药品主档并使用 DRUG_ID:BASE 来源键".into());
    }
    let column_count = structures
        .iter()
        .map(|structure| structure.columns.len())
        .sum();
    MedicineSourceInspection {
        detected,
        adapter_id: ADAPTER_ID.into(),
        adapter_name: ADAPTER_NAME.into(),
        adapter_version: ADAPTER_VERSION,
        schema,
        checked_objects,
        missing_objects,
        object_structures: structures,
        scopes: vec![MedicineSourceScope {
            id: "ALL_MEDICINES".into(),
            label: "全部药品".into(),
            description: "读取 HIS_DRUG 全部药品；存在 HIS_PRODUCT 时同时带出厂家商品".into(),
            estimated_rows: 0,
            medicine_count: 0,
            recommended: true,
        }],
        metrics: vec![
            MedicineSourceMetric {
                id: "READABLE_OBJECTS".into(),
                label: "可读来源表".into(),
                value: shape.objects.len(),
            },
            MedicineSourceMetric {
                id: "READABLE_COLUMNS".into(),
                label: "已核对字段".into(),
                value: column_count,
            },
        ],
        guidance: vec![MedicineSourceGuidance {
            id: "STANDARD_HIS_RELATION".into(),
            title: "教学表关系".into(),
            body: "HIS_DRUG 提供药品主档；HIS_PRODUCT 通过 DRUG_ID 关联厂家商品。示例只演示公共 SDK，复制后必须替换为真实 HIS 关系。".into(),
            tone: "INFO".into(),
        }],
        compatibility: None,
        warnings,
        message: if detected {
            "已识别教学药品主表，可进入引导字段映射".into()
        } else {
            "HIS_DRUG 缺少 DRUG_ID、DRUG_NAME 或 MINIMUM_UNIT，不能安全读取药品主档".into()
        },
    }
}

fn medicine_query(schema: &str, shape: &MedicineShape) -> String {
    let drug_table = teaching_table(schema, "his_drug");
    let product_table = teaching_table(schema, "his_product");
    let drug_specification = optional_projection(
        shape.has_column("HIS_DRUG", "SPECIFICATION"),
        "CAST(d.specification AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let dosage_form = optional_projection(
        shape.has_column("HIS_DRUG", "DOSAGE_FORM"),
        "CAST(d.dosage_form AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let has_product = shape.has_columns("HIS_PRODUCT", &["PRODUCT_ID", "DRUG_ID"]);
    let (source_key, manufacturer, product_name, product_join) = if has_product {
        (
            "CAST(d.drug_id AS TEXT) || ':' || CAST(p.product_id AS TEXT)",
            optional_projection(
                shape.has_column("HIS_PRODUCT", "MANUFACTURER_NAME"),
                "CAST(p.manufacturer_name AS TEXT)",
                "CAST(NULL AS TEXT)",
            ),
            optional_projection(
                shape.has_column("HIS_PRODUCT", "PRODUCT_NAME"),
                "CAST(p.product_name AS TEXT)",
                "CAST(d.drug_name AS TEXT)",
            ),
            format!("LEFT JOIN {product_table} p ON p.drug_id = d.drug_id"),
        )
    } else {
        (
            "CAST(d.drug_id AS TEXT) || ':BASE'",
            "CAST(NULL AS TEXT)",
            "CAST(d.drug_name AS TEXT)",
            String::new(),
        )
    };
    format!(
        "SELECT {source_key} AS \"SOURCE_KEY\", \
                CAST(d.drug_name AS TEXT) AS \"DRUG_NAME\", \
                {drug_specification} AS \"SPECIFICATION\", \
                {dosage_form} AS \"DOSAGE_FORM\", \
                CAST(d.minimum_unit AS TEXT) AS \"MINIMUM_UNIT\", \
                {manufacturer} AS \"MANUFACTURER_NAME\", \
                {product_name} AS \"PRODUCT_NAME\" \
         FROM {drug_table} d {product_join} \
         ORDER BY d.drug_id"
    )
}

fn organization_catalog_query(schema: &str, shape: &MedicineShape) -> String {
    let organization_table = teaching_table(schema, "his_organization");
    let parent_id = optional_projection(
        shape.has_column("HIS_ORGANIZATION", "PARENT_ID"),
        "CAST(o.parent_id AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let active = optional_projection(
        shape.has_column("HIS_ORGANIZATION", "ACTIVE_FLAG"),
        "CAST(o.active_flag AS TEXT)",
        "'1'",
    );
    format!(
        "SELECT CAST(o.org_id AS TEXT) AS \"ORG_ID\", \
                CAST(o.org_name AS TEXT) AS \"ORG_NAME\", \
                {parent_id} AS \"PARENT_ID\", \
                {active} AS \"ACTIVE_FLAG\" \
         FROM {organization_table} o \
         ORDER BY o.org_id"
    )
}

fn storage_catalog_query(schema: &str, shape: &MedicineShape) -> String {
    let storage_table = teaching_table(schema, "his_storage");
    let active = optional_projection(
        shape.has_column("HIS_STORAGE", "ACTIVE_FLAG"),
        "CAST(s.active_flag AS TEXT)",
        "'1'",
    );
    format!(
        "SELECT UPPER(CAST(s.storage_kind AS TEXT)) AS \"SOURCE_KIND\", \
                CAST(s.storage_id AS TEXT) AS \"STORAGE_ID\", \
                CAST(s.storage_name AS TEXT) AS \"STORAGE_NAME\", \
                CAST(s.org_id AS TEXT) AS \"ORG_ID\", \
                {active} AS \"ACTIVE_FLAG\" \
         FROM {storage_table} s \
         WHERE UPPER(CAST(s.storage_kind AS TEXT)) IN ('WAREHOUSE', 'PHARMACY') \
         ORDER BY s.org_id, s.storage_id"
    )
}

fn inventory_catalog_from_previews(
    organizations: &SourcePreview,
    locations: &SourcePreview,
) -> Result<InventorySourceCatalog, String> {
    let organizations = organizations
        .rows
        .iter()
        .map(|row| {
            let id = required_row_text(row, "ORG_ID")?;
            let name = required_row_text(row, "ORG_NAME")?;
            Ok(InventorySourceOrganization::new(id, name)
                .with_parent_id(row_text(row, "PARENT_ID"))
                .with_organization_type("医疗机构")
                .with_active(row_bool(row, "ACTIVE_FLAG", true)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let locations = locations
        .rows
        .iter()
        .map(|row| {
            let kind = required_row_text(row, "SOURCE_KIND")?.to_ascii_uppercase();
            let id = required_row_text(row, "STORAGE_ID")?;
            let name = required_row_text(row, "STORAGE_NAME")?;
            let organization_id = required_row_text(row, "ORG_ID")?;
            Ok(InventorySourceLocationDefinition::new(
                kind.clone(),
                id.clone(),
                id,
                name,
                organization_id,
            )
            .with_category(if kind == "WAREHOUSE" {
                "药库"
            } else {
                "药房"
            })
            .with_active(row_bool(row, "ACTIVE_FLAG", true)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    Ok(InventorySourceCatalog::new(organizations, locations)
        .with_message("已读取教学 HIS 机构与药库药房目录"))
}

fn inventory_summary_query(schema: &str) -> String {
    let inventory_table = teaching_table(schema, "his_inventory");
    let storage_table = teaching_table(schema, "his_storage");
    let product_table = teaching_table(schema, "his_product");
    format!(
        "SELECT UPPER(CAST(s.storage_kind AS TEXT)) AS \"SOURCE_KIND\", \
                CAST(s.storage_id AS TEXT) AS \"STORAGE_ID\", \
                CAST(s.storage_name AS TEXT) AS \"STORAGE_NAME\", \
                CAST(s.org_id AS TEXT) AS \"ORG_ID\", \
                COUNT(*) AS \"STOCK_ROW_COUNT\", \
                COUNT(DISTINCT CAST(p.drug_id AS TEXT) || ':' || CAST(p.product_id AS TEXT)) AS \"STOCK_GROUP_COUNT\", \
                COUNT(DISTINCT p.drug_id) AS \"MEDICINE_COUNT\" \
         FROM {inventory_table} i \
         JOIN {storage_table} s ON s.storage_id = i.storage_id \
         JOIN {product_table} p ON p.product_id = i.product_id \
         WHERE i.quantity <> 0 \
           AND UPPER(CAST(s.storage_kind AS TEXT)) IN ('WAREHOUSE', 'PHARMACY') \
         GROUP BY s.storage_kind, s.storage_id, s.storage_name, s.org_id \
         ORDER BY s.org_id, s.storage_id"
    )
}

fn inventory_readiness_from_preview(
    preview: &SourcePreview,
) -> Result<Vec<InventorySourceLocationReadiness>, String> {
    preview
        .rows
        .iter()
        .map(|row| {
            Ok(InventorySourceLocationReadiness::new(
                required_row_text(row, "SOURCE_KIND")?.to_ascii_uppercase(),
                required_row_text(row, "STORAGE_ID")?,
                required_row_text(row, "STORAGE_NAME")?,
                required_row_text(row, "ORG_ID")?,
            )
            .with_counts(
                1,
                row_usize(row, "STOCK_ROW_COUNT")?,
                row_usize(row, "STOCK_GROUP_COUNT")?,
                row_usize(row, "MEDICINE_COUNT")?,
            ))
        })
        .collect()
}

fn inventory_detail_query(
    schema: &str,
    shape: &MedicineShape,
    selected_organization_ids: &HashSet<String>,
    selected_location_keys: &HashSet<String>,
) -> Result<String, String> {
    let organizations = source_text_filter_list(selected_organization_ids, "机构")?;
    let locations = source_text_filter_list(selected_location_keys, "库房")?;
    let inventory_table = teaching_table(schema, "his_inventory");
    let storage_table = teaching_table(schema, "his_storage");
    let product_table = teaching_table(schema, "his_product");
    let drug_table = teaching_table(schema, "his_drug");
    let specification = optional_projection(
        shape.has_column("HIS_DRUG", "SPECIFICATION"),
        "CAST(d.specification AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let dosage_form = optional_projection(
        shape.has_column("HIS_DRUG", "DOSAGE_FORM"),
        "CAST(d.dosage_form AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let manufacturer = optional_projection(
        shape.has_column("HIS_PRODUCT", "MANUFACTURER_NAME"),
        "CAST(p.manufacturer_name AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let product_name = optional_projection(
        shape.has_column("HIS_PRODUCT", "PRODUCT_NAME"),
        "CAST(p.product_name AS TEXT)",
        "CAST(d.drug_name AS TEXT)",
    );
    let product_sale_unit = optional_projection(
        shape.has_column("HIS_PRODUCT", "SALE_UNIT"),
        "CAST(p.sale_unit AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let product_factor = optional_projection(
        shape.has_column("HIS_PRODUCT", "UNIT_SALE_FACTOR"),
        "CAST(p.unit_sale_factor AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let minimum_package_factor = optional_projection(
        shape.has_column("HIS_DRUG", "MIN_PACKAGE_FACTOR"),
        "CAST(d.min_package_factor AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let purchase_total = optional_projection(
        shape.has_column("HIS_INVENTORY", "PURCHASE_TOTAL"),
        "CAST(i.purchase_total AS TEXT)",
        "CAST(i.quantity * i.purchase_price AS TEXT)",
    );
    let retail_total = optional_projection(
        shape.has_column("HIS_INVENTORY", "RETAIL_TOTAL"),
        "CAST(i.retail_total AS TEXT)",
        "CAST(i.quantity * i.retail_price AS TEXT)",
    );
    let batch_code = optional_projection(
        shape.has_column("HIS_INVENTORY", "BATCH_CODE"),
        "CAST(i.batch_code AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    let expiry_date = optional_projection(
        shape.has_column("HIS_INVENTORY", "EXPIRY_DATE"),
        "CAST(i.expiry_date AS TEXT)",
        "CAST(NULL AS TEXT)",
    );
    Ok(format!(
        "SELECT UPPER(CAST(s.storage_kind AS TEXT)) AS \"SOURCE_KIND\", \
                CAST(i.inventory_id AS TEXT) AS \"INVENTORY_ID\", \
                CAST(s.storage_id AS TEXT) AS \"STORAGE_ID\", \
                CAST(s.storage_name AS TEXT) AS \"STORAGE_NAME\", \
                CAST(s.org_id AS TEXT) AS \"ORG_ID\", \
                CAST(p.drug_id AS TEXT) || ':' || CAST(p.product_id AS TEXT) AS \"SOURCE_PRODUCT_KEY\", \
                CAST(d.drug_name AS TEXT) AS \"DRUG_NAME\", \
                {specification} AS \"SPECIFICATION\", \
                {dosage_form} AS \"DOSAGE_FORM\", \
                CAST(d.minimum_unit AS TEXT) AS \"MINIMUM_UNIT\", \
                CAST(i.sale_unit AS TEXT) AS \"SALE_UNIT\", \
                CAST(i.unit_sale_factor AS TEXT) || ' ' || CAST(d.minimum_unit AS TEXT) || '/' || CAST(i.sale_unit AS TEXT) AS \"SALE_SPECIFICATION\", \
                CAST(i.unit_sale_factor AS TEXT) AS \"UNIT_SALE_FACTOR\", \
                {minimum_package_factor} AS \"MIN_PACKAGE_FACTOR\", \
                {product_sale_unit} AS \"PRODUCT_SALE_UNIT\", \
                {product_factor} AS \"PRODUCT_UNIT_SALE_FACTOR\", \
                {manufacturer} AS \"MANUFACTURER_NAME\", \
                {product_name} AS \"PRODUCT_NAME\", \
                CAST(i.quantity AS TEXT) AS \"QUANTITY\", \
                CAST(i.purchase_price AS TEXT) AS \"PURCHASE_PRICE\", \
                CAST(i.retail_price AS TEXT) AS \"RETAIL_PRICE\", \
                {purchase_total} AS \"PURCHASE_TOTAL\", \
                {retail_total} AS \"RETAIL_TOTAL\", \
                {batch_code} AS \"BATCH_CODE\", \
                {expiry_date} AS \"EXPIRY_DATE\" \
         FROM {inventory_table} i \
         JOIN {storage_table} s ON s.storage_id = i.storage_id \
         JOIN {product_table} p ON p.product_id = i.product_id \
         JOIN {drug_table} d ON d.drug_id = p.drug_id \
         WHERE i.quantity <> 0 \
           AND CAST(s.org_id AS TEXT) IN ({organizations}) \
           AND CAST(s.storage_id AS TEXT) IN ({locations}) \
           AND UPPER(CAST(s.storage_kind AS TEXT)) IN ('WAREHOUSE', 'PHARMACY') \
         ORDER BY s.storage_id, i.inventory_id"
    ))
}

fn stock_item_from_row(row: &Map<String, Value>) -> Result<InventorySourceStockItem, String> {
    let mut item = InventorySourceStockItem::new(
        required_row_text(row, "SOURCE_KIND")?.to_ascii_uppercase(),
        required_row_text(row, "INVENTORY_ID")?,
        required_row_text(row, "STORAGE_ID")?,
        required_row_text(row, "STORAGE_NAME")?,
        required_row_text(row, "ORG_ID")?,
        required_row_text(row, "SOURCE_PRODUCT_KEY")?,
    )
    .with_medicine(
        row_text(row, "DRUG_NAME"),
        row_text(row, "SPECIFICATION"),
        row_text(row, "DOSAGE_FORM"),
        row_text(row, "MINIMUM_UNIT"),
    )
    .with_storage_packaging(
        row_text(row, "SALE_UNIT"),
        row_text(row, "SALE_SPECIFICATION"),
        row_text(row, "UNIT_SALE_FACTOR"),
    )
    .with_product_packaging(
        row_text(row, "PRODUCT_SALE_UNIT"),
        row_text(row, "PRODUCT_UNIT_SALE_FACTOR"),
    )
    .with_product(
        row_text(row, "MANUFACTURER_NAME"),
        row_text(row, "PRODUCT_NAME"),
    )
    .with_quantity_and_prices(
        row_text(row, "QUANTITY"),
        row_text(row, "PURCHASE_PRICE"),
        row_text(row, "RETAIL_PRICE"),
    )
    .with_totals(
        row_text(row, "PURCHASE_TOTAL"),
        row_text(row, "RETAIL_TOTAL"),
    )
    .with_batch(row_text(row, "BATCH_CODE"), row_text(row, "EXPIRY_DATE"));
    let minimum_package_factor = row_text(row, "MIN_PACKAGE_FACTOR");
    if !minimum_package_factor.is_empty() {
        item = item.with_single_minimum_package_evidence(
            minimum_package_factor,
            "HIS_DRUG.MIN_PACKAGE_FACTOR",
        );
    }
    Ok(item)
}

fn optional_projection<'a>(available: bool, value: &'a str, fallback: &'a str) -> &'a str {
    if available {
        value
    } else {
        fallback
    }
}

fn row_text(row: &Map<String, Value>, key: &str) -> String {
    let value = row.get(key).or_else(|| row.get(&key.to_ascii_lowercase()));
    match value {
        Some(Value::String(value)) => value.trim().to_string(),
        Some(Value::Number(value)) => value.to_string(),
        Some(Value::Bool(value)) => value.to_string(),
        _ => String::new(),
    }
}

fn required_row_text(row: &Map<String, Value>, key: &str) -> Result<String, String> {
    let value = row_text(row, key);
    if value.is_empty() {
        Err(format!("教学 HIS 查询结果缺少 {key}"))
    } else {
        Ok(value)
    }
}

fn row_usize(row: &Map<String, Value>, key: &str) -> Result<usize, String> {
    required_row_text(row, key)?
        .parse::<usize>()
        .map_err(|_| format!("教学 HIS 查询结果 {key} 不是非负整数"))
}

fn row_bool(row: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    match row_text(row, key).to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "y" | "是" | "启用" => true,
        "0" | "false" | "no" | "n" | "否" | "停用" => false,
        _ => fallback,
    }
}

static ADAPTER: StandardHisExampleAdapter = StandardHisExampleAdapter;

pub(super) fn binding() -> SourceAdapterBinding {
    SourceAdapterBinding::new(
        "STANDARD_HIS_EXAMPLE_BUILTIN",
        Some(&ADAPTER),
        Some(&ADAPTER),
    )
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Fixture {
        source_objects: Vec<FixtureObject>,
    }

    #[derive(Deserialize)]
    struct FixtureObject {
        name: String,
        columns: Vec<FixtureColumn>,
    }

    #[derive(Deserialize)]
    #[serde(untagged)]
    enum FixtureColumn {
        Legacy(String),
        Typed {
            name: String,
            #[serde(rename = "dataType")]
            data_type: String,
        },
    }

    fn fixture_structures(source: &str) -> Vec<SourceObjectStructure> {
        let fixture: Fixture = serde_json::from_str(source).expect("teaching fixture");
        fixture
            .source_objects
            .into_iter()
            .map(|object| SourceObjectStructure {
                name: object.name,
                columns: object
                    .columns
                    .into_iter()
                    .map(|column| {
                        let (name, data_type) = match column {
                            FixtureColumn::Legacy(name) => (name, "TEXT".into()),
                            FixtureColumn::Typed { name, data_type } => (name, data_type),
                        };
                        SourceObjectColumnStructure { name, data_type }
                    })
                    .collect(),
            })
            .collect()
    }

    #[test]
    fn example_binding_uses_only_public_adapter_sdk() {
        let _binding = binding();
        let source = include_str!("adapter.rs");
        assert!(source.contains("use super::sdk::*;"));
        for internal_module in ["odbc", "pg_protocol", "datasource"] {
            let forbidden = format!("{}::{internal_module}", "crate");
            assert!(!source.contains(&forbidden));
        }
        assert!(source.contains("source_qualified_object"));
        assert!(source.contains("source_text_filter_list"));
        let forbidden_local_filter = ["fn sql_", "text_list"].concat();
        assert!(!source.contains(&forbidden_local_filter));
    }

    #[test]
    fn optional_product_table_keeps_medicine_readable() {
        let structures = fixture_structures(include_str!("fixtures/medicine-without-product.json"));
        let inspection = inspection_from_structures("public".into(), structures.clone());
        assert!(inspection.detected);
        assert_eq!(inspection.checked_objects, vec!["HIS_DRUG"]);
        assert_eq!(inspection.missing_objects, vec!["HIS_PRODUCT"]);
        let query = medicine_query("public", &MedicineShape::from_structures(&structures));
        assert!(query.contains("':BASE'"));
        assert!(!query.contains("JOIN \"public\".his_product"));
        assert!(query.contains("CAST(NULL AS TEXT) AS \"MANUFACTURER_NAME\""));
    }

    #[test]
    fn inventory_core_fixture_maps_to_public_stock_contract() {
        let structures =
            fixture_structures(include_str!("fixtures/inventory-core-postgresql.json"));
        let shape = MedicineShape::from_structures(&structures);
        ensure_inventory_core(&shape).expect("core inventory structure");
        let organizations = preview(vec![row(serde_json::json!({
            "ORG_ID": "ORG-1",
            "ORG_NAME": "教学医院",
            "ACTIVE_FLAG": "1"
        }))]);
        let locations = preview(vec![row(serde_json::json!({
            "SOURCE_KIND": "WAREHOUSE",
            "STORAGE_ID": "STO-1",
            "STORAGE_NAME": "中心药库",
            "ORG_ID": "ORG-1",
            "ACTIVE_FLAG": "1"
        }))]);
        let catalog =
            inventory_catalog_from_previews(&organizations, &locations).expect("public catalog");
        assert_eq!(catalog.organizations.len(), 1);
        assert_eq!(catalog.locations[0].source_kind, "WAREHOUSE");
        let descriptor =
            super::super::parse_manifest("standard_his", include_str!("manifest.json"))
                .expect("teaching descriptor");
        let catalog = super::super::normalize_inventory_catalog(&descriptor, catalog)
            .expect("normalized public catalog");
        assert_eq!(catalog.adapter_id, ADAPTER_ID);

        let summary = preview(vec![row(serde_json::json!({
            "SOURCE_KIND": "WAREHOUSE",
            "STORAGE_ID": "STO-1",
            "STORAGE_NAME": "中心药库",
            "ORG_ID": "ORG-1",
            "STOCK_ROW_COUNT": 1,
            "STOCK_GROUP_COUNT": 1,
            "MEDICINE_COUNT": 1
        }))]);
        let readiness = inventory_readiness_from_preview(&summary).expect("public readiness");
        assert_eq!(readiness[0].stock_row_count, 1);
        let readiness = InventorySourceReadiness::new("public", "teaching", readiness)
            .with_ready_for_location_mapping(true);
        let readiness = super::super::normalize_inventory_readiness(&descriptor, readiness)
            .expect("normalized public readiness");
        assert_eq!(readiness.adapter_id, ADAPTER_ID);

        let stock = stock_item_from_row(&row(serde_json::json!({
            "SOURCE_KIND": "WAREHOUSE",
            "INVENTORY_ID": "INV-1",
            "STORAGE_ID": "STO-1",
            "STORAGE_NAME": "中心药库",
            "ORG_ID": "ORG-1",
            "SOURCE_PRODUCT_KEY": "D-1:P-1",
            "DRUG_NAME": "教学药品",
            "MINIMUM_UNIT": "支",
            "SALE_UNIT": "盒",
            "SALE_SPECIFICATION": "10 支/盒",
            "UNIT_SALE_FACTOR": "10",
            "QUANTITY": "2",
            "PURCHASE_PRICE": "8.50",
            "RETAIL_PRICE": "10.00",
            "PURCHASE_TOTAL": "17.00",
            "RETAIL_TOTAL": "20.00"
        })))
        .expect("public stock item");
        assert_eq!(stock.source_product_key, "D-1:P-1");
        assert_eq!(stock.unit_sale_factor, "10");
        assert_eq!(stock.purchase_total, "17.00");

        let organizations = HashSet::from(["ORG-1".to_string()]);
        let locations = HashSet::from(["STO-1".to_string()]);
        let normalized = super::super::normalize_inventory_stock_items(
            &descriptor,
            vec![stock],
            &organizations,
            &locations,
        )
        .expect("normalized public stock item");
        assert_eq!(normalized.len(), 1);
        let query = inventory_detail_query("public", &shape, &organizations, &locations)
            .expect("bounded detail query");
        assert!(query.contains("i.quantity * i.purchase_price"));
        assert!(!query.contains("i.purchase_total"));
        assert!(!query.contains("i.batch_code"));
        assert!(!query.contains("i.expiry_date"));
    }

    #[test]
    fn inventory_scope_filter_is_bounded_and_escaped() {
        let values = HashSet::from(["ORG'1".to_string(), "ORG-2".to_string()]);
        let filter = source_text_filter_list(&values, "机构").expect("safe filter");
        assert!(filter.contains("'ORG''1'"));
        assert!(source_text_filter_list(&HashSet::new(), "机构")
            .unwrap_err()
            .contains("请先选择"));
        let oversized = (0..501).map(|index| format!("ORG-{index}")).collect();
        assert!(source_text_filter_list(&oversized, "机构")
            .unwrap_err()
            .contains("超过 500"));
    }

    #[test]
    fn adapter_sdk_rejects_non_select_statements_before_connecting() {
        let profile = ConnectionProfile {
            kind: "postgresql".into(),
            host: "127.0.0.1".into(),
            port: 5432,
            database: "teaching".into(),
            username: "reader".into(),
            password: String::new(),
            schema: "public".into(),
            service_name: String::new(),
            driver: String::new(),
            connection_string: String::new(),
        };
        let error = read_source_select(&profile, "DELETE FROM his_drug", 1).unwrap_err();
        assert!(error.contains("SELECT") || error.contains("WITH"));
    }

    fn row(value: Value) -> Map<String, Value> {
        value.as_object().expect("object row").clone()
    }

    fn preview(rows: Vec<Map<String, Value>>) -> SourcePreview {
        SourcePreview {
            columns: Vec::new(),
            column_metadata: Vec::new(),
            rows,
            truncated: false,
            elapsed_ms: 0,
        }
    }
}
