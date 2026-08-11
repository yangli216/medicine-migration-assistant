use crate::datasource::connect_mysql;
use crate::id::new_object_id;
use crate::legacy_phis27::{load_inventory_stock_items, Phis27InventoryStockItem};
use crate::local_store::{InventoryLocationMapping, InventoryOrganizationMapping, LocalStore};
use crate::model::{BatchDetail, ConnectionProfile, MigrationBatch, MigrationRow};
use crate::odbc::{
    configure_target_session, execute_strings, query_optional_row_strings, query_optional_string,
    query_rows_strings, with_connection,
};
use crate::target::target_identity;
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
use std::collections::{BTreeMap, HashMap, HashSet};
use std::str::FromStr;

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

// The inventory workflow is intentionally split by responsibility while the files are
// included in one module namespace. This keeps the Tauri command surface stable and
// lets shared transaction/audit types remain private to the inventory implementation.
include!("inventory/prepare.rs");
include!("inventory/undo.rs");
include!("inventory/write.rs");
include!("inventory/support.rs");
