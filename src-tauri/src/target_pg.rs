use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{BatchDetail, ExecuteBatchRequest, MigrationRow, UndoBatchRequest};
use crate::normalize::value_text;
use crate::target::{
    audit_undo_failure, audit_undo_start, decimal, defaulted, derived_sale_spec, derived_spec,
    finish_batch, finish_undo, integer, limit, optional_decimal, target_identity, text,
    validate_undo_request, UndoEvent, UndoTarget, WriteEvent, WriteOutcome,
};
use crate::target_contract::validate_execution_context;
use chrono::Utc;
use serde_json::{json, Map, Value};
use sqlx_core::query::query;
use sqlx_core::query_builder::QueryBuilder;
use sqlx_core::query_scalar::query_scalar;
use sqlx_core::row::Row;
use sqlx_postgres::{PgPool, PgTransaction, Postgres};
use std::collections::HashSet;

pub async fn execute_batch(
    store: &LocalStore,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.conflict_strategy == "OVERWRITE" {
        return Err(
            "PostgreSQL 通用协议的覆盖迁移将在下一阶段开放；当前请使用增量迁移，或显式配置已验收的厂商 ODBC 回退"
                .into(),
        );
    }
    let pool = crate::pg_protocol::connect(&request.target).await?;
    crate::pg_protocol::inspect_pool(&pool).await?;
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
            "failedOnly": request.failed_only,
            "selectedRowCount": request.selected_row_ids.len(),
            "skipInvalidRows": request.skip_invalid_rows,
            "databaseKind": request.target.kind,
            "protocol": "postgresql-wire",
            "targetIdentity": target_identity(&request.target)
        }),
        "开始使用应用内置 PostgreSQL 通用协议执行迁移批次",
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
        restore_factory_link(
            store,
            &request,
            &detail.batch.source_type,
            &detail.batch.source_name,
            &mut row,
        )?;
        let trace_id = new_object_id();
        match write_row(
            &pool,
            &row,
            &detail.batch.conflict_strategy,
            detail.batch.allow_create_factory,
            &request.tenant_id,
            &request.operator_id,
            &request.organization_id,
        )
        .await
        {
            Ok(outcome) => record_success(
                store,
                &request,
                &detail.batch.source_type,
                &detail.batch.source_name,
                &mut row,
                outcome,
                &trace_id,
            )?,
            Err(error) => record_failure(store, &request, &mut row, &error, &trace_id)?,
        }
    }
    pool.close().await;
    finish_batch(store, &request.batch_id, &request.operator_id)?;
    store.load_batch(&request.batch_id)
}

pub async fn undo_batch(
    store: &LocalStore,
    request: UndoBatchRequest,
    tenant_id: &str,
    operator_id: &str,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    let plan = validate_undo_request(&detail, &request.target)?;
    if !plan.restores.is_empty() {
        return Err(
            "PostgreSQL 通用协议当前不支持撤销覆盖写入；该协议尚未开放覆盖迁移，已停止操作".into(),
        );
    }
    audit_undo_start(store, &request.batch_id, &request.target, operator_id)?;
    let pool = match crate::pg_protocol::connect(&request.target).await {
        Ok(pool) => pool,
        Err(error) => {
            audit_undo_failure(store, &request.batch_id, operator_id, &error)?;
            return Err(error);
        }
    };
    let result = async {
        crate::pg_protocol::inspect_pool(&pool).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| pg_error("migration_batch", "开启安全撤销事务", error))?;
        let mut events = Vec::with_capacity(plan.inserts.len());
        for target in plan.inserts {
            events.push(undo_pg_target(&mut tx, target, tenant_id).await?);
        }
        tx.commit()
            .await
            .map_err(|error| pg_error("migration_batch", "提交安全撤销事务", error))?;
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

async fn undo_pg_target(
    tx: &mut PgTransaction<'_>,
    target: UndoTarget,
    tenant_id: &str,
) -> Result<UndoEvent, String> {
    let (exists_sql, delete_sql) = match target.table.as_str() {
        "hi_bd_med_pro" => (
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=$1 AND id_med_pro=$2",
            "DELETE FROM hi_bd_med_pro WHERE id_tet=$1 AND id_med_pro=$2",
        ),
        "hi_bd_med_alias" => (
            "SELECT COUNT(*) FROM hi_bd_med_alias WHERE id_tet=$1 AND id_med_alias=$2",
            "DELETE FROM hi_bd_med_alias WHERE id_tet=$1 AND id_med_alias=$2",
        ),
        "hi_bd_med_unit" => (
            "SELECT COUNT(*) FROM hi_bd_med_unit WHERE id_tet=$1 AND id_med_unit=$2",
            "DELETE FROM hi_bd_med_unit WHERE id_tet=$1 AND id_med_unit=$2",
        ),
        "hi_bd_med" => (
            "SELECT COUNT(*) FROM hi_bd_med WHERE id_tet=$1 AND id_med=$2",
            "DELETE FROM hi_bd_med WHERE id_tet=$1 AND id_med=$2",
        ),
        "hi_bd_fac" => (
            "SELECT COUNT(*) FROM hi_bd_fac WHERE id_tet=$1 AND id_fac=$2",
            "DELETE FROM hi_bd_fac WHERE id_tet=$1 AND id_fac=$2",
        ),
        _ => return Err("撤销清单包含未授权的目标表".into()),
    };
    if pg_count(tx, exists_sql, tenant_id, &target.target_id).await? == 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_ABSENT",
            result: "SKIPPED",
            message: "该条目标记录已不存在，未重复删除".into(),
        });
    }
    let references = pg_reference_count(tx, &target, tenant_id).await?;
    if references > 0 {
        return Ok(UndoEvent {
            target,
            operation: "UNDO_RETAIN",
            result: "SKIPPED",
            message: format!("检测到{references}条后续引用，为保护业务数据已保留"),
        });
    }
    query::<Postgres>(delete_sql)
        .bind(tenant_id)
        .bind(&target.target_id)
        .execute(&mut **tx)
        .await
        .map_err(|error| pg_error(&target.table, "撤销本批次新增记录", error))?;
    Ok(UndoEvent {
        target,
        operation: "UNDO_DELETE",
        result: "SUCCESS",
        message: "已删除本批次新增且未被后续引用的记录".into(),
    })
}

async fn pg_reference_count(
    tx: &mut PgTransaction<'_>,
    target: &UndoTarget,
    tenant_id: &str,
) -> Result<i64, String> {
    let id = &target.target_id;
    match target.table.as_str() {
        "hi_bd_med_unit" => {
            pg_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=$1 AND id_med_unit=$2",
                tenant_id,
                id,
            )
            .await
        }
        "hi_bd_med" => Ok(pg_count(
            tx,
            "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=$1 AND id_med=$2",
            tenant_id,
            id,
        )
        .await?
            + pg_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_alias WHERE id_tet=$1 AND id_med=$2",
                tenant_id,
                id,
            )
            .await?
            + pg_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_unit WHERE id_tet=$1 AND id_med=$2",
                tenant_id,
                id,
            )
            .await?),
        "hi_bd_fac" => {
            pg_count(
                tx,
                "SELECT COUNT(*) FROM hi_bd_med_pro WHERE id_tet=$1 AND id_fac=$2",
                tenant_id,
                id,
            )
            .await
        }
        _ => Ok(0),
    }
}

async fn pg_count(
    tx: &mut PgTransaction<'_>,
    sql: &str,
    tenant_id: &str,
    target_id: &str,
) -> Result<i64, String> {
    query_scalar::<Postgres, i64>(sql)
        .bind(tenant_id)
        .bind(target_id)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| pg_error("migration_batch", "检查撤销引用", error))
}

fn restore_factory_link(
    store: &LocalStore,
    request: &ExecuteBatchRequest,
    source_type: &str,
    source_name: &str,
    row: &mut MigrationRow,
) -> Result<(), String> {
    let Some(source_factory_key) = row
        .normalized_data
        .get("_sourceFactoryKey")
        .map(value_text)
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if let Some(target_id_fac) = store.find_factory_link(
        &request.tenant_id,
        source_type,
        source_name,
        &source_factory_key,
    )? {
        row.normalized_data.insert(
            "_sourceFactoryTargetId".into(),
            Value::String(target_id_fac),
        );
    }
    Ok(())
}

fn record_success(
    store: &LocalStore,
    request: &ExecuteBatchRequest,
    source_type: &str,
    source_name: &str,
    row: &mut MigrationRow,
    outcome: WriteOutcome,
    trace_id: &str,
) -> Result<(), String> {
    row.status = outcome.status;
    row.id_med = outcome.id_med;
    row.id_med_unit = outcome.id_med_unit;
    row.id_fac = outcome.id_fac;
    row.id_med_pro = outcome.id_med_pro;
    row.error_code.clear();
    row.error_message.clear();
    row.updated_at = Utc::now().to_rfc3339();
    store.update_row_result(row)?;
    if let Some(source_factory_key) = row
        .normalized_data
        .get("_sourceFactoryKey")
        .map(value_text)
        .filter(|value| !value.is_empty())
    {
        store.record_factory_link_upsert(
            &request.tenant_id,
            source_type,
            source_name,
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
        source_type,
        source_name,
        row,
        write_manifest,
        &request.operator_id,
        trace_id,
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
            trace_id,
        )?;
    }
    Ok(())
}

fn record_failure(
    store: &LocalStore,
    request: &ExecuteBatchRequest,
    row: &mut MigrationRow,
    error: &str,
    trace_id: &str,
) -> Result<(), String> {
    row.status = "FAILED".into();
    row.error_code = sqlstate_from_error(error).unwrap_or_else(|| "PG_WRITE_ERROR".into());
    row.error_message = limit(error, 2000);
    row.retry_count += 1;
    row.updated_at = Utc::now().to_rfc3339();
    store.update_row_result(row)?;
    store.audit_event(
        &request.batch_id,
        &row.row_id,
        "ROW_FAILED",
        failed_table(error),
        &row.row_id,
        "FAILED",
        Value::Null,
        Value::Object(row.normalized_data.clone()),
        &row.error_message,
        &request.operator_id,
        trace_id,
    )
}

#[allow(clippy::too_many_arguments)]
async fn write_row(
    pool: &PgPool,
    row: &MigrationRow,
    conflict_strategy: &str,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let now = Utc::now().naive_utc();
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| pg_error("migration_row", "开启逐行事务", error))?;
    let mut events = Vec::new();
    let name = text(data, "naMed");
    let med_type = text(data, "sdMed");
    let spec = derived_spec(data);
    let unit_pre = text(data, "unitPre");
    let fg_pri = defaulted(data, "fgPri", "0");
    validate_execution_context(tenant_id, operator_id, organization_id, fg_pri == "1")?;

    let existing_med = if fg_pri == "1" {
        query_scalar::<Postgres, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=$1 AND na_med=$2 AND COALESCE(spec,'')=$3 AND COALESCE(unit_pre,'')=$4 AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=$5) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&name)
        .bind(&spec)
        .bind(&unit_pre)
        .bind(organization_id)
        .fetch_optional(&mut *tx)
        .await
    } else {
        query_scalar::<Postgres, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=$1 AND na_med=$2 AND COALESCE(spec,'')=$3 AND COALESCE(unit_pre,'')=$4 AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&name)
        .bind(&spec)
        .bind(&unit_pre)
        .fetch_optional(&mut *tx)
        .await
    }
    .map_err(|error| pg_error("hi_bd_med", "按业务键查重", error))?;

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
        insert_medicine(
            &mut tx,
            &id,
            data,
            &name,
            &med_type,
            &spec,
            &unit_pre,
            &fg_pri,
            tenant_id,
            operator_id,
            organization_id,
            now,
        )
        .await?;
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

    ensure_alias(
        &mut tx,
        &id_med,
        &name,
        tenant_id,
        &fg_pri,
        organization_id,
        &mut events,
    )
    .await?;
    let id_med_unit = ensure_unit(&mut tx, &id_med, data, tenant_id, &mut events).await?;
    if !crate::normalize::has_product_data(data) {
        tx.commit()
            .await
            .map_err(|error| pg_error("migration_row", "提交逐行事务", error))?;
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
        query_scalar::<Postgres, String>("SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=$1 AND cd_med_pro=$2 AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=$3) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1")
            .bind(tenant_id).bind(&external_code).bind(organization_id).fetch_optional(&mut *tx).await
            .map_err(|error| pg_error("hi_bd_med_pro", "按三方货品码查重", error))?
    } else {
        query_scalar::<Postgres, String>("SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=$1 AND cd_med_pro=$2 AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1")
            .bind(tenant_id).bind(&external_code).fetch_optional(&mut *tx).await
            .map_err(|error| pg_error("hi_bd_med_pro", "按三方货品码查重", error))?
    };
    let existing_product = if let Some(id) = by_external_code {
        let relation = query::<Postgres>(
            "SELECT id_med,id_fac FROM hi_bd_med_pro WHERE id_med_pro=$1 LIMIT 1",
        )
        .bind(&id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|error| pg_error("hi_bd_med_pro", "核对商品关系", error))?;
        let linked_med = relation
            .try_get::<String, _>(0)
            .map_err(|error| pg_error("hi_bd_med_pro", "读取商品关联药品", error))?;
        let linked_fac = relation
            .try_get::<String, _>(1)
            .map_err(|error| pg_error("hi_bd_med_pro", "读取商品关联厂家", error))?;
        if linked_med != id_med || linked_fac != id_fac {
            return Err(format!("三方货品码“{external_code}”已被目标商品{id}使用，但关联药品或厂家不同；请修正货品码对照"));
        }
        Some((id, "三方货品码"))
    } else {
        let found = if fg_pri == "1" {
            query_scalar::<Postgres, String>("SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=$1 AND id_fac=$2 AND na_med_pro=$3 AND COALESCE(spec_sale,'')=$4 AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=$5) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1")
                .bind(tenant_id).bind(&id_fac).bind(&product_name).bind(&sale_spec).bind(organization_id).fetch_optional(&mut *tx).await
        } else {
            query_scalar::<Postgres, String>("SELECT id_med_pro FROM hi_bd_med_pro WHERE id_tet=$1 AND id_fac=$2 AND na_med_pro=$3 AND COALESCE(spec_sale,'')=$4 AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1")
                .bind(tenant_id).bind(&id_fac).bind(&product_name).bind(&sale_spec).fetch_optional(&mut *tx).await
        }.map_err(|error| pg_error("hi_bd_med_pro", "按商品业务键查重", error))?;
        found.map(|id| (id, "厂家+商品名+销售规格"))
    };
    if let Some((id_med_pro, matched_by)) = existing_product {
        if conflict_strategy.eq_ignore_ascii_case("FAIL") {
            return Err("同厂家、商品名和销售规格的药品商品已存在".into());
        }
        tx.commit()
            .await
            .map_err(|error| pg_error("migration_row", "提交逐行事务", error))?;
        events.push(WriteEvent { operation: "SKIP", table: "hi_bd_med_pro", target_id: id_med_pro.clone(), message: "目标商品已存在，本行幂等跳过".into(), before: json!({"idMedPro":id_med_pro,"matchedBy":matched_by,"businessKey":{"cdMedPro":external_code,"idFac":id_fac,"naMedPro":product_name,"specSale":sale_spec}}), after: json!({"idMedPro":id_med_pro}) });
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
    insert_product(
        &mut tx,
        &id_med_pro,
        &id_med,
        &id_fac,
        &id_med_unit,
        data,
        &product_name,
        &sale_spec,
        &fg_pri,
        tenant_id,
        operator_id,
        organization_id,
        now,
    )
    .await?;
    tx.commit()
        .await
        .map_err(|error| pg_error("migration_row", "提交逐行事务", error))?;
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

#[allow(clippy::too_many_arguments)]
async fn insert_medicine(
    tx: &mut PgTransaction<'_>,
    id: &str,
    data: &Map<String, Value>,
    name: &str,
    med_type: &str,
    spec: &str,
    unit_pre: &str,
    fg_pri: &str,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
    now: chrono::NaiveDateTime,
) -> Result<(), String> {
    let dft_dose_once = if text(data, "dftDoseOnce").is_empty() && med_type == "3" {
        text(data, "dose")
    } else {
        text(data, "dftDoseOnce")
    };
    let dose = optional_decimal(data, "dose")?;
    let ddd = optional_decimal(data, "ddd")?;
    let limit_anti_day = optional_decimal(data, "limitAntiDay")?;
    let drip_rate = optional_decimal(data, "dripRate")?;
    let dft_dose_once = if dft_dose_once.trim().is_empty() {
        None
    } else {
        Some(
            dft_dose_once
                .parse::<rust_decimal::Decimal>()
                .map_err(|_| format!("字段 dftDoseOnce 不是合法数值：{dft_dose_once}"))?,
        )
    };
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"INSERT INTO hi_bd_med(
        id_med,na_med,sd_med,id_cstmg,unit_pre,spec,dose,unit_dose,sd_dose,sd_dose_unit,
        sd_chrgitm_lv,sd_allergy,sd_anti_acl,fg_anti_appr,ddd,sd_bas_med,sd_spe_med,
        limit_anti_day,sd_storage,sd_pharm,sd_value,sd_prod_plac,fg_pois,sd_pois,fg_anti,
        sd_anti,sd_round,sd_dps,fg_med_rx,fg_bas_med,fg_skintest,sd_skintest,drip_rate,
        dft_dose_once,dft_usage,dft_freq,fg_tcd,fg_single,fg_register,fg_active,fg_pri,
        id_org_pri,id_tet,revision,insert_user,insert_time) "#,
    );
    builder.push_values(std::iter::once(()), |mut row, _| {
        row.push_bind(id)
            .push_bind(name)
            .push_bind(med_type)
            .push_bind(text(data, "idCstmg"))
            .push_bind(unit_pre)
            .push_bind(spec)
            .push_bind(dose)
            .push_bind(text(data, "unitDose"))
            .push_bind(text(data, "sdDose"))
            .push_bind(text(data, "sdDoseUnit"))
            .push_bind(text(data, "sdChrgitmLv"))
            .push_bind(text(data, "sdAllergy"))
            .push_bind(text(data, "sdAntiAcl"))
            .push_bind(defaulted(data, "fgAntiAppr", "0"))
            .push_bind(ddd)
            .push_bind(text(data, "sdBasMed"))
            .push_bind(text(data, "sdSpeMed"))
            .push_bind(limit_anti_day)
            .push_bind(text(data, "sdStorage"))
            .push_bind(text(data, "sdPharm"))
            .push_bind(text(data, "sdValue"))
            .push_bind(text(data, "sdProdPlac"))
            .push_bind(defaulted(data, "fgPois", "0"))
            .push_bind(text(data, "sdPois"))
            .push_bind(defaulted(data, "fgAnti", "0"))
            .push_bind(text(data, "sdAnti"))
            .push_bind(text(data, "sdRound"))
            .push_bind(text(data, "sdDps"))
            .push_bind(defaulted(data, "fgMedRx", "2"))
            .push_bind(defaulted(data, "fgBasMed", "0"))
            .push_bind(defaulted(data, "fgSkintest", "0"))
            .push_bind(text(data, "sdSkintest"))
            .push_bind(drip_rate)
            .push_bind(dft_dose_once)
            .push_bind(text(data, "dftUsage"))
            .push_bind(text(data, "dftFreq"))
            .push_bind(defaulted(data, "fgTcd", "0"))
            .push_bind(defaulted(data, "fgSingle", "1"))
            .push_bind(defaulted(data, "fgRegister", "0"))
            .push_bind("1")
            .push_bind(fg_pri)
            .push_bind((fg_pri == "1").then_some(organization_id))
            .push_bind(tenant_id)
            .push_bind(0_i64)
            .push_bind(operator_id)
            .push_bind(now);
    });
    builder
        .build()
        .execute(&mut **tx)
        .await
        .map_err(|error| pg_error("hi_bd_med", "新增药品基本信息", error))?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn insert_product(
    tx: &mut PgTransaction<'_>,
    id_med_pro: &str,
    id_med: &str,
    id_fac: &str,
    id_med_unit: &str,
    data: &Map<String, Value>,
    product_name: &str,
    sale_spec: &str,
    fg_pri: &str,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
    now: chrono::NaiveDateTime,
) -> Result<(), String> {
    let price_sale = decimal(data, "priceSale")?;
    let price_pur = decimal(data, "pricePur")?;
    let unit_sale_factor = integer(data, "unitSaleFactor")?;
    let per = optional_decimal(data, "per")?;
    let mut builder = QueryBuilder::<Postgres>::new(
        r#"INSERT INTO hi_bd_med_pro(
        id_med_pro,id_med,id_fac,id_med_unit,unit_sale,na_med_pro,spec_sale,price_sale,price_pur,
        unit_sale_factor,cd_appr,cd_bar,cd_med_pro,id_tet,revision,insert_user,insert_time,
        fg_active,fg_pri,id_org_pri,sd_per,per,fg_coll_pur,fg_import) "#,
    );
    builder.push_values(std::iter::once(()), |mut row, _| {
        row.push_bind(id_med_pro)
            .push_bind(id_med)
            .push_bind(id_fac)
            .push_bind(id_med_unit)
            .push_bind(text(data, "unitSale"))
            .push_bind(product_name)
            .push_bind(sale_spec)
            .push_bind(price_sale)
            .push_bind(price_pur)
            .push_bind(unit_sale_factor)
            .push_bind(text(data, "cdAppr"))
            .push_bind(text(data, "cdBar"))
            .push_bind(text(data, "cdMedPro"))
            .push_bind(tenant_id)
            .push_bind(0_i64)
            .push_bind(operator_id)
            .push_bind(now)
            .push_bind("1")
            .push_bind(fg_pri)
            .push_bind((fg_pri == "1").then_some(organization_id))
            .push_bind(text(data, "sdPer"))
            .push_bind(per)
            .push_bind(defaulted(data, "fgCollPur", "0"))
            .push_bind(defaulted(data, "fgImport", "1"));
    });
    builder
        .build()
        .execute(&mut **tx)
        .await
        .map_err(|error| pg_error("hi_bd_med_pro", "新增药品商品信息", error))?;
    Ok(())
}

async fn ensure_alias(
    tx: &mut PgTransaction<'_>,
    id_med: &str,
    name: &str,
    tenant_id: &str,
    fg_pri: &str,
    organization_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<(), String> {
    let exists = query_scalar::<Postgres, String>("SELECT id_med_alias FROM hi_bd_med_alias WHERE id_tet=$1 AND id_med=$2 AND na_alias=$3 AND fg_main='1' AND fg_active='1' LIMIT 1")
        .bind(tenant_id).bind(id_med).bind(name).fetch_optional(&mut **tx).await
        .map_err(|error| pg_error("hi_bd_med_alias", "查找药品主别名", error))?;
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
        query::<Postgres>("INSERT INTO hi_bd_med_alias(id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active,fg_pri,id_org) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11)")
            .bind(&id).bind(id_med).bind(name).bind("1").bind("").bind("").bind(name).bind(tenant_id).bind("1").bind(fg_pri)
            .bind((fg_pri == "1").then_some(organization_id)).execute(&mut **tx).await
            .map_err(|error| pg_error("hi_bd_med_alias", "新增药品主别名", error))?;
        events.push(WriteEvent {
            operation: "INSERT",
            table: "hi_bd_med_alias",
            target_id: id,
            message: "新增药品主别名；拼音/五笔码可由新系统后续补齐".into(),
            before: Value::Null,
            after: json!({"idMed":id_med,"naAlias":name}),
        });
    }
    Ok(())
}

async fn ensure_unit(
    tx: &mut PgTransaction<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let unit = defaulted(data, "unitSale", &text(data, "unitPre"));
    let factor = if text(data, "unitSaleFactor").is_empty() {
        1
    } else {
        integer(data, "unitSaleFactor")?
    };
    let exists = query_scalar::<Postgres, String>("SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=$1 AND id_med=$2 AND na_unit=$3 AND unit_factor=$4 LIMIT 1")
        .bind(tenant_id).bind(id_med).bind(&unit).bind(factor).fetch_optional(&mut **tx).await
        .map_err(|error| pg_error("hi_bd_med_unit", "查找药品包装单位", error))?;
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
    query::<Postgres>("INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES ($1,$2,$3,$4,$5)")
        .bind(&id).bind(id_med).bind(&unit).bind(factor).bind(tenant_id).execute(&mut **tx).await
        .map_err(|error| pg_error("hi_bd_med_unit", "新增药品包装单位", error))?;
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

async fn ensure_factory(
    tx: &mut PgTransaction<'_>,
    data: &Map<String, Value>,
    allow_create: bool,
    tenant_id: &str,
    operator_id: &str,
    now: chrono::NaiveDateTime,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let requested_id = text(data, "idFac");
    if !requested_id.is_empty() {
        let id = query_scalar::<Postgres, String>(
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=$1 AND id_tet=$2 AND fg_active='1' LIMIT 1",
        )
        .bind(&requested_id)
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| pg_error("hi_bd_fac", "按主键查找生产厂家", error))?
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
        if let Some(id) = query_scalar::<Postgres, String>(
            "SELECT id_fac FROM hi_bd_fac WHERE id_fac=$1 AND id_tet=$2 AND fg_active='1' LIMIT 1",
        )
        .bind(&mapped_id)
        .bind(tenant_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| pg_error("hi_bd_fac", "按来源台账查找生产厂家", error))?
        {
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
    if let Some(id) = query_scalar::<Postgres, String>(
        "SELECT id_fac FROM hi_bd_fac WHERE id_tet=$1 AND na_fac=$2 AND fg_active='1' LIMIT 1",
    )
    .bind(tenant_id)
    .bind(&name)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| pg_error("hi_bd_fac", "按名称查找生产厂家", error))?
    {
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
    query::<Postgres>("INSERT INTO hi_bd_fac(id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,id_tet,revision,insert_user,insert_time,sd_fac) VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13)")
        .bind(&id).bind(&name).bind(limit(&short_name,32)).bind(defaulted(data,"sdProdPlac","1")).bind(&pinyin).bind("").bind(&name).bind("1")
        .bind(tenant_id).bind(0_i64).bind(operator_id).bind(now).bind("1").execute(&mut **tx).await
        .map_err(|error| pg_error("hi_bd_fac", "新增生产厂家", error))?;
    events.push(WriteEvent { operation:"INSERT", table:"hi_bd_fac", target_id:id.clone(), message:if source_factory_key.is_empty(){"按迁移策略新增生产厂家".into()}else{format!("迁移二系列phis厂家基础数据（YPCD={source_factory_key}）")}, before:Value::Null, after:json!({"idFac":id,"naFac":name,"naFacShort":short_name,"py":pinyin,"sourceFactoryKey":source_factory_key}) });
    Ok(id)
}

fn pg_error(table: &str, stage: &str, error: sqlx_core::Error) -> String {
    let code = error
        .as_database_error()
        .and_then(|item| item.code())
        .map(|value| value.into_owned());
    let code_text = code
        .as_deref()
        .map(|value| format!("，SQLSTATE {value}"))
        .unwrap_or_default();
    format!("目标表 {table} 在“{stage}”阶段失败{code_text}：{error}")
}

fn sqlstate_from_error(error: &str) -> Option<String> {
    let offset = error.find("SQLSTATE ")? + "SQLSTATE ".len();
    let code = error[offset..].chars().take(5).collect::<String>();
    (code.len() == 5).then_some(code)
}

fn failed_table(error: &str) -> &str {
    [
        "hi_bd_med_pro",
        "hi_bd_med_alias",
        "hi_bd_med_unit",
        "hi_bd_fac",
        "hi_bd_med",
    ]
    .into_iter()
    .find(|table| error.contains(table))
    .unwrap_or("migration_row")
}

#[cfg(test)]
mod tests {
    use super::sqlstate_from_error;

    #[test]
    fn pg_failure_keeps_the_sqlstate_for_retry_diagnostics() {
        assert_eq!(
            sqlstate_from_error("目标表 hi_bd_med 在新增阶段失败，SQLSTATE 23505：duplicate key"),
            Some("23505".into())
        );
    }
}
