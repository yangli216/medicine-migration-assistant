use crate::model::{ConnectionCheck, ConnectionProfile, SourcePreview, TargetReadiness};
use crate::target_contract::{validate_schema_identifier, TARGET_TABLE_PROJECTIONS};
use odbc_api::{
    buffers::{Indicator, TextRowSet},
    escape_attribute_value,
    parameter::VarCharBox,
    Connection, ConnectionOptions, Cursor, Environment, ResultSetMetadata,
};
use serde_json::{Map, Value};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;

const QUERY_TIMEOUT_SECONDS: usize = 60;
const MAX_CELL_BYTES: usize = 64 * 1024;
const ORACLE_BUNDLED_ALIAS: &str = "Oracle 19 ODBC driver";
#[cfg(target_os = "windows")]
const ORACLE_WINDOWS_BUNDLE_DIRECTORY: &str = "instantclient_19_31_bsoft_migration";
#[cfg(target_os = "windows")]
const ORACLE_WINDOWS_DRIVER_NAME: &str = "Oracle in instantclient_19_31_bsoft_migration";
static ODBC_RUNTIME_CONFIGURED: OnceLock<()> = OnceLock::new();

pub fn is_odbc_kind(kind: &str) -> bool {
    matches!(
        normalize_kind(kind).as_str(),
        "oracle"
            | "dameng"
            | "opengauss"
            | "kingbase"
            | "postgresql"
            | "vastbase"
            | "gbase8c"
            | "gbase8a"
            | "gbase8s"
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
    #[cfg(target_os = "macos")]
    if bundled_oracle_driver_path().is_some() {
        drivers.push(ORACLE_BUNDLED_ALIAS.to_string());
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
    let row_limit = limit.clamp(1, 10_000) as usize;
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
            column_metadata: Vec::new(),
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
        if std::env::var_os("NLS_LANG").is_none() {
            std::env::set_var("NLS_LANG", "SIMPLIFIED CHINESE_CHINA.AL32UTF8");
        }
        #[cfg(target_os = "windows")]
        if let Some(directory) = bundled_oracle_runtime_directory() {
            let existing = std::env::var_os("PATH").unwrap_or_default();
            let already_present = std::env::split_paths(&existing).any(|entry| entry == directory);
            if !already_present {
                let paths = std::iter::once(directory).chain(std::env::split_paths(&existing));
                if let Ok(value) = std::env::join_paths(paths) {
                    std::env::set_var("PATH", value);
                }
            }
        }
        #[cfg(target_os = "macos")]
        {
            // Oracle Instant Client otherwise inherits the host locale. On macOS this can make
            // AL32UTF8 Chinese text arrive as question marks even though the database stores it correctly.
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
        "opengauss" | "kingbase" | "postgresql" | "vastbase" | "gbase8c" => {
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

fn bundled_oracle_runtime_directory() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(executable) = std::env::current_exe() {
        #[cfg(target_os = "windows")]
        if let Some(application_directory) = executable.parent() {
            candidates.push(
                application_directory
                    .join("resources")
                    .join("oracle")
                    .join(ORACLE_WINDOWS_BUNDLE_DIRECTORY),
            );
        }
        #[cfg(target_os = "macos")]
        if let Some(contents) = executable.parent().and_then(Path::parent) {
            candidates.push(
                contents
                    .join("Resources")
                    .join("oracle")
                    .join("instantclient_19_16"),
            );
        }
    }
    #[cfg(target_os = "windows")]
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("oracle")
            .join(ORACLE_WINDOWS_BUNDLE_DIRECTORY),
    );
    #[cfg(target_os = "macos")]
    candidates.push(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("oracle")
            .join("instantclient_19_16"),
    );
    candidates.into_iter().find(|directory| {
        #[cfg(target_os = "windows")]
        let driver = directory.join("sqora32.dll");
        #[cfg(target_os = "macos")]
        let driver = directory.join("libsqora.dylib.19.1");
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let driver = directory.join("missing-oracle-driver");
        driver.is_file()
    })
}

fn bundled_oracle_driver_path() -> Option<PathBuf> {
    bundled_oracle_runtime_directory().map(|directory| {
        #[cfg(target_os = "windows")]
        let driver = directory.join("sqora32.dll");
        #[cfg(target_os = "macos")]
        let driver = directory.join("libsqora.dylib.19.1");
        #[cfg(not(any(target_os = "windows", target_os = "macos")))]
        let driver = directory.join("missing-oracle-driver");
        driver
    })
}

fn resolved_driver(profile: &ConnectionProfile) -> String {
    if normalize_kind(&profile.kind) == "oracle"
        && profile
            .driver
            .trim()
            .eq_ignore_ascii_case(ORACLE_BUNDLED_ALIAS)
    {
        #[cfg(target_os = "windows")]
        if bundled_oracle_driver_path().is_some() {
            return ORACLE_WINDOWS_DRIVER_NAME.to_string();
        }
        #[cfg(target_os = "macos")]
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
        && driver.eq_ignore_ascii_case(ORACLE_BUNDLED_ALIAS)
    {
        #[cfg(target_os = "windows")]
        if bundled_oracle_driver_path().is_some()
            && installed
                .iter()
                .any(|item| item.eq_ignore_ascii_case(ORACLE_WINDOWS_DRIVER_NAME))
        {
            return Ok(());
        }
        #[cfg(target_os = "macos")]
        if bundled_oracle_driver_path().is_some() {
            return Ok(());
        }
    }
    if installed
        .iter()
        .any(|item| item.eq_ignore_ascii_case(driver))
    {
        return Ok(());
    }

    let platform = format!("{} {}", std::env::consts::OS, std::env::consts::ARCH);
    let requirement = if normalize_kind(&profile.kind) == "oracle" {
        "应用内置的 Oracle Instant Client 19.31 Basic 与 ODBC 组件"
    } else {
        "数据库厂商提供的 64 位 ODBC 驱动"
    };
    let detected = if installed.is_empty() {
        "本机尚未登记任何 ODBC 驱动".to_string()
    } else {
        format!("本机当前可用：{}", installed.join("、"))
    };
    Err(format!(
        "未检测到 {} 驱动“{driver}”。请修复或重新安装适用于 {platform} 的 {requirement}。{detected}",
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
        .map(stable_string_parameter)
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
        .map(stable_string_parameter)
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

pub fn query_optional_row_strings(
    connection: &Connection<'_>,
    statement: &str,
    values: Vec<String>,
) -> Result<Option<Vec<Option<String>>>, String> {
    let parameters = values
        .into_iter()
        .map(stable_string_parameter)
        .collect::<Vec<_>>();
    let Some(mut cursor) = connection
        .execute(statement, &parameters[..], Some(QUERY_TIMEOUT_SECONDS))
        .map_err(odbc_error)?
    else {
        return Ok(None);
    };
    let column_count = cursor.num_result_cols().map_err(odbc_error)? as usize;
    let mut buffer =
        TextRowSet::for_cursor(1, &mut cursor, Some(MAX_CELL_BYTES)).map_err(odbc_error)?;
    let mut row_cursor = cursor.bind_buffer(&mut buffer).map_err(odbc_error)?;
    let Some(batch) = row_cursor.fetch().map_err(odbc_error)? else {
        return Ok(None);
    };
    Ok(Some(
        (0..column_count)
            .map(|index| {
                batch
                    .at(index, 0)
                    .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
            })
            .collect(),
    ))
}

pub fn query_rows_strings(
    connection: &Connection<'_>,
    statement: &str,
    values: Vec<String>,
    limit: usize,
) -> Result<Vec<Vec<Option<String>>>, String> {
    let row_limit = limit.clamp(1, 10_000);
    let parameters = values
        .into_iter()
        .map(stable_string_parameter)
        .collect::<Vec<_>>();
    let Some(mut cursor) = connection
        .execute(statement, &parameters[..], Some(QUERY_TIMEOUT_SECONDS))
        .map_err(odbc_error)?
    else {
        return Ok(Vec::new());
    };
    let column_count = cursor.num_result_cols().map_err(odbc_error)? as usize;
    let mut buffer =
        TextRowSet::for_cursor(row_limit, &mut cursor, Some(MAX_CELL_BYTES)).map_err(odbc_error)?;
    let mut row_cursor = cursor.bind_buffer(&mut buffer).map_err(odbc_error)?;
    let mut rows = Vec::new();
    while rows.len() < row_limit {
        let Some(batch) = row_cursor.fetch().map_err(odbc_error)? else {
            break;
        };
        for row_index in 0..batch.num_rows() {
            rows.push(
                (0..column_count)
                    .map(|column_index| {
                        batch
                            .at(column_index, row_index)
                            .map(|bytes| String::from_utf8_lossy(bytes).into_owned())
                    })
                    .collect(),
            );
            if rows.len() >= row_limit {
                break;
            }
        }
    }
    Ok(rows)
}

/// Oracle 19c's ODBC driver may abort the entire process when an empty Rust
/// `String` is bound with a zero-sized backing buffer. It reports that value as
/// `VARCHAR(0)` and can enter the driver's LOB preprocessing path before
/// raising `SIGABRT`. Keep the logical value empty while providing a one-byte
/// backing buffer so every driver sees a regular `VARCHAR(1)` parameter.
fn stable_string_parameter(value: String) -> VarCharBox {
    let bytes = value.into_bytes();
    let logical_length = bytes.len();
    let buffer = if bytes.is_empty() {
        vec![0].into_boxed_slice()
    } else {
        bytes.into_boxed_slice()
    };
    VarCharBox::from_buffer(buffer, Indicator::Length(logical_length))
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
        "opengauss" | "kingbase" | "postgresql" | "vastbase" | "gbase8c" => format!(
            "Driver={driver};Servername={host};Port={};Database={database};Uid={username};Pwd={password};",
            profile.port
        ),
        "gbase8a" | "gbase8s" => format!(
            "Driver={driver};Server={host};Port={};Database={database};Uid={username};Pwd={password};",
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
        "vastbaseg100" | "vastbase-g100" | "海量" => "vastbase".to_string(),
        "gbase-8c" | "gbase_8c" => "gbase8c".to_string(),
        "gbase-8a" | "gbase_8a" => "gbase8a".to_string(),
        "gbase-8s" | "gbase_8s" => "gbase8s".to_string(),
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
        "vastbase" => "海量 Vastbase",
        "gbase8c" => "南大通用 GBase 8c",
        "gbase8a" => "南大通用 GBase 8a",
        "gbase8s" => "南大通用 GBase 8s",
        _ => "数据库",
    }
}

fn oracle_error_code(raw: &str) -> Option<String> {
    let upper = raw.to_ascii_uppercase();
    if let Some(offset) = upper.find("ORA-") {
        let digits = upper[offset + 4..]
            .chars()
            .take_while(char::is_ascii_digit)
            .take(5)
            .collect::<String>();
        if digits.len() == 5 {
            return Some(format!("ORA-{digits}"));
        }
    }
    let marker = "NATIVE ERROR:";
    let offset = upper.find(marker)? + marker.len();
    let digits = upper[offset..]
        .trim_start()
        .chars()
        .take_while(char::is_ascii_digit)
        .take(5)
        .collect::<String>();
    let number = digits.parse::<u32>().ok()?;
    (number > 0 && number <= 99_999).then(|| format!("ORA-{number:05}"))
}

fn oracle_error_diagnosis(code: &str) -> Option<(&'static str, &'static str)> {
    match code {
        "ORA-00001" => Some((
            "目标唯一键或主键发生冲突",
            "检查是否重复迁移；优先查看目标业务主键和本地来源映射台账",
        )),
        "ORA-00904" => Some((
            "SQL 使用了目标库中不存在或无效的字段",
            "确认数据库版本、Schema 和字段注释是否与当前迁移模板一致",
        )),
        "ORA-00933" => Some((
            "SQL 语句结构不符合当前 Oracle 版本要求",
            "检查语句末尾、分页语法和数据库方言配置",
        )),
        "ORA-00942" => Some((
            "表或视图不存在，或者当前账号没有读取权限",
            "确认 Schema、表名以及当前账号的 SELECT 权限",
        )),
        "ORA-01031" => Some((
            "当前数据库账号权限不足",
            "为连接账号补充所需表的 SELECT 或 INSERT/UPDATE 权限后重试",
        )),
        "ORA-01400" => Some((
            "目标必填字段不允许写入空值",
            "在字段映射或默认值中补齐该字段，并重新执行校验",
        )),
        "ORA-01427" => Some((
            "本应返回一条记录的子查询返回了多条数据",
            "检查目标业务主键是否重复，并先处理重复基础数据",
        )),
        "ORA-01722" => Some((
            "字符值无法转换为有效数字",
            "检查数量、价格、包装系数等数值字段的源值和转换规则",
        )),
        "ORA-01843" => Some((
            "日期格式无效（无效的月份）",
            "检查效期等日期字段，并使用明确的日期格式或数据库日期参数",
        )),
        "ORA-01861" => Some((
            "日期或数字文本与目标格式不匹配",
            "检查字段转换规则，避免依赖数据库会话的隐式格式转换",
        )),
        "ORA-06502" => Some((
            "数值或字符转换、长度处理发生错误",
            "检查字段长度、数值精度以及空值转换规则",
        )),
        "ORA-12154" => Some((
            "无法解析 Oracle 服务名",
            "检查 Service Name、主机、端口和连接串配置",
        )),
        "ORA-12514" => Some((
            "监听程序未识别配置的数据库服务",
            "确认 Service Name 与 Oracle 监听器中注册的服务完全一致",
        )),
        "ORA-12541" => Some((
            "无法连接到 Oracle 监听程序",
            "检查数据库主机、端口、网络和监听服务状态",
        )),
        "ORA-12704" => Some((
            "查询表达式字符集不匹配，通常是 NVARCHAR2 与 VARCHAR2 在 UNION、CASE 或 NVL 中混用",
            "统一相关表达式的字符类型；应用内置库存查询已改为显式 NVARCHAR2 转换",
        )),
        "ORA-12899" => Some((
            "写入值超过目标字段允许长度",
            "查看失败行和目标字段，缩短来源值或调整目标字段长度",
        )),
        _ => None,
    }
}

fn driver_text_is_garbled(raw: &str) -> bool {
    raw.contains('\u{fffd}') || raw.contains('\0')
}

fn odbc_error(error: impl std::fmt::Display) -> String {
    let raw = error.to_string();
    if let Some(code) = oracle_error_code(&raw) {
        if let Some((reason, advice)) = oracle_error_diagnosis(&code) {
            return format!("数据库操作失败：Oracle {code}：{reason}。处理建议：{advice}");
        }
        return if driver_text_is_garbled(&raw) {
            format!(
                "数据库操作失败：Oracle {code}。驱动返回的文字编码无法识别，请根据错误码排查或提供本次操作日志"
            )
        } else {
            format!("数据库操作失败：Oracle {code}；驱动信息：{raw}")
        };
    }
    if driver_text_is_garbled(&raw) {
        "数据库驱动操作失败：驱动返回的文字编码无法识别，请查看 SQLState 和 Native error 编号后重试"
            .into()
    } else {
        format!("数据库驱动操作失败：{raw}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_connection_string, inspect_target_schema, is_odbc_kind, odbc_error, preview_source,
        query_optional_row_strings, stable_string_parameter, test_connection,
        validate_driver_registration, with_connection,
    };
    use crate::model::ConnectionProfile;
    use odbc_api::{handles::HasDataType, DataType};
    use std::num::NonZeroUsize;

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
        assert!(is_odbc_kind("Vastbase-G100"));
        assert!(is_odbc_kind("GBase-8c"));
        assert!(is_odbc_kind("GBase-8a"));
        assert!(is_odbc_kind("GBase-8s"));
    }

    #[test]
    fn connection_string_escapes_password() {
        let value = build_connection_string(&profile("dameng")).unwrap();
        assert!(value.contains("Pwd={p;ass}"));
        assert!(!value.contains("${PASSWORD}"));
    }

    #[test]
    fn gbase8a_odbc_uses_the_vendor_server_style_connection_string() {
        let value = build_connection_string(&profile("gbase-8a")).unwrap();
        assert!(value.contains("Server=10.0.0.5"));
        assert!(value.contains("Port=5236"));
    }

    #[test]
    fn missing_oracle_driver_is_reported_before_connecting() {
        let mut oracle = profile("oracle");
        oracle.driver = "Oracle in instantclient_19_31".to_string();
        let error = validate_driver_registration(&oracle, &[]).unwrap_err();
        assert!(error.contains("请修复或重新安装"));
        assert!(error.contains("Basic 与 ODBC"));

        assert!(validate_driver_registration(
            &oracle,
            &["ORACLE IN INSTANTCLIENT_19_31".to_string()]
        )
        .is_ok());
    }

    #[test]
    fn empty_string_parameter_has_a_non_zero_driver_buffer() {
        let parameter = stable_string_parameter(String::new());
        assert_eq!(parameter.as_bytes(), Some(&[][..]));
        assert_eq!(
            parameter.data_type(),
            DataType::Varchar {
                length: NonZeroUsize::new(1)
            }
        );
    }

    #[test]
    fn non_empty_string_parameter_is_always_bound_as_utf8_varchar() {
        let parameter = stable_string_parameter("阿莫西林".to_string());
        assert_eq!(parameter.as_bytes(), Some("阿莫西林".as_bytes()));
        assert_eq!(
            parameter.data_type(),
            DataType::Varchar {
                length: NonZeroUsize::new("阿莫西林".len())
            }
        );
    }

    #[test]
    fn known_oracle_error_is_explained_even_when_driver_text_is_garbled() {
        let error = odbc_error("State: HY003, Native error: 1843, ORA-01843: �H��");
        assert!(error.contains("Oracle ORA-01843：日期格式无效（无效的月份）"));
        assert!(error.contains("处理建议"));
        assert!(!error.contains('�'));
        assert!(!error.contains("驱动原文"));
    }

    #[test]
    fn oracle_character_set_error_has_actionable_chinese_message_without_garbled_text() {
        let error = odbc_error(
            "State: HY003, Native error: 12704, Message: [Oracle][ODBC][Ora]ORA-12704: W��9M",
        );
        assert!(error.contains("Oracle ORA-12704"));
        assert!(error.contains("NVARCHAR2 与 VARCHAR2"));
        assert!(error.contains("显式 NVARCHAR2 转换"));
        assert!(!error.contains('�'));
    }

    #[test]
    fn native_oracle_number_is_promoted_to_a_known_oracle_code() {
        let error = odbc_error("State: 42S02, Native error: 942, Message: ��");
        assert!(error.contains("Oracle ORA-00942"));
        assert!(error.contains("表或视图不存在"));
    }

    #[test]
    #[ignore = "需要通过环境变量提供可访问的 Oracle 测试库"]
    fn live_oracle_empty_string_parameter_does_not_enter_lob_binding() {
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

        with_connection(&oracle, |connection| {
            let value = super::query_optional_string(
                connection,
                "SELECT NVL(?, 'EMPTY') FROM DUAL",
                vec![String::new()],
            )?;
            if value.as_deref() != Some("EMPTY") {
                return Err(format!("Oracle 空字符串绑定结果异常：{value:?}"));
            }
            Ok(())
        })
        .expect("Oracle 19c empty VARCHAR binding");
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
        with_connection(&oracle, |connection| {
            let values = query_optional_row_strings(
                connection,
                "SELECT 'snapshot',CAST(NULL AS VARCHAR2(10)) FROM DUAL",
                Vec::new(),
            )?
            .ok_or_else(|| "Oracle 多字段快照查询没有返回数据".to_string())?;
            assert_eq!(values, vec![Some("snapshot".into()), None]);
            Ok(())
        })
        .expect("Oracle field snapshot query");
        if std::env::var("ORACLE_EXPECT_EMPTY").as_deref() == Ok("1") {
            for table in [
                "hi_bd_med",
                "hi_bd_med_alias",
                "hi_bd_med_unit",
                "hi_bd_fac",
                "hi_bd_med_pro",
            ] {
                let count = preview_source(
                    &oracle,
                    &format!("SELECT COUNT(*) AS ROW_COUNT FROM {table}"),
                    1,
                )
                .unwrap_or_else(|error| panic!("count {table}: {error}"));
                let value = count.rows[0]
                    .get("ROW_COUNT")
                    .map(crate::normalize::value_text)
                    .unwrap_or_default();
                println!("empty target table {table} rows={value}");
                assert_eq!(value, "0", "target table {table} is not empty");
            }
        }
    }
}
