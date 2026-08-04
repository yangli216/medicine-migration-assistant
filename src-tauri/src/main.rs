#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::var_os("MEDICINE_MIGRATION_ORACLE_SMOKE_TEST").is_some() {
        match medicine_migration_assistant_lib::oracle_smoke_test_from_env() {
            Ok(check) => {
                println!(
                    "oracle-smoke-ok database={} latency_ms={}",
                    check.database_version, check.latency_ms
                );
                return;
            }
            Err(error) => {
                eprintln!("oracle-smoke-failed: {error}");
                std::process::exit(1);
            }
        }
    }
    medicine_migration_assistant_lib::run();
}
