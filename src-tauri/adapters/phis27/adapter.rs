use super::sdk::*;
use crate::legacy_phis27;

const ADAPTER_ID: &str = "PHIS27";
const ADAPTER_NAME: &str = "二系列phis";
const ADAPTER_VERSION: u32 = 1;

struct Phis27SourceAdapter;

impl MedicineSourceAdapter for Phis27SourceAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn inspect(&self, profile: &ConnectionProfile) -> Result<MedicineSourceInspection, String> {
        legacy_phis27::inspect(profile).map(medicine_inspection)
    }

    fn load(&self, request: &LoadMedicineSourceAdapterRequest) -> Result<SourcePreview, String> {
        legacy_phis27::load(&legacy_phis27::LoadPhis27Request {
            connection: request.connection.clone(),
            scope: request.scope.clone(),
            limit: request.limit,
        })
    }
}

impl InventorySourceAdapter for Phis27SourceAdapter {
    fn id(&self) -> &'static str {
        ADAPTER_ID
    }

    fn load_catalog(&self, profile: &ConnectionProfile) -> Result<InventorySourceCatalog, String> {
        legacy_phis27::load_inventory_reference_catalog(profile).map(inventory_catalog)
    }

    fn inspect(
        &self,
        store: &LocalStore,
        tenant_id: &str,
        request: &InspectInventorySourceAdapterRequest,
    ) -> Result<InventorySourceReadiness, String> {
        legacy_phis27::inspect_inventory(
            store,
            tenant_id,
            &legacy_phis27::InspectPhis27InventoryRequest {
                connection: request.connection.clone(),
                source_name: request.source_name.clone(),
            },
        )
        .map(inventory_readiness)
    }

    fn load_stock_items(
        &self,
        profile: &ConnectionProfile,
        selected_organization_ids: &HashSet<String>,
        selected_location_keys: &HashSet<String>,
    ) -> Result<Vec<InventorySourceStockItem>, String> {
        legacy_phis27::load_inventory_stock_items(
            profile,
            selected_organization_ids,
            selected_location_keys,
        )
        .map(|items| items.into_iter().map(inventory_stock_item).collect())
    }
}

fn medicine_inspection(inspection: legacy_phis27::Phis27Inspection) -> MedicineSourceInspection {
    MedicineSourceInspection {
        detected: inspection.detected,
        adapter_id: ADAPTER_ID.into(),
        adapter_name: ADAPTER_NAME.into(),
        adapter_version: ADAPTER_VERSION,
        schema: inspection.schema,
        checked_objects: inspection.checked_tables,
        missing_objects: inspection.missing_tables,
        object_structures: inspection.object_structures,
        scopes: inspection
            .scopes
            .into_iter()
            .map(|scope| MedicineSourceScope {
                id: scope.id,
                label: scope.label,
                description: scope.description,
                estimated_rows: scope.estimated_rows,
                medicine_count: scope.medicine_count,
                recommended: scope.recommended,
            })
            .collect(),
        metrics: vec![
            metric("TOTAL_MEDICINES", "通用药品", inspection.total_medicines),
            metric(
                "CONFIGURED_MEDICINES",
                "机构配置",
                inspection.configured_medicines,
            ),
            metric(
                "ACTIVE_CONFIGURED_MEDICINES",
                "在用药品",
                inspection.active_configured_medicines,
            ),
            metric("PRODUCT_ROWS", "厂家商品", inspection.product_rows),
            metric("STOCK_MEDICINES", "有库存药品", inspection.stock_medicines),
        ],
        guidance: vec![MedicineSourceGuidance {
            id: "MEDICINE_BASE_RELATIONS".into(),
            title: "主数据读取与自动合并".into(),
            body: "药品基础信息来自 YK_TYPK，厂家商品来自 YK_YPCD；机构在用范围只看 YK_CDXX，不关联 YK_YPXX、YF_YPXX。名称、规格、最小单位一致的多个 YPXH 会复用同一新药品，每个 YPXH:YPCD 仍分别保留迁移映射。".into(),
            tone: "SUCCESS".into(),
        }],
        compatibility: None,
        warnings: inspection.warnings,
        message: inspection.message,
    }
}

fn inventory_catalog(
    catalog: legacy_phis27::Phis27InventoryReferenceCatalog,
) -> InventorySourceCatalog {
    let organizations = catalog
        .organizations
        .into_iter()
        .map(|organization| {
            InventorySourceOrganization::new(organization.id, organization.name)
                .with_parent_id(organization.parent_id)
                .with_organization_type(organization.organization_type)
                .with_active(organization.active)
        })
        .collect();
    let locations = catalog
        .locations
        .into_iter()
        .map(|location| {
            InventorySourceLocationDefinition::new(
                location.source_kind,
                location.source_location_key,
                location.id,
                location.name,
                location.organization_id,
            )
            .with_category(location.category)
            .with_active(location.active)
        })
        .collect();
    InventorySourceCatalog::new(organizations, locations)
        .with_warnings(catalog.warnings)
        .with_message(catalog.message)
}

fn inventory_readiness(
    readiness: legacy_phis27::Phis27InventoryReadiness,
) -> InventorySourceReadiness {
    let locations = readiness
        .locations
        .into_iter()
        .map(|location| {
            let requires_source_location_resolution =
                location.mapping_status != "PENDING_TARGET_MAPPING";
            InventorySourceLocationReadiness::new(
                location.source_kind,
                location.source_location_key,
                location.source_location_name,
                location.organization_id,
            )
            .with_counts(
                location.source_option_count,
                location.stock_row_count,
                location.stock_group_count,
                location.medicine_count,
            )
            .with_mapping(
                requires_source_location_resolution,
                location.mapping_status,
                location.mapping_message,
            )
        })
        .collect();
    InventorySourceReadiness::new(readiness.schema, readiness.source_name, locations)
        .with_ready_for_location_mapping(readiness.ready_for_location_mapping)
        .with_medicine_ledger(
            readiness.mapped_medicine_count,
            readiness.unresolved_medicine_count,
            readiness.unresolved_source_keys,
        )
        .with_source_structure(
            readiness.checked_tables,
            readiness.missing_tables,
            readiness.object_structures,
        )
        .with_guidance(vec![InventorySourceGuidance::info(
            "PHIS27_INVENTORY_RELATIONS",
            "药库归属与库存数量依据",
            "药库库存通过 YK_YPXX 的 YKSB + YPXH 唯一关系定位；无法唯一判断时只将该药品列为待确认，不复制库存。药库数量取 YK_KCMX.KCSL，药房数量取 YF_KCMX.YPSL。",
        )])
        .with_warnings(readiness.warnings)
        .with_message(readiness.message)
}

fn inventory_stock_item(
    item: legacy_phis27::Phis27InventoryStockItem,
) -> InventorySourceStockItem {
    InventorySourceStockItem::new(
        item.source_kind,
        item.source_record_id,
        item.source_location_key,
        item.source_location_name,
        item.source_organization_id,
        item.source_product_key,
    )
    .with_medicine(
        item.drug_name,
        item.specification,
        item.dosage_form,
        item.minimum_unit,
    )
    .with_storage_packaging(
        item.sale_unit,
        item.sale_specification,
        item.unit_sale_factor,
    )
    .with_single_minimum_package_evidence(item.typk_unit_sale_factor, "YK_TYPK.ZXBZ")
    .with_product_packaging(item.product_sale_unit, item.product_unit_sale_factor)
    .with_product(item.factory_name, item.product_name)
    .with_quantity_and_prices(item.amount, item.price_pur, item.price_sale)
    .with_totals(item.purchase_total, item.retail_total)
    .with_batch(item.batch_code, item.effective_date)
}

fn metric(id: &str, label: &str, value: usize) -> MedicineSourceMetric {
    MedicineSourceMetric {
        id: id.into(),
        label: label.into(),
        value,
    }
}

static ADAPTER: Phis27SourceAdapter = Phis27SourceAdapter;

pub(super) fn binding() -> SourceAdapterBinding {
    SourceAdapterBinding::new("PHIS27_BUILTIN", Some(&ADAPTER), Some(&ADAPTER))
}
