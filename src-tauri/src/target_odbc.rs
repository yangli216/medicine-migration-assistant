use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{
    BatchDetail, ExecuteBatchRequest, MigrationRow, OverwritePreview, OverwriteRowPreview,
    PreviewOverwriteRequest, UndoBatchRequest,
};
use crate::normalize::value_text;
use crate::odbc::{
    configure_target_session, execute_strings, query_optional_row_strings, query_optional_string,
    with_connection,
};
use crate::overwrite::{medicine_patch, product_patch, restore_patch, ColumnPatch};
use crate::target::{
    alias_event_message, alias_search_fields, audit_overwrite_preview, audit_undo_failure,
    audit_undo_start, build_field_diffs, finish_undo, summarize_overwrite_preview, target_identity,
    validate_overwrite_execution_preview, validate_undo_request, RestoreTarget, UndoEvent,
    UndoTarget,
};
use crate::target_contract::validate_execution_context;
use chrono::Utc;
use odbc_api::Connection;
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

const HI_BD_MED_INSERT_SQL: &str = r#"INSERT INTO hi_bd_med(
    id_med,na_med,sd_med,id_cstmg,unit_pre,spec,dose,unit_dose,sd_dose,sd_dose_unit,
    sd_chrgitm_lv,sd_allergy,sd_anti_acl,fg_anti_appr,ddd,sd_bas_med,sd_spe_med,
    limit_anti_day,sd_storage,sd_pharm,sd_value,sd_prod_plac,fg_pois,sd_pois,fg_anti,
    sd_anti,sd_round,sd_dps,fg_med_rx,fg_bas_med,fg_skintest,sd_skintest,drip_rate,
    dft_dose_once,dft_usage,dft_freq,fg_tcd,fg_single,fg_register,fg_active,fg_pri,
    id_org_pri,id_tet,revision,insert_user,insert_time
) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULLIF(?,''),?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULLIF(?,''),?,?,?,CURRENT_TIMESTAMP)"#;

const HI_BD_MED_PRO_INSERT_SQL: &str = r#"INSERT INTO hi_bd_med_pro(
    id_med_pro,id_med,id_fac,id_med_unit,unit_sale,na_med_pro,spec_sale,price_sale,price_pur,
    unit_sale_factor,cd_appr,cd_bar,cd_med_pro,id_tet,revision,insert_user,insert_time,
    fg_active,fg_pri,id_org_pri,sd_per,per,fg_coll_pur,fg_import
) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,?,?,NULLIF(?,''),?,NULLIF(?,''),?,?)"#;

const HI_BD_FAC_INSERT_SQL: &str = r#"INSERT INTO hi_bd_fac(
    id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,
    id_tet,revision,insert_user,insert_time,sd_fac
) VALUES (?,?,?,?,?,?,?,?,?,?,?,CURRENT_TIMESTAMP,?)"#;

#[derive(Debug)]
struct WriteEvent {
    operation: &'static str,
    table: &'static str,
    target_id: String,
    message: String,
    before: Value,
    after: Value,
}

struct WriteOutcome {
    status: String,
    id_med: String,
    id_med_unit: String,
    id_fac: String,
    id_med_pro: String,
    events: Vec<WriteEvent>,
}

pub fn preview_overwrite(
    store: &LocalStore,
    request: PreviewOverwriteRequest,
    tenant_id: &str,
    operator_id: &str,
) -> Result<OverwritePreview, String> {
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.conflict_strategy != "OVERWRITE" {
        return Err("当前批次不是覆盖迁移批次".into());
    }
    let rows = with_connection(&request.target, |connection| {
        configure_target_session(connection, &request.target)?;
        crate::odbc::inspect_target_schema(&request.target)?;
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
            let before = read_odbc_patch_snapshot(
                connection,
                "hi_bd_med",
                "id_med",
                &row.id_med,
                tenant_id,
                &med_patch,
                false,
            )?;
            changes.extend(build_field_diffs("hi_bd_med", &med_patch, &before));
            if !row.id_med_pro.is_empty() {
                let product_patch = product_patch(
                    &row.normalized_data,
                    &row.id_med,
                    &row.id_fac,
                    &row.id_med_unit,
                );
                let before = read_odbc_patch_snapshot(
                    connection,
                    "hi_bd_med_pro",
                    "id_med_pro",
                    &row.id_med_pro,
                    tenant_id,
                    &product_patch,
                    false,
                )?;
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
        Ok(rows)
    })?;
    let preview = summarize_overwrite_preview(&request.batch_id, rows)?;
    audit_overwrite_preview(store, &preview, &request.target, operator_id)?;
    Ok(preview)
}

pub fn execute_batch(
    store: &LocalStore,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    validate_overwrite_execution_preview(&detail, &request)?;
    let selected_rows = request
        .selected_row_ids
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    with_connection(&request.target, |connection| {
        configure_target_session(connection, &request.target)?;
        crate::odbc::inspect_target_schema(&request.target)?;
        connection.set_autocommit(false).map_err(db_error)?;
        store.update_batch_counts(
            &request.batch_id,
            "RUNNING",
            detail.batch.valid_count,
            detail.batch.success_count,
            detail.batch.fail_count,
            detail.batch.skip_count,
            false,
        )?;
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
            "开始执行企业数据库迁移批次",
            &request.operator_id,
            &new_object_id(),
        )?;

        for mut row in detail.rows.clone() {
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
                        &new_object_id(),
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
                connection,
                &row,
                &detail.batch.conflict_strategy,
                detail.batch.allow_create_factory,
                &request.tenant_id,
                &request.operator_id,
                &request.organization_id,
            ) {
                Ok(outcome) => {
                    if let Err(error) = connection.commit() {
                        let _ = connection.rollback();
                        record_failure(store, &request, &mut row, db_error(error), &trace_id)?;
                        continue;
                    }
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
                    let rollback_error = connection.rollback().err().map(db_error);
                    let message = rollback_error
                        .map(|rollback| format!("{error}；回滚异常：{rollback}"))
                        .unwrap_or(error);
                    record_failure(store, &request, &mut row, message, &trace_id)?;
                }
            }
        }
        connection.set_autocommit(true).map_err(db_error)?;
        Ok(())
    })?;
    finish_batch(store, &request.batch_id, &request.operator_id)?;
    store.load_batch(&request.batch_id)
}

pub fn undo_batch(
    store: &LocalStore,
    request: UndoBatchRequest,
    tenant_id: &str,
    operator_id: &str,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    let plan = validate_undo_request(&detail, &request.target)?;
    audit_undo_start(store, &request.batch_id, &request.target, operator_id)?;
    let result = with_connection(&request.target, |connection| {
        configure_target_session(connection, &request.target)?;
        crate::odbc::inspect_target_schema(&request.target)?;
        connection.set_autocommit(false).map_err(db_error)?;
        let result = (|| {
            let mut events = Vec::with_capacity(plan.restores.len() + plan.inserts.len());
            for target in plan.restores {
                events.push(restore_odbc_target(connection, target, tenant_id)?);
            }
            for target in plan.inserts {
                events.push(undo_odbc_target(connection, target, tenant_id)?);
            }
            Ok::<_, String>(events)
        })();
        match result {
            Ok(events) => {
                if let Err(error) = connection.commit() {
                    let _ = connection.rollback();
                    let _ = connection.set_autocommit(true);
                    return Err(db_error(error));
                }
                connection.set_autocommit(true).map_err(db_error)?;
                Ok(events)
            }
            Err(error) => {
                let rollback_error = connection.rollback().err().map(db_error);
                let _ = connection.set_autocommit(true);
                Err(rollback_error
                    .map(|rollback| format!("{error}；回滚异常：{rollback}"))
                    .unwrap_or(error))
            }
        }
    });
    match result {
        Ok(events) => finish_undo(store, &request.batch_id, operator_id, events),
        Err(error) => {
            audit_undo_failure(store, &request.batch_id, operator_id, &error)?;
            Err(error)
        }
    }
}

fn restore_odbc_target(
    connection: &Connection<'_>,
    target: RestoreTarget,
    tenant_id: &str,
) -> Result<UndoEvent, String> {
    let primary_key = match target.table.as_str() {
        "hi_bd_med" => "id_med",
        "hi_bd_med_pro" => "id_med_pro",
        _ => return Err("覆盖恢复清单包含未授权的目标表".into()),
    };
    let patch = restore_patch(&target.table, &target.before)?;
    apply_odbc_patch(
        connection,
        &target.table,
        primary_key,
        &target.target_id,
        tenant_id,
        &patch,
    )?;
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

fn undo_odbc_target(
    connection: &Connection<'_>,
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
    if odbc_count(connection, exists_sql, tenant_id, &target.target_id)? == 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_ABSENT",
            result: "SKIPPED",
            message: "该条目标记录已不存在，未重复删除".into(),
        });
    }
    let references = odbc_reference_count(connection, &target, tenant_id)?;
    if references > 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_RETAIN",
            result: "SKIPPED",
            message: format!("检测到{references}条后续引用，为保护业务数据已保留"),
        });
    }
    execute_strings(
        connection,
        delete_sql,
        vec![tenant_id.into(), target.target_id.clone()],
    )?;
    Ok(UndoEvent {
        target,
        operation: "UNDO_DELETE",
        result: "SUCCESS",
        message: "已删除本批次新增且未被后续引用的记录".into(),
    })
}

fn odbc_reference_count(
    connection: &Connection<'_>,
    target: &UndoTarget,
    tenant_id: &str,
) -> Result<usize, String> {
    let id = &target.target_id;
    match target.table.as_str() {
        "hi_bd_med_unit" => odbc_count(
            connection,
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_med_unit=?",
            tenant_id,
            id,
        ),
        "hi_bd_med" => Ok(odbc_count(
            connection,
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_med=?",
            tenant_id,
            id,
        )? + odbc_count(
            connection,
            "SELECT COUNT(*) FROM hi_bd_med_alias WHERE id_tet=? AND id_med=?",
            tenant_id,
            id,
        )? + odbc_count(
            connection,
            "SELECT COUNT(*) FROM hi_bd_med_unit WHERE id_tet=? AND id_med=?",
            tenant_id,
            id,
        )?),
        "hi_bd_fac" => odbc_count(
            connection,
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=?",
            tenant_id,
            id,
        ),
        _ => Ok(0),
    }
}

fn odbc_count(
    connection: &Connection<'_>,
    sql: &str,
    tenant_id: &str,
    target_id: &str,
) -> Result<usize, String> {
    query_optional_string(connection, sql, vec![tenant_id.into(), target_id.into()])?
        .unwrap_or_default()
        .trim()
        .parse::<usize>()
        .map_err(|_| "目标数据库返回了无法识别的引用计数".to_string())
}

fn record_failure(
    store: &LocalStore,
    request: &ExecuteBatchRequest,
    row: &mut MigrationRow,
    error: String,
    trace_id: &str,
) -> Result<(), String> {
    row.status = "FAILED".into();
    row.error_code = database_error_code(&error);
    row.error_message = limit(&error, 2000);
    row.retry_count += 1;
    row.updated_at = Utc::now().to_rfc3339();
    store.update_row_result(row)?;
    store.audit_event(
        &request.batch_id,
        &row.row_id,
        "ROW_FAILED",
        failed_target_table(&error),
        &row.row_id,
        "FAILED",
        Value::Null,
        Value::Object(row.normalized_data.clone()),
        &row.error_message,
        &request.operator_id,
        trace_id,
    )
}

fn write_row(
    connection: &Connection<'_>,
    row: &MigrationRow,
    conflict_strategy: &str,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let mut events = Vec::new();
    let name = text(data, "naMed");
    let med_type = text(data, "sdMed");
    let spec = derived_spec(data);
    let unit_pre = text(data, "unitPre");
    let fg_pri = defaulted(data, "fgPri", "0");
    validate_execution_context(tenant_id, operator_id, organization_id, fg_pri == "1")?;
    if conflict_strategy.eq_ignore_ascii_case("OVERWRITE") && !row.id_med.is_empty() {
        return overwrite_odbc_row(
            connection,
            row,
            allow_create_factory,
            tenant_id,
            operator_id,
        );
    }
    let existing_med = if fg_pri == "1" {
        query_optional_string(
            connection,
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL)",
            vec![tenant_id.into(), name.clone(), spec.clone(), unit_pre.clone(), organization_id.into()],
        )?
    } else {
        query_optional_string(
            connection,
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL)",
            vec![tenant_id.into(), name.clone(), spec.clone(), unit_pre.clone()],
        )?
    };
    let id_med = if let Some(id) = existing_med {
        if conflict_strategy.eq_ignore_ascii_case("FAIL") {
            return Err(format!("药品基本信息已存在：{name} / {spec}"));
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
        execute_target_strings(
            connection,
            "hi_bd_med",
            "新增药品基本信息",
            HI_BD_MED_INSERT_SQL,
            vec![
                id.clone(),
                name.clone(),
                med_type.clone(),
                text(data, "idCstmg"),
                unit_pre.clone(),
                spec.clone(),
                text(data, "dose"),
                text(data, "unitDose"),
                text(data, "sdDose"),
                text(data, "sdDoseUnit"),
                text(data, "sdChrgitmLv"),
                text(data, "sdAllergy"),
                text(data, "sdAntiAcl"),
                defaulted(data, "fgAntiAppr", "0"),
                text(data, "ddd"),
                text(data, "sdBasMed"),
                text(data, "sdSpeMed"),
                optional_decimal(data, "limitAntiDay")?,
                text(data, "sdStorage"),
                text(data, "sdPharm"),
                text(data, "sdValue"),
                text(data, "sdProdPlac"),
                defaulted(data, "fgPois", "0"),
                text(data, "sdPois"),
                defaulted(data, "fgAnti", "0"),
                text(data, "sdAnti"),
                text(data, "sdRound"),
                text(data, "sdDps"),
                defaulted(data, "fgMedRx", "2"),
                defaulted(data, "fgBasMed", "0"),
                defaulted(data, "fgSkintest", "0"),
                text(data, "sdSkintest"),
                text(data, "dripRate"),
                dft_dose_once,
                text(data, "dftUsage"),
                text(data, "dftFreq"),
                defaulted(data, "fgTcd", "0"),
                defaulted(data, "fgSingle", "1"),
                defaulted(data, "fgRegister", "0"),
                "1".into(),
                fg_pri.clone(),
                if fg_pri == "1" {
                    organization_id.into()
                } else {
                    String::new()
                },
                tenant_id.into(),
                "0".into(),
                operator_id.into(),
            ],
        )?;
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

    ensure_alias(connection, &id_med, data, tenant_id, &mut events)?;
    let id_med_unit = ensure_unit(connection, &id_med, data, tenant_id, &mut events)?;
    if !crate::normalize::has_product_data(data) {
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
        connection,
        data,
        allow_create_factory,
        tenant_id,
        operator_id,
        &mut events,
    )?;
    let product_name = defaulted(data, "naMedPro", &name);
    let sale_spec = derived_sale_spec(data, &spec);
    let external_code = text(data, "cdMedPro");
    let by_external_code = if external_code.is_empty() {
        None
    } else if fg_pri == "1" {
        query_optional_string(
            connection,
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL)",
            vec![tenant_id.into(), external_code.clone(), organization_id.into()],
        )?
    } else {
        query_optional_string(
            connection,
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL)",
            vec![tenant_id.into(), external_code.clone()],
        )?
    };
    let existing_product = if let Some(id) = by_external_code {
        let linked_med = query_optional_string(
            connection,
            "SELECT id_med FROM hi_bd_med_pro WHERE id_med_pro=?",
            vec![id.clone()],
        )?
        .ok_or_else(|| format!("目标商品{id}在货品码判重后无法再次读取"))?;
        let linked_fac = query_optional_string(
            connection,
            "SELECT id_fac FROM hi_bd_med_pro WHERE id_med_pro=?",
            vec![id.clone()],
        )?
        .ok_or_else(|| format!("目标商品{id}缺少生产厂家关联"))?;
        if linked_med != id_med || linked_fac != id_fac {
            return Err(format!(
                "三方货品码“{external_code}”已被目标商品{id}使用，但关联药品或厂家不同；请修正货品码对照"
            ));
        }
        Some((id, "三方货品码"))
    } else {
        let by_business_key = if fg_pri == "1" {
            query_optional_string(
                connection,
                "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL)",
                vec![tenant_id.into(), id_fac.clone(), product_name.clone(), sale_spec.clone(), organization_id.into()],
            )?
        } else {
            query_optional_string(
                connection,
                "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL)",
                vec![tenant_id.into(), id_fac.clone(), product_name.clone(), sale_spec.clone()],
            )?
        };
        by_business_key.map(|id| (id, "厂家+商品名+销售规格"))
    };
    if let Some((id_med_pro, matched_by)) = existing_product {
        if conflict_strategy.eq_ignore_ascii_case("FAIL") {
            return Err("同厂家、商品名和销售规格的药品商品已存在".into());
        }
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
    execute_target_strings(
        connection,
        "hi_bd_med_pro",
        "新增药品商品信息",
        HI_BD_MED_PRO_INSERT_SQL,
        vec![
            id_med_pro.clone(),
            id_med.clone(),
            id_fac.clone(),
            id_med_unit.clone(),
            text(data, "unitSale"),
            product_name,
            sale_spec,
            decimal(data, "priceSale")?,
            decimal(data, "pricePur")?,
            integer(data, "unitSaleFactor")?,
            text(data, "cdAppr"),
            text(data, "cdBar"),
            text(data, "cdMedPro"),
            tenant_id.into(),
            "0".into(),
            operator_id.into(),
            "1".into(),
            fg_pri.clone(),
            if fg_pri == "1" {
                organization_id.into()
            } else {
                String::new()
            },
            text(data, "sdPer"),
            optional_decimal(data, "per")?,
            defaulted(data, "fgCollPur", "0"),
            defaulted(data, "fgImport", "1"),
        ],
    )?;
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

fn overwrite_odbc_row(
    connection: &Connection<'_>,
    row: &MigrationRow,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let id_med = row.id_med.clone();
    let name = text(data, "naMed");
    let spec = derived_spec(data);
    if let Some(duplicate) = query_optional_string(
        connection,
        "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND COALESCE(spec,'')=? AND COALESCE(unit_pre,'')=? AND id_med<>? AND fg_active='1'",
        vec![tenant_id.into(), name.clone(), spec.clone(), text(data, "unitPre"), id_med.clone()],
    )? {
        return Err(format!(
            "覆盖后的药品名称、规格、单位与目标药品{duplicate}重复，请先处理重复数据"
        ));
    }
    let mut events = Vec::new();
    let med_patch = medicine_patch(data);
    let (before, after) = apply_odbc_patch(
        connection,
        "hi_bd_med",
        "id_med",
        &id_med,
        tenant_id,
        &med_patch,
    )?;
    events.push(WriteEvent {
        operation: "UPDATE",
        table: "hi_bd_med",
        target_id: id_med.clone(),
        message: "覆盖药品基本信息，已保存字段级修改前快照".into(),
        before,
        after,
    });
    ensure_alias(connection, &id_med, data, tenant_id, &mut events)?;
    let id_med_unit = ensure_unit(connection, &id_med, data, tenant_id, &mut events)?;
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
        connection,
        data,
        allow_create_factory,
        tenant_id,
        operator_id,
        &mut events,
    )?;
    ensure_no_odbc_product_conflict(connection, row, &id_fac, &name, &spec, tenant_id)?;
    let product_patch = product_patch(data, &id_med, &id_fac, &id_med_unit);
    let (before, after) = apply_odbc_patch(
        connection,
        "hi_bd_med_pro",
        "id_med_pro",
        &row.id_med_pro,
        tenant_id,
        &product_patch,
    )?;
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

fn ensure_no_odbc_product_conflict(
    connection: &Connection<'_>,
    row: &MigrationRow,
    id_fac: &str,
    base_name: &str,
    base_spec: &str,
    tenant_id: &str,
) -> Result<(), String> {
    let data = &row.normalized_data;
    let external_code = text(data, "cdMedPro");
    if !external_code.is_empty() {
        if let Some(id) = query_optional_string(
            connection,
            "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND cd_med_pro=? AND id_med_pro<>? AND fg_active='1'",
            vec![tenant_id.into(), external_code, row.id_med_pro.clone()],
        )? {
            return Err(format!("覆盖后的三方货品码已被目标商品{id}使用"));
        }
    }
    let product_name = defaulted(data, "naMedPro", base_name);
    let sale_spec = derived_sale_spec(data, base_spec);
    if let Some(id) = query_optional_string(
        connection,
        "SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=? AND id_fac=? AND na_med_pro=? AND COALESCE(spec_sale,'')=? AND id_med_pro<>? AND fg_active='1'",
        vec![tenant_id.into(), id_fac.into(), product_name, sale_spec, row.id_med_pro.clone()],
    )? {
        return Err(format!("覆盖后的厂家、商品名和销售规格与目标商品{id}冲突"));
    }
    Ok(())
}

fn apply_odbc_patch(
    connection: &Connection<'_>,
    table: &str,
    primary_key: &str,
    target_id: &str,
    tenant_id: &str,
    patch: &[ColumnPatch],
) -> Result<(Value, Value), String> {
    let existing = read_odbc_patch_snapshot(
        connection,
        table,
        primary_key,
        target_id,
        tenant_id,
        patch,
        true,
    )?;
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
        .map(|item| {
            if item.value.is_some() {
                format!("{}=?", item.column)
            } else {
                format!("{}=NULL", item.column)
            }
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut values = patch
        .iter()
        .filter_map(|item| item.value.clone())
        .collect::<Vec<_>>();
    values.push(tenant_id.into());
    values.push(target_id.into());
    execute_target_strings(
        connection,
        table,
        "覆盖目标字段",
        &format!("UPDATE {table} SET {assignments} WHERE id_tet=? AND {primary_key}=?"),
        values,
    )?;
    Ok((Value::Object(before), Value::Object(after)))
}

#[allow(clippy::too_many_arguments)]
fn read_odbc_patch_snapshot(
    connection: &Connection<'_>,
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
    let columns = patch
        .iter()
        .map(|item| item.column)
        .collect::<Vec<_>>()
        .join(",");
    let lock = if for_update { " FOR UPDATE" } else { "" };
    let select_sql =
        format!("SELECT {columns} FROM {table} WHERE id_tet=? AND {primary_key}=?{lock}");
    let existing = query_optional_row_strings(
        connection,
        &select_sql,
        vec![tenant_id.into(), target_id.into()],
    )?
    .ok_or_else(|| format!("覆盖目标不存在或不属于当前租户：{table}/{target_id}"))?;
    if existing.len() != patch.len() {
        return Err("覆盖快照字段数量与更新清单不一致".into());
    }
    Ok(existing)
}

fn ensure_alias(
    connection: &Connection<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<(), String> {
    let name = text(data, "naMed");
    let exists = query_optional_string(
        connection,
        "SELECT id_med_alias FROM hi_bd_med_alias WHERE id_tet=? AND id_med=? AND na_alias=? AND fg_main='1' AND fg_active='1'",
        vec![tenant_id.into(), id_med.into(), name.clone()],
    )?;
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
        execute_target_strings(
            connection,
            "hi_bd_med_alias",
            "新增药品主别名",
            "INSERT INTO hi_bd_med_alias(id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active) VALUES (?,?,?,?,?,?,?,?,?)",
            vec![id.clone(), id_med.into(), name.clone(), "1".into(), py.clone(), wb.clone(), instr, tenant_id.into(), "1".into()],
        )?;
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

fn ensure_unit(
    connection: &Connection<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let unit = defaulted(data, "unitSale", &text(data, "unitPre"));
    let factor = if text(data, "unitSaleFactor").is_empty() {
        "1".to_string()
    } else {
        integer(data, "unitSaleFactor")?
    };
    let exists = query_optional_string(
        connection,
        "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=? AND id_med=? AND na_unit=? AND unit_factor=?",
        vec![tenant_id.into(), id_med.into(), unit.clone(), factor.clone()],
    )?;
    if let Some(id) = exists {
        events.push(WriteEvent {
            operation: "REUSE",
            table: "hi_bd_med_unit",
            target_id: id.clone(),
            message: "复用已存在的药品包装单位".into(),
            before: json!({"idMedUnit":id,"idMed":id_med,"naUnit":unit,"unitFactor":factor}),
            after: json!({"idMedUnit":id}),
        });
        return Ok(id);
    }
    let id = new_object_id();
    execute_target_strings(
        connection,
        "hi_bd_med_unit",
        "新增药品包装单位",
        "INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES (?,?,?,?,?)",
        vec![id.clone(), id_med.into(), unit.clone(), factor.clone(), tenant_id.into()],
    )?;
    events.push(WriteEvent {
        operation: "INSERT",
        table: "hi_bd_med_unit",
        target_id: id.clone(),
        message: "新增药品包装单位".into(),
        before: Value::Null,
        after: json!({"idMed":id_med,"naUnit":unit,"unitFactor":factor}),
    });
    Ok(id)
}

fn ensure_factory(
    connection: &Connection<'_>,
    data: &Map<String, Value>,
    allow_create: bool,
    tenant_id: &str,
    operator_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let requested_id = text(data, "idFac");
    if !requested_id.is_empty() {
        let id = query_optional_string(
            connection,
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=? AND id_tet=? AND fg_active='1'",
            vec![requested_id.clone(), tenant_id.into()],
        )?
        .ok_or_else(|| format!("生产厂家主键无效或不属于当前租户：{requested_id}"))?;
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
        if let Some(id) = query_optional_string(
            connection,
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=? AND id_tet=? AND fg_active='1'",
            vec![mapped_id, tenant_id.into()],
        )? {
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
    if let Some(id) = query_optional_string(
        connection,
        "SELECT id_fac FROM hi_bd_fac WHERE id_tet=? AND na_fac=? AND fg_active='1'",
        vec![tenant_id.into(), name.clone()],
    )? {
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
            "生产厂家“{name}”未匹配；请先建立对照或开启允许创建厂家"
        ));
    }
    let id = new_object_id();
    let short_name = defaulted(data, "naFacShort", &limit(&name, 32));
    let pinyin = text(data, "pyFac");
    execute_target_strings(
        connection,
        "hi_bd_fac",
        "新增生产厂家",
        HI_BD_FAC_INSERT_SQL,
        vec![
            id.clone(),
            name.clone(),
            limit(&short_name, 32),
            defaulted(data, "sdProdPlac", "1"),
            pinyin.clone(),
            String::new(),
            name.clone(),
            "1".into(),
            tenant_id.into(),
            "0".into(),
            operator_id.into(),
            "1".into(),
        ],
    )?;
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

fn finish_batch(store: &LocalStore, batch_id: &str, operator_id: &str) -> Result<(), String> {
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
    store.audit_event(
        batch_id,
        "",
        "FINISH",
        "migration_batch",
        batch_id,
        status,
        Value::Null,
        json!({"success":success,"skipped":skipped,"failed":failed,"pending":valid}),
        &format!("执行完成：成功{success}行，跳过{skipped}行，失败{failed}行"),
        operator_id,
        &new_object_id(),
    )
}

fn text(data: &Map<String, Value>, key: &str) -> String {
    data.get(key).map(value_text).unwrap_or_default()
}
fn defaulted(data: &Map<String, Value>, key: &str, default: &str) -> String {
    let value = text(data, key);
    if value.is_empty() {
        default.into()
    } else {
        value
    }
}
fn integer(data: &Map<String, Value>, key: &str) -> Result<String, String> {
    text(data, key)
        .parse::<i64>()
        .map(|value| value.to_string())
        .map_err(|_| format!("{key}必须是整数"))
}
fn decimal(data: &Map<String, Value>, key: &str) -> Result<String, String> {
    text(data, key)
        .parse::<Decimal>()
        .map(|value| value.to_string())
        .map_err(|_| format!("{key}必须是数字"))
}
fn optional_decimal(data: &Map<String, Value>, key: &str) -> Result<String, String> {
    let value = text(data, key);
    if value.is_empty() {
        Ok(String::new())
    } else {
        decimal(data, key)
    }
}
fn derived_spec(data: &Map<String, Value>) -> String {
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
fn derived_sale_spec(data: &Map<String, Value>, base_spec: &str) -> String {
    let spec = text(data, "specSale");
    if !spec.is_empty() {
        return spec;
    }
    let factor = integer(data, "unitSaleFactor")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(1);
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
fn db_error(error: impl std::fmt::Display) -> String {
    format!("目标数据库写入失败：{error}")
}

fn execute_target_strings(
    connection: &Connection<'_>,
    table: &str,
    stage: &str,
    statement: &str,
    values: Vec<String>,
) -> Result<(), String> {
    execute_strings(connection, statement, values)
        .map_err(|error| write_stage_error(table, stage, &error))
}

fn write_stage_error(table: &str, stage: &str, error: &str) -> String {
    format!("目标表 {table} 在“{stage}”时写入失败：{error}")
}

fn database_error_code(error: &str) -> String {
    error
        .find("ORA-")
        .and_then(|start| error.get(start..start + 9))
        .filter(|code| {
            code[4..]
                .chars()
                .all(|character| character.is_ascii_digit())
        })
        .unwrap_or("WRITE_ERROR")
        .to_string()
}

fn failed_target_table(error: &str) -> &str {
    [
        "hi_bd_med_pro",
        "hi_bd_med_alias",
        "hi_bd_med_unit",
        "hi_bd_med",
        "hi_bd_fac",
    ]
    .into_iter()
    .find(|table| error.contains(table))
    .unwrap_or("migration_row")
}
fn limit(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}

#[cfg(test)]
mod tests {
    use super::{
        database_error_code, failed_target_table, write_stage_error, HI_BD_FAC_INSERT_SQL,
        HI_BD_MED_INSERT_SQL, HI_BD_MED_PRO_INSERT_SQL,
    };

    #[test]
    fn target_insert_timestamps_are_database_typed_expressions() {
        for (statement, expected_parameters) in [
            (HI_BD_MED_INSERT_SQL, 45),
            (HI_BD_MED_PRO_INSERT_SQL, 23),
            (HI_BD_FAC_INSERT_SQL, 12),
        ] {
            assert!(statement.contains("CURRENT_TIMESTAMP"));
            assert_eq!(
                statement
                    .chars()
                    .filter(|character| *character == '?')
                    .count(),
                expected_parameters
            );
            assert!(
                !statement.contains("insert_time) VALUES")
                    || statement.contains("CURRENT_TIMESTAMP")
            );
        }
    }

    #[test]
    fn write_failure_carries_table_stage_and_oracle_code() {
        let error = write_stage_error("hi_bd_med", "新增药品基本信息", "ORA-01843: invalid month");
        assert_eq!(failed_target_table(&error), "hi_bd_med");
        assert_eq!(database_error_code(&error), "ORA-01843");
        assert!(error.contains("新增药品基本信息"));
    }
}
