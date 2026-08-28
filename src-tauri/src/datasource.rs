use crate::model::{
    ConnectionCheck, ConnectionProfile, SourceColumnMetadata, SourceObjectCount,
    SourceObjectCountRequest, SourceObjectPreview, SourceObjectPreviewRequest,
    SourceObjectSurveyItem, SourceObjectSurveyRequest, SourcePreview, SourcePreviewRequest,
};
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
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

const SOURCE_OBJECT_BATCH_LIMIT: u32 = 10_000;
const SOURCE_OBJECT_COUNT_PROBE_LIMIT: u32 = SOURCE_OBJECT_BATCH_LIMIT + 1;
const SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS: u64 = 12;
const SOURCE_OBJECT_SURVEY_MAXIMUM: usize = 5;
const SOURCE_OBJECT_SURVEY_ROW_LIMIT: u32 = 2;
const SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS: u64 = 8;

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
        "SELECT table_name FROM information_schema.tables WHERE table_schema = ? AND table_type IN ('BASE TABLE','VIEW') ORDER BY table_name",
    )
    .bind(&profile.database)
    .fetch_all(&pool)
    .await
    .map_err(|error| friendly_database_error(&error.to_string()))?;
    pool.close().await;
    Ok(tables)
}

pub async fn preview_source_object(
    request: &SourceObjectPreviewRequest,
) -> Result<SourceObjectPreview, String> {
    let object_name = request.object_name.trim();
    if object_name.is_empty() || object_name.chars().count() > 256 {
        return Err("请选择有效的来源表或视图".into());
    }
    let available = list_tables(&request.connection).await?;
    let Some(canonical_name) = available.iter().find(|name| name.as_str() == object_name) else {
        return Err(format!(
            "来源表或视图 {object_name} 已不存在或当前账号不可见，请刷新清单后重试"
        ));
    };
    preview_verified_source_object(&request.connection, canonical_name, request.limit, None).await
}

async fn preview_verified_source_object(
    connection: &ConnectionProfile,
    canonical_name: &str,
    limit: u32,
    timeout_seconds: Option<u64>,
) -> Result<SourceObjectPreview, String> {
    let query = source_object_select_query(&connection.kind, &connection.schema, canonical_name)?;
    let preview_request = SourcePreviewRequest {
        connection: connection.clone(),
        query: query.clone(),
        limit,
    };
    let mut preview = if let Some(seconds) = timeout_seconds {
        if odbc::is_odbc_kind(&connection.kind) {
            let profile = connection.clone();
            let source_query = query.clone();
            tauri::async_runtime::spawn_blocking(move || {
                odbc::preview_source_with_timeout(&profile, &source_query, limit, seconds as usize)
            })
            .await
            .map_err(|error| format!("数据库候选预览任务异常：{error}"))??
        } else {
            tokio::time::timeout(
                Duration::from_secs(seconds),
                preview_source(&preview_request),
            )
            .await
            .map_err(|_| format!("候选预览超过 {seconds} 秒，已停止等待"))??
        }
    } else {
        preview_source(&preview_request).await?
    };
    let metadata_result = if let Some(seconds) = timeout_seconds {
        match tokio::time::timeout(
            Duration::from_secs(seconds),
            source_object_column_metadata(connection, canonical_name),
        )
        .await
        {
            Ok(result) => result,
            Err(_) => Err("候选字段注释读取超时".to_string()),
        }
    } else {
        source_object_column_metadata(connection, canonical_name).await
    };
    let metadata_message = match &metadata_result {
        Ok(items) if items.iter().any(|item| !item.comment.trim().is_empty()) => format!(
            "已读取 {} 个字段注释",
            items
                .iter()
                .filter(|item| !item.comment.trim().is_empty())
                .count()
        ),
        Ok(_) => "当前数据库未提供字段注释，将按字段名和样例辅助识别".to_string(),
        Err(_) => "当前驱动无法读取字段注释，将按字段名和样例辅助识别".to_string(),
    };
    let metadata = metadata_result.unwrap_or_default();
    merge_source_object_metadata(&mut preview, canonical_name, metadata);
    Ok(SourceObjectPreview {
        object_name: canonical_name.to_string(),
        query,
        preview,
        metadata_message,
    })
}

fn validate_source_object_survey_names(object_names: &[String]) -> Result<Vec<String>, String> {
    if object_names.is_empty() {
        return Err("候选结构核对至少需要 1 个来源对象".into());
    }
    if object_names.len() > SOURCE_OBJECT_SURVEY_MAXIMUM {
        return Err(format!(
            "候选结构核对一次最多允许 {} 个来源对象",
            SOURCE_OBJECT_SURVEY_MAXIMUM
        ));
    }
    let mut seen = HashSet::new();
    let mut normalized = Vec::with_capacity(object_names.len());
    for object_name in object_names {
        let name = object_name.trim();
        if name.is_empty() || name.chars().count() > 256 {
            return Err("候选结构核对包含无效的来源对象名称".into());
        }
        if !seen.insert(name.to_string()) {
            return Err(format!("候选结构核对包含重复对象 {name}"));
        }
        normalized.push(name.to_string());
    }
    Ok(normalized)
}

pub async fn survey_source_objects(
    request: &SourceObjectSurveyRequest,
) -> Result<Vec<SourceObjectSurveyItem>, String> {
    let object_names = validate_source_object_survey_names(&request.object_names)?;
    let available = match tokio::time::timeout(
        Duration::from_secs(SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS),
        list_tables(&request.connection),
    )
    .await
    {
        Ok(Ok(items)) => items,
        Ok(Err(_)) => return Err("无法读取来源对象清单，请检查只读连接和 Schema 权限".into()),
        Err(_) => {
            return Err(format!(
                "来源对象清单读取超过 {} 秒，已停止等待",
                SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS
            ))
        }
    };
    let mut results = Vec::with_capacity(object_names.len());
    for object_name in object_names {
        let Some(canonical_name) = available.iter().find(|name| name.as_str() == object_name)
        else {
            results.push(SourceObjectSurveyItem {
                object_name,
                status: "FAILED".into(),
                source_result: None,
                message: "对象已不存在或当前账号不可见，可刷新清单后重试".into(),
            });
            continue;
        };
        let surveyed = tokio::time::timeout(
            Duration::from_secs(SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS),
            preview_verified_source_object(
                &request.connection,
                canonical_name,
                SOURCE_OBJECT_SURVEY_ROW_LIMIT,
                Some(SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS),
            ),
        )
        .await;
        match surveyed {
            Ok(Ok(source_result)) => results.push(SourceObjectSurveyItem {
                object_name: canonical_name.clone(),
                status: "READY".into(),
                source_result: Some(source_result),
                message: String::new(),
            }),
            Ok(Err(_)) | Err(_) => results.push(SourceObjectSurveyItem {
                object_name: canonical_name.clone(),
                status: "FAILED".into(),
                source_result: None,
                message: format!(
                    "未能在 {} 秒内读取两行候选样例，可稍后单独预览",
                    SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS
                ),
            }),
        }
    }
    Ok(results)
}

pub async fn count_source_object_rows(
    request: &SourceObjectCountRequest,
) -> Result<SourceObjectCount, String> {
    let object_name = request.object_name.trim();
    if object_name.is_empty() || object_name.chars().count() > 256 {
        return Err("请选择有效的来源表或视图".into());
    }
    let available = list_tables(&request.connection).await?;
    let Some(canonical_name) = available.iter().find(|name| name.as_str() == object_name) else {
        return Err(format!(
            "来源表或视图 {object_name} 已不存在或当前账号不可见，请刷新清单后重试"
        ));
    };
    let query = source_object_count_query(
        &request.connection.kind,
        &request.connection.schema,
        canonical_name,
    )?;
    let preview = if odbc::is_odbc_kind(&request.connection.kind) {
        let profile = request.connection.clone();
        tauri::async_runtime::spawn_blocking(move || {
            odbc::preview_source_with_timeout(
                &profile,
                &query,
                1,
                SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS as usize,
            )
        })
        .await
        .map_err(|error| format!("数据库行数探测任务异常：{error}"))??
    } else {
        let count_request = SourcePreviewRequest {
            connection: request.connection.clone(),
            query,
            limit: 1,
        };
        tokio::time::timeout(
            Duration::from_secs(SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS),
            preview_source(&count_request),
        )
        .await
        .map_err(|_| {
            format!(
                "后台行数探测超过 {} 秒，已停止等待",
                SOURCE_OBJECT_PROBE_TIMEOUT_SECONDS
            )
        })??
    };
    let value = preview
        .rows
        .first()
        .and_then(|row| row.values().next())
        .ok_or_else(|| "数据库没有返回表/视图行数".to_string())?;
    let row_count = match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    }
    .ok_or_else(|| "数据库返回的表/视图行数无法识别".to_string())?;
    Ok(SourceObjectCount {
        object_name: canonical_name.clone(),
        row_count,
        is_exact: row_count < u64::from(SOURCE_OBJECT_COUNT_PROBE_LIMIT),
        probe_limit: SOURCE_OBJECT_COUNT_PROBE_LIMIT,
        elapsed_ms: preview.elapsed_ms,
    })
}

async fn source_object_column_metadata(
    profile: &ConnectionProfile,
    object_name: &str,
) -> Result<Vec<SourceColumnMetadata>, String> {
    if pg_protocol::uses_native_connection(profile) {
        return pg_protocol::source_object_column_metadata(profile, object_name).await;
    }
    if odbc::is_odbc_kind(&profile.kind) {
        let profile = profile.clone();
        let object_name = object_name.to_string();
        return tauri::async_runtime::spawn_blocking(move || {
            odbc::source_object_column_metadata(&profile, &object_name)
        })
        .await
        .map_err(|error| format!("数据库字段注释读取任务异常：{error}"))?;
    }
    let pool = connect_mysql(profile).await?;
    let rows = query::<MySql>(
        "SELECT column_name, COALESCE(column_comment, '') AS column_comment FROM information_schema.columns WHERE table_schema = ? AND table_name = ? ORDER BY ordinal_position",
    )
    .bind(&profile.database)
    .bind(object_name)
    .fetch_all(&pool)
    .await
    .map_err(|error| friendly_database_error(&error.to_string()));
    pool.close().await;
    rows?
        .into_iter()
        .map(|row| {
            let name = row
                .try_get::<String, _>("column_name")
                .map_err(|error| friendly_database_error(&error.to_string()))?;
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

fn merge_source_object_metadata(
    preview: &mut SourcePreview,
    object_name: &str,
    metadata: Vec<SourceColumnMetadata>,
) {
    if preview.columns.is_empty() {
        preview.columns = metadata.iter().map(|item| item.name.clone()).collect();
    }
    let mut metadata_by_name = metadata
        .into_iter()
        .map(|item| (item.name.clone(), item))
        .collect::<HashMap<_, _>>();
    preview.column_metadata = preview
        .columns
        .iter()
        .map(|name| {
            metadata_by_name
                .remove(name)
                .unwrap_or_else(|| SourceColumnMetadata {
                    name: name.clone(),
                    comment: String::new(),
                    source_table: object_name.to_string(),
                    source_column: name.clone(),
                    mapping_eligible: true,
                    source_dictionary: None,
                })
        })
        .collect();
}

fn quote_source_object_identifier(kind: &str, value: &str) -> Result<String, String> {
    if value.is_empty()
        || value.chars().count() > 256
        || value
            .chars()
            .any(|character| character == '\0' || character.is_control())
    {
        return Err("来源表或视图名称无效".into());
    }
    if kind.eq_ignore_ascii_case("mysql") {
        Ok(format!("`{}`", value.replace('`', "``")))
    } else {
        Ok(format!("\"{}\"", value.replace('"', "\"\"")))
    }
}

fn source_object_select_query(kind: &str, schema: &str, object: &str) -> Result<String, String> {
    Ok(format!(
        "SELECT * FROM {}",
        source_object_qualified_name(kind, schema, object)?
    ))
}

fn source_object_count_query(kind: &str, schema: &str, object: &str) -> Result<String, String> {
    let source = source_object_qualified_name(kind, schema, object)?;
    let kind = kind.trim().to_ascii_lowercase();
    if matches!(kind.as_str(), "oracle" | "dameng" | "dm" | "dm8") {
        return Ok(format!(
            "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT 1 FROM {source} WHERE ROWNUM <= {SOURCE_OBJECT_COUNT_PROBE_LIMIT}) MIGRATION_ROW_PROBE"
        ));
    }
    if matches!(kind.as_str(), "gbase8s" | "gbase-8s" | "gbase_8s") {
        return Ok(format!(
            "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT FIRST {SOURCE_OBJECT_COUNT_PROBE_LIMIT} 1 AS PROBE_VALUE FROM {source}) MIGRATION_ROW_PROBE"
        ));
    }
    Ok(format!(
        "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT 1 FROM {source} LIMIT {SOURCE_OBJECT_COUNT_PROBE_LIMIT}) MIGRATION_ROW_PROBE"
    ))
}

pub(crate) fn source_object_qualified_name(
    kind: &str,
    schema: &str,
    object: &str,
) -> Result<String, String> {
    let quoted_object = quote_source_object_identifier(kind, object)?;
    if kind.eq_ignore_ascii_case("mysql") || schema.trim().is_empty() {
        return Ok(quoted_object);
    }
    let quoted_schema = quote_source_object_identifier(kind, schema.trim())?;
    Ok(format!("{quoted_schema}.{quoted_object}"))
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
    if !matches!(
        normalized.split_whitespace().next(),
        Some("select" | "with")
    ) {
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
    use super::{
        merge_source_object_metadata, quote_source_object_identifier, source_object_count_query,
        source_object_select_query, validate_select_query, validate_source_object_survey_names,
        SOURCE_OBJECT_SURVEY_MAXIMUM, SOURCE_OBJECT_SURVEY_ROW_LIMIT,
        SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS,
    };
    use crate::model::{SourceColumnMetadata, SourcePreview};

    #[test]
    fn only_read_queries_are_allowed() {
        assert!(validate_select_query("select * from legacy_drug").is_ok());
        assert!(validate_select_query("select\n  1 from dual").is_ok());
        assert!(validate_select_query("select\t1 from dual").is_ok());
        assert!(validate_select_query("with d as (select 1) select * from d").is_ok());
        assert!(validate_select_query("delete from legacy_drug").is_err());
        assert!(validate_select_query("select 1; drop table x").is_err());
    }

    #[test]
    fn source_object_identifiers_are_quoted_without_becoming_sql() {
        assert_eq!(
            quote_source_object_identifier("mysql", "drug`master").unwrap(),
            "`drug``master`"
        );
        assert_eq!(
            quote_source_object_identifier("oracle", "DRUG\"MASTER").unwrap(),
            "\"DRUG\"\"MASTER\""
        );
        assert!(quote_source_object_identifier("oracle", "bad\nname").is_err());
        assert_eq!(
            source_object_select_query("oracle", "PHIS", "V_DRUG").unwrap(),
            "SELECT * FROM \"PHIS\".\"V_DRUG\""
        );
        assert_eq!(
            source_object_select_query("mysql", "ignored", "drug").unwrap(),
            "SELECT * FROM `drug`"
        );
        assert_eq!(
            source_object_count_query("oracle", "PHIS", "V_DRUG").unwrap(),
            "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT 1 FROM \"PHIS\".\"V_DRUG\" WHERE ROWNUM <= 10001) MIGRATION_ROW_PROBE"
        );
        assert_eq!(
            source_object_count_query("postgresql", "public", "drug").unwrap(),
            "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT 1 FROM \"public\".\"drug\" LIMIT 10001) MIGRATION_ROW_PROBE"
        );
        assert_eq!(
            source_object_count_query("gbase8s", "his", "drug").unwrap(),
            "SELECT COUNT(*) AS MIGRATION_ROW_COUNT FROM (SELECT FIRST 10001 1 AS PROBE_VALUE FROM \"his\".\"drug\") MIGRATION_ROW_PROBE"
        );
    }

    #[test]
    fn source_object_survey_is_strictly_bounded_before_database_access() {
        assert_eq!(SOURCE_OBJECT_SURVEY_MAXIMUM, 5);
        assert_eq!(SOURCE_OBJECT_SURVEY_ROW_LIMIT, 2);
        assert_eq!(SOURCE_OBJECT_SURVEY_TIMEOUT_SECONDS, 8);
        assert!(validate_source_object_survey_names(&[]).is_err());
        assert!(validate_source_object_survey_names(&[
            "A".into(),
            "B".into(),
            "C".into(),
            "D".into(),
            "E".into(),
            "F".into(),
        ])
        .is_err());
        assert!(
            validate_source_object_survey_names(&["YK_TYPK".into(), "YK_TYPK".into()]).is_err()
        );
        assert_eq!(
            validate_source_object_survey_names(&[" YK_TYPK ".into(), "YK_YPCD".into()]).unwrap(),
            vec!["YK_TYPK", "YK_YPCD"]
        );
    }

    #[test]
    fn source_object_metadata_keeps_preview_order_and_describes_empty_objects() {
        let metadata = vec![
            SourceColumnMetadata {
                name: "DRUG_NAME".into(),
                comment: "药品名称".into(),
                source_table: "V_DRUG".into(),
                source_column: "DRUG_NAME".into(),
                mapping_eligible: true,
                source_dictionary: None,
            },
            SourceColumnMetadata {
                name: "SPEC".into(),
                comment: "规格".into(),
                source_table: "V_DRUG".into(),
                source_column: "SPEC".into(),
                mapping_eligible: true,
                source_dictionary: None,
            },
        ];
        let mut empty_preview = SourcePreview {
            columns: Vec::new(),
            column_metadata: Vec::new(),
            rows: Vec::new(),
            truncated: false,
            elapsed_ms: 1,
        };
        merge_source_object_metadata(&mut empty_preview, "V_DRUG", metadata.clone());
        assert_eq!(empty_preview.columns, vec!["DRUG_NAME", "SPEC"]);
        assert_eq!(empty_preview.column_metadata[0].comment, "药品名称");

        let mut ordered_preview = SourcePreview {
            columns: vec!["SPEC".into(), "DRUG_NAME".into(), "EXTRA".into()],
            column_metadata: Vec::new(),
            rows: Vec::new(),
            truncated: false,
            elapsed_ms: 1,
        };
        merge_source_object_metadata(&mut ordered_preview, "V_DRUG", metadata);
        assert_eq!(ordered_preview.column_metadata[0].name, "SPEC");
        assert_eq!(ordered_preview.column_metadata[1].comment, "药品名称");
        assert_eq!(ordered_preview.column_metadata[2].source_table, "V_DRUG");
    }
}
