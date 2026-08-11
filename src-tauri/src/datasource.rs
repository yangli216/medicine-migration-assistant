use crate::model::{ConnectionCheck, ConnectionProfile, SourcePreview, SourcePreviewRequest};
use crate::odbc;
use crate::pg_protocol;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{NaiveDate, NaiveDateTime, NaiveTime};
use rust_decimal::Decimal;
use serde_json::{Map, Number, Value};
use sqlx_core::column::Column;
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_core::type_info::TypeInfo;
use sqlx_core::value::ValueRef;
use sqlx_mysql::{MySql, MySqlConnectOptions, MySqlPool, MySqlPoolOptions, MySqlRow};
use std::time::{Duration, Instant};

pub async fn connect_mysql(profile: &ConnectionProfile) -> Result<MySqlPool, String> {
    if !profile.kind.eq_ignore_ascii_case("mysql") {
        return Err(format!(
            "当前版本的直连适配器暂不支持 {}，可先使用 CSV/JSON 文件迁移",
            profile.kind
        ));
    }
    let options = MySqlConnectOptions::new()
        .host(&profile.host)
        .port(profile.port)
        .username(&profile.username)
        .password(&profile.password)
        .database(&profile.database);
    MySqlPoolOptions::new()
        .max_connections(4)
        .acquire_timeout(Duration::from_secs(8))
        .connect_with(options)
        .await
        .map_err(|error| friendly_database_error(&error.to_string()))
}

pub async fn test_connection(profile: &ConnectionProfile) -> Result<ConnectionCheck, String> {
    if pg_protocol::uses_native_connection(profile) {
        return pg_protocol::test_connection(profile).await;
    }
    if odbc::is_odbc_kind(&profile.kind) {
        let profile = profile.clone();
        return tauri::async_runtime::spawn_blocking(move || odbc::test_connection(&profile))
            .await
            .map_err(|error| format!("数据库连接任务异常：{error}"))?;
    }
    let started = Instant::now();
    let pool = connect_mysql(profile).await?;
    let version: String = query_scalar::<MySql, String>("SELECT VERSION()")
        .fetch_one(&pool)
        .await
        .map_err(|error| friendly_database_error(&error.to_string()))?;
    pool.close().await;
    Ok(ConnectionCheck {
        ok: true,
        database_version: version,
        latency_ms: started.elapsed().as_millis(),
        message: "连接成功，账号具有读取权限".to_string(),
    })
}

pub async fn list_tables(profile: &ConnectionProfile) -> Result<Vec<String>, String> {
    if pg_protocol::uses_native_connection(profile) {
        return pg_protocol::list_tables(profile).await;
    }
    if odbc::is_odbc_kind(&profile.kind) {
        let profile = profile.clone();
        return tauri::async_runtime::spawn_blocking(move || odbc::list_tables(&profile))
            .await
            .map_err(|error| format!("数据库元数据读取任务异常：{error}"))?;
    }
    let pool = connect_mysql(profile).await?;
    let tables = query_scalar::<MySql, String>(
        "SELECT table_name FROM information_schema.tables WHERE table_schema = ? ORDER BY table_name",
    )
    .bind(&profile.database)
    .fetch_all(&pool)
    .await
    .map_err(|error| friendly_database_error(&error.to_string()))?;
    pool.close().await;
    Ok(tables)
}

pub async fn preview_source(request: &SourcePreviewRequest) -> Result<SourcePreview, String> {
    validate_select_query(&request.query)?;
    if pg_protocol::uses_native_connection(&request.connection) {
        return pg_protocol::preview_source(&request.connection, &request.query, request.limit)
            .await;
    }
    if odbc::is_odbc_kind(&request.connection.kind) {
        let profile = request.connection.clone();
        let source_query = request.query.trim().trim_end_matches(';').to_string();
        let limit = request.limit;
        return tauri::async_runtime::spawn_blocking(move || {
            odbc::preview_source(&profile, &source_query, limit)
        })
        .await
        .map_err(|error| format!("数据库预览任务异常：{error}"))?;
    }
    let started = Instant::now();
    let pool = connect_mysql(&request.connection).await?;
    let limit = request.limit.clamp(1, 10_000);
    let source_query = request.query.trim().trim_end_matches(';');
    let preview_sql = format!(
        "SELECT * FROM ({}) migration_source_preview LIMIT {}",
        source_query,
        limit + 1
    );
    let mysql_rows = query::<MySql>(&preview_sql)
        .fetch_all(&pool)
        .await
        .map_err(|error| friendly_database_error(&error.to_string()))?;
    pool.close().await;

    let columns = mysql_rows
        .first()
        .map(|row| {
            row.columns()
                .iter()
                .map(|column| column.name().to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let truncated = mysql_rows.len() > limit as usize;
    let rows = mysql_rows
        .into_iter()
        .take(limit as usize)
        .map(mysql_row_to_json)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(SourcePreview {
        columns,
        column_metadata: Vec::new(),
        rows,
        truncated,
        elapsed_ms: started.elapsed().as_millis(),
    })
}

fn mysql_row_to_json(row: MySqlRow) -> Result<Map<String, Value>, String> {
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

fn decode_value(row: &MySqlRow, index: usize, type_name: &str) -> Value {
    let kind = type_name.to_ascii_uppercase();
    match kind.as_str() {
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "BIGINT" => row
            .try_get::<i64, _>(index)
            .map(Value::from)
            .or_else(|_| row.try_get::<u64, _>(index).map(Value::from))
            .unwrap_or_else(|_| Value::String("[无法读取的整数]".to_string())),
        "FLOAT" | "DOUBLE" => row
            .try_get::<f64, _>(index)
            .ok()
            .and_then(Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        "DECIMAL" | "NEWDECIMAL" => row
            .try_get::<Decimal, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "DATE" => row
            .try_get::<NaiveDate, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "DATETIME" | "TIMESTAMP" => row
            .try_get::<NaiveDateTime, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "TIME" => row
            .try_get::<NaiveTime, _>(index)
            .map(|value| Value::String(value.to_string()))
            .unwrap_or(Value::Null),
        "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "BINARY" | "VARBINARY" => row
            .try_get::<Vec<u8>, _>(index)
            .map(|value| Value::String(format!("base64:{}", BASE64.encode(value))))
            .unwrap_or(Value::Null),
        _ => row
            .try_get::<String, _>(index)
            .map(Value::String)
            .unwrap_or(Value::Null),
    }
}

fn validate_select_query(query: &str) -> Result<(), String> {
    let normalized = query.trim().to_ascii_lowercase();
    if normalized.len() > 50_000 {
        return Err("查询语句过长，请使用视图或拆分查询".to_string());
    }
    if !(normalized.starts_with("select ") || normalized.starts_with("with ")) {
        return Err("为保护老系统，数据源查询只允许 SELECT 或 WITH 语句".to_string());
    }
    let forbidden = [
        " insert ",
        " update ",
        " delete ",
        " drop ",
        " alter ",
        " truncate ",
        " replace ",
    ];
    let padded = format!(" {} ", normalized.replace(['\n', '\r', '\t'], " "));
    if forbidden.iter().any(|keyword| padded.contains(keyword)) {
        return Err("查询中包含写操作关键字；老系统连接必须保持只读".to_string());
    }
    Ok(())
}

fn friendly_database_error(raw: &str) -> String {
    let lower = raw.to_ascii_lowercase();
    if lower.contains("access denied") || lower.contains("authentication") {
        "数据库拒绝登录，请检查账号、密码和访问授权".to_string()
    } else if lower.contains("timed out") || lower.contains("connection refused") {
        "无法连接数据库，请检查地址、端口、防火墙或网络".to_string()
    } else if lower.contains("unknown database") {
        "数据库名称不存在或当前账号无权访问".to_string()
    } else {
        format!("数据库操作失败：{}", raw)
    }
}

#[cfg(test)]
mod tests {
    use super::validate_select_query;

    #[test]
    fn only_read_queries_are_allowed() {
        assert!(validate_select_query("select * from legacy_drug").is_ok());
        assert!(validate_select_query("with d as (select 1) select * from d").is_ok());
        assert!(validate_select_query("delete from legacy_drug").is_err());
        assert!(validate_select_query("select 1; drop table x").is_err());
    }
}
