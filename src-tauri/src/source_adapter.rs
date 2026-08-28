use crate::local_store::LocalStore;
use crate::model::{
    sanitize_source_object_structures, ConnectionProfile, SourceObjectStructure, SourcePreview,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

const ADAPTER_PACKAGE_SCHEMA_VERSION: u32 = 1;
include!(concat!(env!("OUT_DIR"), "/source_adapter_manifests.rs"));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterDescriptor {
    pub package_schema_version: u32,
    pub id: String,
    pub name: String,
    pub version: u32,
    pub template_compatible_from_version: u32,
    pub changes: Vec<SourceAdapterChange>,
    pub summary: String,
    pub source_modes: Vec<String>,
    pub database_families: Vec<String>,
    pub migration_tasks: Vec<String>,
    pub automatic_detection: bool,
    pub reusable_mapping_profiles: bool,
    pub built_in: bool,
    pub implementation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_compatibility: Option<SourceAdapterCompatibilityPolicy>,
    #[serde(default)]
    pub medicine_workflow: SourceAdapterMedicineWorkflow,
    #[serde(default)]
    pub inventory_workflow: SourceAdapterInventoryWorkflow,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterCompatibilityPolicy {
    pub product: String,
    pub mode: String,
    #[serde(default)]
    pub declared_versions: Vec<String>,
    pub gate: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterMedicineWorkflow {
    pub source_key_mode: String,
    pub source_key_field: String,
    pub source_key_label: String,
    pub mapping_preset: String,
    pub batch_source_type: String,
    pub factory_policy: String,
    pub legacy_profile_kind: String,
}

impl Default for SourceAdapterMedicineWorkflow {
    fn default() -> Self {
        Self {
            source_key_mode: "USER_SELECTED".into(),
            source_key_field: String::new(),
            source_key_label: String::new(),
            mapping_preset: "GUIDED".into(),
            batch_source_type: "GENERIC".into(),
            factory_policy: "OPTIONAL_CREATE".into(),
            legacy_profile_kind: "NONE".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterInventoryWorkflow {
    pub normalized_location_kinds: Vec<String>,
    pub source_product_key_label: String,
    #[serde(default = "default_inventory_stock_key_mode")]
    pub source_stock_key_mode: String,
    pub write_mode: String,
    pub organization_mapping: String,
    pub medicine_ledger: String,
    pub trial_policy: String,
    pub undo_policy: String,
}

impl Default for SourceAdapterInventoryWorkflow {
    fn default() -> Self {
        Self {
            normalized_location_kinds: Vec::new(),
            source_product_key_label: String::new(),
            source_stock_key_mode: default_inventory_stock_key_mode(),
            write_mode: "FIRST_STOCKTAKE".into(),
            organization_mapping: "REQUIRED".into(),
            medicine_ledger: "REQUIRED".into(),
            trial_policy: "PER_TARGET_STORAGE".into(),
            undo_policy: "VERIFIED_BATCH_ONLY".into(),
        }
    }
}

fn default_inventory_stock_key_mode() -> String {
    "ADAPTER_SCOPED_V1".into()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceAdapterChange {
    pub version: u32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MedicineSourceScope {
    pub id: String,
    pub label: String,
    pub description: String,
    pub estimated_rows: usize,
    pub medicine_count: usize,
    pub recommended: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MedicineSourceMetric {
    pub id: String,
    pub label: String,
    pub value: usize,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MedicineSourceGuidance {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tone: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceStructureCompatibilityItem {
    pub path: String,
    pub message: String,
    pub used_by: Vec<String>,
    pub fallback: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SourceStructureCompatibility {
    pub status: String,
    pub task: String,
    pub blockers: Vec<SourceStructureCompatibilityItem>,
    pub reviews: Vec<SourceStructureCompatibilityItem>,
    pub compatible_fallbacks: Vec<SourceStructureCompatibilityItem>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MedicineSourceInspection {
    pub detected: bool,
    pub adapter_id: String,
    pub adapter_name: String,
    pub adapter_version: u32,
    pub schema: String,
    pub checked_objects: Vec<String>,
    pub missing_objects: Vec<String>,
    #[serde(default)]
    pub object_structures: Vec<SourceObjectStructure>,
    pub scopes: Vec<MedicineSourceScope>,
    pub metrics: Vec<MedicineSourceMetric>,
    pub guidance: Vec<MedicineSourceGuidance>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<SourceStructureCompatibility>,
    pub warnings: Vec<String>,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectMedicineSourceAdapterRequest {
    pub adapter_id: String,
    pub connection: ConnectionProfile,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceContract {
    adapter_id: String,
    source_structure: SourceAcceptanceStructure,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceStructure {
    #[serde(default)]
    objects: Vec<SourceAcceptanceObject>,
    #[serde(default)]
    object_groups: Vec<SourceAcceptanceObjectGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceObject {
    name: String,
    requirement: String,
    #[serde(default)]
    applies_to_tasks: Vec<String>,
    #[serde(default)]
    required_for_tasks: Vec<String>,
    #[serde(default)]
    required_when_objects_present: Vec<String>,
    #[serde(default)]
    used_by: Vec<String>,
    #[serde(default)]
    fallback: String,
    #[serde(default)]
    columns: Vec<SourceAcceptanceColumn>,
    #[serde(default)]
    column_groups: Vec<SourceAcceptanceColumnGroup>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceColumn {
    name: String,
    requirement: String,
    #[serde(default)]
    applies_to_tasks: Vec<String>,
    #[serde(default)]
    required_for_tasks: Vec<String>,
    #[serde(default)]
    required_when_objects_present: Vec<String>,
    #[serde(default)]
    used_by: Vec<String>,
    #[serde(default)]
    fallback: String,
    #[serde(default)]
    accepted_type_families: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceColumnGroup {
    name: String,
    #[serde(default)]
    columns: Vec<String>,
    requirement: String,
    #[serde(default)]
    applies_to_tasks: Vec<String>,
    #[serde(default)]
    required_for_tasks: Vec<String>,
    #[serde(default)]
    required_when_objects_present: Vec<String>,
    #[serde(default)]
    used_by: Vec<String>,
    #[serde(default)]
    fallback: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SourceAcceptanceObjectGroup {
    name: String,
    #[serde(default)]
    objects: Vec<String>,
    requirement: String,
    #[serde(default)]
    applies_to_tasks: Vec<String>,
    #[serde(default)]
    required_for_tasks: Vec<String>,
    #[serde(default)]
    used_by: Vec<String>,
    #[serde(default)]
    fallback: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadMedicineSourceAdapterRequest {
    pub adapter_id: String,
    pub connection: ConnectionProfile,
    pub scope: String,
    #[serde(default = "default_limit")]
    pub limit: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceOrganization {
    pub id: String,
    pub name: String,
    pub parent_id: String,
    pub organization_type: String,
    pub active: bool,
}

impl InventorySourceOrganization {
    pub fn new(id: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            parent_id: String::new(),
            organization_type: String::new(),
            active: true,
        }
    }

    pub fn with_parent_id(mut self, parent_id: impl Into<String>) -> Self {
        self.parent_id = parent_id.into();
        self
    }

    pub fn with_organization_type(mut self, organization_type: impl Into<String>) -> Self {
        self.organization_type = organization_type.into();
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceLocationDefinition {
    pub source_kind: String,
    pub source_location_key: String,
    pub id: String,
    pub name: String,
    pub organization_id: String,
    pub category: String,
    pub active: bool,
}

impl InventorySourceLocationDefinition {
    pub fn new(
        source_kind: impl Into<String>,
        source_location_key: impl Into<String>,
        id: impl Into<String>,
        name: impl Into<String>,
        organization_id: impl Into<String>,
    ) -> Self {
        Self {
            source_kind: source_kind.into(),
            source_location_key: source_location_key.into(),
            id: id.into(),
            name: name.into(),
            organization_id: organization_id.into(),
            category: String::new(),
            active: true,
        }
    }

    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    pub fn with_active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceCatalog {
    pub adapter_id: String,
    pub adapter_name: String,
    pub adapter_version: u32,
    pub organizations: Vec<InventorySourceOrganization>,
    pub locations: Vec<InventorySourceLocationDefinition>,
    pub warnings: Vec<String>,
    pub message: String,
}

impl InventorySourceCatalog {
    pub fn new(
        organizations: Vec<InventorySourceOrganization>,
        locations: Vec<InventorySourceLocationDefinition>,
    ) -> Self {
        Self {
            adapter_id: String::new(),
            adapter_name: String::new(),
            adapter_version: 0,
            organizations,
            locations,
            warnings: Vec::new(),
            message: String::new(),
        }
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceLocationReadiness {
    pub source_kind: String,
    pub source_location_key: String,
    pub source_location_name: String,
    pub organization_id: String,
    pub source_option_count: usize,
    pub stock_row_count: usize,
    pub stock_group_count: usize,
    pub medicine_count: usize,
    pub requires_source_location_resolution: bool,
    pub mapping_status: String,
    pub mapping_message: String,
}

impl InventorySourceLocationReadiness {
    pub fn new(
        source_kind: impl Into<String>,
        source_location_key: impl Into<String>,
        source_location_name: impl Into<String>,
        organization_id: impl Into<String>,
    ) -> Self {
        Self {
            source_kind: source_kind.into(),
            source_location_key: source_location_key.into(),
            source_location_name: source_location_name.into(),
            organization_id: organization_id.into(),
            source_option_count: 0,
            stock_row_count: 0,
            stock_group_count: 0,
            medicine_count: 0,
            requires_source_location_resolution: false,
            mapping_status: "PENDING_TARGET_MAPPING".into(),
            mapping_message: "待选择目标库房".into(),
        }
    }

    pub fn with_counts(
        mut self,
        source_option_count: usize,
        stock_row_count: usize,
        stock_group_count: usize,
        medicine_count: usize,
    ) -> Self {
        self.source_option_count = source_option_count;
        self.stock_row_count = stock_row_count;
        self.stock_group_count = stock_group_count;
        self.medicine_count = medicine_count;
        self
    }

    pub fn with_mapping(
        mut self,
        requires_source_location_resolution: bool,
        status: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        self.requires_source_location_resolution = requires_source_location_resolution;
        self.mapping_status = status.into();
        self.mapping_message = message.into();
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceReadiness {
    pub adapter_id: String,
    pub adapter_name: String,
    pub adapter_version: u32,
    pub ready_for_location_mapping: bool,
    pub schema: String,
    pub source_name: String,
    pub stock_row_count: usize,
    pub stock_group_count: usize,
    pub medicine_count: usize,
    pub mapped_medicine_count: usize,
    pub unresolved_medicine_count: usize,
    pub locations: Vec<InventorySourceLocationReadiness>,
    pub unresolved_source_keys: Vec<String>,
    #[serde(default)]
    pub guidance: Vec<InventorySourceGuidance>,
    #[serde(default)]
    pub checked_objects: Vec<String>,
    #[serde(default)]
    pub missing_objects: Vec<String>,
    #[serde(default)]
    pub object_structures: Vec<SourceObjectStructure>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility: Option<SourceStructureCompatibility>,
    pub warnings: Vec<String>,
    pub message: String,
}

impl InventorySourceReadiness {
    pub fn new(
        schema: impl Into<String>,
        source_name: impl Into<String>,
        locations: Vec<InventorySourceLocationReadiness>,
    ) -> Self {
        let stock_row_count = locations.iter().map(|item| item.stock_row_count).sum();
        let stock_group_count = locations.iter().map(|item| item.stock_group_count).sum();
        let medicine_count = locations.iter().map(|item| item.medicine_count).sum();
        Self {
            adapter_id: String::new(),
            adapter_name: String::new(),
            adapter_version: 0,
            ready_for_location_mapping: !locations.is_empty(),
            schema: schema.into(),
            source_name: source_name.into(),
            stock_row_count,
            stock_group_count,
            medicine_count,
            mapped_medicine_count: 0,
            unresolved_medicine_count: 0,
            locations,
            unresolved_source_keys: Vec::new(),
            guidance: Vec::new(),
            checked_objects: Vec::new(),
            missing_objects: Vec::new(),
            object_structures: Vec::new(),
            compatibility: None,
            warnings: Vec::new(),
            message: String::new(),
        }
    }

    pub fn with_ready_for_location_mapping(mut self, ready: bool) -> Self {
        self.ready_for_location_mapping = ready;
        self
    }

    pub fn with_medicine_ledger(
        mut self,
        mapped_medicine_count: usize,
        unresolved_medicine_count: usize,
        unresolved_source_keys: Vec<String>,
    ) -> Self {
        self.mapped_medicine_count = mapped_medicine_count;
        self.unresolved_medicine_count = unresolved_medicine_count;
        self.unresolved_source_keys = unresolved_source_keys;
        self
    }

    pub fn with_guidance(mut self, guidance: Vec<InventorySourceGuidance>) -> Self {
        self.guidance = guidance;
        self
    }

    pub fn with_source_structure(
        mut self,
        checked_objects: Vec<String>,
        missing_objects: Vec<String>,
        object_structures: Vec<SourceObjectStructure>,
    ) -> Self {
        self.checked_objects = checked_objects;
        self.missing_objects = missing_objects;
        self.object_structures = object_structures;
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }

    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct InventorySourceGuidance {
    pub id: String,
    pub title: String,
    pub body: String,
    pub tone: String,
}

impl InventorySourceGuidance {
    pub fn info(id: impl Into<String>, title: impl Into<String>, body: impl Into<String>) -> Self {
        Self::new(id, title, body, "INFO")
    }

    pub fn success(
        id: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::new(id, title, body, "SUCCESS")
    }

    pub fn warning(
        id: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self::new(id, title, body, "WARNING")
    }

    fn new(
        id: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
        tone: &'static str,
    ) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            body: body.into(),
            tone: tone.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InventorySourceStockItem {
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
    pub single_minimum_package_factor: String,
    pub single_minimum_package_factor_source: String,
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

impl InventorySourceStockItem {
    pub fn new(
        source_kind: impl Into<String>,
        source_record_id: impl Into<String>,
        source_location_key: impl Into<String>,
        source_location_name: impl Into<String>,
        source_organization_id: impl Into<String>,
        source_product_key: impl Into<String>,
    ) -> Self {
        Self {
            source_kind: source_kind.into(),
            source_record_id: source_record_id.into(),
            source_location_key: source_location_key.into(),
            source_location_name: source_location_name.into(),
            source_organization_id: source_organization_id.into(),
            source_product_key: source_product_key.into(),
            drug_name: String::new(),
            specification: String::new(),
            dosage_form: String::new(),
            minimum_unit: String::new(),
            sale_unit: String::new(),
            sale_specification: String::new(),
            unit_sale_factor: String::new(),
            single_minimum_package_factor: String::new(),
            single_minimum_package_factor_source: String::new(),
            product_sale_unit: String::new(),
            product_unit_sale_factor: String::new(),
            factory_name: String::new(),
            product_name: String::new(),
            amount: String::new(),
            price_pur: String::new(),
            price_sale: String::new(),
            purchase_total: String::new(),
            retail_total: String::new(),
            batch_code: String::new(),
            effective_date: String::new(),
        }
    }

    pub fn with_medicine(
        mut self,
        drug_name: impl Into<String>,
        specification: impl Into<String>,
        dosage_form: impl Into<String>,
        minimum_unit: impl Into<String>,
    ) -> Self {
        self.drug_name = drug_name.into();
        self.specification = specification.into();
        self.dosage_form = dosage_form.into();
        self.minimum_unit = minimum_unit.into();
        self
    }

    pub fn with_storage_packaging(
        mut self,
        sale_unit: impl Into<String>,
        sale_specification: impl Into<String>,
        unit_sale_factor: impl Into<String>,
    ) -> Self {
        self.sale_unit = sale_unit.into();
        self.sale_specification = sale_specification.into();
        self.unit_sale_factor = unit_sale_factor.into();
        self
    }

    pub fn with_single_minimum_package_evidence(
        mut self,
        factor: impl Into<String>,
        source: impl Into<String>,
    ) -> Self {
        self.single_minimum_package_factor = factor.into();
        self.single_minimum_package_factor_source = source.into();
        self
    }

    pub fn with_product_packaging(
        mut self,
        sale_unit: impl Into<String>,
        unit_sale_factor: impl Into<String>,
    ) -> Self {
        self.product_sale_unit = sale_unit.into();
        self.product_unit_sale_factor = unit_sale_factor.into();
        self
    }

    pub fn with_product(
        mut self,
        factory_name: impl Into<String>,
        product_name: impl Into<String>,
    ) -> Self {
        self.factory_name = factory_name.into();
        self.product_name = product_name.into();
        self
    }

    pub fn with_quantity_and_prices(
        mut self,
        amount: impl Into<String>,
        price_pur: impl Into<String>,
        price_sale: impl Into<String>,
    ) -> Self {
        self.amount = amount.into();
        self.price_pur = price_pur.into();
        self.price_sale = price_sale.into();
        self
    }

    pub fn with_totals(
        mut self,
        purchase_total: impl Into<String>,
        retail_total: impl Into<String>,
    ) -> Self {
        self.purchase_total = purchase_total.into();
        self.retail_total = retail_total.into();
        self
    }

    pub fn with_batch(
        mut self,
        batch_code: impl Into<String>,
        effective_date: impl Into<String>,
    ) -> Self {
        self.batch_code = batch_code.into();
        self.effective_date = effective_date.into();
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadInventorySourceCatalogRequest {
    pub adapter_id: String,
    pub connection: ConnectionProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectInventorySourceAdapterRequest {
    pub adapter_id: String,
    pub connection: ConnectionProfile,
    pub source_name: String,
}

pub(crate) trait MedicineSourceAdapter: Sync {
    fn id(&self) -> &'static str;
    fn inspect(&self, profile: &ConnectionProfile) -> Result<MedicineSourceInspection, String>;
    fn load(&self, request: &LoadMedicineSourceAdapterRequest) -> Result<SourcePreview, String>;
}

pub(crate) trait InventorySourceAdapter: Sync {
    fn id(&self) -> &'static str;
    fn load_catalog(&self, profile: &ConnectionProfile) -> Result<InventorySourceCatalog, String>;
    fn inspect(
        &self,
        store: &LocalStore,
        tenant_id: &str,
        request: &InspectInventorySourceAdapterRequest,
    ) -> Result<InventorySourceReadiness, String>;
    fn load_stock_items(
        &self,
        profile: &ConnectionProfile,
        selected_organization_ids: &HashSet<String>,
        selected_location_keys: &HashSet<String>,
    ) -> Result<Vec<InventorySourceStockItem>, String>;
}

#[derive(Clone, Copy)]
pub(crate) struct SourceAdapterBinding {
    implementation: &'static str,
    medicine: Option<&'static dyn MedicineSourceAdapter>,
    inventory: Option<&'static dyn InventorySourceAdapter>,
}

/// Execute one bounded read-only source query through the application's
/// database-family router. Adapter implementations remain synchronous because
/// the desktop commands already run them away from the WebView thread.
pub(crate) fn read_source_select(
    profile: &ConnectionProfile,
    query: impl Into<String>,
    limit: u32,
) -> Result<SourcePreview, String> {
    let request = crate::model::SourcePreviewRequest {
        connection: profile.clone(),
        query: query.into(),
        limit: limit.clamp(1, 10_000),
    };
    tauri::async_runtime::block_on(crate::datasource::preview_source(&request))
}

/// Quote a source object through the same database-family rules used by the
/// generic source browser. Adapter code should never interpolate a project
/// schema or object name directly into SQL.
pub(crate) fn source_qualified_object(
    database_family: &str,
    schema: &str,
    object: &str,
) -> Result<String, String> {
    crate::datasource::source_object_qualified_name(database_family, schema.trim(), object.trim())
}

/// Build one bounded, deterministic list of SQL text literals for source-only
/// scope filters. This is intentionally limited to identifiers selected by the
/// user; adapters must not use it for arbitrary business values or unbounded
/// row data.
pub(crate) fn source_text_filter_list(
    values: &HashSet<String>,
    label: &str,
) -> Result<String, String> {
    if values.is_empty() {
        return Err(format!("请先选择本批{label}"));
    }
    if values.len() > 500 {
        return Err(format!("本批{label}超过 500 项，请缩小范围"));
    }
    let mut values = values.iter().collect::<Vec<_>>();
    values.sort();
    values
        .into_iter()
        .map(|value| {
            let value = value.trim();
            if value.is_empty()
                || value.chars().count() > 256
                || value.chars().any(char::is_control)
            {
                return Err(format!("本批{label}包含无效来源标识"));
            }
            Ok(format!("'{}'", value.replace('\'', "''")))
        })
        .collect::<Result<Vec<_>, String>>()
        .map(|items| items.join(", "))
}

impl SourceAdapterBinding {
    pub(crate) const fn new(
        implementation: &'static str,
        medicine: Option<&'static dyn MedicineSourceAdapter>,
        inventory: Option<&'static dyn InventorySourceAdapter>,
    ) -> Self {
        Self {
            implementation,
            medicine,
            inventory,
        }
    }
}

/// Intentional compile-time surface for package-local source adapters.
///
/// Adapter packages should import only this module instead of the parent module,
/// so unrelated orchestration internals cannot accidentally become dependencies.
pub(crate) mod sdk {
    pub(crate) use super::{
        read_source_select, source_qualified_object, source_text_filter_list,
        InspectInventorySourceAdapterRequest, InventorySourceAdapter, InventorySourceCatalog,
        InventorySourceGuidance, InventorySourceLocationDefinition,
        InventorySourceLocationReadiness, InventorySourceOrganization, InventorySourceReadiness,
        InventorySourceStockItem, LoadMedicineSourceAdapterRequest, MedicineSourceAdapter,
        MedicineSourceGuidance, MedicineSourceInspection, MedicineSourceMetric,
        MedicineSourceScope, SourceAdapterBinding,
    };
    pub(crate) use crate::local_store::LocalStore;
    #[allow(unused_imports)]
    pub(crate) use crate::model::{
        ConnectionProfile, SourceObjectColumnStructure, SourceObjectStructure, SourcePreview,
    };
    pub(crate) use std::collections::HashSet;
}

#[cfg(test)]
#[path = "../adapters/_examples/standard_his/adapter.rs"]
mod standard_his_adapter_example;

pub fn registry() -> Result<Vec<SourceAdapterDescriptor>, String> {
    ADAPTER_MANIFESTS
        .iter()
        .map(|(package_name, source)| parse_manifest(package_name, source))
        .collect()
}

pub fn list() -> Result<Vec<SourceAdapterDescriptor>, String> {
    let adapters = registry()?;
    let mut ids = HashSet::new();
    if adapters.iter().any(|adapter| !ids.insert(&adapter.id)) {
        return Err("来源适配器注册表存在重复 ID".into());
    }
    Ok(adapters)
}

fn parse_manifest(package_name: &str, source: &str) -> Result<SourceAdapterDescriptor, String> {
    let manifest_value: serde_json::Value = serde_json::from_str(source)
        .map_err(|error| format!("来源适配器包 {package_name} 清单解析失败：{error}"))?;
    let has_medicine_workflow = manifest_value.get("medicineWorkflow").is_some();
    let has_inventory_workflow = manifest_value.get("inventoryWorkflow").is_some();
    let has_inventory_stock_key_mode = manifest_value
        .pointer("/inventoryWorkflow/sourceStockKeyMode")
        .is_some();
    let has_source_compatibility = manifest_value.get("sourceCompatibility").is_some();
    let mut adapter: SourceAdapterDescriptor = serde_json::from_value(manifest_value)
        .map_err(|error| format!("来源适配器包 {package_name} 清单解析失败：{error}"))?;
    if adapter.package_schema_version != ADAPTER_PACKAGE_SCHEMA_VERSION {
        return Err(format!(
            "来源适配器包 {} 的清单版本 {} 不受支持",
            adapter.id, adapter.package_schema_version
        ));
    }
    if !has_source_compatibility && adapter.automatic_detection {
        adapter.source_compatibility = Some(SourceAdapterCompatibilityPolicy {
            product: adapter.name.trim().to_string(),
            mode: "STRUCTURE_CONTRACT".into(),
            declared_versions: Vec::new(),
            gate:
                "旧版适配器未声明厂商版本范围；连接后必须通过任务级来源表字段契约和项目变体回归。"
                    .into(),
        });
    }
    if !has_medicine_workflow && adapter.automatic_detection {
        adapter.medicine_workflow = if adapter
            .implementation
            .trim()
            .eq_ignore_ascii_case("PHIS27_BUILTIN")
        {
            SourceAdapterMedicineWorkflow {
                source_key_mode: "ADAPTER_PROVIDED".into(),
                source_key_field: "SOURCE_KEY".into(),
                source_key_label: "YPXH:YPCD".into(),
                mapping_preset: "PHIS27".into(),
                batch_source_type: "PHIS27".into(),
                factory_policy: "ADAPTER_MANAGED".into(),
                legacy_profile_kind: "PHIS27_V2".into(),
            }
        } else {
            SourceAdapterMedicineWorkflow {
                source_key_mode: "ADAPTER_PROVIDED".into(),
                source_key_field: "SOURCE_KEY".into(),
                source_key_label: "适配器稳定来源键".into(),
                ..SourceAdapterMedicineWorkflow::default()
            }
        };
    }
    if !has_inventory_workflow
        && adapter
            .migration_tasks
            .iter()
            .any(|task| task.trim().eq_ignore_ascii_case("INVENTORY"))
    {
        adapter.inventory_workflow = SourceAdapterInventoryWorkflow {
            normalized_location_kinds: vec!["WAREHOUSE".into(), "PHARMACY".into()],
            source_product_key_label: "来源药品商品键".into(),
            ..SourceAdapterInventoryWorkflow::default()
        };
    }
    if !has_inventory_stock_key_mode
        && adapter
            .migration_tasks
            .iter()
            .any(|task| task.trim().eq_ignore_ascii_case("INVENTORY"))
    {
        adapter.inventory_workflow.source_stock_key_mode = if adapter
            .implementation
            .trim()
            .eq_ignore_ascii_case("PHIS27_BUILTIN")
        {
            "PHIS27_LEGACY".into()
        } else {
            default_inventory_stock_key_mode()
        };
    }
    adapter.id = adapter.id.trim().to_ascii_uppercase();
    adapter.database_families = adapter
        .database_families
        .into_iter()
        .map(|family| family.trim().to_ascii_lowercase())
        .filter(|family| !family.is_empty())
        .collect();
    adapter.migration_tasks = adapter
        .migration_tasks
        .into_iter()
        .map(|task| task.trim().to_ascii_uppercase())
        .filter(|task| !task.is_empty())
        .collect();
    if let Some(policy) = adapter.source_compatibility.as_mut() {
        policy.product = policy.product.trim().to_string();
        policy.mode = policy.mode.trim().to_ascii_uppercase();
        policy.declared_versions = policy
            .declared_versions
            .iter()
            .map(|version| version.trim().to_string())
            .filter(|version| !version.is_empty())
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();
        policy.declared_versions.sort();
        policy.gate = policy.gate.trim().to_string();
    }
    adapter.medicine_workflow.source_key_mode = adapter
        .medicine_workflow
        .source_key_mode
        .trim()
        .to_ascii_uppercase();
    adapter.medicine_workflow.source_key_field = adapter
        .medicine_workflow
        .source_key_field
        .trim()
        .to_string();
    adapter.medicine_workflow.source_key_label = adapter
        .medicine_workflow
        .source_key_label
        .trim()
        .to_string();
    adapter.medicine_workflow.mapping_preset = adapter
        .medicine_workflow
        .mapping_preset
        .trim()
        .to_ascii_uppercase();
    adapter.medicine_workflow.batch_source_type = adapter
        .medicine_workflow
        .batch_source_type
        .trim()
        .to_ascii_uppercase();
    adapter.medicine_workflow.factory_policy = adapter
        .medicine_workflow
        .factory_policy
        .trim()
        .to_ascii_uppercase();
    adapter.medicine_workflow.legacy_profile_kind = adapter
        .medicine_workflow
        .legacy_profile_kind
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.normalized_location_kinds = adapter
        .inventory_workflow
        .normalized_location_kinds
        .into_iter()
        .map(|kind| kind.trim().to_ascii_uppercase())
        .filter(|kind| !kind.is_empty())
        .collect();
    adapter.inventory_workflow.source_product_key_label = adapter
        .inventory_workflow
        .source_product_key_label
        .trim()
        .to_string();
    adapter.inventory_workflow.source_stock_key_mode = adapter
        .inventory_workflow
        .source_stock_key_mode
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.write_mode = adapter
        .inventory_workflow
        .write_mode
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.organization_mapping = adapter
        .inventory_workflow
        .organization_mapping
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.medicine_ledger = adapter
        .inventory_workflow
        .medicine_ledger
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.trial_policy = adapter
        .inventory_workflow
        .trial_policy
        .trim()
        .to_ascii_uppercase();
    adapter.inventory_workflow.undo_policy = adapter
        .inventory_workflow
        .undo_policy
        .trim()
        .to_ascii_uppercase();
    if adapter.id.is_empty()
        || adapter.name.trim().is_empty()
        || adapter.version == 0
        || adapter.template_compatible_from_version == 0
        || adapter.template_compatible_from_version > adapter.version
        || !adapter
            .changes
            .iter()
            .any(|change| change.version == adapter.version && !change.summary.trim().is_empty())
        || adapter.implementation.trim().is_empty()
    {
        return Err(format!("来源适配器包 {package_name} 缺少必要标识或版本"));
    }
    if adapter.automatic_detection && adapter.source_compatibility.is_none() {
        return Err(format!(
            "来源适配器包 {package_name} 缺少 sourceCompatibility 来源兼容策略"
        ));
    }
    if let Some(policy) = adapter.source_compatibility.as_ref() {
        let declared_version_policy = policy.mode == "DECLARED_VERSION_RANGE";
        if policy.product.is_empty()
            || policy.product.chars().count() > 120
            || policy.gate.is_empty()
            || policy.gate.chars().count() > 300
            || !matches!(
                policy.mode.as_str(),
                "STRUCTURE_CONTRACT" | "DECLARED_VERSION_RANGE"
            )
            || policy.declared_versions.len() > 20
            || policy
                .declared_versions
                .iter()
                .any(|version| version.chars().count() > 80)
            || (declared_version_policy && policy.declared_versions.is_empty())
            || (!declared_version_policy && !policy.declared_versions.is_empty())
        {
            return Err(format!(
                "来源适配器包 {package_name} 的 sourceCompatibility 来源兼容策略无效"
            ));
        }
    }
    if !matches!(
        adapter.medicine_workflow.source_key_mode.as_str(),
        "USER_SELECTED" | "ADAPTER_PROVIDED"
    ) || !matches!(
        adapter.medicine_workflow.factory_policy.as_str(),
        "OPTIONAL_CREATE" | "ADAPTER_MANAGED"
    ) || !matches!(
        adapter.medicine_workflow.mapping_preset.as_str(),
        "GUIDED" | "PHIS27"
    ) || !matches!(
        adapter.medicine_workflow.batch_source_type.as_str(),
        "GENERIC" | "PHIS27"
    ) || !matches!(
        adapter.medicine_workflow.legacy_profile_kind.as_str(),
        "NONE" | "PHIS27_V2"
    ) || (adapter.automatic_detection
        && adapter.medicine_workflow.source_key_mode != "ADAPTER_PROVIDED")
        || (adapter.medicine_workflow.source_key_mode == "ADAPTER_PROVIDED"
            && (adapter.medicine_workflow.source_key_field.is_empty()
                || adapter.medicine_workflow.source_key_label.is_empty()
                || !adapter
                    .medicine_workflow
                    .source_key_field
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric() || character == '_')))
    {
        return Err(format!(
            "来源适配器包 {package_name} 的 medicineWorkflow 工作流声明无效"
        ));
    }
    let inventory_capable = adapter
        .migration_tasks
        .iter()
        .any(|task| task == "INVENTORY");
    let inventory_kinds = adapter
        .inventory_workflow
        .normalized_location_kinds
        .iter()
        .collect::<HashSet<_>>();
    if inventory_capable
        && (adapter
            .inventory_workflow
            .normalized_location_kinds
            .is_empty()
            || inventory_kinds.len() != adapter.inventory_workflow.normalized_location_kinds.len()
            || adapter
                .inventory_workflow
                .normalized_location_kinds
                .iter()
                .any(|kind| !matches!(kind.as_str(), "WAREHOUSE" | "PHARMACY"))
            || adapter
                .inventory_workflow
                .source_product_key_label
                .is_empty()
            || !matches!(
                adapter.inventory_workflow.source_stock_key_mode.as_str(),
                "ADAPTER_SCOPED_V1" | "PHIS27_LEGACY"
            )
            || (adapter.inventory_workflow.source_stock_key_mode == "PHIS27_LEGACY"
                && !adapter
                    .implementation
                    .trim()
                    .eq_ignore_ascii_case("PHIS27_BUILTIN"))
            || adapter.inventory_workflow.write_mode != "FIRST_STOCKTAKE"
            || adapter.inventory_workflow.organization_mapping != "REQUIRED"
            || adapter.inventory_workflow.medicine_ledger != "REQUIRED"
            || adapter.inventory_workflow.trial_policy != "PER_TARGET_STORAGE"
            || adapter.inventory_workflow.undo_policy != "VERIFIED_BATCH_ONLY")
    {
        return Err(format!(
            "来源适配器包 {package_name} 的 inventoryWorkflow 库存安全契约无效"
        ));
    }
    Ok(adapter)
}

fn source_acceptance_contract(
    adapter_id: &str,
) -> Result<Option<SourceAcceptanceContract>, String> {
    for (_, source) in ADAPTER_ACCEPTANCE_CONTRACTS {
        let contract = serde_json::from_str::<SourceAcceptanceContract>(source)
            .map_err(|error| format!("来源适配器验收契约无法读取：{error}"))?;
        if contract.adapter_id.eq_ignore_ascii_case(adapter_id) {
            return Ok(Some(contract));
        }
    }
    Ok(None)
}

fn task_applies(tasks: &[String], task: &str) -> bool {
    tasks.is_empty() || tasks.iter().any(|item| item.eq_ignore_ascii_case(task))
}

fn effective_source_requirement(
    requirement: &str,
    required_for_tasks: &[String],
    required_when_objects_present: &[String],
    checked_objects: &HashSet<String>,
    task: &str,
) -> String {
    if requirement.eq_ignore_ascii_case("CONDITIONAL")
        && required_for_tasks
            .iter()
            .any(|item| item.eq_ignore_ascii_case(task))
    {
        "REQUIRED".into()
    } else if requirement.eq_ignore_ascii_case("CONDITIONAL")
        && !required_when_objects_present.is_empty()
    {
        if required_when_objects_present
            .iter()
            .any(|item| checked_objects.contains(&item.trim().to_ascii_uppercase()))
        {
            "REQUIRED".into()
        } else {
            "OPTIONAL".into()
        }
    } else {
        requirement.trim().to_ascii_uppercase()
    }
}

fn source_type_family(data_type: &str) -> &'static str {
    let value = data_type.trim().to_ascii_uppercase();
    if value.is_empty() {
        ""
    } else if value.contains("BOOL") || value == "BIT" {
        "BOOLEAN"
    } else if ["DATE", "TIME", "INTERVAL", "YEAR"]
        .iter()
        .any(|token| value.contains(token))
    {
        "TEMPORAL"
    } else if ["BLOB", "BINARY", "VARBINARY", "BYTEA", "RAW", "IMAGE"]
        .iter()
        .any(|token| value.contains(token))
    {
        "BINARY"
    } else if [
        "CHAR", "TEXT", "CLOB", "STRING", "JSON", "XML", "UUID", "ENUM", "SET",
    ]
    .iter()
    .any(|token| value.contains(token))
    {
        "CHARACTER"
    } else if [
        "NUMBER", "NUMERIC", "DECIMAL", "INT", "FLOAT", "DOUBLE", "REAL", "MONEY",
    ]
    .iter()
    .any(|token| value.contains(token))
    {
        "NUMERIC"
    } else {
        "OTHER"
    }
}

fn add_source_compatibility_item(
    requirement: &str,
    item: SourceStructureCompatibilityItem,
    blockers: &mut Vec<SourceStructureCompatibilityItem>,
    reviews: &mut Vec<SourceStructureCompatibilityItem>,
    compatible_fallbacks: &mut Vec<SourceStructureCompatibilityItem>,
) {
    match requirement {
        "REQUIRED" => blockers.push(item),
        "CONDITIONAL" => reviews.push(item),
        _ => compatible_fallbacks.push(item),
    }
}

fn unavailable_source_compatibility(task: &str) -> SourceStructureCompatibility {
    SourceStructureCompatibility {
        status: "UNAVAILABLE".into(),
        task: task.into(),
        blockers: Vec::new(),
        reviews: Vec::new(),
        compatible_fallbacks: Vec::new(),
        message: "当前适配器没有可用于应用内判断的表字段契约".into(),
    }
}

fn evaluate_source_structure_compatibility(
    descriptor: &SourceAdapterDescriptor,
    checked_objects: &[String],
    object_structures: &[SourceObjectStructure],
    task: &str,
) -> Result<SourceStructureCompatibility, String> {
    let Some(contract) = source_acceptance_contract(&descriptor.id)? else {
        return Ok(unavailable_source_compatibility(task));
    };
    let checked = checked_objects
        .iter()
        .map(|name| name.trim().to_ascii_uppercase())
        .collect::<HashSet<_>>();
    let structures = object_structures
        .iter()
        .map(|structure| (structure.name.trim().to_ascii_uppercase(), structure))
        .collect::<HashMap<_, _>>();
    let mut blockers = Vec::new();
    let mut reviews = Vec::new();
    let mut compatible_fallbacks = Vec::new();

    for object in contract.source_structure.objects {
        if !task_applies(&object.applies_to_tasks, task) {
            continue;
        }
        let object_name = object.name.trim().to_ascii_uppercase();
        let object_requirement = effective_source_requirement(
            &object.requirement,
            &object.required_for_tasks,
            &object.required_when_objects_present,
            &checked,
            task,
        );
        if !checked.contains(&object_name) {
            add_source_compatibility_item(
                &object_requirement,
                SourceStructureCompatibilityItem {
                    path: object_name,
                    message: "来源表缺失或当前账号不可见".into(),
                    used_by: object.used_by,
                    fallback: object.fallback,
                },
                &mut blockers,
                &mut reviews,
                &mut compatible_fallbacks,
            );
            continue;
        }
        let Some(structure) = structures.get(&object_name) else {
            reviews.push(SourceStructureCompatibilityItem {
                path: object_name,
                message: "来源表可读，但没有字段结构快照".into(),
                used_by: object.used_by,
                fallback: "重新识别结构或核对读取账号的元数据权限".into(),
            });
            continue;
        };
        let columns = structure
            .columns
            .iter()
            .map(|column| (column.name.trim().to_ascii_uppercase(), column))
            .collect::<HashMap<_, _>>();
        for column in object.columns {
            if !task_applies(&column.applies_to_tasks, task) {
                continue;
            }
            let column_name = column.name.trim().to_ascii_uppercase();
            let path = format!("{object_name}.{column_name}");
            let requirement = effective_source_requirement(
                &column.requirement,
                &column.required_for_tasks,
                &column.required_when_objects_present,
                &checked,
                task,
            );
            let Some(actual) = columns.get(&column_name) else {
                add_source_compatibility_item(
                    &requirement,
                    SourceStructureCompatibilityItem {
                        path,
                        message: "字段缺失或当前账号不可见".into(),
                        used_by: column.used_by,
                        fallback: column.fallback,
                    },
                    &mut blockers,
                    &mut reviews,
                    &mut compatible_fallbacks,
                );
                continue;
            };
            if column.accepted_type_families.is_empty() {
                continue;
            }
            let family = source_type_family(&actual.data_type);
            if column
                .accepted_type_families
                .iter()
                .any(|item| item.eq_ignore_ascii_case(family))
            {
                continue;
            }
            let item = SourceStructureCompatibilityItem {
                path,
                message: if family.is_empty() {
                    format!(
                        "数据库类型未记录，契约接受 {}",
                        column.accepted_type_families.join("/")
                    )
                } else {
                    format!(
                        "数据库类型 {} 属于 {family}，契约接受 {}",
                        actual.data_type,
                        column.accepted_type_families.join("/")
                    )
                },
                used_by: column.used_by,
                fallback: column.fallback,
            };
            if requirement == "REQUIRED" && !family.is_empty() {
                blockers.push(item);
            } else {
                reviews.push(item);
            }
        }
        for group in object.column_groups {
            if !task_applies(&group.applies_to_tasks, task) {
                continue;
            }
            let candidates = group
                .columns
                .iter()
                .map(|name| name.trim().to_ascii_uppercase())
                .collect::<Vec<_>>();
            if candidates.iter().any(|name| columns.contains_key(name)) {
                continue;
            }
            let requirement = effective_source_requirement(
                &group.requirement,
                &group.required_for_tasks,
                &group.required_when_objects_present,
                &checked,
                task,
            );
            add_source_compatibility_item(
                &requirement,
                SourceStructureCompatibilityItem {
                    path: format!("{object_name}.{{{}}}", candidates.join("|")),
                    message: format!("字段组“{}”没有可用字段", group.name),
                    used_by: group.used_by,
                    fallback: group.fallback,
                },
                &mut blockers,
                &mut reviews,
                &mut compatible_fallbacks,
            );
        }
    }
    for group in contract.source_structure.object_groups {
        if !task_applies(&group.applies_to_tasks, task) {
            continue;
        }
        let candidates = group
            .objects
            .iter()
            .map(|name| name.trim().to_ascii_uppercase())
            .collect::<Vec<_>>();
        if candidates.iter().any(|name| checked.contains(name)) {
            continue;
        }
        let requirement = effective_source_requirement(
            &group.requirement,
            &group.required_for_tasks,
            &[],
            &checked,
            task,
        );
        add_source_compatibility_item(
            &requirement,
            SourceStructureCompatibilityItem {
                path: format!("{{{}}}", candidates.join("|")),
                message: format!("来源表组“{}”没有可用对象", group.name),
                used_by: group.used_by,
                fallback: group.fallback,
            },
            &mut blockers,
            &mut reviews,
            &mut compatible_fallbacks,
        );
    }
    let status = if !blockers.is_empty() {
        "BLOCKED"
    } else if !reviews.is_empty() {
        "REVIEW"
    } else {
        "COMPATIBLE"
    };
    let message = match status {
        "BLOCKED" => format!(
            "发现 {} 个阻断项，修正来源字段或权限后才能继续",
            blockers.len()
        ),
        "REVIEW" => format!(
            "核心结构可读取，另有 {} 项需要在字段映射中核对",
            reviews.len()
        ),
        _ if task.eq_ignore_ascii_case("INVENTORY") => {
            "当前来源字段结构满足库存迁移核心契约".into()
        }
        _ => "当前来源字段结构满足药品迁移核心契约".into(),
    };
    Ok(SourceStructureCompatibility {
        status: status.into(),
        task: task.into(),
        blockers,
        reviews,
        compatible_fallbacks,
        message,
    })
}

pub fn inspect_medicine(
    request: InspectMedicineSourceAdapterRequest,
) -> Result<MedicineSourceInspection, String> {
    let adapter = executable_medicine_adapter(&request.adapter_id)?;
    ensure_database_family(adapter.id(), &request.connection)?;
    let inspection = adapter
        .inspect(&request.connection)
        .map_err(|error| format!("{}识别失败：{error}", adapter_name(adapter.id())))?;
    let descriptor = adapter_descriptor(adapter.id())?;
    let mut inspection = normalize_medicine_inspection(&descriptor, inspection)?;
    let compatibility = evaluate_source_structure_compatibility(
        &descriptor,
        &inspection.checked_objects,
        &inspection.object_structures,
        "MEDICINE_BASE",
    )?;
    if compatibility.status == "BLOCKED" {
        inspection.detected = false;
        inspection.message = compatibility.message.clone();
    }
    inspection.compatibility = Some(compatibility);
    Ok(inspection)
}

pub fn load_medicine(request: LoadMedicineSourceAdapterRequest) -> Result<SourcePreview, String> {
    let adapter = executable_medicine_adapter(&request.adapter_id)?;
    ensure_database_family(adapter.id(), &request.connection)?;
    adapter
        .load(&request)
        .map_err(|error| format!("{}数据读取失败：{error}", adapter_name(adapter.id())))
}

pub fn load_inventory_catalog(
    request: LoadInventorySourceCatalogRequest,
) -> Result<InventorySourceCatalog, String> {
    let adapter = executable_inventory_adapter(&request.adapter_id)?;
    let adapter_id = InventorySourceAdapter::id(adapter);
    ensure_database_family(adapter_id, &request.connection)?;
    let catalog = adapter.load_catalog(&request.connection).map_err(|error| {
        format!(
            "{}机构和库房目录读取失败：{error}",
            adapter_name(adapter_id)
        )
    })?;
    let descriptor = adapter_descriptor(adapter_id)?;
    normalize_inventory_catalog(&descriptor, catalog)
}

pub fn inspect_inventory(
    store: &LocalStore,
    tenant_id: &str,
    request: InspectInventorySourceAdapterRequest,
) -> Result<InventorySourceReadiness, String> {
    let adapter = executable_inventory_adapter(&request.adapter_id)?;
    let adapter_id = InventorySourceAdapter::id(adapter);
    ensure_database_family(adapter_id, &request.connection)?;
    let readiness = adapter
        .inspect(store, tenant_id, &request)
        .map_err(|error| format!("{}库存范围识别失败：{error}", adapter_name(adapter_id)))?;
    let descriptor = adapter_descriptor(adapter_id)?;
    let mut readiness = normalize_inventory_readiness(&descriptor, readiness)?;
    let compatibility = evaluate_source_structure_compatibility(
        &descriptor,
        &readiness.checked_objects,
        &readiness.object_structures,
        "INVENTORY",
    )?;
    if compatibility.status == "BLOCKED" {
        readiness.ready_for_location_mapping = false;
        readiness.message = compatibility.message.clone();
    }
    readiness.compatibility = Some(compatibility);
    Ok(readiness)
}

pub fn inventory_adapter_id(adapter_id: &str) -> Result<&'static str, String> {
    executable_inventory_adapter(adapter_id).map(InventorySourceAdapter::id)
}

pub fn inventory_adapter_name(adapter_id: &str) -> Result<String, String> {
    inventory_adapter_id(adapter_id).map(adapter_name)
}

pub fn inventory_workflow(adapter_id: &str) -> Result<SourceAdapterInventoryWorkflow, String> {
    let canonical_id = inventory_adapter_id(adapter_id)?;
    adapter_descriptor(canonical_id).map(|descriptor| descriptor.inventory_workflow)
}

pub fn load_inventory_stock_items(
    adapter_id: &str,
    profile: &ConnectionProfile,
    selected_organization_ids: &HashSet<String>,
    selected_location_keys: &HashSet<String>,
) -> Result<Vec<InventorySourceStockItem>, String> {
    let adapter = executable_inventory_adapter(adapter_id)?;
    let canonical_id = InventorySourceAdapter::id(adapter);
    ensure_database_family(canonical_id, profile)?;
    let items = adapter
        .load_stock_items(profile, selected_organization_ids, selected_location_keys)
        .map_err(|error| format!("{}库存明细读取失败：{error}", adapter_name(canonical_id)))?;
    let descriptor = adapter_descriptor(canonical_id)?;
    normalize_inventory_stock_items(
        &descriptor,
        items,
        selected_organization_ids,
        selected_location_keys,
    )
}

fn normalize_medicine_inspection(
    descriptor: &SourceAdapterDescriptor,
    mut inspection: MedicineSourceInspection,
) -> Result<MedicineSourceInspection, String> {
    inspection.adapter_id = descriptor.id.clone();
    inspection.adapter_name = descriptor.name.clone();
    inspection.adapter_version = descriptor.version;
    inspection.schema = inspection.schema.trim().to_string();
    (
        inspection.checked_objects,
        inspection.missing_objects,
        inspection.object_structures,
    ) = normalize_source_structure_snapshot(
        &descriptor.name,
        inspection.checked_objects,
        inspection.missing_objects,
        inspection.object_structures,
    )?;
    Ok(inspection)
}

fn normalize_source_structure_snapshot(
    adapter_name: &str,
    checked_objects: Vec<String>,
    missing_objects: Vec<String>,
    object_structures: Vec<SourceObjectStructure>,
) -> Result<(Vec<String>, Vec<String>, Vec<SourceObjectStructure>), String> {
    let checked_objects = normalize_inspection_object_names(checked_objects, "已核对对象")?;
    let missing_objects = normalize_inspection_object_names(missing_objects, "缺失对象")?;
    let checked = checked_objects
        .iter()
        .map(|name| name.to_ascii_uppercase())
        .collect::<HashSet<_>>();
    if let Some(name) = missing_objects
        .iter()
        .find(|name| checked.contains(&name.to_ascii_uppercase()))
    {
        return Err(format!(
            "{adapter_name}来源结构契约无效：对象 {name} 同时被标记为已核对和缺失"
        ));
    }
    validate_inspection_object_structures(&object_structures)?;
    let original_object_count = object_structures.len();
    let original_column_count = object_structures
        .iter()
        .map(|structure| structure.columns.len())
        .sum::<usize>();
    let object_structures = sanitize_source_object_structures(object_structures, &checked_objects);
    let normalized_column_count = object_structures
        .iter()
        .map(|structure| structure.columns.len())
        .sum::<usize>();
    if object_structures.len() != original_object_count
        || normalized_column_count != original_column_count
    {
        return Err(format!(
            "{adapter_name}来源结构契约无效：字段结构包含未核对、重复或超出上限的表字段"
        ));
    }
    Ok((checked_objects, missing_objects, object_structures))
}

fn normalize_inspection_object_names(
    values: Vec<String>,
    label: &str,
) -> Result<Vec<String>, String> {
    if values.len() > 160 {
        return Err(format!("{label}超过 160 项上限"));
    }
    let mut seen = HashSet::new();
    values
        .into_iter()
        .map(|value| {
            let value = value.trim().to_string();
            if value.is_empty()
                || value.chars().count() > 120
                || value.chars().any(char::is_control)
            {
                return Err(format!("{label}包含无效名称"));
            }
            if !seen.insert(value.to_ascii_uppercase()) {
                return Err(format!("{label}包含重复对象 {value}"));
            }
            Ok(value)
        })
        .collect()
}

fn validate_inspection_object_structures(
    structures: &[SourceObjectStructure],
) -> Result<(), String> {
    if structures.len() > 80 {
        return Err("字段结构超过 80 张表上限".into());
    }
    let total_columns = structures
        .iter()
        .map(|structure| structure.columns.len())
        .sum::<usize>();
    if total_columns > 5_000 {
        return Err("字段结构超过 5,000 个字段上限".into());
    }
    for structure in structures {
        if structure.name.trim().is_empty()
            || structure.name.chars().count() > 120
            || structure.name.chars().any(char::is_control)
            || structure.columns.len() > 500
        {
            return Err("字段结构包含无效表名或单表超过 500 个字段".into());
        }
        for column in &structure.columns {
            if column.name.trim().is_empty()
                || column.name.chars().count() > 120
                || column.name.chars().any(char::is_control)
                || column.data_type.chars().count() > 120
                || column.data_type.chars().any(char::is_control)
            {
                return Err(format!(
                    "字段结构 {} 包含无效字段名或数据库类型",
                    structure.name
                ));
            }
        }
    }
    Ok(())
}

fn normalize_inventory_catalog(
    descriptor: &SourceAdapterDescriptor,
    mut catalog: InventorySourceCatalog,
) -> Result<InventorySourceCatalog, String> {
    catalog.adapter_id = descriptor.id.clone();
    catalog.adapter_name = descriptor.name.clone();
    catalog.adapter_version = descriptor.version;
    catalog.message = catalog.message.trim().to_string();
    catalog.warnings = catalog
        .warnings
        .into_iter()
        .map(|warning| warning.trim().to_string())
        .filter(|warning| !warning.is_empty())
        .collect();
    let allowed_kinds = descriptor
        .inventory_workflow
        .normalized_location_kinds
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut organization_ids = HashSet::with_capacity(catalog.organizations.len());
    let mut problems = Vec::new();
    for (index, organization) in catalog.organizations.iter_mut().enumerate() {
        for value in [
            &mut organization.id,
            &mut organization.name,
            &mut organization.parent_id,
            &mut organization.organization_type,
        ] {
            *value = value.trim().to_string();
        }
        if organization.id.is_empty() || organization.name.is_empty() {
            problems.push(format!("第 {} 个机构缺少稳定标识或名称", index + 1));
        } else if !organization_ids.insert(organization.id.clone()) {
            problems.push(format!("机构标识 {} 重复", organization.id));
        }
    }
    let mut location_keys = HashSet::with_capacity(catalog.locations.len());
    for (index, location) in catalog.locations.iter_mut().enumerate() {
        location.source_kind = location.source_kind.trim().to_ascii_uppercase();
        for value in [
            &mut location.source_location_key,
            &mut location.id,
            &mut location.name,
            &mut location.organization_id,
            &mut location.category,
        ] {
            *value = value.trim().to_string();
        }
        let position = index + 1;
        if !allowed_kinds.contains(location.source_kind.as_str()) {
            problems.push(format!(
                "第 {position} 个库存位置类型 {} 未在 inventoryWorkflow 中声明",
                location.source_kind
            ));
        }
        for (label, value) in [
            ("sourceLocationKey", location.source_location_key.as_str()),
            ("id", location.id.as_str()),
            ("name", location.name.as_str()),
            ("organizationId", location.organization_id.as_str()),
        ] {
            if value.is_empty() {
                problems.push(format!("第 {position} 个库存位置缺少 {label}"));
            }
        }
        if !location.organization_id.is_empty()
            && !organization_ids.contains(&location.organization_id)
        {
            problems.push(format!(
                "库存位置 {} 引用了目录中不存在的机构 {}",
                location.source_location_key, location.organization_id
            ));
        }
        if !location.source_location_key.is_empty()
            && !location_keys.insert(location.source_location_key.clone())
        {
            problems.push(format!(
                "库存位置稳定键 {} 重复",
                location.source_location_key
            ));
        }
    }
    finish_inventory_contract_validation(&descriptor.name, "机构和库房目录", problems)?;
    Ok(catalog)
}

fn normalize_inventory_readiness(
    descriptor: &SourceAdapterDescriptor,
    mut readiness: InventorySourceReadiness,
) -> Result<InventorySourceReadiness, String> {
    readiness.adapter_id = descriptor.id.clone();
    readiness.adapter_name = descriptor.name.clone();
    readiness.adapter_version = descriptor.version;
    readiness.schema = readiness.schema.trim().to_string();
    readiness.source_name = readiness.source_name.trim().to_string();
    readiness.message = readiness.message.trim().to_string();
    (
        readiness.checked_objects,
        readiness.missing_objects,
        readiness.object_structures,
    ) = normalize_source_structure_snapshot(
        &descriptor.name,
        readiness.checked_objects,
        readiness.missing_objects,
        readiness.object_structures,
    )?;
    readiness.warnings = readiness
        .warnings
        .into_iter()
        .map(|warning| warning.trim().to_string())
        .filter(|warning| !warning.is_empty())
        .collect();
    let allowed_kinds = descriptor
        .inventory_workflow
        .normalized_location_kinds
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut location_keys = HashSet::with_capacity(readiness.locations.len());
    let mut problems = Vec::new();
    let mut stock_row_count = 0usize;
    let mut stock_group_count = 0usize;
    let mut medicine_count = 0usize;
    for (index, location) in readiness.locations.iter_mut().enumerate() {
        location.source_kind = location.source_kind.trim().to_ascii_uppercase();
        for value in [
            &mut location.source_location_key,
            &mut location.source_location_name,
            &mut location.organization_id,
            &mut location.mapping_status,
            &mut location.mapping_message,
        ] {
            *value = value.trim().to_string();
        }
        let position = index + 1;
        if !allowed_kinds.contains(location.source_kind.as_str()) {
            problems.push(format!(
                "第 {position} 个库存范围类型 {} 未在 inventoryWorkflow 中声明",
                location.source_kind
            ));
        }
        for (label, value) in [
            ("sourceLocationKey", location.source_location_key.as_str()),
            ("sourceLocationName", location.source_location_name.as_str()),
            ("organizationId", location.organization_id.as_str()),
            ("mappingStatus", location.mapping_status.as_str()),
        ] {
            if value.is_empty() {
                problems.push(format!("第 {position} 个库存范围缺少 {label}"));
            }
        }
        if !location.source_location_key.is_empty()
            && !location_keys.insert(location.source_location_key.clone())
        {
            problems.push(format!(
                "库存范围稳定键 {} 重复",
                location.source_location_key
            ));
        }
        stock_row_count = stock_row_count.saturating_add(location.stock_row_count);
        stock_group_count = stock_group_count.saturating_add(location.stock_group_count);
        medicine_count = medicine_count.saturating_add(location.medicine_count);
    }
    if (
        readiness.stock_row_count,
        readiness.stock_group_count,
        readiness.medicine_count,
    ) != (stock_row_count, stock_group_count, medicine_count)
    {
        problems.push(format!(
            "库存范围汇总与位置明细不一致：应为 {stock_row_count} 行、{stock_group_count} 组、{medicine_count} 种药品，实际为 {} 行、{} 组、{} 种药品；请使用 InventorySourceReadiness::new 自动汇总",
            readiness.stock_row_count, readiness.stock_group_count, readiness.medicine_count
        ));
    }
    if readiness.ready_for_location_mapping && readiness.locations.is_empty() {
        problems.push("库存范围标记为可映射，但没有任何机构库房明细".into());
    }
    if readiness
        .mapped_medicine_count
        .saturating_add(readiness.unresolved_medicine_count)
        > readiness.medicine_count
    {
        problems.push(format!(
            "药品台账汇总超过库存药品总数：已匹配 {}、未匹配 {}、库存药品 {}",
            readiness.mapped_medicine_count,
            readiness.unresolved_medicine_count,
            readiness.medicine_count
        ));
    }
    let mut unresolved_source_keys = HashSet::new();
    readiness.unresolved_source_keys = readiness
        .unresolved_source_keys
        .into_iter()
        .map(|key| key.trim().to_string())
        .filter(|key| !key.is_empty())
        .filter(|key| unresolved_source_keys.insert(key.clone()))
        .collect();
    let mut guidance_ids = HashSet::with_capacity(readiness.guidance.len());
    for (index, guidance) in readiness.guidance.iter_mut().enumerate() {
        for value in [&mut guidance.id, &mut guidance.title, &mut guidance.body] {
            *value = value.trim().to_string();
        }
        guidance.tone = guidance.tone.trim().to_ascii_uppercase();
        let position = index + 1;
        if guidance.id.is_empty() || guidance.title.is_empty() || guidance.body.is_empty() {
            problems.push(format!(
                "第 {position} 条库存业务说明缺少 id、title 或 body"
            ));
        } else if !guidance_ids.insert(guidance.id.clone()) {
            problems.push(format!("库存业务说明标识 {} 重复", guidance.id));
        }
        if !matches!(guidance.tone.as_str(), "INFO" | "SUCCESS" | "WARNING") {
            problems.push(format!(
                "第 {position} 条库存业务说明 tone {} 无效",
                guidance.tone
            ));
        }
    }
    finish_inventory_contract_validation(&descriptor.name, "库存范围", problems)?;
    Ok(readiness)
}

fn normalize_inventory_stock_items(
    descriptor: &SourceAdapterDescriptor,
    items: Vec<InventorySourceStockItem>,
    selected_organization_ids: &HashSet<String>,
    selected_location_keys: &HashSet<String>,
) -> Result<Vec<InventorySourceStockItem>, String> {
    let allowed_kinds = descriptor
        .inventory_workflow
        .normalized_location_kinds
        .iter()
        .map(String::as_str)
        .collect::<HashSet<_>>();
    let mut normalized = Vec::with_capacity(items.len());
    let mut record_keys = HashSet::with_capacity(items.len());
    let mut problems = Vec::new();
    for (index, mut item) in items.into_iter().enumerate() {
        item.source_kind = item.source_kind.trim().to_ascii_uppercase();
        for value in [
            &mut item.source_record_id,
            &mut item.source_location_key,
            &mut item.source_location_name,
            &mut item.source_organization_id,
            &mut item.source_product_key,
            &mut item.drug_name,
            &mut item.specification,
            &mut item.dosage_form,
            &mut item.minimum_unit,
            &mut item.sale_unit,
            &mut item.sale_specification,
            &mut item.unit_sale_factor,
            &mut item.single_minimum_package_factor,
            &mut item.single_minimum_package_factor_source,
            &mut item.product_sale_unit,
            &mut item.product_unit_sale_factor,
            &mut item.factory_name,
            &mut item.product_name,
            &mut item.amount,
            &mut item.price_pur,
            &mut item.price_sale,
            &mut item.purchase_total,
            &mut item.retail_total,
            &mut item.batch_code,
            &mut item.effective_date,
        ] {
            *value = value.trim().to_string();
        }
        let row = index + 1;
        if !allowed_kinds.contains(item.source_kind.as_str()) {
            problems.push(format!(
                "第 {row} 行 sourceKind“{}”未在 inventoryWorkflow 中声明",
                item.source_kind
            ));
        }
        for (label, value) in [
            ("sourceRecordId", item.source_record_id.as_str()),
            ("sourceOrganizationId", item.source_organization_id.as_str()),
            ("sourceLocationKey", item.source_location_key.as_str()),
            ("sourceProductKey", item.source_product_key.as_str()),
        ] {
            if value.is_empty() {
                problems.push(format!("第 {row} 行缺少稳定来源字段 {label}"));
            }
        }
        if !item.single_minimum_package_factor.is_empty()
            && item.single_minimum_package_factor_source.is_empty()
        {
            problems.push(format!(
                "第 {row} 行提供了 singleMinimumPackageFactor，但缺少可核对的 singleMinimumPackageFactorSource"
            ));
        }
        if !item.source_organization_id.is_empty()
            && !selected_organization_ids.contains(&item.source_organization_id)
        {
            problems.push(format!(
                "第 {row} 行机构 {} 不在本批已选机构范围内",
                item.source_organization_id
            ));
        }
        if !item.source_location_key.is_empty()
            && !selected_location_keys.contains(&item.source_location_key)
        {
            problems.push(format!(
                "第 {row} 行库房 {} 不在本批已选库房范围内",
                item.source_location_key
            ));
        }
        if !item.source_record_id.is_empty() {
            let record_key = format!("{}:{}", item.source_kind, item.source_record_id);
            if !record_keys.insert(record_key.clone()) {
                problems.push(format!(
                    "第 {row} 行来源记录 {record_key} 重复，可能存在一对多关联放大库存"
                ));
            }
        }
        normalized.push(item);
    }
    finish_inventory_contract_validation(&descriptor.name, "库存明细", problems)?;
    Ok(normalized)
}

fn finish_inventory_contract_validation(
    adapter_name: &str,
    stage: &str,
    problems: Vec<String>,
) -> Result<(), String> {
    if problems.is_empty() {
        return Ok(());
    }
    let hidden = problems.len().saturating_sub(12);
    let mut message = problems.into_iter().take(12).collect::<Vec<_>>().join("；");
    if hidden > 0 {
        message.push_str(&format!("；另有 {hidden} 项未显示"));
    }
    Err(format!("{adapter_name}{stage}来源契约校验失败：{message}"))
}

fn executable_medicine_adapter(
    adapter_id: &str,
) -> Result<&'static dyn MedicineSourceAdapter, String> {
    let descriptor = adapter_descriptor(adapter_id)?;
    if !descriptor.automatic_detection {
        return Err(format!(
            "{}使用通用字段映射流程，不提供自动结构识别",
            descriptor.name
        ));
    }
    adapter_binding(&descriptor.implementation)
        .and_then(|binding| binding.medicine)
        .ok_or_else(|| {
            format!(
                "来源适配器 {} 的后端实现 {} 尚未绑定药品读取契约",
                descriptor.id, descriptor.implementation
            )
        })
}

fn executable_inventory_adapter(
    adapter_id: &str,
) -> Result<&'static dyn InventorySourceAdapter, String> {
    let descriptor = adapter_descriptor(adapter_id)?;
    if !descriptor
        .migration_tasks
        .iter()
        .any(|task| task == "INVENTORY")
    {
        return Err(format!("{}未声明机构库存迁移能力", descriptor.name));
    }
    adapter_binding(&descriptor.implementation)
        .and_then(|binding| binding.inventory)
        .ok_or_else(|| {
            format!(
                "库存来源适配器 {} 的后端实现 {} 尚未绑定库存读取契约",
                descriptor.id, descriptor.implementation
            )
        })
}

fn adapter_binding(implementation: &str) -> Option<SourceAdapterBinding> {
    registered_adapter_bindings()
        .into_iter()
        .find(|binding| binding.implementation == implementation)
}

fn ensure_database_family(adapter_id: &str, profile: &ConnectionProfile) -> Result<(), String> {
    let descriptor = adapter_descriptor(adapter_id)?;
    let kind = profile.kind.trim().to_ascii_lowercase();
    if descriptor
        .database_families
        .iter()
        .any(|item| item == &kind)
    {
        return Ok(());
    }
    Err(format!(
        "{}不支持当前数据库类型 {}，支持范围：{}",
        descriptor.name,
        kind,
        descriptor.database_families.join("、")
    ))
}

fn adapter_descriptor(adapter_id: &str) -> Result<SourceAdapterDescriptor, String> {
    let adapter_id = adapter_id.trim().to_ascii_uppercase();
    registry()?
        .into_iter()
        .find(|adapter| adapter.id == adapter_id)
        .ok_or_else(|| format!("未注册来源适配器 {adapter_id}"))
}

fn adapter_name(adapter_id: &str) -> String {
    adapter_descriptor(adapter_id)
        .map(|adapter| adapter.name)
        .unwrap_or_else(|_| adapter_id.to_string())
}

fn default_limit() -> u32 {
    10_000
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::SourceObjectColumnStructure;

    #[test]
    fn adapter_sdk_quotes_source_objects_by_database_family() {
        assert_eq!(
            source_qualified_object("oracle", "PHIS", "YK_TYPK").unwrap(),
            "\"PHIS\".\"YK_TYPK\""
        );
        assert_eq!(
            source_qualified_object("postgresql", "public", "his_drug").unwrap(),
            "\"public\".\"his_drug\""
        );
        assert_eq!(
            source_qualified_object("mysql", "ignored", "drug`master").unwrap(),
            "`drug``master`"
        );
        assert_eq!(
            source_qualified_object("oracle", "PHIS", "A\"; DROP TABLE X--").unwrap(),
            "\"PHIS\".\"A\"\"; DROP TABLE X--\""
        );
        assert!(source_qualified_object("oracle", "PHIS", "bad\nname").is_err());
    }

    #[test]
    fn adapter_sdk_builds_bounded_deterministic_scope_literals() {
        let values = HashSet::from(["ORG-2".into(), "ORG'1".into()]);
        assert_eq!(
            source_text_filter_list(&values, "机构").unwrap(),
            "'ORG''1', 'ORG-2'"
        );
        assert!(source_text_filter_list(&HashSet::new(), "机构").is_err());
        let oversized = (0..501).map(|index| format!("ORG-{index}")).collect();
        assert!(source_text_filter_list(&oversized, "机构").is_err());
        assert!(source_text_filter_list(&HashSet::from(["bad\nvalue".into()]), "机构").is_err());
    }

    fn stock_item(
        record: &str,
        kind: &str,
        organization: &str,
        location: &str,
    ) -> InventorySourceStockItem {
        InventorySourceStockItem::new(
            kind,
            record,
            location,
            " 西药库 ",
            organization,
            " 101:2001 ",
        )
        .with_medicine(" 测试药品 ", "1g", "片剂", "片")
        .with_storage_packaging("盒", "10片/盒", "10")
        .with_single_minimum_package_evidence("10", "DRUG_MASTER.MIN_PACKAGE_FACTOR")
        .with_product_packaging("盒", "10")
        .with_product("厂家", "商品")
        .with_quantity_and_prices("2", "1", "1.2")
        .with_totals("2", "2.4")
        .with_batch("B01", "2028-01-01")
    }

    fn medicine_inspection_with_structure(
        checked_objects: Vec<String>,
        object_structures: Vec<SourceObjectStructure>,
    ) -> MedicineSourceInspection {
        MedicineSourceInspection {
            detected: true,
            adapter_id: "WRONG".into(),
            adapter_name: "错误名称".into(),
            adapter_version: 99,
            schema: " PHIS ".into(),
            checked_objects,
            missing_objects: Vec::new(),
            object_structures,
            scopes: Vec::new(),
            metrics: Vec::new(),
            guidance: Vec::new(),
            compatibility: None,
            warnings: Vec::new(),
            message: "结构通过".into(),
        }
    }

    #[test]
    fn medicine_inspection_boundary_owns_identity_and_redacted_structure() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let inspection = medicine_inspection_with_structure(
            vec![" YK_TYPK ".into()],
            vec![SourceObjectStructure {
                name: " YK_TYPK ".into(),
                columns: vec![SourceObjectColumnStructure {
                    name: " YPMC ".into(),
                    data_type: " NVARCHAR2 ".into(),
                }],
            }],
        );
        let normalized = normalize_medicine_inspection(&descriptor, inspection).unwrap();
        assert_eq!(normalized.adapter_id, "PHIS27");
        assert_eq!(normalized.adapter_name, "二系列phis");
        assert_eq!(normalized.adapter_version, descriptor.version);
        assert_eq!(normalized.schema, "PHIS");
        assert_eq!(normalized.object_structures[0].name, "YK_TYPK");
        assert_eq!(normalized.object_structures[0].columns[0].name, "YPMC");
        assert_eq!(
            normalized.object_structures[0].columns[0].data_type,
            "NVARCHAR2"
        );
    }

    #[test]
    fn medicine_inspection_reports_contract_impact_without_inventory_noise() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let structures = vec![
            SourceObjectStructure {
                name: "YK_TYPK".into(),
                columns: vec![
                    SourceObjectColumnStructure {
                        name: "YPXH".into(),
                        data_type: "NUMBER".into(),
                    },
                    SourceObjectColumnStructure {
                        name: "YPMC".into(),
                        data_type: "NVARCHAR2".into(),
                    },
                ],
            },
            SourceObjectStructure {
                name: "YK_YPCD".into(),
                columns: vec![
                    SourceObjectColumnStructure {
                        name: "YPXH".into(),
                        data_type: "NUMBER".into(),
                    },
                    SourceObjectColumnStructure {
                        name: "YPCD".into(),
                        data_type: "NUMBER".into(),
                    },
                ],
            },
            SourceObjectStructure {
                name: "YK_CDDZ".into(),
                columns: vec![
                    SourceObjectColumnStructure {
                        name: "YPCD".into(),
                        data_type: "NUMBER".into(),
                    },
                    SourceObjectColumnStructure {
                        name: "CDMC".into(),
                        data_type: "NVARCHAR2".into(),
                    },
                ],
            },
            SourceObjectStructure {
                name: "YK_CDXX".into(),
                columns: vec![SourceObjectColumnStructure {
                    name: "YPXH".into(),
                    data_type: "NUMBER".into(),
                }],
            },
        ];
        let inspection = medicine_inspection_with_structure(
            vec![
                "YK_TYPK".into(),
                "YK_YPCD".into(),
                "YK_CDDZ".into(),
                "YK_CDXX".into(),
            ],
            structures,
        );
        let compatibility = evaluate_source_structure_compatibility(
            &descriptor,
            &inspection.checked_objects,
            &inspection.object_structures,
            "MEDICINE_BASE",
        )
        .unwrap();
        assert_eq!(compatibility.status, "REVIEW");
        assert!(compatibility.blockers.is_empty());
        assert!(compatibility
            .reviews
            .iter()
            .any(|item| item.path == "YK_TYPK.ZXDW"));
        assert!(compatibility
            .compatible_fallbacks
            .iter()
            .any(|item| item.path == "YK_YPCD.PZWH"));
        assert!(!compatibility
            .reviews
            .iter()
            .chain(compatibility.compatible_fallbacks.iter())
            .any(|item| item.path == "YK_TYPK.ZXBZ" || item.path == "YK_TYPK.YPDW"));

        let mut incompatible = inspection;
        incompatible.object_structures[0]
            .columns
            .retain(|column| column.name != "YPMC");
        let blocked = evaluate_source_structure_compatibility(
            &descriptor,
            &incompatible.checked_objects,
            &incompatible.object_structures,
            "MEDICINE_BASE",
        )
        .unwrap();
        assert_eq!(blocked.status, "BLOCKED");
        assert!(blocked
            .blockers
            .iter()
            .any(|item| item.path == "YK_TYPK.YPMC"));
    }

    #[test]
    fn inventory_compatibility_requires_one_stock_branch_and_activates_its_dependencies() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let mut structures = vec![
            SourceObjectStructure {
                name: "YK_TYPK".into(),
                columns: [
                    ("YPXH", "NUMBER"),
                    ("YPMC", "NVARCHAR2"),
                    ("ZXDW", "NVARCHAR2"),
                    ("YPDW", "NVARCHAR2"),
                    ("ZXBZ", "NUMBER"),
                ]
                .into_iter()
                .map(|(name, data_type)| SourceObjectColumnStructure {
                    name: name.into(),
                    data_type: data_type.into(),
                })
                .collect(),
            },
            SourceObjectStructure {
                name: "YK_YPCD".into(),
                columns: [("YPXH", "NUMBER"), ("YPCD", "NUMBER")]
                    .into_iter()
                    .map(|(name, data_type)| SourceObjectColumnStructure {
                        name: name.into(),
                        data_type: data_type.into(),
                    })
                    .collect(),
            },
            SourceObjectStructure {
                name: "YK_CDDZ".into(),
                columns: [("YPCD", "NUMBER"), ("CDMC", "NVARCHAR2")]
                    .into_iter()
                    .map(|(name, data_type)| SourceObjectColumnStructure {
                        name: name.into(),
                        data_type: data_type.into(),
                    })
                    .collect(),
            },
            SourceObjectStructure {
                name: "SYS_ORGANIZATION".into(),
                columns: [("ORGANIZCODE", "NVARCHAR2"), ("ORGANIZNAME", "NVARCHAR2")]
                    .into_iter()
                    .map(|(name, data_type)| SourceObjectColumnStructure {
                        name: name.into(),
                        data_type: data_type.into(),
                    })
                    .collect(),
            },
            SourceObjectStructure {
                name: "YK_KCMX".into(),
                columns: ["SBXH", "JGID", "YPXH", "YPCD", "KCSL", "JHJG", "LSJG"]
                    .into_iter()
                    .map(|name| SourceObjectColumnStructure {
                        name: name.into(),
                        data_type: "NUMBER".into(),
                    })
                    .collect(),
            },
        ];
        let checked = structures
            .iter()
            .map(|structure| structure.name.clone())
            .collect::<Vec<_>>();
        let compatible = evaluate_source_structure_compatibility(
            &descriptor,
            &checked,
            &structures,
            "INVENTORY",
        )
        .unwrap();
        assert_eq!(compatible.status, "COMPATIBLE");
        assert!(compatible.blockers.is_empty());
        assert!(compatible
            .compatible_fallbacks
            .iter()
            .any(|item| item.path == "YF_YPXX"));

        structures[0].columns.retain(|column| column.name != "YPDW");
        let missing_warehouse_pack = evaluate_source_structure_compatibility(
            &descriptor,
            &checked,
            &structures,
            "INVENTORY",
        )
        .unwrap();
        assert!(missing_warehouse_pack
            .blockers
            .iter()
            .any(|item| item.path == "YK_TYPK.YPDW"));

        let no_stock_structures = structures
            .iter()
            .filter(|structure| structure.name != "YK_KCMX")
            .cloned()
            .collect::<Vec<_>>();
        let no_stock_checked = no_stock_structures
            .iter()
            .map(|structure| structure.name.clone())
            .collect::<Vec<_>>();
        let no_stock = evaluate_source_structure_compatibility(
            &descriptor,
            &no_stock_checked,
            &no_stock_structures,
            "INVENTORY",
        )
        .unwrap();
        assert!(no_stock
            .blockers
            .iter()
            .any(|item| item.path == "{YK_KCMX|YF_KCMX}"));

        let mut pharmacy_structures = no_stock_structures;
        pharmacy_structures.push(SourceObjectStructure {
            name: "YF_KCMX".into(),
            columns: [
                "SBXH", "YFSB", "JGID", "YPXH", "YPCD", "YPSL", "JHJG", "LSJG",
            ]
            .into_iter()
            .map(|name| SourceObjectColumnStructure {
                name: name.into(),
                data_type: "NUMBER".into(),
            })
            .collect(),
        });
        let pharmacy_checked = pharmacy_structures
            .iter()
            .map(|structure| structure.name.clone())
            .collect::<Vec<_>>();
        let pharmacy_without_pack = evaluate_source_structure_compatibility(
            &descriptor,
            &pharmacy_checked,
            &pharmacy_structures,
            "INVENTORY",
        )
        .unwrap();
        assert!(pharmacy_without_pack
            .blockers
            .iter()
            .any(|item| item.path == "YF_YPXX"));
        assert!(pharmacy_without_pack
            .blockers
            .iter()
            .any(|item| item.path == "YF_YFLB"));
    }

    #[test]
    fn medicine_inspection_boundary_rejects_unchecked_structure_objects() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let inspection = medicine_inspection_with_structure(
            vec!["YK_TYPK".into()],
            vec![SourceObjectStructure {
                name: "PRIVATE_TABLE".into(),
                columns: vec![SourceObjectColumnStructure {
                    name: "SECRET_VALUE".into(),
                    data_type: "VARCHAR2".into(),
                }],
            }],
        );
        let error = normalize_medicine_inspection(&descriptor, inspection).unwrap_err();
        assert!(error.contains("未核对、重复或超出上限"));
    }

    #[test]
    fn inventory_builders_require_identity_and_default_optional_state_safely() {
        let organization = InventorySourceOrganization::new("ORG1", "一院");
        assert!(organization.active);
        assert!(organization.parent_id.is_empty());

        let location = InventorySourceLocationDefinition::new(
            "WAREHOUSE",
            "WAREHOUSE:1",
            "1",
            "西药库",
            "ORG1",
        );
        assert!(location.active);
        assert!(location.category.is_empty());

        let readiness =
            InventorySourceLocationReadiness::new("WAREHOUSE", "WAREHOUSE:1", "西药库", "ORG1");
        assert_eq!(readiness.mapping_status, "PENDING_TARGET_MAPPING");
        assert!(!readiness.requires_source_location_resolution);

        let item = InventorySourceStockItem::new(
            "WAREHOUSE",
            "9001",
            "WAREHOUSE:1",
            "西药库",
            "ORG1",
            "101:2001",
        );
        assert_eq!(item.source_record_id, "9001");
        assert!(item.drug_name.is_empty());
        assert!(item.single_minimum_package_factor_source.is_empty());

        let catalog = InventorySourceCatalog::new(vec![organization], vec![location]);
        assert!(catalog.adapter_id.is_empty());
        assert_eq!(catalog.adapter_version, 0);

        let summary = InventorySourceReadiness::new(
            " PHIS ",
            " legacy-a ",
            vec![readiness.with_counts(1, 3, 2, 2)],
        )
        .with_guidance(vec![InventorySourceGuidance::info(
            "RELATION",
            "库存关系",
            "使用稳定库房键",
        )]);
        assert_eq!(summary.stock_row_count, 3);
        assert_eq!(summary.stock_group_count, 2);
        assert_eq!(summary.medicine_count, 2);
        assert!(summary.ready_for_location_mapping);
        assert_eq!(summary.guidance[0].tone, "INFO");
        assert_eq!(
            InventorySourceGuidance::success("OK", "完成", "可继续").tone,
            "SUCCESS"
        );
        assert_eq!(
            InventorySourceGuidance::warning("WARN", "注意", "需核对").tone,
            "WARNING"
        );
    }

    #[test]
    fn inventory_readiness_boundary_rejects_manual_aggregate_drift() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let location =
            InventorySourceLocationReadiness::new("WAREHOUSE", "WAREHOUSE:1", "西药库", "ORG1")
                .with_counts(1, 3, 2, 2);
        let mut readiness = InventorySourceReadiness::new("PHIS", "legacy-a", vec![location]);
        readiness.stock_row_count = 99;
        let error = normalize_inventory_readiness(&descriptor, readiness).unwrap_err();
        assert!(error.contains("库存范围汇总与位置明细不一致"));
        assert!(error.contains("InventorySourceReadiness::new"));
    }

    #[test]
    fn inventory_summary_builders_receive_manifest_identity_at_the_boundary() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let catalog = InventorySourceCatalog::new(
            vec![InventorySourceOrganization::new(" ORG1 ", " 一院 ")],
            vec![InventorySourceLocationDefinition::new(
                " warehouse ",
                " WAREHOUSE:1 ",
                " 1 ",
                " 西药库 ",
                " ORG1 ",
            )],
        );
        let normalized = normalize_inventory_catalog(&descriptor, catalog).unwrap();
        assert_eq!(normalized.adapter_id, "PHIS27");
        assert_eq!(normalized.adapter_name, "二系列phis");
        assert_eq!(normalized.adapter_version, descriptor.version);
        assert_eq!(normalized.organizations[0].name, "一院");
        assert_eq!(normalized.locations[0].source_kind, "WAREHOUSE");
    }

    #[test]
    fn built_in_registry_has_unique_ids() {
        let adapters = list().expect("adapter registry");
        assert!(adapters.len() >= 3);
        assert_eq!(
            adapters
                .iter()
                .map(|adapter| adapter.id.as_str())
                .collect::<HashSet<_>>()
                .len(),
            adapters.len()
        );
        assert!(adapters.iter().all(|adapter| {
            adapter.package_schema_version == ADAPTER_PACKAGE_SCHEMA_VERSION
                && !adapter.implementation.is_empty()
                && adapter.template_compatible_from_version >= 1
                && adapter.template_compatible_from_version <= adapter.version
                && adapter
                    .changes
                    .iter()
                    .any(|change| change.version == adapter.version && !change.summary.is_empty())
                && matches!(
                    adapter.medicine_workflow.source_key_mode.as_str(),
                    "USER_SELECTED" | "ADAPTER_PROVIDED"
                )
                && !adapter.medicine_workflow.mapping_preset.is_empty()
                && !adapter.medicine_workflow.batch_source_type.is_empty()
        }));
        for required in ["GENERIC_FILE", "GENERIC_DATABASE", "PHIS27"] {
            assert!(adapters.iter().any(|adapter| adapter.id == required));
        }
        let phis27 = adapters
            .iter()
            .find(|adapter| adapter.id == "PHIS27")
            .unwrap();
        assert_eq!(phis27.medicine_workflow.source_key_mode, "ADAPTER_PROVIDED");
        assert_eq!(phis27.medicine_workflow.source_key_field, "SOURCE_KEY");
        assert_eq!(phis27.medicine_workflow.source_key_label, "YPXH:YPCD");
        assert_eq!(phis27.medicine_workflow.mapping_preset, "PHIS27");
        assert_eq!(phis27.medicine_workflow.batch_source_type, "PHIS27");
        assert_eq!(
            phis27.inventory_workflow.normalized_location_kinds,
            vec!["WAREHOUSE", "PHARMACY"]
        );
        assert_eq!(phis27.inventory_workflow.write_mode, "FIRST_STOCKTAKE");
        assert_eq!(phis27.inventory_workflow.trial_policy, "PER_TARGET_STORAGE");
        assert_eq!(
            phis27.inventory_workflow.source_stock_key_mode,
            "PHIS27_LEGACY"
        );
    }

    #[test]
    fn generated_registry_contains_every_adapter_directory() {
        let adapters_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("adapters");
        let expected_ids = std::fs::read_dir(adapters_dir)
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| {
                entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false)
                    && !entry.file_name().to_string_lossy().starts_with('_')
                    && entry.path().join("manifest.json").is_file()
            })
            .map(|entry| {
                let source = std::fs::read_to_string(entry.path().join("manifest.json")).unwrap();
                serde_json::from_str::<SourceAdapterDescriptor>(&source)
                    .unwrap()
                    .id
                    .to_ascii_uppercase()
            })
            .collect::<HashSet<_>>();
        let registered_ids = registry()
            .unwrap()
            .into_iter()
            .map(|adapter| adapter.id)
            .collect::<HashSet<_>>();
        assert_eq!(registered_ids, expected_ids);
    }

    #[test]
    fn unsupported_adapter_package_schema_is_rejected() {
        let invalid = r#"{
            "packageSchemaVersion":99,
            "id":"TEST_HIS",
            "name":"测试 HIS",
            "version":1,
            "templateCompatibleFromVersion":1,
            "changes":[{"version":1,"summary":"初始版本"}],
            "summary":"test",
            "sourceModes":["database"],
            "databaseFamilies":["oracle"],
            "migrationTasks":["MEDICINE_BASE"],
            "automaticDetection":true,
            "reusableMappingProfiles":true,
            "builtIn":true,
            "implementation":"TEST"
        }"#;
        assert!(parse_manifest("test", invalid)
            .unwrap_err()
            .contains("清单版本 99 不受支持"));
    }

    #[test]
    fn unregistered_medicine_workflow_semantics_are_rejected() {
        let invalid = r#"{
            "packageSchemaVersion":1,
            "id":"TEST_HIS",
            "name":"测试 HIS",
            "version":1,
            "templateCompatibleFromVersion":1,
            "changes":[{"version":1,"summary":"初始版本"}],
            "summary":"test",
            "sourceModes":["database"],
            "databaseFamilies":["oracle"],
            "migrationTasks":["MEDICINE_BASE"],
            "automaticDetection":true,
            "reusableMappingProfiles":true,
            "builtIn":true,
            "implementation":"TEST",
            "medicineWorkflow":{
                "sourceKeyMode":"ADAPTER_PROVIDED",
                "sourceKeyField":"SOURCE KEY",
                "sourceKeyLabel":"测试来源键",
                "mappingPreset":"UNKNOWN",
                "batchSourceType":"UNKNOWN",
                "factoryPolicy":"ADAPTER_MANAGED",
                "legacyProfileKind":"UNKNOWN"
            }
        }"#;
        assert!(parse_manifest("test", invalid)
            .unwrap_err()
            .contains("medicineWorkflow 工作流声明无效"));
    }

    #[test]
    fn legacy_automatic_manifest_receives_a_safe_workflow_default() {
        let legacy = r#"{
            "packageSchemaVersion":1,
            "id":"LEGACY_HIS",
            "name":"旧版自动适配器",
            "version":1,
            "templateCompatibleFromVersion":1,
            "changes":[{"version":1,"summary":"初始版本"}],
            "summary":"test",
            "sourceModes":["database"],
            "databaseFamilies":["oracle"],
            "migrationTasks":["MEDICINE_BASE"],
            "automaticDetection":true,
            "reusableMappingProfiles":true,
            "builtIn":true,
            "implementation":"LEGACY_HIS_BUILTIN"
        }"#;
        let parsed = parse_manifest("legacy_his", legacy).unwrap();
        assert_eq!(parsed.medicine_workflow.source_key_mode, "ADAPTER_PROVIDED");
        assert_eq!(parsed.medicine_workflow.source_key_field, "SOURCE_KEY");
        assert_eq!(parsed.medicine_workflow.mapping_preset, "GUIDED");
        assert_eq!(parsed.medicine_workflow.batch_source_type, "GENERIC");
        let policy = parsed.source_compatibility.expect("legacy safe policy");
        assert_eq!(policy.mode, "STRUCTURE_CONTRACT");
        assert!(policy.declared_versions.is_empty());
        assert!(policy.gate.contains("未声明厂商版本范围"));
    }

    #[test]
    fn source_compatibility_policy_rejects_unverified_version_claims() {
        let invalid = r#"{
            "packageSchemaVersion":1,
            "id":"TEST_HIS",
            "name":"测试 HIS",
            "version":1,
            "templateCompatibleFromVersion":1,
            "changes":[{"version":1,"summary":"初始版本"}],
            "summary":"test",
            "sourceModes":["database"],
            "databaseFamilies":["oracle"],
            "migrationTasks":["MEDICINE_BASE"],
            "automaticDetection":true,
            "reusableMappingProfiles":true,
            "builtIn":true,
            "implementation":"TEST_HIS_BUILTIN",
            "sourceCompatibility":{
                "product":"测试 HIS",
                "mode":"DECLARED_VERSION_RANGE",
                "declaredVersions":[],
                "gate":"按版本验收"
            },
            "medicineWorkflow":{
                "sourceKeyMode":"ADAPTER_PROVIDED",
                "sourceKeyField":"SOURCE_KEY",
                "sourceKeyLabel":"测试来源键",
                "mappingPreset":"GUIDED",
                "batchSourceType":"GENERIC",
                "factoryPolicy":"OPTIONAL_CREATE",
                "legacyProfileKind":"NONE"
            }
        }"#;
        assert!(parse_manifest("test_his", invalid)
            .unwrap_err()
            .contains("sourceCompatibility 来源兼容策略无效"));
    }

    #[test]
    fn legacy_phis_inventory_manifest_keeps_historical_stock_keys() {
        let legacy = r#"{
            "packageSchemaVersion":1,
            "id":"PHIS27",
            "name":"二系列phis",
            "version":1,
            "templateCompatibleFromVersion":1,
            "changes":[{"version":1,"summary":"初始版本"}],
            "summary":"test",
            "sourceModes":["database"],
            "databaseFamilies":["oracle"],
            "migrationTasks":["MEDICINE_BASE","INVENTORY"],
            "automaticDetection":true,
            "reusableMappingProfiles":true,
            "builtIn":true,
            "implementation":"PHIS27_BUILTIN",
            "medicineWorkflow":{
                "sourceKeyMode":"ADAPTER_PROVIDED",
                "sourceKeyField":"SOURCE_KEY",
                "sourceKeyLabel":"YPXH:YPCD",
                "mappingPreset":"PHIS27",
                "batchSourceType":"PHIS27",
                "factoryPolicy":"ADAPTER_MANAGED",
                "legacyProfileKind":"PHIS27_V2"
            },
            "inventoryWorkflow":{
                "normalizedLocationKinds":["WAREHOUSE","PHARMACY"],
                "sourceProductKeyLabel":"YPXH:YPCD",
                "writeMode":"FIRST_STOCKTAKE",
                "organizationMapping":"REQUIRED",
                "medicineLedger":"REQUIRED",
                "trialPolicy":"PER_TARGET_STORAGE",
                "undoPolicy":"VERIFIED_BATCH_ONLY"
            }
        }"#;
        let parsed = parse_manifest("phis27", legacy).unwrap();
        assert_eq!(
            parsed.inventory_workflow.source_stock_key_mode,
            "PHIS27_LEGACY"
        );
    }

    #[test]
    fn phis27_declares_inventory_and_oracle_capabilities() {
        let adapter = registry()
            .unwrap()
            .into_iter()
            .find(|adapter| adapter.id == "PHIS27")
            .expect("PHIS27 adapter");
        assert!(adapter.automatic_detection);
        assert!(adapter
            .migration_tasks
            .iter()
            .any(|task| task == "INVENTORY"));
        assert_eq!(adapter.database_families, vec!["oracle"]);
        let policy = adapter.source_compatibility.expect("source compatibility");
        assert_eq!(policy.mode, "STRUCTURE_CONTRACT");
        assert!(policy.declared_versions.is_empty());
    }

    #[test]
    fn generic_sources_are_mapping_profile_aware() {
        for adapter in registry()
            .unwrap()
            .into_iter()
            .filter(|adapter| adapter.id.starts_with("GENERIC_"))
        {
            assert!(adapter.reusable_mapping_profiles);
            assert_eq!(adapter.migration_tasks, vec!["MEDICINE_BASE"]);
        }
    }

    #[test]
    fn automatic_medicine_adapter_dispatch_is_explicit() {
        assert_eq!(
            executable_medicine_adapter("phis27").unwrap().id(),
            "PHIS27"
        );
        assert!(executable_medicine_adapter("GENERIC_DATABASE").is_err());
        assert!(executable_medicine_adapter("UNKNOWN_HIS").is_err());
    }

    #[test]
    fn inventory_adapter_dispatch_requires_inventory_capability() {
        assert_eq!(
            InventorySourceAdapter::id(executable_inventory_adapter("phis27").unwrap()),
            "PHIS27"
        );
        assert!(executable_inventory_adapter("GENERIC_DATABASE").is_err());
        assert!(executable_inventory_adapter("UNKNOWN_HIS").is_err());
    }

    #[test]
    fn adapter_database_family_is_checked_before_execution() {
        let profile = ConnectionProfile {
            kind: "mysql".into(),
            host: "127.0.0.1".into(),
            port: 3306,
            database: "legacy".into(),
            username: "reader".into(),
            password: String::new(),
            schema: String::new(),
            service_name: String::new(),
            driver: String::new(),
            connection_string: String::new(),
        };
        let error = ensure_database_family("PHIS27", &profile).unwrap_err();
        assert!(error.contains("不支持当前数据库类型 mysql"));
    }

    #[test]
    fn inventory_contract_normalizes_identity_fields_at_the_adapter_boundary() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let organizations = HashSet::from(["ORG1".into()]);
        let locations = HashSet::from(["YK:1".into()]);
        let rows = normalize_inventory_stock_items(
            &descriptor,
            vec![stock_item(" 9001 ", " warehouse ", "ORG1", "YK:1")],
            &organizations,
            &locations,
        )
        .unwrap();
        assert_eq!(rows[0].source_kind, "WAREHOUSE");
        assert_eq!(rows[0].source_record_id, "9001");
        assert_eq!(rows[0].source_product_key, "101:2001");
        assert_eq!(rows[0].drug_name, "测试药品");
    }

    #[test]
    fn inventory_contract_rejects_out_of_scope_and_join_duplicated_rows() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let organizations = HashSet::from(["ORG1".into()]);
        let locations = HashSet::from(["YK:1".into()]);
        let error = normalize_inventory_stock_items(
            &descriptor,
            vec![
                stock_item("9001", "WAREHOUSE", "ORG2", "YK:2"),
                stock_item("9001", "WAREHOUSE", "ORG2", "YK:2"),
            ],
            &organizations,
            &locations,
        )
        .unwrap_err();
        assert!(error.contains("不在本批已选机构范围内"));
        assert!(error.contains("不在本批已选库房范围内"));
        assert!(error.contains("一对多关联放大库存"));
    }

    #[test]
    fn inventory_contract_requires_an_explainable_package_evidence_source() {
        let descriptor = adapter_descriptor("PHIS27").unwrap();
        let organizations = HashSet::from(["ORG1".into()]);
        let locations = HashSet::from(["YK:1".into()]);
        let mut row = stock_item("9001", "WAREHOUSE", "ORG1", "YK:1");
        row.single_minimum_package_factor_source.clear();
        let error =
            normalize_inventory_stock_items(&descriptor, vec![row], &organizations, &locations)
                .unwrap_err();
        assert!(error.contains("singleMinimumPackageFactorSource"));
    }
}
