mod batch;
mod datasource;
mod driver_pack;
mod id;
mod legacy_phis27;
mod local_store;
mod model;
mod normalize;
mod odbc;
mod overwrite;
mod target;
mod target_contract;
mod target_dictionary;
mod target_odbc;
mod target_reference;
mod target_system;

use local_store::LocalStore;
use model::{
    BatchDetail, ConnectionCheck, ConnectionProfile, ExecuteBatchRequest, MigrationBatch,
    OverwritePreview, PrepareBatchRequest, PreviewOverwriteRequest, SourcePreview,
    SourcePreviewRequest, TargetField, TargetReadiness, UndoBatchRequest,
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
fn app_health() -> serde_json::Value {
    serde_json::json!({
        "ready": true,
        "runtime": "tauri-rust",
        "keyRule": "bson-object-id-24-hex",
        "supportedDirectDatabase": ["mysql", "oracle", "dameng", "opengauss", "kingbase", "postgresql"]
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
        .map_err(|error| format!("PHIS27 识别任务异常：{error}"))?
}

#[tauri::command]
async fn load_phis27_medicine(
    request: legacy_phis27::LoadPhis27Request,
) -> Result<SourcePreview, String> {
    tauri::async_runtime::spawn_blocking(move || legacy_phis27::load(&request))
        .await
        .map_err(|error| format!("PHIS27 数据读取任务异常：{error}"))?
}

#[tauri::command]
fn prepare_migration_batch(
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
    let (tenant_id, operator_id) = client.execution_identity()?;
    // Base medicine data is tenant-wide: identity comes exclusively from the authenticated
    // system session, and organization-private scope is deliberately disabled for this task.
    request.tenant_id = tenant_id;
    request.operator_id = operator_id;
    request.organization_id.clear();
    target::execute_batch(&store, request).await
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
            let store = LocalStore::open(&app_data_dir.join("medicine-migration.sqlite"))
                .map_err(std::io::Error::other)?;
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
            test_database_connection,
            inspect_target_schema,
            list_source_tables,
            preview_source,
            inspect_phis27_source,
            load_phis27_medicine,
            prepare_migration_batch,
            preview_overwrite_batch,
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
