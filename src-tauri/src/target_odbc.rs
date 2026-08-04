use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{BatchDetail, ExecuteBatchRequest, MigrationRow};
use crate::normalize::value_text;
use crate::odbc::{
    configure_target_session, execute_strings, query_optional_string, with_connection,
};
use crate::target_contract::validate_execution_context;
use chrono::Utc;
use odbc_api::Connection;
use rust_decimal::Decimal;
use serde_json::{json, Map, Value};

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

pub fn execute_batch(
    store: &LocalStore,
    request: ExecuteBatchRequest,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.status == "RUNNING" {
        return Err("该迁移批次正在执行，请勿重复提交".into());
    }
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
            json!({"failedOnly":request.failed_only,"databaseKind":request.target.kind}),
            "开始执行企业数据库迁移批次",
            &request.operator_id,
            &new_object_id(),
        )?;

        for mut row in detail.rows.clone() {
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

fn record_failure(
    store: &LocalStore,
    request: &ExecuteBatchRequest,
    row: &mut MigrationRow,
    error: String,
    trace_id: &str,
) -> Result<(), String> {
    row.status = "FAILED".into();
    row.error_code = "WRITE_ERROR".into();
    row.error_message = limit(&error, 2000);
    row.retry_count += 1;
    row.updated_at = Utc::now().to_rfc3339();
    store.update_row_result(row)?;
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
    let now = Utc::now().format("%Y-%m-%d %H:%M:%S%.3f").to_string();
    let mut events = Vec::new();
    let name = text(data, "naMed");
    let med_type = text(data, "sdMed");
    let spec = derived_spec(data);
    let fg_pri = defaulted(data, "fgPri", "0");
    validate_execution_context(tenant_id, operator_id, organization_id, fg_pri == "1")?;
    let existing_med = if fg_pri == "1" {
        query_optional_string(
            connection,
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND sd_med=? AND COALESCE(spec,'')=? AND fg_active='1' AND ((fg_pri='1' AND id_org_pri=?) OR fg_pri='0' OR fg_pri IS NULL)",
            vec![tenant_id.into(), name.clone(), med_type.clone(), spec.clone(), organization_id.into()],
        )?
    } else {
        query_optional_string(
            connection,
            "SELECT id_med FROM hi_bd_med WHERE id_tet=? AND na_med=? AND sd_med=? AND COALESCE(spec,'')=? AND fg_active='1' AND (fg_pri<>'1' OR fg_pri IS NULL)",
            vec![tenant_id.into(), name.clone(), med_type.clone(), spec.clone()],
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
        execute_strings(
            connection,
            r#"INSERT INTO hi_bd_med(
                id_med,na_med,sd_med,id_cstmg,unit_pre,spec,dose,unit_dose,sd_dose,sd_dose_unit,
                sd_chrgitm_lv,sd_allergy,sd_anti_acl,fg_anti_appr,ddd,sd_bas_med,sd_spe_med,
                limit_anti_day,sd_storage,sd_pharm,sd_value,sd_prod_plac,fg_pois,sd_pois,fg_anti,
                sd_anti,sd_round,sd_dps,fg_med_rx,fg_bas_med,fg_skintest,sd_skintest,drip_rate,
                dft_dose_once,dft_usage,dft_freq,fg_tcd,fg_single,fg_register,fg_active,fg_pri,
                id_org_pri,id_tet,revision,insert_user,insert_time
            ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULLIF(?,''),?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULLIF(?,''),?,?,?,?)"#,
            vec![
                id.clone(),
                name.clone(),
                med_type.clone(),
                text(data, "idCstmg"),
                text(data, "unitPre"),
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
                defaulted(data, "fgMedRx", "0"),
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
                now.clone(),
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

    ensure_alias(
        connection,
        &id_med,
        &name,
        tenant_id,
        &fg_pri,
        organization_id,
        &mut events,
    )?;
    let id_med_unit = ensure_unit(connection, &id_med, data, tenant_id, &mut events)?;
    let id_fac = ensure_factory(
        connection,
        data,
        allow_create_factory,
        tenant_id,
        operator_id,
        &now,
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
    execute_strings(
        connection,
        r#"INSERT INTO hi_bd_med_pro(
            id_med_pro,id_med,id_fac,id_med_unit,unit_sale,na_med_pro,spec_sale,price_sale,price_pur,
            unit_sale_factor,cd_appr,cd_bar,cd_med_pro,id_tet,revision,insert_user,insert_time,
            fg_active,fg_pri,id_org_pri,sd_per,per,fg_coll_pur,fg_import
        ) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,?,NULLIF(?,''),?,NULLIF(?,''),?,?)"#,
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
            now,
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

fn ensure_alias(
    connection: &Connection<'_>,
    id_med: &str,
    name: &str,
    tenant_id: &str,
    fg_pri: &str,
    organization_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<(), String> {
    let exists = query_optional_string(
        connection,
        "SELECT id_med_alias FROM hi_bd_med_alias WHERE id_tet=? AND id_med=? AND na_alias=? AND fg_main='1' AND fg_active='1'",
        vec![tenant_id.into(), id_med.into(), name.into()],
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
        execute_strings(
            connection,
            "INSERT INTO hi_bd_med_alias(id_med_alias,id_med,na_alias,fg_main,py,wb,instr,id_tet,fg_active,fg_pri,id_org) VALUES (?,?,?,?,?,?,?,?,?,?,NULLIF(?,''))",
            vec![id.clone(), id_med.into(), name.into(), "1".into(), String::new(), String::new(), name.into(), tenant_id.into(), "1".into(), fg_pri.into(), if fg_pri == "1" { organization_id.into() } else { String::new() }],
        )?;
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

fn ensure_unit(
    connection: &Connection<'_>,
    id_med: &str,
    data: &Map<String, Value>,
    tenant_id: &str,
    events: &mut Vec<WriteEvent>,
) -> Result<String, String> {
    let unit = text(data, "unitSale");
    let factor = integer(data, "unitSaleFactor")?;
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
    execute_strings(
        connection,
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
    now: &str,
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
    let name = text(data, "naFac");
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
    execute_strings(
        connection,
        r#"INSERT INTO hi_bd_fac(id_fac,na_fac,na_fac_short,sd_prod_plac,py,wb,instr,fg_active,
            id_tet,revision,insert_user,insert_time,sd_fac) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?)"#,
        vec![
            id.clone(),
            name.clone(),
            limit(&name, 32),
            defaulted(data, "sdProdPlac", "1"),
            String::new(),
            String::new(),
            name.clone(),
            "1".into(),
            tenant_id.into(),
            "0".into(),
            operator_id.into(),
            now.into(),
            "1".into(),
        ],
    )?;
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
fn limit(text: &str, max: usize) -> String {
    text.chars().take(max).collect()
}
