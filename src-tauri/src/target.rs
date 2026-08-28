use crate::datasource::connect_mysql;
use crate::id::new_object_id;
use crate::local_store::{LocalStore, SourceLinkSnapshot};
use crate::model::{
    BatchDetail, ConnectionProfile, ExecuteBatchRequest, MigrationAudit, MigrationRow,
    OverwriteFieldDiff, OverwritePreview, OverwriteRowPreview, PreviewOverwriteRequest,
    TargetReadiness, TrialMigrationRequest, TrialMigrationResponse, TrialMigrationResult,
    UndoBatchRequest,
};
use crate::normalize::value_text;
use crate::overwrite::{
    column_label, medicine_patch, product_patch, restore_patch, values_equal, ColumnPatch,
};
use crate::target_contract::{validate_execution_context, TARGET_TABLE_PROJECTIONS};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_mysql::{MySql, MySqlPool, MySqlTransaction};
use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};

static ACTIVE_BATCHES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct ActiveBatchGuard {
    batch_id: String,
}

impl ActiveBatchGuard {
    pub(crate) fn enter(batch_id: &str) -> Result<Self, String> {
        let batches = ACTIVE_BATCHES.get_or_init(|| Mutex::new(HashSet::new()));
        let mut active = batches
            .lock()
            .map_err(|_| "迁移执行状态暂时不可用，请重启应用后重试".to_string())?;
        if !active.insert(batch_id.to_string()) {
            return Err("该迁移批次正在执行，请勿重复提交".into());
        }
        Ok(Self {
            batch_id: batch_id.to_string(),
        })
    }
}

impl Drop for ActiveBatchGuard {
    fn drop(&mut self) {
        if let Some(batches) = ACTIVE_BATCHES.get() {
            if let Ok(mut active) = batches.lock() {
                active.remove(&self.batch_id);
            }
        }
    }
}

#[derive(Debug)]
pub(crate) struct WriteEvent {
    pub(crate) operation: &'static str,
    pub(crate) table: &'static str,
    pub(crate) target_id: String,
    pub(crate) message: String,
    pub(crate) before: Value,
    pub(crate) after: Value,
}

pub(crate) struct WriteOutcome {
    pub(crate) status: String,
    pub(crate) id_med: String,
    pub(crate) id_med_unit: String,
    pub(crate) id_fac: String,
    pub(crate) id_med_pro: String,
    pub(crate) events: Vec<WriteEvent>,
}

pub(crate) struct TrialWriteSummary {
    pub(crate) outcome_status: String,
    pub(crate) checked_tables: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UndoTarget {
    pub row_id: String,
    pub table: String,
    pub target_id: String,
    pub priority: u8,
}

#[derive(Debug, Clone)]
pub(crate) struct RestoreTarget {
    pub row_id: String,
    pub table: String,
    pub target_id: String,
    pub before: Value,
    pub priority: u8,
}

pub(crate) struct UndoPlan {
    pub restores: Vec<RestoreTarget>,
    pub inserts: Vec<UndoTarget>,
}

#[derive(Debug)]
pub(crate) struct UndoEvent {
    pub target: UndoTarget,
    pub operation: &'static str,
    pub result: &'static str,
    pub message: String,
}

pub(crate) fn inserted_targets(audits: &[MigrationAudit]) -> Vec<UndoTarget> {
    let mut targets = audits
        .iter()
        .filter(|audit| audit.operation == "INSERT" && audit.result == "SUCCESS")
        .filter_map(|audit| {
            let priority = match audit.target_table.as_str() {
                "hi_bd_med_pro" => 0,
                "hi_bd_med_alias" | "hi_bd_med_unit" => 1,
                "hi_bd_med" => 2,
                "hi_bd_fac" => 3,
                _ => return None,
            };
            (!audit.target_id.trim().is_empty()).then(|| UndoTarget {
                row_id: audit.row_id.clone(),
                table: audit.target_table.clone(),
                target_id: audit.target_id.clone(),
                priority,
            })
        })
        .collect::<Vec<_>>();
    targets.sort_by(|left, right| {
        left.priority
            .cmp(&right.priority)
            .then_with(|| left.table.cmp(&right.table))
            .then_with(|| left.target_id.cmp(&right.target_id))
    });
    targets.dedup_by(|left, right| left.table == right.table && left.target_id == right.target_id);
    targets
}

pub(crate) fn updated_targets(audits: &[MigrationAudit]) -> Vec<RestoreTarget> {
    let mut targets = audits
        .iter()
        .filter(|audit| audit.operation == "UPDATE" && audit.result == "SUCCESS")
        .filter_map(|audit| {
            let priority = match audit.target_table.as_str() {
                "hi_bd_med_pro" => 0,
                "hi_bd_med" => 1,
                _ => return None,
            };
            (!audit.target_id.trim().is_empty() && audit.before_data.is_object()).then(|| {
                RestoreTarget {
                    row_id: audit.row_id.clone(),
                    table: audit.target_table.clone(),
                    target_id: audit.target_id.clone(),
                    before: audit.before_data.clone(),
                    priority,
                }
            })
        })
        .collect::<Vec<_>>();
    targets.sort_by_key(|target| target.priority);
    targets
}

pub(crate) fn target_identity(profile: &ConnectionProfile) -> Value {
    json!({
        "kind": profile.kind.trim().to_ascii_lowercase(),
        "host": profile.host.trim().to_ascii_lowercase(),
        "port": profile.port,
        "database": profile.database.trim().to_ascii_lowercase(),
        "schema": profile.schema.trim().to_ascii_lowercase(),
        "serviceName": profile.service_name.trim().to_ascii_lowercase(),
        "username": profile.username.trim().to_ascii_lowercase()
    })
}

pub(crate) fn has_successful_trial(detail: &BatchDetail, profile: &ConnectionProfile) -> bool {
    let expected_target = target_identity(profile);
    detail.audits.iter().any(|audit| {
        if audit.operation != "TRIAL_ROLLBACK" || audit.result != "SUCCESS" {
            return false;
        }
        let Some(row) = detail.rows.iter().find(|row| row.row_id == audit.row_id) else {
            return false;
        };
        audit.after_data.get("targetIdentity") == Some(&expected_target)
            && audit.after_data.get("sourceHash").and_then(Value::as_str)
                == Some(row.source_hash.as_str())
            && audit.after_data.get("rolledBack").and_then(Value::as_bool) == Some(true)
    })
}

pub(crate) fn validate_undo_request(
    detail: &BatchDetail,
    profile: &ConnectionProfile,
) -> Result<UndoPlan, String> {
    match detail.batch.status.as_str() {
        "SUCCESS" | "PARTIAL" => {}
        "UNDONE" | "UNDO_PARTIAL" => return Err("该批次已经执行过撤销，不能重复操作".into()),
        _ => return Err("只有已完成或部分完成的迁移批次可以撤销".into()),
    }
    let expected = detail
        .audits
        .iter()
        .find(|audit| audit.operation == "EXECUTE")
        .and_then(|audit| audit.after_data.get("targetIdentity"))
        .ok_or_else(|| "该批次缺少目标库身份记录，出于安全考虑不能自动撤销".to_string())?;
    if expected != &target_identity(profile) {
        return Err("当前目标库与批次执行时的目标库不一致，已阻止撤销".into());
    }
    let plan = UndoPlan {
        restores: updated_targets(&detail.audits),
        inserts: inserted_targets(&detail.audits),
    };
    if plan.inserts.is_empty() && plan.restores.is_empty() {
        return Err("该批次没有由本工具新增或覆盖的目标记录，无需撤销".into());
    }
    Ok(plan)
}

pub(crate) fn finish_undo(
    store: &LocalStore,
    batch_id: &str,
    operator_id: &str,
    events: Vec<UndoEvent>,
) -> Result<BatchDetail, String> {
    let retained = events
        .iter()
        .filter(|event| event.operation == "UNDO_RETAIN")
        .count();
    let deleted = events
        .iter()
        .filter(|event| event.operation == "UNDO_DELETE")
        .count();
    let absent = events
        .iter()
        .filter(|event| event.operation == "UNDO_ABSENT")
        .count();
    let restored = events
        .iter()
        .filter(|event| event.operation == "UNDO_RESTORE")
        .count();
    let retained_rows = events
        .iter()
        .filter(|event| event.operation == "UNDO_RETAIN")
        .map(|event| event.target.row_id.clone())
        .collect::<HashSet<_>>();
    let releasable_rows = events
        .iter()
        .map(|event| event.target.row_id.clone())
        .filter(|row_id| !retained_rows.contains(row_id))
        .collect::<HashSet<_>>();
    let trace_id = new_object_id();
    for event in events {
        let before = json!({
            "table": event.target.table,
            "targetId": event.target.target_id
        });
        store.audit_event(
            batch_id,
            &event.target.row_id,
            event.operation,
            &event.target.table,
            &event.target.target_id,
            event.result,
            before,
            Value::Null,
            &event.message,
            operator_id,
            &trace_id,
        )?;
    }
    let status = if retained == 0 {
        "UNDONE"
    } else {
        "UNDO_PARTIAL"
    };
    store.update_batch_status(batch_id, status)?;
    let detail = store.load_batch(batch_id)?;
    for row_id in releasable_rows {
        let source_audit = detail
            .audits
            .iter()
            .find(|audit| audit.operation == "SOURCE_LINK_UPSERT" && audit.row_id == row_id);
        match source_audit.map(|audit| &audit.before_data) {
            Some(before) if before.is_object() => {
                let snapshot: SourceLinkSnapshot = serde_json::from_value(before.clone())
                    .map_err(|error| format!("无法恢复覆盖前来源台账：{error}"))?;
                store.restore_source_link(&snapshot)?;
            }
            _ => store.deactivate_source_link_for_row(batch_id, &row_id)?,
        }
    }
    store.audit_event(
        batch_id,
        "",
        "UNDO",
        "migration_batch",
        batch_id,
        status,
        Value::Null,
        json!({"restored":restored,"deleted":deleted,"retained":retained,"alreadyAbsent":absent}),
        &format!("撤销完成：恢复{restored}条覆盖记录，删除{deleted}条新增记录，因后续引用保留{retained}条，已不存在{absent}条"),
        operator_id,
        &trace_id,
    )?;
    store.load_batch(batch_id)
}

pub(crate) fn audit_undo_start(
    store: &LocalStore,
    batch_id: &str,
    profile: &ConnectionProfile,
    operator_id: &str,
) -> Result<(), String> {
    store.audit_event(
        batch_id,
        "",
        "UNDO_START",
        "migration_batch",
        batch_id,
        "RUNNING",
        Value::Null,
        json!({"targetIdentity":target_identity(profile)}),
        "开始安全撤销本批次新增数据",
        operator_id,
        &new_object_id(),
    )
}

pub(crate) fn audit_undo_failure(
    store: &LocalStore,
    batch_id: &str,
    operator_id: &str,
    error: &str,
) -> Result<(), String> {
    store.audit_event(
        batch_id,
        "",
        "UNDO_FAILED",
        "migration_batch",
        batch_id,
        "FAILED",
        Value::Null,
        json!({"error":limit(error, 2000)}),
        &format!("撤销未完成，本地批次状态保持不变：{}", limit(error, 500)),
        operator_id,
        &new_object_id(),
    )
}

pub async fn inspect_schema(profile: &ConnectionProfile) -> Result<TargetReadiness, String> {
    if crate::pg_protocol::uses_native_connection(profile) {
        return crate::pg_protocol::inspect_target_schema(profile).await;
    }
    if crate::odbc::is_odbc_kind(&profile.kind) {
        return crate::odbc::inspect_target_schema(profile);
    }
    let pool = connect_mysql(profile).await?;
    let result = inspect_mysql_pool(&pool).await;
    pool.close().await;
    result
}

async fn inspect_mysql_pool(pool: &MySqlPool) -> Result<TargetReadiness, String> {
    let mut checked_tables = Vec::new();
    for (table, columns) in TARGET_TABLE_PROJECTIONS {
        let statement = format!("SELECT {columns} FROM {table} WHERE 1=0");
        query::<MySql>(&statement)
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                format!(
                    "目标表结构预检失败（{table}）：{}。请检查数据库、字段版本和查询权限",
                    db_error(error)
                )
            })?;
        checked_tables.push((*table).to_string());
    }
    Ok(TargetReadiness {
        ok: true,
        checked_tables,
        warnings: vec![
            "已验证五张药品表及所需字段可读取；实际 INSERT 权限会在逐行事务写入时验证".into(),
            "直接写表不会触发原 Java 的 medSaveSuccessEvent，ID_SRV 基础服务关联需在上线前单独确认"
                .into(),
        ],
        message: "目标药品表结构与当前迁移版本兼容".into(),
    })
}

pub async fn preview_overwrite(
    store: &LocalStore,
    request: PreviewOverwriteRequest,
    tenant_id: &str,
    operator_id: &str,
) -> Result<OverwritePreview, String> {
    if crate::pg_protocol::uses_native_connection(&request.target) {
        return Err(
            "PostgreSQL 通用协议暂不开放覆盖迁移；请使用增量迁移，或显式配置已验收的厂商 ODBC 回退"
                .into(),
        );
    }
    if crate::odbc::is_odbc_kind(&request.target.kind) {
        return crate::target_odbc::preview_overwrite(store, request, tenant_id, operator_id);
    }
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.conflict_strategy != "OVERWRITE" {
        return Err("当前批次不是覆盖迁移批次".into());
    }
    let pool = connect_mysql(&request.target).await?;
    inspect_mysql_pool(&pool).await?;
    let mut tx = pool.begin().await.map_err(db_error)?;
    let result = async {
        let mut rows = Vec::new();
        for row in detail.rows.iter().filter(|row| row.status == "VALIDATED") {
            if row.id_med.is_empty() {
                rows.push(OverwriteRowPreview {
                    row_id: row.row_id.clone(),
                    row_no: row.row_no,
                    source_key: row.source_key.clone(),
                    action: "INSERT".into(),
                    changes: Vec::new(),
                    message: "新来源记录，将按普通新增迁移执行".into(),
                });
                continue;
            }
            let mut changes = Vec::new();
            let med_patch = medicine_patch(&row.normalized_data);
            let before = read_mysql_patch_snapshot(
                &mut tx,
                "hi_bd_med",
                "id_med",
                &row.id_med,
                tenant_id,
                &med_patch,
                false,
            )
            .await?;
            changes.extend(build_field_diffs("hi_bd_med", &med_patch, &before));
            if !row.id_med_pro.is_empty() {
                let product_patch = product_patch(
                    &row.normalized_data,
                    &row.id_med,
                    &row.id_fac,
                    &row.id_med_unit,
                );
                let before = read_mysql_patch_snapshot(
                    &mut tx,
                    "hi_bd_med_pro",
                    "id_med_pro",
                    &row.id_med_pro,
                    tenant_id,
                    &product_patch,
                    false,
                )
                .await?;
                changes.extend(build_field_diffs("hi_bd_med_pro", &product_patch, &before));
            }
            rows.push(OverwriteRowPreview {
                row_id: row.row_id.clone(),
                row_no: row.row_no,
                source_key: row.source_key.clone(),
                action: if changes.is_empty() {
                    "UNCHANGED".into()
                } else {
                    "UPDATE".into()
                },
                message: if changes.is_empty() {
                    "目标字段与本次迁移值一致，无需覆盖".into()
                } else {
                    format!("检测到 {} 个字段变化", changes.len())
                },
                changes,
            });
        }
        Ok::<_, String>(rows)
    }
    .await;
    let _ = tx.rollback().await;
    pool.close().await;
    let preview = summarize_overwrite_preview(&request.batch_id, result?)?;
    audit_overwrite_preview(store, &preview, &request.target, operator_id)?;
    Ok(preview)
}

pub(crate) fn audit_overwrite_preview(
    store: &LocalStore,
    preview: &OverwritePreview,
    profile: &ConnectionProfile,
    operator_id: &str,
) -> Result<(), String> {
    store.audit_event(
        &preview.batch_id,
        "",
        "PREVIEW_OVERWRITE",
        "migration_batch",
        &preview.batch_id,
        "SUCCESS",
        Value::Null,
        json!({
            "insertCount":preview.insert_count,
            "updateCount":preview.update_count,
            "unchangedCount":preview.unchanged_count,
            "changedFieldCount":preview.changed_field_count,
            "selectableRowIds":preview.rows.iter()
                .filter(|row| row.action != "UNCHANGED")
                .map(|row| row.row_id.clone())
                .collect::<Vec<_>>(),
            "targetIdentity":target_identity(profile)
        }),
        &preview.message,
        operator_id,
        &new_object_id(),
    )
}

pub(crate) fn validate_overwrite_execution_preview(
    detail: &BatchDetail,
    request: &ExecuteBatchRequest,
) -> Result<(), String> {
    if detail.batch.conflict_strategy != "OVERWRITE" {
        return Ok(());
    }
    if !request.overwrite_preview_confirmed || request.selected_row_ids.is_empty() {
        return Err("覆盖迁移必须先生成差异预览并至少勾选一条记录".into());
    }
    let preview = detail
        .audits
        .iter()
        .find(|audit| audit.operation == "PREVIEW_OVERWRITE" && audit.result == "SUCCESS")
        .ok_or_else(|| "没有找到该批次的目标库覆盖差异预览记录".to_string())?;
    if preview.after_data.get("targetIdentity") != Some(&target_identity(&request.target)) {
        return Err("当前目标库与覆盖差异预览时不一致，请重新读取差异".into());
    }
    let selectable = preview
        .after_data
        .get("selectableRowIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect::<HashSet<_>>();
    if request
        .selected_row_ids
        .iter()
        .any(|row_id| !selectable.contains(row_id.as_str()))
    {
        return Err("勾选记录与最近一次覆盖差异预览不一致，请重新读取差异".into());
    }
    Ok(())
}

pub async fn execute_batch(
    store: &LocalStore,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    // RUNNING is persisted for auditability, but it may be left behind after an
    // OS-level driver abort. Duplicate execution is guarded by this process-local
    // lease, so restarting the application safely makes an interrupted batch
    // retryable without manually editing the local history database.
    let _active_batch = ActiveBatchGuard::enter(&request.batch_id)?;
    if crate::pg_protocol::uses_native_connection(&request.target) {
        return crate::target_pg::execute_batch(store, request).await;
    }
    if crate::odbc::is_odbc_kind(&request.target.kind) {
        return crate::target_odbc::execute_batch(store, request);
    }
    let detail = store.load_batch(&request.batch_id)?;
    validate_overwrite_execution_preview(&detail, &request)?;
    let pool = connect_mysql(&request.target).await?;
    inspect_mysql_pool(&pool).await?;
    store.update_batch_counts(
        &request.batch_id,
        "RUNNING",
        detail.batch.valid_count,
        detail.batch.success_count,
        detail.batch.fail_count,
        detail.batch.skip_count,
        false,
    )?;
    let batch_trace = new_object_id();
    store.audit_event(
        &request.batch_id,
        "",
        "EXECUTE",
        "migration_batch",
        &request.batch_id,
        "RUNNING",
        Value::Null,
        json!({
            "failedOnly":request.failed_only,
            "selectedRowCount":request.selected_row_ids.len(),
            "overwritePreviewConfirmed":request.overwrite_preview_confirmed,
            "skipInvalidRows":request.skip_invalid_rows,
            "databaseKind":request.target.kind,
            "targetIdentity":target_identity(&request.target)
        }),
        "开始执行迁移批次",
        &request.operator_id,
        &batch_trace,
    )?;

    let selected_rows = request
        .selected_row_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    for mut row in detail.rows {
        if !selected_rows.is_empty() && !selected_rows.contains(&row.row_id) {
            if detail.batch.conflict_strategy == "OVERWRITE" && row.status == "VALIDATED" {
                row.status = "SKIPPED".into();
                row.error_message = "覆盖差异确认中未勾选（无变化或用户取消）".into();
                row.updated_at = Utc::now().to_rfc3339();
                store.update_row_result(&row)?;
                store.audit_event(
                    &request.batch_id,
                    &row.row_id,
                    "SELECTION_SKIP",
                    "migration_row",
                    &row.row_id,
                    "SKIPPED",
                    Value::Object(row.normalized_data.clone()),
                    Value::Null,
                    &row.error_message,
                    &request.operator_id,
                    &batch_trace,
                )?;
            }
            continue;
        }
        let executable = if request.failed_only {
            row.status == "FAILED"
        } else {
            row.status == "VALIDATED" || row.status == "FAILED"
        };
        if !executable {
            continue;
        }
        if let Some(source_factory_key) = row
            .normalized_data
            .get("_sourceFactoryKey")
            .map(value_text)
            .filter(|value| !value.is_empty())
        {
            if let Some(target_id_fac) = store.find_factory_link(
                &request.tenant_id,
                &detail.batch.source_type,
                &detail.batch.source_name,
                &source_factory_key,
            )? {
                row.normalized_data.insert(
                    "_sourceFactoryTargetId".into(),
                    Value::String(target_id_fac),
                );
            }
        }
        let trace_id = new_object_id();
        match write_row(
            &pool,
            &row,
            &detail.batch.conflict_strategy,
            detail.batch.allow_create_factory,
            &request.tenant_id,
            &request.operator_id,
            &request.organization_id,
            true,
        )
        .await
        {
            Ok(outcome) => {
                row.status = outcome.status;
                row.id_med = outcome.id_med;
                row.id_med_unit = outcome.id_med_unit;
                row.id_fac = outcome.id_fac;
                row.id_med_pro = outcome.id_med_pro;
                row.error_code.clear();
                row.error_message.clear();
                row.updated_at = Utc::now().to_rfc3339();
                store.update_row_result(&row)?;
                if let Some(source_factory_key) = row
                    .normalized_data
                    .get("_sourceFactoryKey")
                    .map(value_text)
                    .filter(|value| !value.is_empty())
                {
                    store.record_factory_link_upsert(
                        &request.tenant_id,
                        &detail.batch.source_type,
                        &detail.batch.source_name,
                        &source_factory_key,
                        &row.id_fac,
                        &text(&row.normalized_data, "naFac"),
                        &request.batch_id,
                        &row.row_id,
                    )?;
                }
                let write_manifest = Value::Array(
                    outcome
                        .events
                        .iter()
                        .map(|event| {
                            json!({
                                "operation": event.operation,
                                "table": event.table,
                                "targetId": event.target_id,
                                "before": event.before,
                                "after": event.after
                            })
                        })
                        .collect(),
                );
                store.record_source_link_upsert(
                    &request.tenant_id,
                    &detail.batch.source_type,
                    &detail.batch.source_name,
                    &row,
                    write_manifest,
                    &request.operator_id,
                    &trace_id,
                )?;
                for event in outcome.events {
                    store.audit_event(
                        &request.batch_id,
                        &row.row_id,
                        event.operation,
                        event.table,
                        &event.target_id,
                        "SUCCESS",
                        event.before,
                        event.after,
                        &event.message,
                        &request.operator_id,
                        &trace_id,
                    )?;
                }
            }
            Err(error) => {
                row.status = "FAILED".into();
                row.error_code = "WRITE_ERROR".into();
                row.error_message = limit(&error, 2000);
                row.retry_count += 1;
                row.updated_at = Utc::now().to_rfc3339();
                store.update_row_result(&row)?;
                store.audit_event(
                    &request.batch_id,
                    &row.row_id,
                    "ROW_FAILED",
                    "migration_row",
                    &row.row_id,
                    "FAILED",
                    Value::Null,
                    Value::Object(row.normalized_data.clone()),
                    &row.error_message,
                    &request.operator_id,
                    &trace_id,
                )?;
            }
        }
    }
    pool.close().await;
    finish_batch(store, &request.batch_id, &request.operator_id)?;
    store.load_batch(&request.batch_id)
}

pub async fn trial_row(
    store: &LocalStore,
    request: TrialMigrationRequest,
) -> Result<TrialMigrationResponse, String> {
    let _active_batch = ActiveBatchGuard::enter(&request.batch_id)?;
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.source_type == "PHIS27_INVENTORY" {
        return Err("机构库存必须按首次盘点整体核对，不支持单条试迁移".into());
    }
    let mut row = detail
        .rows
        .iter()
        .find(|row| row.row_id == request.row_id)
        .cloned()
        .ok_or_else(|| "未找到需要试迁移的明细行".to_string())?;
    if !matches!(row.status.as_str(), "VALIDATED" | "FAILED") {
        return Err("只有校验通过或上次写入失败的明细可以试迁移".into());
    }
    if let Some(source_factory_key) = row
        .normalized_data
        .get("_sourceFactoryKey")
        .map(value_text)
        .filter(|value| !value.is_empty())
    {
        if let Some(target_id_fac) = store.find_factory_link(
            &request.tenant_id,
            &detail.batch.source_type,
            &detail.batch.source_name,
            &source_factory_key,
        )? {
            row.normalized_data.insert(
                "_sourceFactoryTargetId".into(),
                Value::String(target_id_fac),
            );
        }
    }

    let write_result = if crate::pg_protocol::uses_native_connection(&request.target) {
        crate::target_pg::trial_write_row(
            &request,
            &row,
            &detail.batch.conflict_strategy,
            detail.batch.allow_create_factory,
        )
        .await
    } else if crate::odbc::is_odbc_kind(&request.target.kind) {
        crate::target_odbc::trial_write_row(
            &request,
            &row,
            &detail.batch.conflict_strategy,
            detail.batch.allow_create_factory,
        )
    } else {
        trial_mysql_write_row(
            &request,
            &row,
            &detail.batch.conflict_strategy,
            detail.batch.allow_create_factory,
        )
        .await
    };

    let trace_id = new_object_id();
    let identity = target_identity(&request.target);
    let result = match write_result {
        Ok(summary) => {
            let message = if summary.outcome_status == "SKIPPED" {
                "单条试迁移通过：目标记录已存在，查重与关联逻辑正常；目标事务已自动回滚".to_string()
            } else {
                "单条试迁移通过：字段、约束和关联写入均成功；目标事务已自动回滚".to_string()
            };
            store.audit_event(
                &request.batch_id,
                &row.row_id,
                "TRIAL_ROLLBACK",
                "migration_row",
                &row.row_id,
                "SUCCESS",
                Value::Object(row.normalized_data.clone()),
                json!({
                    "targetIdentity": identity,
                    "sourceHash": row.source_hash,
                    "outcomeStatus": summary.outcome_status,
                    "checkedTables": summary.checked_tables,
                    "rolledBack": true
                }),
                &message,
                &request.operator_id,
                &trace_id,
            )?;
            TrialMigrationResult {
                ok: true,
                row_id: row.row_id.clone(),
                row_no: row.row_no,
                source_key: row.source_key.clone(),
                message,
                checked_tables: summary.checked_tables,
            }
        }
        Err(error) => {
            let rollback_confirmed = !error.contains("试迁移回滚失败")
                && !error.contains("回滚单条试迁移事务")
                && !error.contains("试迁移结束后回滚失败");
            let message = if rollback_confirmed {
                format!(
                    "单条试迁移未通过：{}；目标事务未提交，已自动回滚",
                    limit(&error, 1800)
                )
            } else {
                format!(
                    "单条试迁移未通过，且无法确认目标事务已回滚：{}；请停止正式迁移并立即核对目标库",
                    limit(&error, 1750)
                )
            };
            store.audit_event(
                &request.batch_id,
                &row.row_id,
                "TRIAL_ROLLBACK",
                "migration_row",
                &row.row_id,
                "FAILED",
                Value::Object(row.normalized_data.clone()),
                json!({
                    "targetIdentity": identity,
                    "sourceHash": row.source_hash,
                    "checkedTables": [],
                    "rolledBack": rollback_confirmed
                }),
                &message,
                &request.operator_id,
                &trace_id,
            )?;
            TrialMigrationResult {
                ok: false,
                row_id: row.row_id.clone(),
                row_no: row.row_no,
                source_key: row.source_key.clone(),
                message,
                checked_tables: Vec::new(),
            }
        }
    };
    Ok(TrialMigrationResponse {
        result,
        detail: store.load_batch(&request.batch_id)?,
    })
}

async fn trial_mysql_write_row(
    request: &TrialMigrationRequest,
    row: &MigrationRow,
    conflict_strategy: &str,
    allow_create_factory: bool,
) -> Result<TrialWriteSummary, String> {
    let pool = connect_mysql(&request.target).await?;
    let result = async {
        inspect_mysql_pool(&pool).await?;
        write_row(
            &pool,
            row,
            conflict_strategy,
            allow_create_factory,
            &request.tenant_id,
            &request.operator_id,
            &request.organization_id,
            false,
        )
        .await
        .map(trial_summary)
    }
    .await;
    pool.close().await;
    result
}

pub(crate) fn trial_summary(outcome: WriteOutcome) -> TrialWriteSummary {
    let mut checked_tables = outcome
        .events
        .iter()
        .map(|event| event.table.to_string())
        .collect::<Vec<_>>();
    checked_tables.sort();
    checked_tables.dedup();
    TrialWriteSummary {
        outcome_status: outcome.status,
        checked_tables,
    }
}

pub async fn undo_batch(
    store: &LocalStore,
    request: UndoBatchRequest,
    tenant_id: &str,
    operator_id: &str,
) -> Result<BatchDetail, String> {
    if crate::pg_protocol::uses_native_connection(&request.target) {
        return crate::target_pg::undo_batch(store, request, tenant_id, operator_id).await;
    }
    if crate::odbc::is_odbc_kind(&request.target.kind) {
        return crate::target_odbc::undo_batch(store, request, tenant_id, operator_id);
    }
    let detail = store.load_batch(&request.batch_id)?;
    let plan = validate_undo_request(&detail, &request.target)?;
    audit_undo_start(store, &request.batch_id, &request.target, operator_id)?;
    let pool = match connect_mysql(&request.target).await {
        Ok(pool) => pool,
        Err(error) => {
            audit_undo_failure(store, &request.batch_id, operator_id, &error)?;
            return Err(error);
        }
    };
    let result = async {
        inspect_mysql_pool(&pool).await?;
        let mut tx = pool.begin().await.map_err(db_error)?;
        let mut events = Vec::with_capacity(plan.restores.len() + plan.inserts.len());
        for target in plan.restores {
            events.push(restore_mysql_target(&mut tx, target, tenant_id).await?);
        }
        for target in plan.inserts {
            events.push(undo_mysql_target(&mut tx, target, tenant_id).await?);
        }
        tx.commit().await.map_err(db_error)?;
        Ok::<_, String>(events)
    }
    .await;
    pool.close().await;
    match result {
        Ok(events) => finish_undo(store, &request.batch_id, operator_id, events),
        Err(error) => {
            audit_undo_failure(store, &request.batch_id, operator_id, &error)?;
            Err(error)
        }
    }
}

async fn restore_mysql_target(
    tx: &mut MySqlTransaction<'_>,
    target: RestoreTarget,
    tenant_id: &str,
) -> Result<UndoEvent, String> {
    let primary_key = match target.table.as_str() {
        "hi_bd_med" => "id_med",
        "hi_bd_med_pro" => "id_med_pro",
        _ => return Err("覆盖恢复清单包含未授权的目标表".into()),
    };
    let patch = restore_patch(&target.table, &target.before)?;
    apply_mysql_patch(
        tx,
        &target.table,
        primary_key,
        &target.target_id,
        tenant_id,
        &patch,
    )
    .await?;
    Ok(UndoEvent {
        target: UndoTarget {
            row_id: target.row_id,
            table: target.table,
            target_id: target.target_id,
            priority: target.priority,
        },
        operation: "UNDO_RESTORE",
        result: "SUCCESS",
        message: "已按字段级修改前快照恢复覆盖记录".into(),
    })
}

async fn undo_mysql_target(
    tx: &mut MySqlTransaction<'_>,
    target: UndoTarget,
    tenant_id: &str,
) -> Result<UndoEvent, String> {
    let (exists_sql, delete_sql) = match target.table.as_str() {
        "hi_bd_med_pro" => (
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_med_pro=?",
            "DELETE FROM hi_bd_med_pro WHERE id_tet=? AND id_med_pro=?",
        ),
        "hi_bd_med_alias" => (
            "SELECT COUNT(*) FROM hi_bd_med_alias WHERE id_tet=? AND id_med_alias=?",
            "DELETE FROM hi_bd_med_alias WHERE id_tet=? AND id_med_alias=?",
        ),
        "hi_bd_med_unit" => (
            "SELECT COUNT(*) FROM hi_bd_med_unit WHERE id_tet=? AND id_med_unit=?",
            "DELETE FROM hi_bd_med_unit WHERE id_tet=? AND id_med_unit=?",
        ),
        "hi_bd_med" => (
            "SELECT COUNT(*) FROM hi_bd_med WHERE id_tet=? AND id_med=?",
            "DELETE FROM hi_bd_med WHERE id_tet=? AND id_med=?",
        ),
        "hi_bd_fac" => (
            "SELECT COUNT(*) FROM hi_bd_fac WHERE id_tet=? AND id_fac=?",
            "DELETE FROM hi_bd_fac WHERE id_tet=? AND id_fac=?",
        ),
        _ => return Err("撤销清单包含未授权的目标表".into()),
    };
    let exists = mysql_count(tx, exists_sql, tenant_id, &target.target_id).await?;
    if exists == 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_ABSENT",
            result: "SKIPPED",
            message: "该条目标记录已不存在，未重复删除".into(),
        });
    }
    let references = mysql_reference_count(tx, &target, tenant_id).await?;
    if references > 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_RETAIN",
            result: "SKIPPED",
            message: format!("检测到{references}条后续引用，为保护业务数据已保留"),
        });
    }
    query::<MySql>(delete_sql)
        .bind(tenant_id)
        .bind(&target.target_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    Ok(UndoEvent {
        target,
        operation: "UNDO_DELETE",
        result: "SUCCESS",
        message: "已删除本批次新增且未被后续引用的记录".into(),
    })
}

async fn mysql_reference_count(
    tx: &mut MySqlTransaction<'_>,
    target: &UndoTarget,
    tenant_id: &str,
) -> Result<i64, String> {
    let id = &target.target_id;
    match target.table.as_str() {
        "hi_bd_med_unit" => {
            mysql_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_med_unit=?",
                tenant_id,
                id,
            )
            .await
        }
        "hi_bd_med" => {
            let products = mysql_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_med=?",
                tenant_id,
                id,
            )
            .await?;
            let aliases = mysql_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_alias WHERE id_tet=? AND id_med=?",
                tenant_id,
                id,
            )
            .await?;
            let units = mysql_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_unit WHERE id_tet=? AND id_med=?",
                tenant_id,
                id,
            )
            .await?;
            Ok(products + aliases + units)
        }
        "hi_bd_fac" => {
            mysql_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=?",
                tenant_id,
                id,
            )
            .await
        }
        _ => Ok(0),
    }
}

async fn mysql_count(
    tx: &mut MySqlTransaction<'_>,
    sql: &str,
    tenant_id: &str,
    target_id: &str,
) -> Result<i64, String> {
    query_scalar::<MySql, i64>(sql)
        .bind(tenant_id)
        .bind(target_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(db_error)
}

#[allow(clippy::too_many_arguments)]
async fn write_row(
    pool: &MySqlPool,
    row: &MigrationRow,
    conflict_strategy: &str,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
    persist: bool,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let now = Utc::now().naive_utc();
    let mut tx = pool.begin().await.map_err(db_error)?;
    let mut events = Vec::new();

    let name = text(data, "naMed");
    let med_type = text(data, "sdMed");
    let spec = derived_spec(data);
    let unit_pre = text(data, "unitPre");
    let fg_pri = defaulted(data, "fgPri", "0");
    validate_execution_context(tenant_id, operator_id, organization_id, fg_pri == "1")?;
    if conflict_strategy.eq_ignore_ascii_case("OVERWRITE") && !row.id_med.is_empty() {
        let outcome = overwrite_mysql_row(
            &mut tx,
            row,
            allow_create_factory,
            tenant_id,
            operator_id,
            now,
        )
        .await?;
        finish_mysql_row_transaction(tx, persist).await?;
        return Ok(outcome);
    }
    let existing_med = if fg_pri == "1" {
        query_scalar::<MySql, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id).bind(&name).bind(&spec).bind(&unit_pre).bind(organization_id)
        .fetch_optional(&mut *tx).await.map_err(db_error)?
    } else {
        query_scalar::<MySql, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id).bind(&name).bind(&spec).bind(&unit_pre)
        .fetch_optional(&mut *tx).await.map_err(db_error)?
    };
    let id_med = if let Some(id) = existing_med {
        if conflict_strategy.eq_ignore_ascii_case("FAIL") {
            return Err(format!("药品基本信息已存在：{} / {}", name, spec));
        }
        events.push(WriteEvent {
            operation: "REUSE",
            table: "hi_bd_med",
            target_id: id.clone(),
            message: "名称、规格、单位一致，自动合并并复用药品基本信息".into(),
            before: json!({"idMed":id,"businessKey":{"naMed":name,"spec":spec,"unitPre":unit_pre}}),
            after: json!({"idMed":id,"naMed":name,"spec":spec,"unitPre":unit_pre}),
        });
        id
    } else {
        let id = new_object_id();
        let dft_dose_once = if text(data, "dftDoseOnce").is_empty() && med_type == "3" {
            text(data, "dose")
        } else {
            text(data, "dftDoseOnce")
        };
        query::<MySql>(
            r#"INSERT INTO hi_bd_med(
            id_med,na_med,sd_med,id_cstmg,unit_pre,spec,dose,unit_dose,sd_dose,sd_dose_unit,
            sd_chrgitm_lv,sd_allergy,sd_anti_acl,fg_anti_appr,ddd,sd_bas_med,sd_spe_med,
            limit_anti_day,sd_storage,sd_pharm,sd_value,sd_prod_plac,fg_pois,sd_pois,fg_anti,
            sd_anti,sd_round,sd_dps,fg_med_rx,fg_bas_med,fg_skintest,sd_skintest,drip_rate,
            dft_dose_once,dft_usage,dft_freq,fg_tcd,fg_single,fg_register,fg_active,fg_pri,
            id_org_pri,id_tet,revision,insert_user,insert_time
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
        )
        .bind(&id)
        .bind(&name)
        .bind(&med_type)
        .bind(text(data, "idCstmg"))
        .bind(&unit_pre)
        .bind(&spec)
        .bind(text(data, "dose"))
        .bind(text(data, "unitDose"))
        .bind(text(data, "sdDose"))
        .bind(text(data, "sdDoseUnit"))
        .bind(text(data, "sdChrgitmLv"))
        .bind(text(data, "sdAllergy"))
        .bind(text(data, "sdAntiAcl"))
        .bind(defaulted(data, "fgAntiAppr", "0"))
        .bind(text(data, "ddd"))
        .bind(text(data, "sdBasMed"))
        .bind(text(data, "sdSpeMed"))
        .bind(optional_decimal(data, "limitAntiDay")?)
        .bind(text(data, "sdStorage"))
        .bind(text(data, "sdPharm"))
        .bind(text(data, "sdValue"))
        .bind(text(data, "sdProdPlac"))
        .bind(defaulted(data, "fgPois", "0"))
        .bind(text(data, "sdPois"))
        .bind(defaulted(data, "fgAnti", "0"))
        .bind(text(data, "sdAnti"))
        .bind(text(data, "sdRound"))
        .bind(text(data, "sdDps"))
        .bind(defaulted(data, "fgMedRx", "2"))
        .bind(defaulted(data, "fgBasMed", "0"))
        .bind(defaulted(data, "fgSkintest", "0"))
        .bind(text(data, "sdSkintest"))
        .bind(text(data, "dripRate"))
        .bind(dft_dose_once)
        .bind(text(data, "dftUsage"))
        .bind(text(data, "dftFreq"))
        .bind(defaulted(data, "fgTcd", "0"))
        .bind(defaulted(data, "fgSingle", "1"))
        .bind(defaulted(data, "fgRegister", "0"))
        .bind("1")
        .bind(&fg_pri)
        .bind((fg_pri == "1").then_some(organization_id))
        .bind(tenant_id)
        .bind("0")
        .bind(operator_id)
        .bind(now)
        .execute(&mut *tx)
        .await
        .map_err(db_error)?;
        events.push(WriteEvent {
            operation: "INSERT",
            table: "hi_bd_med",
            target_id: id.clone(),
            message: "新增药品基本信息".into(),
            before: Value::Null,
            after: json!({"idMed":id,"naMed":name,"spec":spec}),
        });
        id
    };

    ensure_alias(&mut tx, &id_med, data, tenant_id, &mut events).await?;
    let id_med_unit = ensure_unit(&mut tx, &id_med, data, tenant_id, &mut events).await?;
    if !crate::normalize::has_product_data(data) {
        finish_mysql_row_transaction(tx, persist).await?;
        return Ok(WriteOutcome {
            status: "SUCCESS".into(),
            id_med,
            id_med_unit,
            id_fac: String::new(),
            id_med_pro: String::new(),
            events,
        });
    }
    let id_fac = ensure_factory(
        &mut tx,
        data,
        allow_create_factory,
        tenant_id,
        operator_id,
        now,
        &mut events,
    )
    .await?;
    let product_name = defaulted(data, "naMedPro", &name);
    let sale_spec = derived_sale_spec(data, &spec);
    let external_code = text(data, "cdMedPro");
    let by_external_code = if external_code.is_empty() {
        None
    } else if fg_pri == "1" {
        query_scalar::<MySql, String>(
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1",
        ).bind(tenant_id).bind(&external_code).bind(organization_id)
            .fetch_optional(&mut *tx).await.map_err(db_error)?
    } else {
        query_scalar::<MySql, String>(
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1",
        ).bind(tenant_id).bind(&external_code)
            .fetch_optional(&mut *tx).await.map_err(db_error)?
    };
    let existing_product = if let Some(id) = by_external_code {
        let linked_med = query_scalar::<MySql, String>(
            "SELECT id_med FROM hi_bd_med_pro WHERE id_med_pro=? LIMIT 1",
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        let linked_fac = query_scalar::<MySql, String>(
            "SELECT id_fac FROM hi_bd_med_pro WHERE id_med_pro=? LIMIT 1",
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(db_error)?;
        if linked_med != id_med || linked_fac != id_fac {
            return Err(format!(
                "三方货品码“{external_code}”已被目标商品{id}使用，但关联药品或厂家不同；请修正货品码对照"
            ));
        }
        Some((id, "三方货品码"))
    } else {
        let by_business_key = if fg_pri == "1" {
            query_scalar::<MySql, String>(
                "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1",
            ).bind(tenant_id).bind(&id_fac).bind(&product_name).bind(&sale_spec).bind(organization_id)
                .fetch_optional(&mut *tx).await.map_err(db_error)?
        } else {
            query_scalar::<MySql, String>(
                "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1",
            ).bind(tenant_id).bind(&id_fac).bind(&product_name).bind(&sale_spec)
                .fetch_optional(&mut *tx).await.map_err(db_error)?
        };
        by_business_key.map(|id| (id, "厂家+商品名+销售规格"))
    };
    if let Some((id_med_pro, matched_by)) = existing_product {
        if conflict_strategy.eq_ignore_ascii_case("FAIL") {
            return Err("同厂家、商品名和销售规格的药品商品已存在".into());
        }
        finish_mysql_row_transaction(tx, persist).await?;
        events.push(WriteEvent {
            operation: "SKIP",
            table: "hi_bd_med_pro",
            target_id: id_med_pro.clone(),
            message: "目标商品已存在，本行幂等跳过".into(),
            before: json!({"idMedPro":id_med_pro,"matchedBy":matched_by,"businessKey":{"cdMedPro":external_code,"idFac":id_fac,"naMedPro":product_name,"specSale":sale_spec}}),
            after: json!({"idMedPro":id_med_pro}),
        });
        return Ok(WriteOutcome {
            status: "SKIPPED".into(),
            id_med,
            id_med_unit,
            id_fac,
            id_med_pro,
            events,
        });
    }

    let id_med_pro = new_object_id();
    query::<MySql>(
        r#"INSERT INTO hi_bd_med_pro(
        id_med_pro,id_med,id_fac,id_med_unit,unit_sale,na_med_pro,spec_sale,price_sale,price_pur,
        unit_sale_factor,cd_appr,cd_bar,cd_med_pro,id_tet,revision,insert_user,insert_time,
        fg_active,fg_pri,id_org_pri,sd_per,per,fg_coll_pur,fg_import
    ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
    )
    .bind(&id_med_pro)
    .bind(&id_med)
    .bind(&id_fac)
    .bind(&id_med_unit)
    .bind(text(data, "unitSale"))
    .bind(&product_name)
    .bind(&sale_spec)
    .bind(decimal(data, "priceSale")?)
    .bind(decimal(data, "pricePur")?)
    .bind(integer(data, "unitSaleFactor")?)
    .bind(text(data, "cdAppr"))
    .bind(text(data, "cdBar"))
    .bind(text(data, "cdMedPro"))
    .bind(tenant_id)
    .bind("0")
    .bind(operator_id)
    .bind(now)
    .bind("1")
    .bind(&fg_pri)
    .bind((fg_pri == "1").then_some(organization_id))
    .bind(text(data, "sdPer"))
    .bind(optional_decimal(data, "per")?)
    .bind(defaulted(data, "fgCollPur", "0"))
    .bind(defaulted(data, "fgImport", "1"))
    .execute(&mut *tx)
    .await
    .map_err(db_error)?;
    finish_mysql_row_transaction(tx, persist).await?;
    events.push(WriteEvent {
        operation: "INSERT",
        table: "hi_bd_med_pro",
        target_id: id_med_pro.clone(),
        message: "新增药品商品信息".into(),
        before: Value::Null,
        after: json!({"idMedPro":id_med_pro,"idMed":id_med,"idFac":id_fac}),
    });
    Ok(WriteOutcome {
        status: "SUCCESS".into(),
        id_med,
        id_med_unit,
        id_fac,
        id_med_pro,
        events,
    })
}

async fn finish_mysql_row_transaction(
    tx: MySqlTransaction<'_>,
    persist: bool,
) -> Result<(), String> {
    if persist {
        tx.commit().await.map_err(db_error)
    } else {
        tx.rollback()
            .await
            .map_err(|error| format!("单条试迁移回滚失败：{}", db_error(error)))
    }
}

async fn overwrite_mysql_row(
    tx: &mut MySqlTransaction<'_>,
    row: &MigrationRow,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
    now: chrono::NaiveDateTime,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let id_med = row.id_med.clone();
    let name = text(data, "naMed");
    let spec = derived_spec(data);
    let duplicate_med = query_scalar::<MySql, String>(
        "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND id_med<>? AND fg_active='1' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(&name)
    .bind(&spec)
    .bind(text(data, "unitPre"))
    .bind(&id_med)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    if let Some(duplicate) = duplicate_med {
        return Err(format!(
            "覆盖后的药品名称、规格、单位与目标药品{duplicate}重复，请先处理重复数据"
        ));
    }
    let mut events = Vec::new();
    let med_patch = medicine_patch(data);
    let (before, after) =
        apply_mysql_patch(tx, "hi_bd_med", "id_med", &id_med, tenant_id, &med_patch).await?;
    events.push(WriteEvent {
        operation: "UPDATE",
        table: "hi_bd_med",
        target_id: id_med.clone(),
        message: "覆盖药品基本信息，已保存字段级修改前快照".into(),
        before,
        after,
    });
    ensure_alias(tx, &id_med, data, tenant_id, &mut events).await?;
    let id_med_unit = ensure_unit(tx, &id_med, data, tenant_id, &mut events).await?;
    if row.id_med_pro.is_empty() {
        return Ok(WriteOutcome {
            status: "SUCCESS".into(),
            id_med,
            id_med_unit,
            id_fac: String::new(),
            id_med_pro: String::new(),
            events,
        });
    }
    let id_fac = ensure_factory(
        tx,
        data,
        allow_create_factory,
        tenant_id,
        operator_id,
        now,
        &mut events,
    )
    .await?;
    ensure_no_mysql_product_conflict(tx, row, &id_fac, &name, &spec, tenant_id).await?;
    let product_patch = product_patch(data, &id_med, &id_fac, &id_med_unit);
    let (before, after) = apply_mysql_patch(
        tx,
        "hi_bd_med_pro",
        "id_med_pro",
        &row.id_med_pro,
        tenant_id,
        &product_patch,
    )
    .await?;
    events.push(WriteEvent {
        operation: "UPDATE",
        table: "hi_bd_med_pro",
        target_id: row.id_med_pro.clone(),
        message: "覆盖药品商品信息，已保存字段级修改前快照".into(),
        before,
        after,
    });
    Ok(WriteOutcome {
        status: "SUCCESS".into(),
        id_med,
        id_med_unit,
        id_fac,
        id_med_pro: row.id_med_pro.clone(),
        events,
    })
}

async fn ensure_no_mysql_product_conflict(
    tx: &mut MySqlTransaction<'_>,
    row: &MigrationRow,
    id_fac: &str,
    base_name: &str,
    base_spec: &str,
    tenant_id: &str,
) -> Result<(), String> {
    let data = &row.normalized_data;
    let external_code = text(data, "cdMedPro");
    if !external_code.is_empty() {
        let duplicate = query_scalar::<MySql, String>(
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND id_med_pro<>? AND fg_active='1' LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&external_code)
        .bind(&row.id_med_pro)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
        if let Some(id) = duplicate {
            return Err(format!("覆盖后的三方货品码已被目标商品{id}使用"));
        }
    }
    let product_name = defaulted(data, "naMedPro", base_name);
    let sale_spec = derived_sale_spec(data, base_spec);
    let duplicate = query_scalar::<MySql, String>(
        "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND id_med_pro<>? AND fg_active='1' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(id_fac)
    .bind(&product_name)
    .bind(&sale_spec)
    .bind(&row.id_med_pro)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    if let Some(id) = duplicate {
        return Err(format!("覆盖后的厂家、商品名和销售规格与目标商品{id}冲突"));
    }
    Ok(())
}

async fn apply_mysql_patch(
    tx: &mut MySqlTransaction<'_>,
    table: &str,
    primary_key: &str,
    target_id: &str,
    tenant_id: &str,
    patch: &[ColumnPatch],
) -> Result<(Value, Value), String> {
    let existing =
        read_mysql_patch_snapshot(tx, table, primary_key, target_id, tenant_id, patch, true)
            .await?;
    let mut before = Map::new();
    let mut after = Map::new();
    for (item, value) in patch.iter().zip(existing) {
        before.insert(
            item.column.into(),
            value.map(Value::String).unwrap_or(Value::Null),
        );
        after.insert(
            item.column.into(),
            item.value.clone().map(Value::String).unwrap_or(Value::Null),
        );
    }
    let assignments = patch
        .iter()
        .map(|item| format!("{}=?", item.column))
        .collect::<Vec<_>>()
        .join(",");
    let update_sql = format!("UPDATE {table} SET {assignments} WHERE id_tet=? AND {primary_key}=?");
    let mut statement = query::<MySql>(&update_sql);
    for item in patch {
        statement = statement.bind(item.value.clone());
    }
    statement
        .bind(tenant_id)
        .bind(target_id)
        .execute(&mut **tx)
        .await
        .map_err(db_error)?;
    Ok((Value::Object(before), Value::Object(after)))
}

#[allow(clippy::too_many_arguments)]
async fn read_mysql_patch_snapshot(
    tx: &mut MySqlTransaction<'_>,
    table: &str,
    primary_key: &str,
    target_id: &str,
    tenant_id: &str,
    patch: &[ColumnPatch],
    for_update: bool,
) -> Result<Vec<Option<String>>, String> {
    if patch.is_empty() {
        return Err("覆盖字段清单为空".into());
    }
    let select_columns = patch
        .iter()
        .map(|item| format!("CAST({} AS CHAR)", item.column))
        .collect::<Vec<_>>()
        .join(",");
    let lock = if for_update { " FOR UPDATE" } else { "" };
    let select_sql =
        format!("SELECT {select_columns} FROM {table} WHERE id_tet=? AND {primary_key}=?{lock}");
    let existing = query::<MySql>(&select_sql)
        .bind(tenant_id)
        .bind(target_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?
        .ok_or_else(|| format!("覆盖目标不存在或不属于当前租户：{table}/{target_id}"))?;
    (0..patch.len())
        .map(|index| {
            existing
                .try_get::<Option<String>, usize>(index)
                .map_err(db_error)
        })
        .collect()
}

pub(crate) fn build_field_diffs(
    table: &str,
    patch: &[ColumnPatch],
    before: &[Option<String>],
) -> Vec<OverwriteFieldDiff> {
    patch
        .iter()
        .zip(before)
        .filter(|(item, old)| !values_equal(item.column, old, &item.value))
        .map(|(item, old)| OverwriteFieldDiff {
            table: table.into(),
            column: item.column.into(),
            label: column_label(table, item.column),
            before: old.clone().map(Value::String).unwrap_or(Value::Null),
            after: item.value.clone().map(Value::String).unwrap_or(Value::Null),
        })
        .collect()
}

pub(crate) fn summarize_overwrite_preview(
    batch_id: &str,
    rows: Vec<OverwriteRowPreview>,
) -> Result<OverwritePreview, String> {
    if rows.is_empty() {
        return Err("覆盖批次没有待执行的有效记录".into());
    }
    let insert_count = rows.iter().filter(|row| row.action == "INSERT").count();
    let update_count = rows.iter().filter(|row| row.action == "UPDATE").count();
    let unchanged_count = rows.iter().filter(|row| row.action == "UNCHANGED").count();
    let changed_field_count = rows.iter().map(|row| row.changes.len()).sum();
    Ok(OverwritePreview {
        batch_id: batch_id.into(),
        rows,
        insert_count,
        update_count,
        unchanged_count,
        changed_field_count,
        message: format!(
            "差异读取完成：新增{insert_count}条，覆盖{update_count}条，无变化{unchanged_count}条，共{changed_field_count}个字段变化"
        ),
    })
}

async fn ensure_alias(
    tx: &mut MySqlTransaction<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<(), String> {
    let name = text(data, "naMed");
    let exists: Option<String> = query_scalar::<MySql, String>(
        "SELECT id_med_alias FROM hi_bd_med_alias WHERE id_tet=? AND id_med=? AND na_alias=? AND fg_main='1' AND fg_active='1' LIMIT 1"
    ).bind(tenant_id).bind(id_med).bind(&name).fetch_optional(&mut **tx).await.map_err(db_error)?;
    if let Some(id) = exists {
        events.push(WriteEvent {
            operation: "REUSE",
            table: "hi_bd_med_alias",
            target_id: id.clone(),
            message: "复用已存在的药品主别名".into(),
            before: json!({"idMedAlias":id,"idMed":id_med,"naAlias":name}),
            after: json!({"idMedAlias":id}),
        });
    } else {
        let id = new_object_id();
        let (py, wb, instr) = alias_search_fields(data, &name);
        query::<MySql>("INSERT INTO hi_bd_med_alias(id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active) VALUES (?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(id_med).bind(&name).bind("1").bind(&py).bind(&wb)
            .bind(&instr).bind(tenant_id).bind("1")
            .execute(&mut **tx).await.map_err(db_error)?;
        events.push(WriteEvent {
            operation: "INSERT",
            table: "hi_bd_med_alias",
            target_id: id,
            message: alias_event_message(&py, &wb),
            before: Value::Null,
            after: json!({"idMed":id_med,"naAlias":name}),
        });
    }
    Ok(())
}

async fn ensure_unit(
    tx: &mut MySqlTransaction<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let mut primary_id = String::new();
    for (index, (unit, factor)) in medicine_unit_specs(data)?.into_iter().enumerate() {
        let factor = factor.to_string();
        let exists: Option<String> = query_scalar::<MySql, String>(
            "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=? AND id_med=? AND na_unit=? AND unit_factor=? LIMIT 1"
        ).bind(tenant_id).bind(id_med).bind(&unit).bind(&factor).fetch_optional(&mut **tx).await.map_err(db_error)?;
        let id = if let Some(id) = exists {
            events.push(WriteEvent {
                operation: "REUSE",
                table: "hi_bd_med_unit",
                target_id: id.clone(),
                message: if index == 0 {
                    "复用已存在的药品最小单位"
                } else {
                    "复用已存在的药品包装单位"
                }
                .into(),
                before: json!({"idMedUnit":id,"idMed":id_med,"naUnit":unit,"unitFactor":factor}),
                after: json!({"idMedUnit":id}),
            });
            id
        } else {
            let id = new_object_id();
            query::<MySql>("INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES (?,?,?,?,?)")
                .bind(&id).bind(id_med).bind(&unit).bind(&factor).bind(tenant_id)
                .execute(&mut **tx).await.map_err(db_error)?;
            events.push(WriteEvent {
                operation: "INSERT",
                table: "hi_bd_med_unit",
                target_id: id.clone(),
                message: if index == 0 {
                    "新增药品最小单位"
                } else {
                    "新增药品包装单位"
                }
                .into(),
                before: Value::Null,
                after: json!({"idMed":id_med,"naUnit":unit,"unitFactor":factor}),
            });
            id
        };
        primary_id = id;
    }
    Ok(primary_id)
}

async fn ensure_factory(
    tx: &mut MySqlTransaction<'_>,
    data: &Map<String, Value>,
    allow_create: bool,
    tenant_id: &str,
    operator_id: &str,
    now: chrono::NaiveDateTime,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let requested_id = text(data, "idFac");
    if !requested_id.is_empty() {
        let exists: Option<String> = query_scalar::<MySql, String>(
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=? AND id_tet=? AND fg_active='1' LIMIT 1",
        )
        .bind(&requested_id)
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
        let id =
            exists.ok_or_else(|| format!("生产厂家主键无效或不属于当前租户：{}", requested_id))?;
        events.push(WriteEvent {
            operation: "REUSE",
            table: "hi_bd_fac",
            target_id: id.clone(),
            message: "按已建立的厂家主键对照复用生产厂家".into(),
            before: json!({"idFac":id}),
            after: json!({"idFac":id}),
        });
        return Ok(id);
    }
    let source_factory_key = text(data, "_sourceFactoryKey");
    let mapped_id = text(data, "_sourceFactoryTargetId");
    if !mapped_id.is_empty() {
        let exists: Option<String> = query_scalar::<MySql, String>(
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=? AND id_tet=? AND fg_active='1' LIMIT 1",
        )
        .bind(&mapped_id)
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(db_error)?;
        if let Some(id) = exists {
            events.push(WriteEvent {
                operation: "REUSE",
                table: "hi_bd_fac",
                target_id: id.clone(),
                message: format!("按二系列phis厂家主键 YPCD={source_factory_key} 复用生产厂家"),
                before: json!({"idFac":id,"sourceFactoryKey":source_factory_key}),
                after: json!({"idFac":id}),
            });
            return Ok(id);
        }
    }
    let name = text(data, "naFac");
    if name.is_empty() {
        return Err(format!(
            "二系列phis厂家 YPCD={} 未在 YK_CDDZ 中找到有效名称，无法迁移厂家基础数据",
            if source_factory_key.is_empty() {
                "未知"
            } else {
                &source_factory_key
            }
        ));
    }
    let existing: Option<String> = query_scalar::<MySql, String>(
        "SELECT id_fac FROM hi_bd_fac WHERE id_tet=? AND na_fac=? AND fg_active='1' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(&name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(db_error)?;
    if let Some(id) = existing {
        events.push(WriteEvent {
            operation: "REUSE",
            table: "hi_bd_fac",
            target_id: id.clone(),
            message: "按厂家名称精确匹配并复用生产厂家".into(),
            before: json!({"idFac":id,"naFac":name}),
            after: json!({"idFac":id}),
        });
        return Ok(id);
    }
    if !allow_create {
        return Err(format!(
            "生产厂家“{}”未匹配；请先建立对照或开启允许创建厂家",
            name
        ));
    }
    let id = new_object_id();
    let short_name = defaulted(data, "naFacShort", &limit(&name, 32));
    let pinyin = text(data, "pyFac");
    query::<MySql>(
        r#"INSERT INTO hi_bd_fac(id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,
        id_tet,revision,insert_user,insert_time) VALUES (?,?,?,?,?,?,?,?,?,?,?,?)"#,
    )
    .bind(&id)
    .bind(&name)
    .bind(limit(&short_name, 32))
    .bind(defaulted(data, "sdProdPlac", "1"))
    .bind(&pinyin)
    .bind("")
    .bind(&name)
    .bind("1")
    .bind(tenant_id)
    .bind("0")
    .bind(operator_id)
    .bind(now)
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    events.push(WriteEvent {
        operation: "INSERT",
        table: "hi_bd_fac",
        target_id: id.clone(),
        message: if source_factory_key.is_empty() {
            "按迁移策略新增生产厂家".into()
        } else {
            format!("迁移二系列phis厂家基础数据（YPCD={source_factory_key}）")
        },
        before: Value::Null,
        after: json!({"idFac":id,"naFac":name,"naFacShort":short_name,"py":pinyin,"sourceFactoryKey":source_factory_key}),
    });
    Ok(id)
}

pub(crate) fn finish_batch(
    store: &LocalStore,
    batch_id: &str,
    operator_id: &str,
) -> Result<(), String> {
    let detail = store.load_batch(batch_id)?;
    let success = detail
        .rows
        .iter()
        .filter(|row| row.status == "SUCCESS")
        .count();
    let skipped = detail
        .rows
        .iter()
        .filter(|row| row.status == "SKIPPED")
        .count();
    let failed = detail
        .rows
        .iter()
        .filter(|row| row.status == "FAILED" || row.status == "INVALID")
        .count();
    let valid = detail
        .rows
        .iter()
        .filter(|row| row.status == "VALIDATED")
        .count();
    let status = if failed == 0 && valid == 0 {
        "SUCCESS"
    } else if success + skipped > 0 {
        "PARTIAL"
    } else {
        "FAILED"
    };
    store.update_batch_counts(batch_id, status, valid, success, failed, skipped, true)?;
    let trace_id = new_object_id();
    store.audit_event(
        batch_id,
        "",
        "FINISH",
        "migration_batch",
        batch_id,
        status,
        Value::Null,
        json!({"success":success,"skipped":skipped,"failed":failed,"pending":valid}),
        &format!(
            "执行完成：成功{}行，跳过{}行，失败{}行",
            success, skipped, failed
        ),
        operator_id,
        &trace_id,
    )
}

pub(crate) fn text(data: &Map<String, Value>, key: &str) -> String {
    data.get(key).map(value_text).unwrap_or_default()
}
pub(crate) fn defaulted(data: &Map<String, Value>, key: &str, default: &str) -> String {
    let value = text(data, key);
    if value.is_empty() {
        default.into()
    } else {
        value
    }
}
pub(crate) fn integer(data: &Map<String, Value>, key: &str) -> Result<i64, String> {
    text(data, key)
        .parse::<i64>()
        .map_err(|_| format!("{}必须是整数", key))
}
pub(crate) fn medicine_unit_specs(data: &Map<String, Value>) -> Result<Vec<(String, i64)>, String> {
    let minimum_unit = text(data, "unitPre");
    if minimum_unit.is_empty() {
        return Err("unitPre不能为空".into());
    }
    let sale_unit = defaulted(data, "unitSale", &minimum_unit);
    let sale_factor = if text(data, "unitSaleFactor").is_empty() {
        1
    } else {
        integer(data, "unitSaleFactor")?
    };
    let mut specs = vec![(minimum_unit, 1)];
    let sale_spec = (sale_unit, sale_factor);
    if specs[0] != sale_spec {
        specs.push(sale_spec);
    }
    Ok(specs)
}
pub(crate) fn decimal(data: &Map<String, Value>, key: &str) -> Result<Decimal, String> {
    text(data, key)
        .parse::<Decimal>()
        .map_err(|_| format!("{}必须是数字", key))
}
pub(crate) fn optional_decimal(
    data: &Map<String, Value>,
    key: &str,
) -> Result<Option<Decimal>, String> {
    let value = text(data, key);
    if value.is_empty() {
        Ok(None)
    } else {
        value
            .parse::<Decimal>()
            .map(Some)
            .map_err(|_| format!("{}必须是数字", key))
    }
}
pub(crate) fn derived_spec(data: &Map<String, Value>) -> String {
    let spec = text(data, "spec");
    if !spec.is_empty() {
        spec
    } else {
        format!(
            "{}{}/{}",
            text(data, "dose"),
            text(data, "unitDose"),
            text(data, "unitPre")
        )
    }
}
pub(crate) fn derived_sale_spec(data: &Map<String, Value>, base_spec: &str) -> String {
    let spec = text(data, "specSale");
    if !spec.is_empty() {
        return spec;
    }
    let factor = integer(data, "unitSaleFactor").unwrap_or(1);
    if factor <= 1 {
        base_spec.into()
    } else {
        format!(
            "{}{}*{}{}/{}",
            text(data, "dose"),
            text(data, "unitDose"),
            factor,
            text(data, "unitPre"),
            text(data, "unitSale")
        )
    }
}
pub(crate) fn alias_search_fields(
    data: &Map<String, Value>,
    name: &str,
) -> (String, String, String) {
    let py = limit(&text(data, "_aliasPy").to_lowercase(), 32);
    let wb = limit(&text(data, "_aliasWb").to_lowercase(), 32);
    let instr = limit(&format!("{name},{py},{wb}"), 255);
    (py, wb, instr)
}
pub(crate) fn alias_event_message(py: &str, wb: &str) -> String {
    if py.is_empty() && wb.is_empty() {
        "新增药品主别名；来源未提供拼音/五笔码，保留为空".into()
    } else {
        "新增药品主别名；已沿用来源拼音/五笔检索码".into()
    }
}
fn db_error(error: sqlx_core::Error) -> String {
    format!("目标数据库写入失败：{}", error)
}
pub(crate) fn limit(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        alias_search_fields, has_successful_trial, inserted_targets, medicine_unit_specs,
        target_identity, updated_targets, ActiveBatchGuard,
    };
    use crate::model::{
        BatchDetail, ConnectionProfile, MigrationAudit, MigrationBatch, MigrationRow,
    };
    use serde_json::{Map, Value};

    fn audit(operation: &str, table: &str, target_id: &str) -> MigrationAudit {
        MigrationAudit {
            audit_id: format!("audit-{table}-{target_id}"),
            batch_id: "batch".into(),
            row_id: "row-1".into(),
            trace_id: "trace".into(),
            operation: operation.into(),
            target_table: table.into(),
            target_id: target_id.into(),
            result: "SUCCESS".into(),
            before_data: Value::Null,
            after_data: Value::Null,
            message: String::new(),
            operator_id: "operator".into(),
            operated_at: "2026-08-04T00:00:00Z".into(),
        }
    }

    #[test]
    fn undo_manifest_contains_only_inserted_whitelisted_rows_in_dependency_order() {
        let audits = vec![
            audit("INSERT", "hi_bd_med", "med"),
            audit("INSERT", "hi_bd_med_alias", "alias"),
            audit("INSERT", "hi_bd_med_unit", "minimum-unit"),
            audit("INSERT", "hi_bd_med_unit", "sale-unit"),
            audit("INSERT", "hi_bd_med_pro", "product"),
            audit("REUSE", "hi_bd_fac", "reused-factory"),
            audit("INSERT", "unrelated_table", "unsafe"),
            audit("INSERT", "hi_bd_med_pro", "product"),
        ];
        let targets = inserted_targets(&audits);
        assert_eq!(targets.len(), 5);
        assert_eq!(targets[0].table, "hi_bd_med_pro");
        assert_eq!(targets[1].table, "hi_bd_med_alias");
        assert_eq!(targets[2].target_id, "minimum-unit");
        assert_eq!(targets[3].target_id, "sale-unit");
        assert_eq!(targets[4].table, "hi_bd_med");
        assert!(targets.iter().all(|target| target.target_id != "unsafe"));
        assert!(targets
            .iter()
            .all(|target| target.target_id != "reused-factory"));
    }

    #[test]
    fn overwrite_restore_runs_product_before_base_medicine() {
        let mut med = audit("UPDATE", "hi_bd_med", "med");
        med.before_data = serde_json::json!({"na_med":"旧药品"});
        let mut product = audit("UPDATE", "hi_bd_med_pro", "product");
        product.before_data = serde_json::json!({"na_med_pro":"旧商品"});
        let targets = updated_targets(&[med, product]);
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].table, "hi_bd_med_pro");
        assert_eq!(targets[1].table, "hi_bd_med");
    }

    #[test]
    fn active_batch_guard_blocks_only_concurrent_execution() {
        let first = ActiveBatchGuard::enter("guard-batch").unwrap();
        assert!(ActiveBatchGuard::enter("guard-batch")
            .unwrap_err()
            .contains("正在执行"));
        drop(first);
        assert!(ActiveBatchGuard::enter("guard-batch").is_ok());
    }

    #[test]
    fn alias_search_fields_use_legacy_codes_without_exceeding_target_lengths() {
        let data = serde_json::json!({
            "_aliasPy":"AMXLJN",
            "_aliasWb":"BSOEXA"
        })
        .as_object()
        .unwrap()
        .clone();
        let (py, wb, instr) = alias_search_fields(&data, "阿莫西林胶囊");
        assert_eq!(py, "amxljn");
        assert_eq!(wb, "bsoexa");
        assert_eq!(instr, "阿莫西林胶囊,amxljn,bsoexa");
    }

    #[test]
    fn medicine_units_always_start_with_the_phis_minimum_unit() {
        let packaged = serde_json::json!({
            "unitPre":"支",
            "unitSale":"盒",
            "unitSaleFactor":"10"
        })
        .as_object()
        .unwrap()
        .clone();
        assert_eq!(
            medicine_unit_specs(&packaged).unwrap(),
            vec![("支".into(), 1), ("盒".into(), 10)]
        );

        let minimum_only = serde_json::json!({
            "unitPre":"支",
            "unitSale":"支",
            "unitSaleFactor":"1"
        })
        .as_object()
        .unwrap()
        .clone();
        assert_eq!(
            medicine_unit_specs(&minimum_only).unwrap(),
            vec![("支".into(), 1)]
        );
    }

    #[test]
    fn formal_execution_trial_must_match_current_target_and_source_hash() {
        let profile = ConnectionProfile {
            kind: "oracle".into(),
            host: "db.example.com".into(),
            port: 1521,
            database: String::new(),
            username: "phis".into(),
            password: String::new(),
            schema: "phis".into(),
            service_name: "orcl".into(),
            driver: String::new(),
            connection_string: String::new(),
        };
        let mut trial = audit("TRIAL_ROLLBACK", "migration_row", "row-1");
        trial.after_data = serde_json::json!({
            "targetIdentity": target_identity(&profile),
            "sourceHash": "hash-1",
            "rolledBack": true
        });
        let detail = BatchDetail {
            batch: MigrationBatch {
                batch_id: "batch".into(),
                batch_name: "batch".into(),
                source_type: "PHIS27".into(),
                source_name: "source".into(),
                source_description: String::new(),
                conflict_strategy: "INCREMENTAL".into(),
                allow_create_factory: false,
                idempotency_key: "key".into(),
                status: "VALIDATED".into(),
                total_count: 1,
                valid_count: 1,
                success_count: 0,
                fail_count: 0,
                skip_count: 0,
                created_at: String::new(),
                updated_at: String::new(),
                finished_at: None,
            },
            rows: vec![MigrationRow {
                row_id: "row-1".into(),
                batch_id: "batch".into(),
                row_no: 1,
                source_key: "1:1001".into(),
                source_hash: "hash-1".into(),
                status: "VALIDATED".into(),
                raw_data: Map::new(),
                normalized_data: Map::new(),
                error_code: String::new(),
                error_message: String::new(),
                id_med: String::new(),
                id_med_unit: String::new(),
                id_fac: String::new(),
                id_med_pro: String::new(),
                retry_count: 0,
                updated_at: String::new(),
            }],
            audits: vec![trial],
        };
        assert!(has_successful_trial(&detail, &profile));
        assert!(!has_successful_trial(
            &detail,
            &ConnectionProfile {
                host: "other.example.com".into(),
                ..profile
            }
        ));
    }
}
