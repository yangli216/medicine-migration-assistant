use crate::datasource::connect_mysql;
use crate::id::new_object_id;
use crate::legacy_phis27::{load_inventory_stock_items, Phis27InventoryStockItem};
use crate::local_store::{InventoryLocationMapping, InventoryOrganizationMapping, LocalStore};
use crate::model::{BatchDetail, ConnectionProfile, MigrationBatch, MigrationRow};
use crate::odbc::{
    configure_target_session, execute_strings, query_optional_row_strings, query_optional_string,
    query_rows_strings, with_connection,
};
use crate::target::{target_identity, ActiveBatchGuard};
use chrono::{Local, NaiveDate, Utc};
use odbc_api::Connection;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_mysql::{MySql, MySqlPool, MySqlTransaction};
use sqlx_postgres::{PgPool, PgTransaction, Postgres};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;
use std::sync::{Arc, RwLock};

const INVENTORY_TARGET_TABLE_PROJECTIONS: &[(&str, &str)] = &[
    (
        "hi_sto_dept",
        "id_sto,na_sto,id_org,id_tet,sd_sto,fg_active",
    ),
    (
        "hi_bd_med_unit",
        "id_med_unit,id_med,na_unit,unit_factor,id_tet",
    ),
    ("hi_bd_med_pro", "id_med_pro,id_med,id_tet,fg_active"),
    (
        "hi_sto_med",
        "id_sto_med,id_med,id_med_pro,id_med_unit,unit_sale,spec_sale,price_sale,price_pur,\
         unit_sale_factor,fg_active,id_sto,id_org,id_tet,revision,\
         insert_user,insert_time",
    ),
    (
        "hi_sto_check",
        "id_sto_check,id_sto,cd_sto_check,dt_check_begin,dt_check_end,fg_sto_check,sd_pol,\
         sd_check,des_sto_check,id_org,id_tet,revision,insert_user,insert_time",
    ),
    (
        "hi_sto_check_sub",
        "id,id_sto_check,id_med_pro,id_sto_inv,cd_batch,dt_effect,amt_check_bgn,amt_check_end,\
         amt_change,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur,\
         unit_sale,unit_sale_factor",
    ),
    (
        "hi_sto_inv",
        "id_sto_inv,id_med_pro,amount,price_sale,price_pur,cd_batch,dt_effect,fg_active,id_sto,\
         id_org,id_tet,revision,insert_user,insert_time",
    ),
    (
        "hi_sto_inv_log",
        "id_inv_log,id_sto_inv,id_med_pro,sd_amt_change,des_reason,id_biz_ori,amt_change,\
         amt_before,amt_after,unit_sale,unit_sale_factor,id_sto,id_org,id_tet,revision,\
         insert_user,insert_time,price_sale,price_pur",
    ),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InventoryTargetBackend {
    PostgreSqlWire,
    Odbc,
    MySql,
}

fn inventory_target_backend(profile: &ConnectionProfile) -> InventoryTargetBackend {
    if crate::pg_protocol::uses_native_connection(profile) {
        InventoryTargetBackend::PostgreSqlWire
    } else if crate::odbc::is_odbc_kind(&profile.kind) {
        InventoryTargetBackend::Odbc
    } else {
        InventoryTargetBackend::MySql
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStorage {
    pub id_sto: String,
    pub name: String,
    pub storage_type: String,
    pub storage_type_name: String,
    pub product_types: String,
    pub organization_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetStorageCatalog {
    pub storages: Vec<TargetStorage>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryTargetMedicine {
    pub id_med: String,
    pub id_med_pro: String,
    pub drug_name: String,
    pub specification: String,
    pub minimum_unit: String,
    pub product_name: String,
    pub sale_specification: String,
    pub factory_name: String,
    pub external_code: String,
    pub approval_code: String,
    pub private: bool,
    pub organization_id: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryTargetMedicineCatalog {
    pub medicines: Vec<InventoryTargetMedicine>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMedicineMatchSource {
    pub key: String,
    pub source_product_key: String,
    pub target_organization_id: String,
    #[serde(default)]
    pub drug_name: String,
    #[serde(default)]
    pub specification: String,
    #[serde(default)]
    pub factory_name: String,
    #[serde(default)]
    pub product_name: String,
    #[serde(default)]
    pub row_count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendInventoryMedicineMatchesRequest {
    pub target: ConnectionProfile,
    pub sources: Vec<InventoryMedicineMatchSource>,
    #[serde(default = "default_inventory_match_limit")]
    pub limit: usize,
}

fn default_inventory_match_limit() -> usize {
    8
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMedicineMatchCandidate {
    pub target: InventoryTargetMedicine,
    pub score: f64,
    pub score_percent: u8,
    pub exact: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMedicineMatchSuggestion {
    #[serde(flatten)]
    pub source: InventoryMedicineMatchSource,
    pub candidates: Vec<InventoryMedicineMatchCandidate>,
    pub auto_candidate: Option<InventoryTargetMedicine>,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecommendInventoryMedicineMatchesResponse {
    pub suggestions: Vec<InventoryMedicineMatchSuggestion>,
    pub catalog_count: usize,
    pub compared_count: usize,
    pub elapsed_ms: u128,
    pub cache_hit: bool,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInventoryTargetMedicinesRequest {
    pub target: ConnectionProfile,
    pub target_organization_id: String,
    pub query: String,
    #[serde(default = "default_inventory_search_limit")]
    pub limit: usize,
}

fn default_inventory_search_limit() -> usize {
    60
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchInventoryTargetMedicinesResponse {
    pub medicines: Vec<InventoryTargetMedicine>,
    pub catalog_count: usize,
    pub message: String,
}

#[derive(Default)]
pub struct InventoryMedicineMatchEngine {
    indexes: RwLock<HashMap<String, Arc<MedicineMatchIndex>>>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryMedicineMatchSelection {
    pub source_product_key: String,
    pub target_organization_id: String,
    pub id_med: String,
    pub id_med_pro: String,
    pub match_method: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInventoryMedicineMatchesRequest {
    pub batch_id: String,
    pub target: ConnectionProfile,
    pub matches: Vec<InventoryMedicineMatchSelection>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveInventoryMedicineMatchesResponse {
    pub saved_count: usize,
    pub message: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareInventoryRequest {
    pub source: ConnectionProfile,
    pub target: ConnectionProfile,
    pub source_name: String,
    #[serde(default)]
    pub organization_mappings: Vec<InventoryOrganizationMapping>,
    pub mappings: Vec<InventoryLocationMapping>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteInventoryRequest {
    pub batch_id: String,
    pub target: ConnectionProfile,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialInventoryRequest {
    pub batch_id: String,
    pub row_id: String,
    pub target: ConnectionProfile,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialInventoryResult {
    pub ok: bool,
    pub row_id: String,
    pub row_no: usize,
    pub source_key: String,
    pub id_sto: String,
    pub storage_name: String,
    pub message: String,
    pub checked_tables: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TrialInventoryResponse {
    pub result: TrialInventoryResult,
    pub detail: BatchDetail,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UndoInventoryRequest {
    pub batch_id: String,
    pub target: ConnectionProfile,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryUndoStoragePreview {
    pub id_sto: String,
    pub name: String,
    pub cd_sto_check: String,
    pub inventory_count: usize,
    pub can_undo: bool,
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryUndoPreview {
    pub batch_id: String,
    pub can_undo: bool,
    pub storage_count: usize,
    pub inventory_count: usize,
    pub blocker_count: usize,
    pub storages: Vec<InventoryUndoStoragePreview>,
    pub message: String,
}

#[derive(Debug, Clone)]
struct InventoryWriteResult {
    row_id: String,
    id_med_unit: String,
    id_sto_med: String,
    id_sto_inv: String,
    id_inv_log: String,
    id_check_sub: String,
}

#[derive(Debug, Clone)]
struct StorageWriteResult {
    id_sto_check: String,
    cd_sto_check: String,
    rows: Vec<InventoryWriteResult>,
}

#[derive(Debug, Clone)]
struct InventoryUndoRow {
    row_id: String,
    id_sto_inv: String,
    id_inv_log: String,
    id_check_sub: String,
    expected_amount: Decimal,
}

#[derive(Debug, Clone)]
struct InventoryUndoStorage {
    id_sto: String,
    name: String,
    id_sto_check: String,
    cd_sto_check: String,
    rows: Vec<InventoryUndoRow>,
}

#[derive(Debug, Clone)]
struct InventoryGroup {
    source_kind: String,
    source_stock_key: String,
    source_location_key: String,
    source_location_name: String,
    source_product_key: String,
    source_record_ids: Vec<String>,
    drug_name: String,
    specification: String,
    dosage_form: String,
    minimum_unit: String,
    sale_unit: String,
    sale_specification: String,
    unit_sale_factor: String,
    product_sale_unit: String,
    product_unit_sale_factor: String,
    factory_name: String,
    product_name: String,
    amount: Decimal,
    price_pur: Decimal,
    price_sale: Decimal,
    purchase_total: Option<Decimal>,
    retail_total: Option<Decimal>,
    batch_code: String,
    effective_date: String,
}

#[derive(Debug, Clone, Default)]
struct ResolvedInventoryMedicine {
    id_med: String,
    id_med_unit: String,
    id_fac: String,
    id_med_pro: String,
}

impl ResolvedInventoryMedicine {
    fn complete(&self) -> bool {
        !self.id_med.trim().is_empty() && !self.id_med_pro.trim().is_empty()
    }
}

// The inventory workflow is intentionally split by responsibility while the files are
// included in one module namespace. This keeps the Tauri command surface stable and
// lets shared transaction/audit types remain private to the inventory implementation.
include!("inventory/prepare.rs");
include!("inventory/matching.rs");
include!("inventory/trial.rs");
include!("inventory/undo.rs");
include!("inventory/write.rs");
include!("inventory/pg.rs");
include!("inventory/support.rs");
