use crate::model::{
    ConnectionCheck, ConnectionProfile, SourceColumnMetadata, SourcePreview, TargetReadiness,
};
use crate::target_contract::{validate_schema_identifier, TARGET_TABLE_PROJECTIONS};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde_json::{Map, Number, Value};
use sqlx_core::column::Column;
use sqlx_core::error::Error as SqlxError;
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_core::type_info::TypeInfo;
use sqlx_core::value::ValueRef;
use sqlx_postgres::{PgConnectOptions, PgPool, PgPoolOptions, PgRow, PgSslMode, Postgres};
use std::time::{Duration, Instant};

const PHIS_BUSINESS_TIME_ZONE: &str = "Asia/Shanghai";

pub fn is_pg_protocol_kind(kind: &str) -> bool {
    matches!(
        normalize_kind(kind).as_str(),
        "postgresql" | "opengauss" | "vastbase" | "gbase8c" | "kingbase"
    )
}

pub fn uses_native_connection(profile: &ConnectionProfile) -> bool {
    is_pg_protocol_kind(&profile.kind)
        && profile.driver.trim().is_empty()
        && profile.connection_string.trim().is_empty()
}

pub async fn connect(profile: &ConnectionProfile) -> Result<PgPool, String> {
    if !is_pg_protocol_kind(&profile.kind) {
        return Err(format!("{} 不是 PostgreSQL 协议兼容数据库", profile.kind));
    }
    let options = PgConnectOptions::new()
        .host(profile.host.trim())
        .port(profile.port)
        .username(profile.username.trim())
        .password(&profile.password)
        .database(profile.database.trim())
        .application_name("medicine-migration-assistant")
        // PHIS stores business timestamps in TIMESTAMP columns without a time-zone
        // component. Fix the session to the PHIS business time zone so CURRENT_TIMESTAMP
        // is persisted on the same wall clock used by its date-range queries.
        .options([("timezone", PHIS_BUSINESS_TIME_ZONE)])
        .ssl_mode(PgSslMode::Prefer);
    let pool = PgPoolOptions::new()
        // A single connection keeps the optional search_path deterministic for
        // arbitrary read-only preview SQL without relying on session defaults.
        .max_connections(1)
        .acquire_timeout(Duration::from_secs(8))
        .connect_with(options)
        .await
        .map_err(|error| friendly_sqlx_error(&error))?;
    configure_schema(&pool, &profile.schema).await?;
    Ok(pool)
}

async fn configure_schema(pool: &PgPool, schema: &str) -> Result<(), String> {
    let schema = validate_schema_identifier(schema)?;
    if !schema.is_empty() {
        query::<Postgres>(&format!("SET search_path TO {schema}"))
            .execute(pool)
            .await
            .map_err(|error| friendly_sqlx_error(&error))?;
    }
    Ok(())
}

pub async fn test_connection(profile: &ConnectionProfile) -> Result<ConnectionCheck, String> {
    let started = Instant::now();
    let pool = connect(profile).await?;
    let version: String = query_scalar::<Postgres, String>("SELECT version()")
        .fetch_one(&pool)
        .await
        .map_err(|error| friendly_sqlx_error(&error))?;
    pool.close().await;
    Ok(ConnectionCheck {
        ok: true,
        database_version: version,
        latency_ms: started.elapsed().as_millis(),
        message: "连接成功，正在使用应用内置的 PostgreSQL 通用协议，无需安装 ODBC 驱动".into(),
    })
}

pub async fn list_tables(profile: &ConnectionProfile) -> Result<Vec<String>, String> {
    let pool = connect(profile).await?;
    let schema = validate_schema_identifier(&profile.schema)?;
    let tables = if schema.is_empty() {
        query_scalar::<Postgres, String>(
            "SELECT table_name FROM information_schema.tables WHERE table_schema=current_schema() AND table_type IN ('BASE TABLE','VIEW') ORDER BY table_name",
        )
        .fetch_all(&pool)
        .await
    } else {
        query_scalar::<Postgres, String>(
            "SELECT table_name FROM information_schema.tables WHERE table_schema=$1 AND table_type IN ('BASE TABLE','VIEW') ORDER BY table_name",
        )
        .bind(schema)
        .fetch_all(&pool)
        .await
    }
    .map_err(|error| friendly_sqlx_error(&error))?;
    pool.close().await;
    Ok(tables)
}

pub async fn preview_source(
    profile: &ConnectionProfile,
    source_query: &str,
    limit: u32,
) -> Result<SourcePreview, String> {
    let started = Instant::now();
    let pool = connect(profile).await?;
    let limit = limit.clamp(1, 10_000);
    let source_query = source_query.trim().trim_end_matches(';');
    let statement = format!(
        "SELECT * FROM ({source_query}) migration_source_preview LIMIT {}",
        limit + 1
    );
    let result = query::<Postgres>(&statement).fetch_all(&pool).await;
    pool.close().await;
    let pg_rows = result.map_err(|error| friendly_sqlx_error(&error))?;
    let columns = pg_rows
        .first()
        .map(|row| {
            row.columns()
                .iter()
                .map(|column| column.name().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let truncated = pg_rows.len() > limit as usize;
    let rows = pg_rows
        .into_iter()
        .take(limit as usize)
        .map(pg_row_to_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SourcePreview {
        columns,
        column_metadata: Vec::new(),
        rows,
        truncated,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

pub async fn source_object_column_metadata(
    profile: &ConnectionProfile,
    object_name: &str,
) -> Result<Vec<SourceColumnMetadata>, String> {
    let pool = connect(profile).await?;
    let schema = validate_schema_identifier(&profile.schema)?;
    let rows = if schema.is_empty() {
        query::<Postgres>(
            "SELECT a.attname AS column_name, COALESCE(pg_catalog.col_description(c.oid, a.attnum), '') AS column_comment FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid WHERE n.nspname = current_schema() AND c.relname = $1 AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum",
        )
        .bind(object_name)
        .fetch_all(&pool)
        .await
    } else {
        query::<Postgres>(
            "SELECT a.attname AS column_name, COALESCE(pg_catalog.col_description(c.oid, a.attnum), '') AS column_comment FROM pg_catalog.pg_class c JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace JOIN pg_catalog.pg_attribute a ON a.attrelid = c.oid WHERE n.nspname = $1 AND c.relname = $2 AND a.attnum > 0 AND NOT a.attisdropped ORDER BY a.attnum",
        )
        .bind(&schema)
        .bind(object_name)
        .fetch_all(&pool)
        .await
    };
    pool.close().await;
    rows.map_err(|error| friendly_sqlx_error(&error))?
        .into_iter()
        .map(|row| {
            let name = row
                .try_get::<String, _>("column_name")
                .map_err(|error| friendly_sqlx_error(&error))?;
            let comment = row
                .try_get::<String, _>("column_comment")
                .unwrap_or_default();
            Ok(SourceColumnMetadata {
                source_table: object_name.to_string(),
                source_column: name.clone(),
                name,
                comment,
                mapping_eligible: true,
                source_dictionary: None,
            })
        })
        .collect()
}

pub async fn inspect_target_schema(profile: &ConnectionProfile) -> Result<TargetReadiness, String> {
    let pool = connect(profile).await?;
    let result = inspect_pool(&pool).await;
    pool.close().await;
    result
}

pub(crate) async fn inspect_pool(pool: &PgPool) -> Result<TargetReadiness, String> {
    let mut checked_tables = Vec::new();
    for (table, columns) in TARGET_TABLE_PROJECTIONS {
        query::<Postgres>(&format!("SELECT {columns} FROM {table} WHERE 1=0"))
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                format!(
                    "目标表结构预检失败（{table}）：{}。请检查数据库、Schema、字段版本和查询权限",
                    friendly_sqlx_error(&error)
                )
            })?;
        checked_tables.push((*table).to_string());
    }
    Ok(TargetReadiness {
        ok: true,
        checked_tables,
        warnings: vec![
            "结构检查正在使用应用内置的 PostgreSQL 通用协议".into(),
            "正式迁移会对每条来源记录开启独立事务，任一步骤失败都会整行回滚".into(),
        ],
        message: "目标药品表结构与当前迁移版本兼容".into(),
    })
}

fn pg_row_to_json(row: PgRow) -> Result<Map<String, Value>, String> {
    let mut result = Map::new();
    for (index, column) in row.columns().iter().enumerate() {
        let raw = row.try_get_raw(index).map_err(|error| error.to_string())?;
        let value = if raw.is_null() {
            Value::Null
        } else {
            decode_value(&row, index, column.type_info().name())
        };
        result.insert(column.name().to_string(), value);
    }
    Ok(result)
}

fn decode_value(row: &PgRow, index: usize, type_name: &str) -> Value {
    match type_name.to_ascii_uppercase().as_str() {
        "BOOL" => row
            .try_get::<bool, _>(index)
            .map(Value::Bool)
            .unwrap_or(Value::Null),
        "INT2" => row
            .try_get::<i16, _>(index)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "INT4" => row
            .try_get::<i32, _>(index)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "INT8" => row
            .try_get::<i64, _>(index)
            .map(Value::from)
            .unwrap_or(Value::Null),
        "FLOAT4" => row
            .try_get::<f32, _>(index)
            .ok()
            .and_then(|value| Number::from_f64(value as f64))
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "FLOAT8" => row
            .try_get::<f64, _>(index)
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "NUMERIC" => row
            .try_get::<Decimal, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "DATE" => row
            .try_get::<NaiveDate, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "TIME" => row
            .try_get::<NaiveTime, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "TIMESTAMPTZ" => row
            .try_get::<DateTime<Utc>, _>(index)
            .map(|value| Value::String(value.to_rfc3339()))
            .unwrap_or(Value::Null),
        "BYTEA" => row
            .try_get::<Vec<u8>, _>(index)
            .map(|value| Value::String(format!("base64:{}", BASE64.encode(value))))
            .unwrap_or(Value::Null),
        "JSON" | "JSONB" => row.try_get::<Value, _>(index).unwrap_or(Value::Null),
        _ => row
            .try_get::<String, _>(index)
            .map(Value::String)
            .unwrap_or_else(|_| Value::String("[当前类型请使用文本转换后预览]".into())),
    }
}

fn normalize_kind(kind: &str) -> String {
    match kind.trim().to_ascii_lowercase().as_str() {
        "postgres" | "pg" => "postgresql".into(),
        "gauss" | "gaussdb" | "open_gauss" => "opengauss".into(),
        "vastbaseg100" | "vastbase-g100" => "vastbase".into(),
        "gbase-8c" | "gbase_8c" => "gbase8c".into(),
        "kingbasees" | "kingbase_es" => "kingbase".into(),
        other => other.into(),
    }
}

fn friendly_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("password authentication failed") || lower.contains("authentication") {
        "数据库拒绝登录，请检查账号、密码、pg_hba.conf 和访问授权".into()
    } else if lower.contains("connection refused") || lower.contains("timed out") {
        "无法连接数据库，请检查地址、端口、防火墙和服务监听配置".into()
    } else if lower.contains("database") && lower.contains("does not exist") {
        "数据库名称不存在或当前账号无权访问".into()
    } else if lower.contains("no pg_hba.conf entry") {
        "服务器未允许当前电脑访问，请由数据库管理员配置 pg_hba.conf".into()
    } else {
        format!("PostgreSQL 协议操作失败：{raw}")
    }
}

fn friendly_sqlx_error(error: &SqlxError) -> String {
    if let Some(database_error) = error.as_database_error() {
        let code = database_error.code().map(|value| value.into_owned());
        return format_database_error(code.as_deref(), database_error.message());
    }
    friendly_error(&error.to_string())
}

fn format_database_error(code: Option<&str>, message: &str) -> String {
    let cause = match code {
        Some("42P01") => Some("目标表不存在或当前 Schema 不正确"),
        Some("42703") => Some("目标字段不存在或字段版本不兼容"),
        Some("42501") => Some("当前账号没有执行该操作的权限"),
        Some("3F000") => Some("目标 Schema 不存在"),
        _ => None,
    };
    match (code, cause) {
        (Some(code), Some(cause)) => {
            format!("数据库返回 SQLSTATE {code}（{cause}）：{message}")
        }
        (Some(code), None) => format!("数据库返回 SQLSTATE {code}：{message}"),
        (None, _) => format!("数据库返回错误：{message}"),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        format_database_error, is_pg_protocol_kind, uses_native_connection, PHIS_BUSINESS_TIME_ZONE,
    };
    use crate::model::ConnectionProfile;

    #[test]
    fn pg_compatible_families_use_the_bundled_protocol() {
        for kind in [
            "postgresql",
            "openGauss",
            "Vastbase",
            "gbase-8c",
            "KingbaseES",
        ] {
            assert!(is_pg_protocol_kind(kind), "{kind}");
        }
        for kind in ["oracle", "dameng", "gbase8a", "gbase8s", "mysql"] {
            assert!(!is_pg_protocol_kind(kind), "{kind}");
        }
    }

    #[test]
    fn explicit_odbc_configuration_enables_the_compatibility_fallback() {
        let mut profile = ConnectionProfile {
            kind: "vastbase".into(),
            host: "127.0.0.1".into(),
            port: 5432,
            database: "test".into(),
            username: "tester".into(),
            password: String::new(),
            schema: String::new(),
            service_name: String::new(),
            driver: String::new(),
            connection_string: String::new(),
        };
        assert!(uses_native_connection(&profile));
        profile.driver = "Vastbase ODBC Driver".into();
        assert!(!uses_native_connection(&profile));
    }

    #[test]
    fn database_error_keeps_sqlstate_and_original_message() {
        assert_eq!(
            format_database_error(Some("42703"), "字段 id_alias 不存在"),
            "数据库返回 SQLSTATE 42703（目标字段不存在或字段版本不兼容）：字段 id_alias 不存在"
        );
        assert_eq!(
            format_database_error(Some("XX999"), "兼容数据库自定义错误"),
            "数据库返回 SQLSTATE XX999：兼容数据库自定义错误"
        );
    }

    #[test]
    fn pg_compatible_connections_use_the_phis_business_time_zone() {
        assert_eq!(PHIS_BUSINESS_TIME_ZONE, "Asia/Shanghai");
    }
}
