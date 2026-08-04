use crate::model::{ConnectionCheck, ConnectionProfile, SourcePreview, TargetReadiness};
use crate::target_contract::{validate_schema_identifier, TARGET_TABLE_PROJECTIONS};
use odbc_api::{
    buffers::TextRowSet, escape_attribute_value, Connection, ConnectionOptions, Cursor,
    Environment, IntoParameter, ResultSetMetadata,
};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

const QUERY_TIMEOUT_SECONDS: usize = 60;
const MAX_CELL_BYTES: usize = 64 * 1024;
static ODBC_RUNTIME_CONFIGURED: OnceLock<()> = OnceLock::new();

pub fn is_odbc_kind(kind: &str) -> bool {
    matches!(
        normalize_kind(kind).as_str(),
        "oracle" | "dameng" | "opengauss" | "kingbase" | "postgresql"
    )
}

pub fn list_installed_drivers() -> Result<Vec<String>, String> {
    configure_odbc_runtime();
    let environment = Environment::new().map_err(odbc_error)?;
    let mut drivers = environment
        .drivers()
        .map_err(odbc_error)?
        .into_iter()
        .map(|driver| driver.description)
        .collect::<Vec<_>>();
    if bundled_oracle_driver_path().is_some() {
        drivers.push("Oracle 19 ODBC driver".to_string());
    }
    drivers.sort();
    drivers.dedup();
    Ok(drivers)
}

pub fn test_connection(profile: &ConnectionProfile) -> Result<ConnectionCheck, String> {
    let started = Instant::now();
    with_connection(profile, |connection| {
        let product = connection
            .database_management_system_name()
            .unwrap_or_else(|_| display_kind(&profile.kind).to_string());
        let ping_sql = match normalize_kind(&profile.kind).as_str() {
            "oracle" | "dameng" => "SELECT 1 FROM DUAL",
            _ => "SELECT 1",
        };
        connection
            .execute(ping_sql, (), Some(QUERY_TIMEOUT_SECONDS))
            .map_err(odbc_error)?;
        Ok(ConnectionCheck {
            ok: true,
            database_version: product,
            latency_ms: started.elapsed().as_millis(),
            message: "连接成功，企业数据库驱动工作正常".to_string(),
        })
    })
}

pub fn list_tables(profile: &ConnectionProfile) -> Result<Vec<String>, String> {
    with_connection(profile, |connection| {
        let schema = if profile.schema.trim().is_empty() {
            "%"
        } else {
            profile.schema.trim()
        };
        let mut tables = Vec::new();
        for item in connection
            .tables("", schema, "%", "TABLE")
            .map_err(odbc_error)?
        {
            let item = item.map_err(odbc_error)?;
            if let Some(table) = item.table.as_str().map_err(odbc_error)? {
                tables.push(table.to_string());
            }
        }
        tables.sort();
        tables.dedup();
        Ok(tables)
    })
}

pub fn preview_source(
    profile: &ConnectionProfile,
    query: &str,
    limit: u32,
) -> Result<SourcePreview, String> {
    let started = Instant::now();
    let row_limit = limit.clamp(1, 1000) as usize;
    with_connection(profile, |connection| {
        let mut cursor = connection
            .execute(query, (), Some(QUERY_TIMEOUT_SECONDS))
            .map_err(odbc_error)?
            .ok_or_else(|| "只读查询没有返回结果集".to_string())?;
        let columns = cursor
            .column_names()
            .map_err(odbc_error)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(odbc_error)?;
        let mut buffer = TextRowSet::for_cursor(row_limit + 1, &mut cursor, Some(MAX_CELL_BYTES))
            .map_err(odbc_error)?;
        let mut row_cursor = cursor.bind_buffer(&mut buffer).map_err(odbc_error)?;
        let mut rows = Vec::new();
        while rows.len() <= row_limit {
            let Some(batch) = row_cursor.fetch().map_err(odbc_error)? else {
                break;
            };
            for row_index in 0..batch.num_rows() {
                let mut row = Map::new();
                for (column_index, column_name) in columns.iter().enumerate() {
                    let value = batch
                        .at(column_index, row_index)
                        .map(|bytes| Value::String(String::from_utf8_lossy(bytes).into_owned()))
                        .unwrap_or(Value::Null);
                    row.insert(column_name.clone(), value);
                }
                rows.push(row);
                if rows.len() > row_limit {
                    break;
                }
            }
        }
        let truncated = rows.len() > row_limit;
        rows.truncate(row_limit);
        Ok(SourcePreview {
            columns,
            rows,
            truncated,
            elapsed_ms: started.elapsed().as_millis(),
        })
    })
}

pub fn with_connection<T>(
    profile: &ConnectionProfile,
    operation: impl FnOnce(&Connection<'_>) -> Result<T, String>,
) -> Result<T, String> {
    if !is_odbc_kind(&profile.kind) {
        return Err(format!("{} 不是企业数据库 ODBC 适配类型", profile.kind));
    }
    configure_odbc_runtime();
    let environment = Environment::new().map_err(odbc_error)?;
    validate_driver_available(profile, &environment)?;
    let connection_string = build_connection_string(profile)?;
    let connection = environment
        .connect_with_connection_string(
            &connection_string,
            ConnectionOptions {
                login_timeout_sec: Some(10),
                packet_size: None,
            },
        )
        .map_err(|error| {
            let detail = odbc_error(error);
            if detail.to_ascii_lowercase().contains("driver") {
                format!(
                    "{}；请确认已安装与应用位数一致的 {} 官方 ODBC 驱动",
                    detail,
                    display_kind(&profile.kind)
                )
            } else {
                detail
            }
        })?;
    operation(&connection)
}

fn configure_odbc_runtime() {
    ODBC_RUNTIME_CONFIGURED.get_or_init(|| {
        #[cfg(target_os = "macos")]
        {
            // Oracle Instant Client otherwise inherits the host locale. On macOS this can make
            // AL32UTF8 Chinese text arrive as question marks even though the database stores it correctly.
            if std::env::var_os("NLS_LANG").is_none() {
                std::env::set_var("NLS_LANG", "SIMPLIFIED CHINESE_CHINA.AL32UTF8");
            }
            let system_ini = Path::new("/usr/local/etc/odbcinst.ini");
            if std::env::var_os("ODBCSYSINI").is_none() && system_ini.is_file() {
                std::env::set_var("ODBCSYSINI", "/usr/local/etc");
            }
        }
    });
}

pub fn configure_target_session(
    connection: &Connection<'_>,
    profile: &ConnectionProfile,
) -> Result<(), String> {
    let schema = validate_schema_identifier(&profile.schema)?;
    if schema.is_empty() {
        return Ok(());
    }
    let statement = match normalize_kind(&profile.kind).as_str() {
        "oracle" => format!("ALTER SESSION SET CURRENT_SCHEMA = {schema}"),
        "dameng" => format!("SET SCHEMA {schema}"),
        "opengauss" | "kingbase" | "postgresql" => {
            format!("SET search_path TO {schema}")
        }
        _ => return Ok(()),
    };
    connection
        .execute(&statement, (), Some(QUERY_TIMEOUT_SECONDS))
        .map_err(odbc_error)?;
    Ok(())
}

pub fn inspect_target_schema(profile: &ConnectionProfile) -> Result<TargetReadiness, String> {
    with_connection(profile, |connection| {
        configure_target_session(connection, profile)?;
        let mut checked_tables = Vec::new();
        for (table, columns) in TARGET_TABLE_PROJECTIONS {
            let statement = format!("SELECT {columns} FROM {table} WHERE 1=0");
            connection
                .execute(&statement, (), Some(QUERY_TIMEOUT_SECONDS))
                .map_err(|error| {
                    format!(
                        "目标表结构预检失败（{table}）：{}。请检查 Schema、字段版本和查询权限",
                        odbc_error(error)
                    )
                })?;
            checked_tables.push((*table).to_string());
        }
        Ok(TargetReadiness {
            ok: true,
            checked_tables,
            warnings: vec![
                "已验证五张药品表及所需字段可读取；实际 INSERT 权限会在逐行事务写入时验证".into(),
                "直接写表不会触发原 Java 的 medSaveSuccessEvent，ID_SRV 基础服务关联需在上线前单独确认".into(),
            ],
            message: "目标药品表结构与当前迁移版本兼容".into(),
        })
    })
}

fn bundled_oracle_driver_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        if let Some(contents) = executable.parent().and_then(Path::parent) {
            candidates.push(
                contents
                    .join("Resources")
                    .join("oracle")
                    .join("instantclient_19_16"),
            );
        }
    }
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("oracle")
            .join("instantclient_19_16"),
    );
    candidates
        .into_iter()
        .map(|directory| directory.join("libsqora.dylib.19.1"))
        .find(|driver| driver.is_file())
}

fn resolved_driver(profile: &ConnectionProfile) -> String {
    if normalize_kind(&profile.kind) == "oracle"
        && profile
            .driver
            .trim()
            .eq_ignore_ascii_case("Oracle 19 ODBC driver")
    {
        if let Some(path) = bundled_oracle_driver_path() {
            return path.to_string_lossy().into_owned();
        }
    }
    profile.driver.trim().to_string()
}

fn validate_driver_available(
    profile: &ConnectionProfile,
    environment: &Environment,
) -> Result<(), String> {
    if !profile.connection_string.trim().is_empty() {
        return Ok(());
    }
    let installed = environment
        .drivers()
        .map_err(odbc_error)?
        .into_iter()
        .map(|item| item.description)
        .collect::<Vec<_>>();
    validate_driver_registration(profile, &installed)
}

fn validate_driver_registration(
    profile: &ConnectionProfile,
    installed: &[String],
) -> Result<(), String> {
    let driver = profile.driver.trim();
    if driver.is_empty() {
        return Err(format!(
            "请选择已安装的 {} ODBC 驱动，或填写高级连接串",
            display_kind(&profile.kind)
        ));
    }
    let looks_like_path = driver.contains('/') || driver.contains('\\');
    if looks_like_path {
        return if Path::new(driver).is_file() {
            Ok(())
        } else {
            Err(format!(
                "ODBC 驱动文件不存在：{driver}。请选择当前电脑上的有效驱动文件"
            ))
        };
    }
    if normalize_kind(&profile.kind) == "oracle"
        && driver.eq_ignore_ascii_case("Oracle 19 ODBC driver")
        && bundled_oracle_driver_path().is_some()
    {
        return Ok(());
    }
    if installed
        .iter()
        .any(|item| item.eq_ignore_ascii_case(driver))
    {
        return Ok(());
    }

    let platform = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let requirement = if normalize_kind(&profile.kind) == "oracle" {
        "Oracle Instant Client 19c 的 Basic Light 与 ODBC 组件"
    } else {
        "数据库厂商提供的 64 位 ODBC 驱动"
    };
    let detected = if installed.is_empty() {
        "本机尚未登记任何 ODBC 驱动".to_string()
    } else {
        format!("本机当前可用：{}", installed.join("、"))
    };
    Err(format!(
        "未检测到 {} 驱动“{driver}”。当前应用只预置了连接配置，驱动文件尚未安装；请先安装适用于 {platform} 的 {requirement}。{detected}",
        display_kind(&profile.kind)
    ))
}

pub fn execute_strings(
    connection: &Connection<'_>,
    statement: &str,
    values: Vec<String>,
) -> Result<(), String> {
    let parameters = values
        .into_iter()
        .map(IntoParameter::into_parameter)
        .collect::<Vec<_>>();
    connection
        .execute(statement, &parameters[..], Some(QUERY_TIMEOUT_SECONDS))
        .map_err(odbc_error)?;
    Ok(())
}

pub fn query_optional_string(
    connection: &Connection<'_>,
    statement: &str,
    values: Vec<String>,
) -> Result<Option<String>, String> {
    let parameters = values
        .into_iter()
        .map(IntoParameter::into_parameter)
        .collect::<Vec<_>>();
    let Some(mut cursor) = connection
        .execute(statement, &parameters[..], Some(QUERY_TIMEOUT_SECONDS))
        .map_err(odbc_error)?
    else {
        return Ok(None);
    };
    let mut buffer =
        TextRowSet::for_cursor(1, &mut cursor, Some(MAX_CELL_BYTES)).map_err(odbc_error)?;
    let mut row_cursor = cursor.bind_buffer(&mut buffer).map_err(odbc_error)?;
    let Some(batch) = row_cursor.fetch().map_err(odbc_error)? else {
        return Ok(None);
    };
    Ok(batch
        .at(0, 0)
        .map(|bytes| String::from_utf8_lossy(bytes).into_owned()))
}

pub fn build_connection_string(profile: &ConnectionProfile) -> Result<String, String> {
    if !profile.connection_string.trim().is_empty() {
        return Ok(expand_connection_template(
            profile.connection_string.trim(),
            profile,
        ));
    }
    if profile.driver.trim().is_empty() {
        return Err(format!(
            "请选择已安装的 {} ODBC 驱动，或填写高级连接串",
            display_kind(&profile.kind)
        ));
    }
    let resolved_driver = resolved_driver(profile);
    let driver = escape_attribute_value(&resolved_driver);
    let host = escape_attribute_value(profile.host.trim());
    let database = escape_attribute_value(profile.database.trim());
    let username = escape_attribute_value(profile.username.trim());
    let password = escape_attribute_value(&profile.password);
    let service = if profile.service_name.trim().is_empty() {
        profile.database.trim()
    } else {
        profile.service_name.trim()
    };
    let value = match normalize_kind(&profile.kind).as_str() {
        "oracle" => format!(
            "Driver={driver};Dbq={host}:{}/{};Uid={username};Pwd={password};",
            profile.port,
            service
        ),
        "dameng" => format!(
            "Driver={driver};Server={host};Port={};Database={database};Uid={username};Pwd={password};",
            profile.port
        ),
        "opengauss" | "kingbase" | "postgresql" => format!(
            "Driver={driver};Servername={host};Port={};Database={database};Uid={username};Pwd={password};",
            profile.port
        ),
        _ => return Err(format!("不支持的数据库类型：{}", profile.kind)),
    };
    Ok(value)
}

fn expand_connection_template(template: &str, profile: &ConnectionProfile) -> String {
    template
        .replace("${HOST}", profile.host.trim())
        .replace("${PORT}", &profile.port.to_string())
        .replace("${DATABASE}", profile.database.trim())
        .replace("${SERVICE}", profile.service_name.trim())
        .replace("${SCHEMA}", profile.schema.trim())
        .replace("${USER}", profile.username.trim())
        .replace("${PASSWORD}", &escape_attribute_value(&profile.password))
}

pub fn normalize_kind(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "dm" | "dm8" | "dameng" => "dameng".to_string(),
        "gauss" | "gaussdb" | "opengauss" => "opengauss".to_string(),
        "kingbase" | "kingbasees" | "人大金仓" => "kingbase".to_string(),
        "postgres" | "postgresql" | "pg" => "postgresql".to_string(),
        value => value.to_string(),
    }
}

fn display_kind(kind: &str) -> &'static str {
    match normalize_kind(kind).as_str() {
        "oracle" => "Oracle",
        "dameng" => "达梦 DM8",
        "opengauss" => "Gauss/openGauss",
        "kingbase" => "人大金仓 KingbaseES",
        "postgresql" => "PostgreSQL",
        _ => "数据库",
    }
}

fn odbc_error(error: impl std::fmt::Display) -> String {
    format!("数据库驱动操作失败：{error}")
}

#[cfg(test)]
mod tests {
    use super::{
        build_connection_string, inspect_target_schema, is_odbc_kind, preview_source,
        test_connection, validate_driver_registration,
    };
    use crate::model::ConnectionProfile;

    fn profile(kind: &str) -> ConnectionProfile {
        ConnectionProfile {
            kind: kind.to_string(),
            host: "10.0.0.5".to_string(),
            port: 5236,
            database: "HIS".to_string(),
            username: "migration".to_string(),
            password: "p;ass".to_string(),
            schema: "HIS".to_string(),
            service_name: "ORCLPDB".to_string(),
            driver: "Vendor ODBC".to_string(),
            connection_string: String::new(),
        }
    }

    #[test]
    fn recognizes_enterprise_database_aliases() {
        assert!(is_odbc_kind("oracle"));
        assert!(is_odbc_kind("DM8"));
        assert!(is_odbc_kind("GaussDB"));
        assert!(is_odbc_kind("人大金仓"));
    }

    #[test]
    fn connection_string_escapes_password() {
        let value = build_connection_string(&profile("dameng")).unwrap();
        assert!(value.contains("Pwd={p;ass}"));
        assert!(!value.contains("${PASSWORD}"));
    }

    #[test]
    fn missing_oracle_driver_is_reported_before_connecting() {
        let mut oracle = profile("oracle");
        oracle.driver = "Oracle in instantclient_19_31".to_string();
        let error = validate_driver_registration(&oracle, &[]).unwrap_err();
        assert!(error.contains("当前应用只预置了连接配置"));
        assert!(error.contains("Basic Light 与 ODBC"));

        assert!(validate_driver_registration(
            &oracle,
            &["ORACLE IN INSTANTCLIENT_19_31".to_string()]
        )
        .is_ok());
    }

    #[test]
    #[ignore = "需要通过环境变量提供可访问的 Oracle 测试库"]
    fn live_oracle_connection_uses_registered_19c_driver() {
        let mut oracle = profile("oracle");
        oracle.host = std::env::var("ORACLE_TEST_HOST").expect("ORACLE_TEST_HOST");
        oracle.port = std::env::var("ORACLE_TEST_PORT")
            .unwrap_or_else(|_| "1521".to_string())
            .parse()
            .expect("ORACLE_TEST_PORT");
        oracle.database = std::env::var("ORACLE_TEST_SERVICE").expect("ORACLE_TEST_SERVICE");
        oracle.service_name = oracle.database.clone();
        oracle.username = std::env::var("ORACLE_TEST_USER").expect("ORACLE_TEST_USER");
        oracle.schema = oracle.username.clone();
        oracle.password = std::env::var("ORACLE_TEST_PASSWORD").expect("ORACLE_TEST_PASSWORD");
        oracle.driver = std::env::var("ORACLE_TEST_DRIVER")
            .unwrap_or_else(|_| "Oracle 19 ODBC driver".to_string());

        let check = test_connection(&oracle).expect("Oracle 19c ODBC connection");
        assert!(check.ok);
        let readiness = inspect_target_schema(&oracle).expect("Oracle medicine target schema");
        assert_eq!(readiness.checked_tables.len(), 5);
        let chinese = preview_source(&oracle, "SELECT UNISTR('\\5F53\\5F52') AS CN FROM DUAL", 1)
            .expect("Oracle AL32UTF8 Chinese query");
        assert_eq!(
            chinese.rows[0].get("CN").and_then(|value| value.as_str()),
            Some("当归")
        );
    }
}
