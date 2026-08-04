use crate::datasource::connect_mysql;
use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{
    BatchDetail, ConnectionProfile, ExecuteBatchRequest, MigrationRow, TargetReadiness,
};
use crate::normalize::value_text;
use crate::target_contract::{validate_execution_context, TARGET_TABLE_PROJECTIONS};
use chrono::Utc;
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};
use sqlx_core::query::query;
use sqlx_core::query_scalar::query_scalar;
use sqlx_mysql::{MySql, MySqlPool, MySqlTransaction};

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

pub async fn inspect_schema(profile: &ConnectionProfile) -> Result<TargetReadiness, String> {
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

pub async fn execute_batch(
    store: &LocalStore,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    if crate::odbc::is_odbc_kind(&request.target.kind) {
        return crate::target_odbc::execute_batch(store, request);
    }
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.status == "RUNNING" {
        return Err("该迁移批次正在执行，请勿重复提交".into());
    }
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
        json!({"failedOnly":request.failed_only,"databaseKind":request.target.kind}),
        "开始执行迁移批次",
        &request.operator_id,
        &batch_trace,
    )?;

    for mut row in detail.rows {
        let executable = if request.failed_only {
            row.status == "FAILED"
        } else {
            row.status == "VALIDATED" || row.status == "FAILED"
        };
        if !executable {
            continue;
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

async fn write_row(
    pool: &MySqlPool,
    row: &MigrationRow,
    conflict_strategy: &str,
    allow_create_factory: bool,
    tenant_id: &str,
    operator_id: &str,
    organization_id: &str,
) -> Result<WriteOutcome, String> {
    let data = &row.normalized_data;
    let now = Utc::now().naive_utc();
    let mut tx = pool.begin().await.map_err(db_error)?;
    let mut events = Vec::new();

    let name = text(data, "naMed");
    let med_type = text(data, "sdMed");
    let spec = derived_spec(data);
    let fg_pri = defaulted(data, "fgPri", "0");
    validate_execution_context(tenant_id, operator_id, organization_id, fg_pri == "1")?;
    let existing_med = if fg_pri == "1" {
        query_scalar::<MySql, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND sd_med=? AND COALESCE(spec,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id).bind(&name).bind(&med_type).bind(&spec).bind(organization_id)
        .fetch_optional(&mut *tx).await.map_err(db_error)?
    } else {
        query_scalar::<MySql, String>(
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND sd_med=? AND COALESCE(spec,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL) LIMIT 1",
        )
        .bind(tenant_id).bind(&name).bind(&med_type).bind(&spec)
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
            message: "复用已存在的药品基本信息".into(),
            before: json!({"idMed":id,"businessKey":{"naMed":name,"sdMed":med_type,"spec":spec}}),
            after: json!({"idMed":id,"naMed":name,"spec":spec}),
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
        .bind(text(data, "unitPre"))
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
        .bind(defaulted(data, "fgMedRx", "0"))
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
        tx.commit().await.map_err(db_error)?;
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
    tx.commit().await.map_err(db_error)?;
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

async fn ensure_alias(
    tx: &mut MySqlTransaction<'_>,
    id_med: &str,
    name: &str,
    tenant_id: &str,
    fg_pri: &str,
    organization_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<(), String> {
    let exists: Option<String> = query_scalar::<MySql, String>(
        "SELECT id_med_alias FROM hi_bd_med_alias WHERE id_tet=? AND id_med=? AND na_alias=? AND fg_main='1' AND fg_active='1' LIMIT 1"
    ).bind(tenant_id).bind(id_med).bind(name).fetch_optional(&mut **tx).await.map_err(db_error)?;
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
        query::<MySql>("INSERT INTO hi_bd_med_alias(id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active,fg_pri,id_org) VALUES (?,?,?,?,?,?,?,?,?,?,?)")
            .bind(&id).bind(id_med).bind(name).bind("1").bind("").bind("")
            .bind(name).bind(tenant_id).bind("1").bind(fg_pri)
            .bind((fg_pri == "1").then_some(organization_id))
            .execute(&mut **tx).await.map_err(db_error)?;
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
    tx: &mut MySqlTransaction<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let unit = text(data, "unitSale");
    let factor = integer(data, "unitSaleFactor")?.to_string();
    let exists: Option<String> = query_scalar::<MySql, String>(
        "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=? AND id_med=? AND na_unit=? AND unit_factor=? LIMIT 1"
    ).bind(tenant_id).bind(id_med).bind(&unit).bind(&factor).fetch_optional(&mut **tx).await.map_err(db_error)?;
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
    query::<MySql>("INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES (?,?,?,?,?)")
        .bind(&id).bind(id_med).bind(&unit).bind(&factor).bind(tenant_id)
        .execute(&mut **tx).await.map_err(db_error)?;
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
    let name = text(data, "naFac");
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
    query::<MySql>(
        r#"INSERT INTO hi_bd_fac(id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,
        id_tet,revision,insert_user,insert_time,sd_fac) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
    )
    .bind(&id)
    .bind(&name)
    .bind(limit(&name, 32))
    .bind(defaulted(data, "sdProdPlac", "1"))
    .bind("")
    .bind("")
    .bind(&name)
    .bind("1")
    .bind(tenant_id)
    .bind("0")
    .bind(operator_id)
    .bind(now)
    .bind("1")
    .execute(&mut **tx)
    .await
    .map_err(db_error)?;
    events.push(WriteEvent {
        operation: "INSERT",
        table: "hi_bd_fac",
        target_id: id.clone(),
        message: "按迁移策略新增生产厂家".into(),
        before: Value::Null,
        after: json!({"idFac":id,"naFac":name}),
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
fn integer(data: &Map<String, Value>, key: &str) -> Result<i64, String> {
    text(data, key)
        .parse::<i64>()
        .map_err(|_| format!("{}必须是整数", key))
}
fn decimal(data: &Map<String, Value>, key: &str) -> Result<Decimal, String> {
    text(data, key)
        .parse::<Decimal>()
        .map_err(|_| format!("{}必须是数字", key))
}
fn optional_decimal(data: &Map<String, Value>, key: &str) -> Result<Option<Decimal>, String> {
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
fn db_error(error: sqlx_core::Error) -> String {
    format!("目标数据库写入失败：{}", error)
}
fn limit(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
