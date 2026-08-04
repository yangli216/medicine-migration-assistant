mod batch;
mod datasource;
mod driver_pack;
mod id;
mod local_store;
mod model;
mod normalize;
mod odbc;
mod target;
mod target_contract;
mod target_odbc;

use local_store::LocalStore;
use model::{
    BatchDetail, ConnectionCheck, ConnectionProfile, ExecuteBatchRequest, MigrationBatch,
    PrepareBatchRequest, SourcePreview, SourcePreviewRequest, TargetField, TargetReadiness,
};
use tauri::{Manager, State};

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
fn prepare_migration_batch(
    store: State<'_, LocalStore>,
    request: PrepareBatchRequest,
) -> Result<BatchDetail, String> {
    batch::prepare_batch(&store, request)
}

#[tauri::command]
async fn execute_migration_batch(
    store: State<'_, LocalStore>,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    target::execute_batch(&store, request).await
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
            prepare_migration_batch,
            execute_migration_batch,
            load_migration_batch,
            list_recent_batches
        ])
        .run(tauri::generate_context!())
        .expect("failed to run medicine migration assistant");
}
