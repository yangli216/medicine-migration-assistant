use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProfile {
    pub kind: String,
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    #[serde(default)]
    pub password: String,
    #[serde(default)]
    pub schema: String,
    #[serde(default)]
    pub service_name: String,
    #[serde(default)]
    pub driver: String,
    #[serde(default)]
    pub connection_string: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionCheck {
    pub ok: bool,
    pub database_version: String,
    pub latency_ms: u128,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePreviewRequest {
    pub connection: ConnectionProfile,
    pub query: String,
    #[serde(default = "default_preview_limit")]
    pub limit: u32,
}

fn default_preview_limit() -> u32 {
    200
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePreview {
    pub columns: Vec<String>,
    pub rows: Vec<Map<String, Value>>,
    pub truncated: bool,
    pub elapsed_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FieldMapping {
    pub source_field: String,
    pub target_field: String,
    #[serde(default)]
    pub transform: String,
    #[serde(default)]
    pub default_value: String,
    #[serde(default)]
    pub value_mappings: Map<String, Value>,
    #[serde(default)]
    pub value_mapping_case_insensitive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PrepareBatchRequest {
    pub batch_name: String,
    pub source_type: String,
    pub source_name: String,
    #[serde(default)]
    pub source_description: String,
    #[serde(default = "default_conflict_strategy")]
    pub conflict_strategy: String,
    #[serde(default)]
    pub allow_create_factory: bool,
    #[serde(default)]
    pub idempotency_key: String,
    #[serde(default)]
    pub mappings: Vec<FieldMapping>,
    pub rows: Vec<Map<String, Value>>,
}

fn default_conflict_strategy() -> String {
    "REUSE".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecuteBatchRequest {
    pub batch_id: String,
    pub target: ConnectionProfile,
    #[serde(default)]
    pub failed_only: bool,
    pub tenant_id: String,
    pub operator_id: String,
    #[serde(default)]
    pub organization_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationBatch {
    pub batch_id: String,
    pub batch_name: String,
    pub source_type: String,
    pub source_name: String,
    pub source_description: String,
    pub conflict_strategy: String,
    pub allow_create_factory: bool,
    pub idempotency_key: String,
    pub status: String,
    pub total_count: usize,
    pub valid_count: usize,
    pub success_count: usize,
    pub fail_count: usize,
    pub skip_count: usize,
    pub created_at: String,
    pub updated_at: String,
    pub finished_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationRow {
    pub row_id: String,
    pub batch_id: String,
    pub row_no: usize,
    pub source_key: String,
    pub source_hash: String,
    pub status: String,
    pub raw_data: Map<String, Value>,
    pub normalized_data: Map<String, Value>,
    pub error_code: String,
    pub error_message: String,
    pub id_med: String,
    pub id_med_unit: String,
    pub id_fac: String,
    pub id_med_pro: String,
    pub retry_count: usize,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationAudit {
    pub audit_id: String,
    pub batch_id: String,
    pub row_id: String,
    pub trace_id: String,
    pub operation: String,
    pub target_table: String,
    pub target_id: String,
    pub result: String,
    pub before_data: Value,
    pub after_data: Value,
    pub message: String,
    pub operator_id: String,
    pub operated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BatchDetail {
    pub batch: MigrationBatch,
    pub rows: Vec<MigrationRow>,
    pub audits: Vec<MigrationAudit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetField {
    pub key: String,
    pub label: String,
    pub required: bool,
    pub group: String,
    pub value_type: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetReadiness {
    pub ok: bool,
    pub checked_tables: Vec<String>,
    pub warnings: Vec<String>,
    pub message: String,
}
