mod adapter_settings;
mod batch;
mod connection_settings;
mod cost_merge_settings;
mod datasource;
mod driver_pack;
mod id;
mod inventory;
mod legacy_phis27;
mod local_store;
mod model;
mod normalize;
mod odbc;
mod overwrite;
mod pg_protocol;
mod target;
mod target_contract;
mod target_dictionary;
mod target_odbc;
mod target_pg;
mod target_reference;
mod target_system;

use local_store::LocalStore;
use model::{
    BatchDetail, ConnectionCheck, ConnectionProfile, ExecuteBatchRequest, MigrationBatch,
    OverwritePreview, PrepareBatchRequest, PreviewOverwriteRequest, SourcePreview,
    SourcePreviewRequest, TargetField, TargetReadiness, TrialMigrationRequest,
    TrialMigrationResponse, UndoBatchRequest,
};
use tauri::{Manager, State};

#[tauri::command]
async fn probe_target_system(
    client: State<'_, target_system::TargetSystemClient>,
    request: target_system::TargetSystemProbeRequest,
) -> Result<target_system::TargetSystemProbe, String> {
    target_system::probe(&client, request).await
}

#[tauri::command]
async fn login_target_system(
    client: State<'_, target_system::TargetSystemClient>,
    request: target_system::TargetSystemLoginRequest,
) -> Result<target_system::TargetSystemLogin, String> {
    target_system::login(&client, request).await
}

#[tauri::command]
async fn load_target_dictionaries(
    client: State<'_, target_system::TargetSystemClient>,
) -> Result<target_dictionary::TargetDictionaryCatalog, String> {
    target_dictionary::load(&client).await
}

#[tauri::command]
async fn load_medicine_cost_merges(
    client: State<'_, target_system::TargetSystemClient>,
) -> Result<target_reference::MedicineCostMergeCatalog, String> {
    target_reference::load_medicine_cost_merges(&client).await
}

#[tauri::command]
async fn load_inventory_target_organizations(
    client: State<'_, target_system::TargetSystemClient>,
) -> Result<target_system::TargetOrganizationCatalog, String> {
    target_system::load_organizations(&client).await
}

#[tauri::command]
fn app_health() -> serde_json::Value {
    serde_json::json!({
        "ready": true,
        "runtime": "tauri-rust",
        "keyRule": "bson-object-id-24-hex",
        "supportedDirectDatabase": ["mysql", "oracle", "dameng", "opengauss", "vastbase", "gbase8c", "gbase8a", "gbase8s", "kingbase", "postgresql"]
    })
}

#[tauri::command]
fn generate_object_id() -> String {
    id::new_object_id()
}

#[tauri::command]
fn get_target_fields() -> Vec<TargetField> {
    normalize::target_fields()
}

#[tauri::command]
fn list_database_drivers() -> Result<Vec<String>, String> {
    odbc::list_installed_drivers()
}

#[tauri::command]
fn list_driver_packs() -> Result<Vec<driver_pack::DriverPackStatus>, String> {
    let installed = odbc::list_installed_drivers().unwrap_or_default();
    driver_pack::list(&installed)
}

#[tauri::command]
fn load_saved_connections(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
) -> Result<connection_settings::SavedConnections, String> {
    connection_settings::load(&store, &cipher)
}

#[tauri::command]
fn list_database_connections(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
) -> Result<Vec<connection_settings::SavedDatabaseConnection>, String> {
    connection_settings::list_database_connections(&store, &cipher)
}

#[tauri::command]
fn save_database_connection(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
    request: connection_settings::SaveDatabaseConnectionRequest,
) -> Result<connection_settings::SavedDatabaseConnection, String> {
    connection_settings::save_database_connection(&store, &cipher, request)
}

#[tauri::command]
fn delete_database_connection(
    store: State<'_, LocalStore>,
    connection_id: String,
) -> Result<(), String> {
    connection_settings::delete_database_connection(&store, &connection_id)
}

#[tauri::command]
fn save_source_connection(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
    request: connection_settings::SaveSourceConnectionRequest,
) -> Result<connection_settings::SavedSourceConnection, String> {
    connection_settings::save_source(&store, &cipher, request)
}

#[tauri::command]
fn save_target_system_connection(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
    request: connection_settings::SaveTargetSystemConnectionRequest,
) -> Result<connection_settings::SavedTargetSystemConnection, String> {
    connection_settings::save_target_system(&store, &cipher, request)
}

#[tauri::command]
fn save_target_database_connection(
    store: State<'_, LocalStore>,
    cipher: State<'_, connection_settings::LocalCredentialCipher>,
    request: connection_settings::SaveSourceConnectionRequest,
) -> Result<connection_settings::SavedSourceConnection, String> {
    connection_settings::save_target_database(&store, &cipher, request)
}

#[tauri::command]
fn forget_source_connection(store: State<'_, LocalStore>) -> Result<(), String> {
    connection_settings::forget_source(&store)
}

#[tauri::command]
fn forget_target_system_connection(store: State<'_, LocalStore>) -> Result<(), String> {
    connection_settings::forget_target_system(&store)
}

#[tauri::command]
fn forget_target_database_connection(store: State<'_, LocalStore>) -> Result<(), String> {
    connection_settings::forget_target_database(&store)
}

#[tauri::command]
fn load_phis27_mapping_profile(
    store: State<'_, LocalStore>,
) -> Result<Option<adapter_settings::Phis27MappingProfile>, String> {
    adapter_settings::load(&store)
}

#[tauri::command]
fn save_phis27_mapping_profile(
    store: State<'_, LocalStore>,
    request: adapter_settings::SavePhis27MappingProfileRequest,
) -> Result<adapter_settings::Phis27MappingProfile, String> {
    adapter_settings::save(&store, request)
}

#[tauri::command]
fn load_cost_merge_mapping_profile(
    store: State<'_, LocalStore>,
    request: cost_merge_settings::CostMergeMappingProfileScope,
) -> Result<Option<cost_merge_settings::CostMergeMappingProfile>, String> {
    cost_merge_settings::load(&store, request)
}

#[tauri::command]
fn save_cost_merge_mapping_profile(
    store: State<'_, LocalStore>,
    request: cost_merge_settings::SaveCostMergeMappingProfileRequest,
) -> Result<cost_merge_settings::CostMergeMappingProfile, String> {
    cost_merge_settings::save(&store, request)
}

#[tauri::command]
async fn test_database_connection(profile: ConnectionProfile) -> Result<ConnectionCheck, String> {
    datasource::test_connection(&profile).await
}

#[tauri::command]
async fn inspect_target_schema(profile: ConnectionProfile) -> Result<TargetReadiness, String> {
    target::inspect_schema(&profile).await
}

#[tauri::command]
async fn list_source_tables(profile: ConnectionProfile) -> Result<Vec<String>, String> {
    datasource::list_tables(&profile).await
}

#[tauri::command]
async fn preview_source(request: SourcePreviewRequest) -> Result<SourcePreview, String> {
    datasource::preview_source(&request).await
}

#[tauri::command]
async fn inspect_phis27_source(
    profile: ConnectionProfile,
) -> Result<legacy_phis27::Phis27Inspection, String> {
    tauri::async_runtime::spawn_blocking(move || legacy_phis27::inspect(&profile))
        .await
        .map_err(|error| format!("二系列phis识别任务异常：{error}"))?
}

#[tauri::command]
async fn load_phis27_medicine(
    request: legacy_phis27::LoadPhis27Request,
) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || legacy_phis27::load(&request))
        .await
        .map_err(|error| format!("二系列phis数据读取任务异常：{error}"))?
}

#[tauri::command]
async fn load_phis27_inventory_catalog(
    profile: ConnectionProfile,
) -> Result<legacy_phis27::Phis27InventoryReferenceCatalog, String> {
    tauri::async_runtime::spawn_blocking(move || {
        legacy_phis27::load_inventory_reference_catalog(&profile)
    })
    .await
    .map_err(|error| format!("二系列phis机构和库房读取任务异常：{error}"))?
}

#[tauri::command]
async fn load_inventory_target_storages(
    client: State<'_, target_system::TargetSystemClient>,
    target: ConnectionProfile,
) -> Result<inventory::TargetStorageCatalog, String> {
    let (tenant_id, _) = client.execution_identity()?;
    inventory::load_target_storages(&target, &tenant_id).await
}

#[tauri::command]
fn load_inventory_location_mappings(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    source_name: String,
    target: ConnectionProfile,
) -> Result<Vec<local_store::InventoryLocationMapping>, String> {
    let (tenant_id, _) = client.execution_identity()?;
    inventory::load_saved_mappings(&store, &tenant_id, &source_name, &target)
}

#[tauri::command]
fn load_inventory_organization_mappings(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    source_name: String,
) -> Result<Vec<local_store::InventoryOrganizationMapping>, String> {
    let (tenant_id, _) = client.execution_identity()?;
    inventory::load_saved_organization_mappings(&store, &tenant_id, &source_name)
}

#[tauri::command]
async fn prepare_phis27_inventory(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: inventory::PrepareInventoryRequest,
) -> Result<BatchDetail, String> {
    let (tenant_id, _) = client.execution_identity()?;
    inventory::prepare(&store, &tenant_id, request).await
}

#[tauri::command]
async fn execute_phis27_inventory(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: inventory::ExecuteInventoryRequest,
) -> Result<BatchDetail, String> {
    let (tenant_id, operator_id) = client.execution_identity()?;
    inventory::execute(&store, &tenant_id, &operator_id, request).await
}

#[tauri::command]
async fn trial_phis27_inventory(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: inventory::TrialInventoryRequest,
) -> Result<inventory::TrialInventoryResponse, String> {
    let (tenant_id, operator_id) = client.execution_identity()?;
    inventory::trial(&store, &tenant_id, &operator_id, request).await
}

#[tauri::command]
async fn preview_phis27_inventory_undo(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: inventory::UndoInventoryRequest,
) -> Result<inventory::InventoryUndoPreview, String> {
    let (tenant_id, _) = client.execution_identity()?;
    inventory::preview_undo(&store, &tenant_id, request).await
}

#[tauri::command]
async fn undo_phis27_inventory(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: inventory::UndoInventoryRequest,
) -> Result<BatchDetail, String> {
    let (tenant_id, operator_id) = client.execution_identity()?;
    inventory::undo(&store, &tenant_id, &operator_id, request).await
}

#[tauri::command]
fn inspect_phis27_inventory(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: legacy_phis27::InspectPhis27InventoryRequest,
) -> Result<legacy_phis27::Phis27InventoryReadiness, String> {
    let (tenant_id, _) = client.execution_identity()?;
    legacy_phis27::inspect_inventory(&store, &tenant_id, &request)
}

#[tauri::command]
async fn prepare_migration_batch(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: PrepareBatchRequest,
) -> Result<BatchDetail, String> {
    let (tenant_id, _) = client.execution_identity()?;
    let dictionary_values = client.dictionary_values()?;
    target_dictionary::ensure_required_loaded(&dictionary_values)?;
    batch::prepare_batch(&store, request, &dictionary_values, &tenant_id)
}

#[tauri::command]
async fn execute_migration_batch(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    mut request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.source_type == "PHIS27_INVENTORY" {
        return Err("库存批次必须使用“首次盘点”专用执行入口，已阻止误写药品基础表".into());
    }
    if !request.failed_only
        && detail.batch.success_count == 0
        && !target::has_successful_trial(&detail, &request.target)
    {
        return Err(
            "正式迁移前请先在迁移明细中选择一条数据完成单条试迁移；试迁移会真实执行目标写入并自动回滚"
                .into(),
        );
    }
    let (tenant_id, operator_id) = client.execution_identity()?;
    let invalid_count = detail
        .rows
        .iter()
        .filter(|row| row.status == "INVALID")
        .count();
    if !request.failed_only && invalid_count > 0 {
        if !request.skip_invalid_rows {
            return Err(format!(
                "本批次还有 {invalid_count} 条校验失败数据；请返回校验页修正，或明确选择“仅迁移校验通过的数据”"
            ));
        }
        batch::skip_invalid_rows(&store, &request.batch_id, &operator_id)?;
    }
    // Base medicine data is tenant-wide: identity comes exclusively from the authenticated
    // system session, and organization-private scope is deliberately disabled for this task.
    request.tenant_id = tenant_id;
    request.operator_id = operator_id;
    request.organization_id.clear();
    target::execute_batch(&store, request).await
}

#[tauri::command]
async fn trial_migration_row(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    mut request: TrialMigrationRequest,
) -> Result<TrialMigrationResponse, String> {
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.source_type == "PHIS27_INVENTORY" {
        return Err("机构库存必须按首次盘点整体核对，不支持单条试迁移".into());
    }
    let (tenant_id, operator_id) = client.execution_identity()?;
    request.tenant_id = tenant_id;
    request.operator_id = operator_id;
    request.organization_id.clear();
    target::trial_row(&store, request).await
}

#[tauri::command]
async fn preview_overwrite_batch(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: PreviewOverwriteRequest,
) -> Result<OverwritePreview, String> {
    let (tenant_id, operator_id) = client.execution_identity()?;
    target::preview_overwrite(&store, request, &tenant_id, &operator_id).await
}

#[tauri::command]
async fn undo_migration_batch(
    store: State<'_, LocalStore>,
    client: State<'_, target_system::TargetSystemClient>,
    request: UndoBatchRequest,
) -> Result<BatchDetail, String> {
    let (tenant_id, operator_id) = client.execution_identity()?;
    target::undo_batch(&store, request, &tenant_id, &operator_id).await
}

#[tauri::command]
fn load_migration_batch(
    store: State<'_, LocalStore>,
    batch_id: String,
) -> Result<BatchDetail, String> {
    store.load_batch(&batch_id)
}

#[tauri::command]
fn list_recent_batches(
    store: State<'_, LocalStore>,
    limit: Option<usize>,
) -> Result<Vec<MigrationBatch>, String> {
    store.recent_batches(limit.unwrap_or(30).clamp(1, 100))
}

#[doc(hidden)]
pub fn oracle_smoke_test_from_env() -> Result<ConnectionCheck, String> {
    let service =
        std::env::var("ORACLE_TEST_SERVICE").map_err(|_| "缺少 ORACLE_TEST_SERVICE".to_string())?;
    let profile = ConnectionProfile {
        kind: "oracle".to_string(),
        host: std::env::var("ORACLE_TEST_HOST").map_err(|_| "缺少 ORACLE_TEST_HOST".to_string())?,
        port: std::env::var("ORACLE_TEST_PORT")
            .unwrap_or_else(|_| "1521".to_string())
            .parse()
            .map_err(|_| "ORACLE_TEST_PORT 不是有效端口".to_string())?,
        database: service.clone(),
        username: std::env::var("ORACLE_TEST_USER")
            .map_err(|_| "缺少 ORACLE_TEST_USER".to_string())?,
        password: std::env::var("ORACLE_TEST_PASSWORD")
            .map_err(|_| "缺少 ORACLE_TEST_PASSWORD".to_string())?,
        schema: std::env::var("ORACLE_TEST_USER")
            .map_err(|_| "缺少 ORACLE_TEST_USER".to_string())?,
        service_name: service,
        driver: std::env::var("ORACLE_TEST_DRIVER")
            .unwrap_or_else(|_| "Oracle 19 ODBC driver".to_string()),
        connection_string: String::new(),
    };
    let check = odbc::test_connection(&profile)?;
    let readiness = odbc::inspect_target_schema(&profile)?;
    if readiness.checked_tables.len() != 5 {
        return Err("Oracle 药品目标表结构预检数量异常".into());
    }
    let chinese =
        odbc::preview_source(&profile, "SELECT UNISTR('\\5F53\\5F52') AS CN FROM DUAL", 1)?;
    if chinese.rows.first().and_then(|row| row.get("CN"))
        != Some(&serde_json::Value::String("当归".into()))
    {
        return Err("Oracle 中文字符集校验失败，预期读取“当归”".into());
    }
    Ok(check)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&app_data_dir)?;
            let credential_cipher = connection_settings::LocalCredentialCipher::open(&app_data_dir)
                .map_err(std::io::Error::other)?;
            let store = LocalStore::open(&app_data_dir.join("medicine-migration.sqlite"))
                .map_err(std::io::Error::other)?;
            app.manage(credential_cipher);
            app.manage(store);
            app.manage(target_system::TargetSystemClient::new().map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_health,
            generate_object_id,
            get_target_fields,
            list_database_drivers,
            list_driver_packs,
            load_saved_connections,
            list_database_connections,
            save_database_connection,
            delete_database_connection,
            save_source_connection,
            save_target_system_connection,
            save_target_database_connection,
            forget_source_connection,
            forget_target_system_connection,
            forget_target_database_connection,
            load_phis27_mapping_profile,
            save_phis27_mapping_profile,
            load_cost_merge_mapping_profile,
            save_cost_merge_mapping_profile,
            test_database_connection,
            inspect_target_schema,
            list_source_tables,
            preview_source,
            inspect_phis27_source,
            load_phis27_medicine,
            load_phis27_inventory_catalog,
            inspect_phis27_inventory,
            load_inventory_target_organizations,
            load_inventory_target_storages,
            load_inventory_location_mappings,
            load_inventory_organization_mappings,
            prepare_phis27_inventory,
            execute_phis27_inventory,
            trial_phis27_inventory,
            preview_phis27_inventory_undo,
            undo_phis27_inventory,
            prepare_migration_batch,
            preview_overwrite_batch,
            trial_migration_row,
            execute_migration_batch,
            undo_migration_batch,
            load_migration_batch,
            list_recent_batches,
            probe_target_system,
            login_target_system,
            load_target_dictionaries,
            load_medicine_cost_merges
        ])
        .run(tauri::generate_context!())
        .expect("failed to run medicine migration assistant");
}
