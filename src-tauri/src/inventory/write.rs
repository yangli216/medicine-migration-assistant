fn execute_odbc_groups(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    detail: &BatchDetail,
    request: &ExecuteInventoryRequest,
    groups: BTreeMap<(String, String), Vec<MigrationRow>>,
) -> Result<(), String> {
    let target_identity_text = target_identity(&request.target).to_string();
    with_connection(&request.target, |connection| {
        configure_target_session(connection, &request.target)?;
        validate_inventory_schema_odbc(connection)?;
        connection
            .set_autocommit(false)
            .map_err(|error| format!("开启库存事务失败：{error}"))?;
        for ((id_org, id_sto), rows) in groups {
            let trace_id = new_object_id();
            let date_parameter_sql = inventory_date_parameter_sql(&request.target.kind);
            match write_storage_odbc(
                connection,
                tenant_id,
                operator_id,
                &request.batch_id,
                &id_org,
                &id_sto,
                &rows,
                date_parameter_sql,
            ) {
                Ok(outcome) => {
                    if let Err(error) = connection.commit() {
                        let _ = connection.rollback();
                        record_inventory_group_failure(
                            store,
                            &request.batch_id,
                            operator_id,
                            &trace_id,
                            rows,
                            format!("提交库房首次盘点事务失败：{error}"),
                        )?;
                        continue;
                    }
                    record_inventory_group_success(
                        store,
                        tenant_id,
                        operator_id,
                        detail,
                        &target_identity_text,
                        &trace_id,
                        &id_sto,
                        rows,
                        outcome,
                    )?;
                }
                Err(error) => {
                    let rollback = connection.rollback().err().map(|value| value.to_string());
                    let message = rollback
                        .map(|rollback| format!("{error}；回滚异常：{rollback}"))
                        .unwrap_or(error);
                    record_inventory_group_failure(
                        store,
                        &request.batch_id,
                        operator_id,
                        &trace_id,
                        rows,
                        message,
                    )?;
                }
            }
        }
        connection
            .set_autocommit(true)
            .map_err(|error| format!("恢复数据库自动提交失败：{error}"))?;
        Ok(())
    })
}

fn inventory_date_parameter_sql(kind: &str) -> &'static str {
    match crate::odbc::normalize_kind(kind).as_str() {
        "oracle" | "dameng" => "TO_DATE(NULLIF(?,''),'YYYY-MM-DD')",
        "gbase8s" => "TO_DATE(NULLIF(?,''),'%Y-%m-%d')",
        _ => "CAST(NULLIF(?,'') AS DATE)",
    }
}

fn validate_inventory_schema_odbc(connection: &Connection<'_>) -> Result<(), String> {
    for (table, columns) in INVENTORY_TARGET_TABLE_PROJECTIONS {
        connection
            .execute(
                &format!("SELECT {columns} FROM {table} WHERE 1=0"),
                (),
                Some(30),
            )
            .map_err(|error| format!("首次盘点目标表结构检查失败（{table}）：{error}"))?;
    }
    Ok(())
}

async fn validate_inventory_schema_mysql(pool: &MySqlPool) -> Result<(), String> {
    for (table, columns) in INVENTORY_TARGET_TABLE_PROJECTIONS {
        query::<MySql>(&format!("SELECT {columns} FROM {table} WHERE 1=0"))
            .fetch_optional(pool)
            .await
            .map_err(|error| {
                format!(
                    "首次盘点目标表结构检查失败（{table}）：{error}。请核对目标系统版本和字段权限"
                )
            })?;
    }
    Ok(())
}

fn ensure_inventory_unit_odbc(
    connection: &Connection<'_>,
    tenant_id: &str,
    id_med: &str,
    unit: &str,
    factor: &str,
) -> Result<String, String> {
    if let Some(id) = query_optional_string(
        connection,
        "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=? AND id_med=? AND na_unit=? AND unit_factor=?",
        vec![tenant_id.into(), id_med.into(), unit.into(), factor.into()],
    )? {
        return Ok(id);
    }
    let id = new_object_id();
    execute_strings(
        connection,
        "INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES(?,?,?,?,?)",
        vec![id.clone(), id_med.into(), unit.into(), factor.into(), tenant_id.into()],
    )
    .map_err(|error| inventory_write_error("hi_bd_med_unit", "建立库房包装单位", error))?;
    Ok(id)
}

async fn ensure_inventory_unit_mysql(
    tx: &mut MySqlTransaction<'_>,
    tenant_id: &str,
    id_med: &str,
    unit: &str,
    factor: i64,
) -> Result<String, String> {
    if let Some(id) = query_scalar::<MySql, String>(
        "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=? AND id_med=? AND na_unit=? AND unit_factor=? LIMIT 1",
    )
    .bind(tenant_id)
    .bind(id_med)
    .bind(unit)
    .bind(factor)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_bd_med_unit", "查找库房包装单位", error.to_string()))?
    {
        return Ok(id);
    }
    let id = new_object_id();
    query::<MySql>(
        "INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES(?,?,?,?,?)",
    )
    .bind(&id)
    .bind(id_med)
    .bind(unit)
    .bind(factor)
    .bind(tenant_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_bd_med_unit", "建立库房包装单位", error.to_string()))?;
    Ok(id)
}

#[allow(clippy::too_many_arguments)]
fn write_storage_odbc(
    connection: &Connection<'_>,
    tenant_id: &str,
    operator_id: &str,
    batch_id: &str,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
    date_parameter_sql: &str,
) -> Result<StorageWriteResult, String> {
    assert_storage_is_empty_odbc(connection, tenant_id, id_org, id_sto)?;
    let prefix = Local::now().format("%Y%m%d").to_string();
    let latest = query_optional_string(
        connection,
        "SELECT MAX(cd_sto_check) FROM hi_sto_check WHERE id_tet=? AND id_sto=? AND cd_sto_check LIKE ? AND LENGTH(cd_sto_check)=11",
        vec![tenant_id.into(), id_sto.into(), format!("{prefix}%")],
    )?;
    let cd_sto_check = next_check_number(&prefix, latest.as_deref())?;
    let id_sto_check = new_object_id();
    execute_strings(
        connection,
        "INSERT INTO hi_sto_check(id_sto_check,id_sto,cd_sto_check,dt_check_begin,dt_check_end,fg_sto_check,sd_pol,sd_check,des_sto_check,id_org,id_tet,revision,insert_user,insert_time) VALUES(?,?,?,CURRENT_TIMESTAMP,NULL,'0','1','1',?,?,?,?,?,CURRENT_TIMESTAMP)",
        vec![
            id_sto_check.clone(), id_sto.into(), cd_sto_check.clone(),
            format!("数据迁移助手首次盘点，来源二系列phis，批次{batch_id}"),
            id_org.into(), tenant_id.into(), "0".into(), operator_id.into(),
        ],
    )
    .map_err(|error| inventory_write_error("hi_sto_check", "创建首次盘点主单", error))?;

    let mut outcomes = Vec::with_capacity(rows.len());
    for row in rows {
        let id_med = required_normalized(row, "idMed")?;
        let id_med_pro = required_normalized(row, "idMedPro")?;
        let amount = required_decimal(row, "amount")?;
        let price_pur = required_decimal(row, "pricePur")?;
        let price_sale = required_decimal(row, "priceSale")?;
        let batch_code = normalized_text(row, "batchCode");
        let effective_date = normalized_text(row, "effectiveDate");
        validate_effective_date(&effective_date)?;
        let product = query_optional_row_strings(
            connection,
            "SELECT id_med FROM hi_bd_med_pro WHERE id_tet=? AND id_med_pro=? AND fg_active='1'",
            vec![tenant_id.into(), id_med_pro.clone()],
        )?
        .ok_or_else(|| format!("药品商品 {id_med_pro} 在目标租户中不存在或已停用"))?;
        let product_text = |index: usize| {
            product
                .get(index)
                .and_then(Clone::clone)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        if product_text(0) != id_med {
            return Err(format!(
                "药品商品 {id_med_pro} 的 id_med 与基础迁移台账不一致"
            ));
        }
        let unit_sale = required_normalized(row, "unitSale")?;
        let spec_sale = normalized_text(row, "specSale");
        let unit_sale_factor = required_normalized(row, "unitSaleFactor")?
            .parse::<i64>()
            .ok()
            .filter(|factor| *factor > 0)
            .ok_or_else(|| format!("库存行 {} 的库房包装系数无效", row.row_no))?
            .to_string();
        let id_med_unit = ensure_inventory_unit_odbc(
            connection,
            tenant_id,
            &id_med,
            &unit_sale,
            &unit_sale_factor,
        )?;
        let existing_sto_med = query_optional_string(
            connection,
            "SELECT id_sto_med FROM hi_sto_med WHERE id_tet=? AND id_org=? AND id_sto=? AND id_med_pro=?",
            vec![tenant_id.into(), id_org.into(), id_sto.into(), id_med_pro.clone()],
        )?;
        let id_sto_med = existing_sto_med.unwrap_or_else(new_object_id);
        if query_optional_string(
            connection,
            "SELECT id_sto_med FROM hi_sto_med WHERE id_sto_med=?",
            vec![id_sto_med.clone()],
        )?
        .is_some()
        {
            execute_strings(
                connection,
                "UPDATE hi_sto_med SET id_med=?,id_med_unit=?,unit_sale=?,spec_sale=?,price_sale=?,price_pur=?,unit_sale_factor=?,fg_active='1' WHERE id_sto_med=?",
                vec![id_med.clone(),id_med_unit.clone(),unit_sale.clone(),spec_sale.clone(),price_sale.clone(),price_pur.clone(),unit_sale_factor.clone(),id_sto_med.clone()],
            ).map_err(|error| inventory_write_error("hi_sto_med", "更新库房药品属性", error))?;
        } else {
            execute_strings(
                connection,
                "INSERT INTO hi_sto_med(id_sto_med,id_med,id_med_pro,unit_sale,spec_sale,price_sale,price_pur,unit_sale_factor,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time,id_med_unit) VALUES(?,?,?,?,?,?,?,?,'1',?,?,?,'0',?,CURRENT_TIMESTAMP,?)",
                vec![id_sto_med.clone(),id_med.clone(),id_med_pro.clone(),unit_sale.clone(),spec_sale.clone(),price_sale.clone(),price_pur.clone(),unit_sale_factor.clone(),id_sto.into(),id_org.into(),tenant_id.into(),operator_id.into(),id_med_unit.clone()],
            ).map_err(|error| inventory_write_error("hi_sto_med", "建立库房药品属性", error))?;
        }

        let id_sto_inv = new_object_id();
        let id_check_sub = new_object_id();
        let id_inv_log = new_object_id();
        execute_strings(
            connection,
            &format!("INSERT INTO hi_sto_check_sub(id,id_sto_check,id_med_pro,id_sto_inv,cd_batch,dt_effect,amt_check_bgn,amt_check_end,amt_change,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur,unit_sale,unit_sale_factor) VALUES(?,?,?,?,?,{date_parameter_sql},0,?,?,?,?,'0',?,CURRENT_TIMESTAMP,?,?,?,?)"),
            vec![id_check_sub.clone(),id_sto_check.clone(),id_med_pro.clone(),id_sto_inv.clone(),batch_code.clone(),effective_date.clone(),amount.clone(),amount.clone(),id_org.into(),tenant_id.into(),operator_id.into(),price_sale.clone(),price_pur.clone(),unit_sale.clone(),unit_sale_factor.clone()],
        ).map_err(|error| inventory_write_error("hi_sto_check_sub", "写入首次盘点明细", error))?;
        execute_strings(
            connection,
            &format!("INSERT INTO hi_sto_inv(id_sto_inv,id_med_pro,amount,price_sale,price_pur,cd_batch,dt_effect,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time) VALUES(?,?,?,?,?,?,{date_parameter_sql},'1',?,?,?,'0',?,CURRENT_TIMESTAMP)"),
            vec![id_sto_inv.clone(),id_med_pro.clone(),amount.clone(),price_sale.clone(),price_pur.clone(),batch_code,effective_date,id_sto.into(),id_org.into(),tenant_id.into(),operator_id.into()],
        ).map_err(|error| inventory_write_error("hi_sto_inv", "建立初始库存", error))?;
        execute_strings(
            connection,
            "INSERT INTO hi_sto_inv_log(id_inv_log,id_sto_inv,id_med_pro,sd_amt_change,des_reason,id_biz_ori,amt_change,amt_before,amt_after,unit_sale,unit_sale_factor,id_sto,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur) VALUES(?,?,?,'100',?,?,?,0,?,?,?,?,?,?,'0',?,CURRENT_TIMESTAMP,?,?)",
            vec![id_inv_log.clone(),id_sto_inv.clone(),id_med_pro,"首次盘点建立初始账簿".into(),id_check_sub.clone(),amount.clone(),amount,unit_sale,unit_sale_factor,id_sto.into(),id_org.into(),tenant_id.into(),operator_id.into(),price_sale,price_pur],
        ).map_err(|error| inventory_write_error("hi_sto_inv_log", "记录初始账簿变动", error))?;
        outcomes.push(InventoryWriteResult {
            row_id: row.row_id.clone(),
            id_med_unit,
            id_sto_med,
            id_sto_inv,
            id_inv_log,
            id_check_sub,
        });
    }
    execute_strings(
        connection,
        "UPDATE hi_sto_check SET fg_sto_check='1',sd_pol='1',dt_check_end=CURRENT_TIMESTAMP WHERE id_sto_check=? AND fg_sto_check='0'",
        vec![id_sto_check.clone()],
    ).map_err(|error| inventory_write_error("hi_sto_check", "完成首次盘点", error))?;
    Ok(StorageWriteResult {
        id_sto_check,
        cd_sto_check,
        rows: outcomes,
    })
}

fn assert_storage_is_empty_odbc(
    connection: &Connection<'_>,
    tenant_id: &str,
    id_org: &str,
    id_sto: &str,
) -> Result<(), String> {
    let storage = query_optional_string(
        connection,
        "SELECT id_sto FROM hi_sto_dept WHERE id_tet=? AND id_org=? AND id_sto=? AND fg_active='1' AND sd_sto IN ('1','2') FOR UPDATE",
        vec![tenant_id.into(), id_org.into(), id_sto.into()],
    )?;
    if storage.is_none() {
        return Err("目标药库/药房已停用、不属于所选机构，或仓储类型不是药库/药房".into());
    }
    if query_count_odbc(
        connection,
        "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=? AND id_org=? AND id_sto=? AND sd_check='1' AND fg_sto_check<>'9'",
        vec![tenant_id.into(), id_org.into(), id_sto.into()],
    )? > 0 {
        return Err("目标库房已经存在完成或进行中的库存盘点，不允许再次建立初始账簿".into());
    }
    if query_count_odbc(
        connection,
        "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=? AND id_org=? AND id_sto=?",
        vec![tenant_id.into(), id_org.into(), id_sto.into()],
    )? > 0
    {
        return Err("目标库房已经存在库存数据，不允许执行首次盘点迁移".into());
    }
    Ok(())
}

fn query_count_odbc(
    connection: &Connection<'_>,
    statement: &str,
    values: Vec<String>,
) -> Result<i64, String> {
    query_optional_string(connection, statement, values)?
        .unwrap_or_default()
        .trim()
        .parse::<i64>()
        .map_err(|_| "目标数据库返回了无法识别的计数结果".into())
}

async fn execute_mysql_groups(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    detail: &BatchDetail,
    request: &ExecuteInventoryRequest,
    groups: BTreeMap<(String, String), Vec<MigrationRow>>,
) -> Result<(), String> {
    let pool = connect_mysql(&request.target).await?;
    validate_inventory_schema_mysql(&pool).await?;
    let target_identity_text = target_identity(&request.target).to_string();
    for ((id_org, id_sto), rows) in groups {
        let trace_id = new_object_id();
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("开启库存事务失败：{error}"))?;
        match write_storage_mysql(
            &mut tx,
            tenant_id,
            operator_id,
            &request.batch_id,
            &id_org,
            &id_sto,
            &rows,
        )
        .await
        {
            Ok(outcome) => {
                if let Err(error) = tx.commit().await {
                    record_inventory_group_failure(
                        store,
                        &request.batch_id,
                        operator_id,
                        &trace_id,
                        rows,
                        format!("提交库房首次盘点事务失败：{error}"),
                    )?;
                    continue;
                }
                record_inventory_group_success(
                    store,
                    tenant_id,
                    operator_id,
                    detail,
                    &target_identity_text,
                    &trace_id,
                    &id_sto,
                    rows,
                    outcome,
                )?;
            }
            Err(error) => {
                let rollback = tx.rollback().await.err().map(|value| value.to_string());
                let message = rollback
                    .map(|rollback| format!("{error}；回滚异常：{rollback}"))
                    .unwrap_or(error);
                record_inventory_group_failure(
                    store,
                    &request.batch_id,
                    operator_id,
                    &trace_id,
                    rows,
                    message,
                )?;
            }
        }
    }
    pool.close().await;
    Ok(())
}

async fn write_storage_mysql(
    tx: &mut MySqlTransaction<'_>,
    tenant_id: &str,
    operator_id: &str,
    batch_id: &str,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
) -> Result<StorageWriteResult, String> {
    let storage = query_scalar::<MySql, String>(
        "SELECT id_sto FROM hi_sto_dept WHERE id_tet=? AND id_org=? AND id_sto=? AND fg_active='1' AND sd_sto IN ('1','2') LIMIT 1 FOR UPDATE",
    )
    .bind(tenant_id)
    .bind(id_org)
    .bind(id_sto)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_dept", "确认目标库房", error.to_string()))?;
    if storage.is_none() {
        return Err("目标药库/药房已停用、不属于所选机构，或仓储类型不是药库/药房".into());
    }
    let check_count = query_scalar::<MySql, i64>(
        "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=? AND id_org=? AND id_sto=? AND sd_check='1' AND fg_sto_check<>'9'",
    )
    .bind(tenant_id)
    .bind(id_org)
    .bind(id_sto)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_check", "检查历史盘点", error.to_string()))?;
    if check_count > 0 {
        return Err("目标库房已经存在完成或进行中的库存盘点，不允许再次建立初始账簿".into());
    }
    let inventory_count = query_scalar::<MySql, i64>(
        "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=? AND id_org=? AND id_sto=?",
    )
    .bind(tenant_id)
    .bind(id_org)
    .bind(id_sto)
    .fetch_one(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_inv", "检查现有库存", error.to_string()))?;
    if inventory_count > 0 {
        return Err("目标库房已经存在库存数据，不允许执行首次盘点迁移".into());
    }
    let prefix = Local::now().format("%Y%m%d").to_string();
    let latest = query_scalar::<MySql, String>(
        "SELECT cd_sto_check FROM hi_sto_check WHERE id_tet=? AND id_sto=? AND cd_sto_check LIKE ? AND CHAR_LENGTH(cd_sto_check)=11 ORDER BY cd_sto_check DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(id_sto)
    .bind(format!("{prefix}%"))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_check", "生成盘点单号", error.to_string()))?;
    let cd_sto_check = next_check_number(&prefix, latest.as_deref())?;
    let id_sto_check = new_object_id();
    query::<MySql>(
        "INSERT INTO hi_sto_check(id_sto_check,id_sto,cd_sto_check,dt_check_begin,dt_check_end,fg_sto_check,sd_pol,sd_check,des_sto_check,id_org,id_tet,revision,insert_user,insert_time) VALUES(?,?,?,CURRENT_TIMESTAMP,NULL,'0','1','1',?,?,?,?,?,CURRENT_TIMESTAMP)",
    )
    .bind(&id_sto_check)
    .bind(id_sto)
    .bind(&cd_sto_check)
    .bind(format!("数据迁移助手首次盘点，来源二系列phis，批次{batch_id}"))
    .bind(id_org)
    .bind(tenant_id)
    .bind("0")
    .bind(operator_id)
    .execute(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_check", "创建首次盘点主单", error.to_string()))?;

    let mut outcomes = Vec::with_capacity(rows.len());
    for row in rows {
        let id_med = required_normalized(row, "idMed")?;
        let id_med_pro = required_normalized(row, "idMedPro")?;
        let amount = parse_decimal(&required_normalized(row, "amount")?, "库存数量")?;
        let price_pur = parse_decimal(&required_normalized(row, "pricePur")?, "进货价格")?;
        let price_sale = parse_decimal(&required_normalized(row, "priceSale")?, "零售价格")?;
        let batch_code = normalized_text(row, "batchCode");
        let effective_date = validate_effective_date(&normalized_text(row, "effectiveDate"))?;
        let product = query::<MySql>(
            "SELECT id_med FROM hi_bd_med_pro WHERE id_tet=? AND id_med_pro=? AND fg_active='1' LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&id_med_pro)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| inventory_write_error("hi_bd_med_pro", "读取商品属性", error.to_string()))?
        .ok_or_else(|| format!("药品商品 {id_med_pro} 在目标租户中不存在或已停用"))?;
        let product_id_med = product.try_get::<String, _>(0).unwrap_or_default();
        if product_id_med != id_med {
            return Err(format!(
                "药品商品 {id_med_pro} 的 id_med 与基础迁移台账不一致"
            ));
        }
        let unit_sale = required_normalized(row, "unitSale")?;
        let spec_sale = normalized_text(row, "specSale");
        let unit_sale_factor = required_normalized(row, "unitSaleFactor")?
            .parse::<i64>()
            .ok()
            .filter(|factor| *factor > 0)
            .ok_or_else(|| format!("库存行 {} 的库房包装系数无效", row.row_no))?;
        let id_med_unit =
            ensure_inventory_unit_mysql(tx, tenant_id, &id_med, &unit_sale, unit_sale_factor)
                .await?;
        let existing_sto_med = query_scalar::<MySql, String>(
            "SELECT id_sto_med FROM hi_sto_med WHERE id_tet=? AND id_org=? AND id_sto=? AND id_med_pro=? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(id_org)
        .bind(id_sto)
        .bind(&id_med_pro)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| inventory_write_error("hi_sto_med", "查找库房药品属性", error.to_string()))?;
        let id_sto_med = existing_sto_med.unwrap_or_else(new_object_id);
        if query_scalar::<MySql, i64>("SELECT COUNT(*) FROM hi_sto_med WHERE id_sto_med=?")
            .bind(&id_sto_med)
            .fetch_one(&mut **tx)
            .await
            .map_err(|error| {
                inventory_write_error("hi_sto_med", "确认库房药品属性", error.to_string())
            })?
            > 0
        {
            query::<MySql>("UPDATE hi_sto_med SET id_med=?,id_med_unit=?,unit_sale=?,spec_sale=?,price_sale=?,price_pur=?,unit_sale_factor=?,fg_active='1' WHERE id_sto_med=?")
                .bind(&id_med).bind(&id_med_unit).bind(&unit_sale).bind(&spec_sale)
                .bind(price_sale).bind(price_pur).bind(unit_sale_factor).bind(&id_sto_med)
                .execute(&mut **tx).await
                .map_err(|error| inventory_write_error("hi_sto_med", "更新库房药品属性", error.to_string()))?;
        } else {
            query::<MySql>("INSERT INTO hi_sto_med(id_sto_med,id_med,id_med_pro,unit_sale,spec_sale,price_sale,price_pur,unit_sale_factor,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time,id_med_unit) VALUES(?,?,?,?,?,?,?,?,'1',?,?,?,'0',?,CURRENT_TIMESTAMP,?)")
                .bind(&id_sto_med).bind(&id_med).bind(&id_med_pro).bind(&unit_sale).bind(&spec_sale)
                .bind(price_sale).bind(price_pur).bind(unit_sale_factor).bind(id_sto).bind(id_org)
                .bind(tenant_id).bind(operator_id).bind(&id_med_unit)
                .execute(&mut **tx).await
                .map_err(|error| inventory_write_error("hi_sto_med", "建立库房药品属性", error.to_string()))?;
        }
        let id_sto_inv = new_object_id();
        let id_check_sub = new_object_id();
        let id_inv_log = new_object_id();
        query::<MySql>("INSERT INTO hi_sto_check_sub(id,id_sto_check,id_med_pro,id_sto_inv,cd_batch,dt_effect,amt_check_bgn,amt_check_end,amt_change,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur,unit_sale,unit_sale_factor) VALUES(?,?,?,?,?,?,0,?,?,?,?,'0',?,CURRENT_TIMESTAMP,?,?,?,?)")
            .bind(&id_check_sub).bind(&id_sto_check).bind(&id_med_pro).bind(&id_sto_inv).bind(&batch_code)
            .bind(effective_date).bind(amount).bind(amount).bind(id_org).bind(tenant_id).bind(operator_id)
            .bind(price_sale).bind(price_pur).bind(&unit_sale).bind(unit_sale_factor)
            .execute(&mut **tx).await
            .map_err(|error| inventory_write_error("hi_sto_check_sub", "写入首次盘点明细", error.to_string()))?;
        query::<MySql>("INSERT INTO hi_sto_inv(id_sto_inv,id_med_pro,amount,price_sale,price_pur,cd_batch,dt_effect,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time) VALUES(?,?,?,?,?,?,?,'1',?,?,?,'0',?,CURRENT_TIMESTAMP)")
            .bind(&id_sto_inv).bind(&id_med_pro).bind(amount).bind(price_sale).bind(price_pur).bind(&batch_code)
            .bind(effective_date).bind(id_sto).bind(id_org).bind(tenant_id).bind(operator_id)
            .execute(&mut **tx).await
            .map_err(|error| inventory_write_error("hi_sto_inv", "建立初始库存", error.to_string()))?;
        query::<MySql>("INSERT INTO hi_sto_inv_log(id_inv_log,id_sto_inv,id_med_pro,sd_amt_change,des_reason,id_biz_ori,amt_change,amt_before,amt_after,unit_sale,unit_sale_factor,id_sto,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur) VALUES(?,?,?,'100',?,?,?,0,?,?,?,?,?,?,'0',?,CURRENT_TIMESTAMP,?,?)")
            .bind(&id_inv_log).bind(&id_sto_inv).bind(&id_med_pro).bind("首次盘点建立初始账簿")
            .bind(&id_check_sub).bind(amount).bind(amount).bind(&unit_sale).bind(unit_sale_factor)
            .bind(id_sto).bind(id_org).bind(tenant_id).bind(operator_id).bind(price_sale).bind(price_pur)
            .execute(&mut **tx).await
            .map_err(|error| inventory_write_error("hi_sto_inv_log", "记录初始账簿变动", error.to_string()))?;
        outcomes.push(InventoryWriteResult {
            row_id: row.row_id.clone(),
            id_med_unit,
            id_sto_med,
            id_sto_inv,
            id_inv_log,
            id_check_sub,
        });
    }
    query::<MySql>("UPDATE hi_sto_check SET fg_sto_check='1',sd_pol='1',dt_check_end=CURRENT_TIMESTAMP WHERE id_sto_check=? AND fg_sto_check='0'")
        .bind(&id_sto_check).execute(&mut **tx).await
        .map_err(|error| inventory_write_error("hi_sto_check", "完成首次盘点", error.to_string()))?;
    Ok(StorageWriteResult {
        id_sto_check,
        cd_sto_check,
        rows: outcomes,
    })
}

#[allow(clippy::too_many_arguments)]
fn record_inventory_group_success(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    detail: &BatchDetail,
    target_identity_text: &str,
    trace_id: &str,
    id_sto: &str,
    rows: Vec<MigrationRow>,
    outcome: StorageWriteResult,
) -> Result<(), String> {
    store.audit_event(
        &detail.batch.batch_id,
        "",
        "INSERT",
        "hi_sto_check",
        &outcome.id_sto_check,
        "SUCCESS",
        Value::Null,
        json!({"idSto":id_sto,"cdStoCheck":outcome.cd_sto_check,"fgStoCheck":"1","sdCheck":"1"}),
        &format!("首次盘点单 {} 已完成", outcome.cd_sto_check),
        operator_id,
        trace_id,
    )?;
    let result_by_row = outcome
        .rows
        .into_iter()
        .map(|item| (item.row_id.clone(), item))
        .collect::<HashMap<_, _>>();
    for mut row in rows {
        let result = result_by_row
            .get(&row.row_id)
            .ok_or_else(|| format!("库存行 {} 缺少事务写入结果", row.row_no))?;
        row.status = "SUCCESS".into();
        row.error_code.clear();
        row.error_message = format!("首次盘点 {} 建立初始账簿", outcome.cd_sto_check);
        row.updated_at = Utc::now().to_rfc3339();
        store.update_row_result(&row)?;
        store.record_inventory_link(
            tenant_id,
            &detail.batch.source_name,
            &row.source_key,
            target_identity_text,
            &row.source_hash,
            &detail.batch.batch_id,
            &row.row_id,
            id_sto,
            &result.id_sto_med,
            &result.id_sto_inv,
            &result.id_inv_log,
        )?;
        let after = json!({
            "cdStoCheck":outcome.cd_sto_check,
            "idStoCheck":outcome.id_sto_check,
            "idCheckSub":result.id_check_sub,
            "idMedUnit":result.id_med_unit,
            "idStoMed":result.id_sto_med,
            "idStoInv":result.id_sto_inv,
            "idInvLog":result.id_inv_log,
            "changeWay":"100",
            "amount":normalized_text(&row,"amount")
        });
        for (table, target_id, operation) in [
            ("hi_bd_med_unit", result.id_med_unit.as_str(), "ENSURE"),
            ("hi_sto_med", result.id_sto_med.as_str(), "ENSURE"),
            ("hi_sto_check_sub", result.id_check_sub.as_str(), "INSERT"),
            ("hi_sto_inv", result.id_sto_inv.as_str(), "INSERT"),
            ("hi_sto_inv_log", result.id_inv_log.as_str(), "INSERT"),
        ] {
            store.audit_event(
                &detail.batch.batch_id,
                &row.row_id,
                operation,
                table,
                target_id,
                "SUCCESS",
                Value::Null,
                after.clone(),
                &format!("按首次盘点 {} 写入库存初始账簿", outcome.cd_sto_check),
                operator_id,
                trace_id,
            )?;
        }
    }
    Ok(())
}

fn record_inventory_group_failure(
    store: &LocalStore,
    batch_id: &str,
    operator_id: &str,
    trace_id: &str,
    rows: Vec<MigrationRow>,
    message: String,
) -> Result<(), String> {
    let code = inventory_error_code(&message);
    let table = failed_inventory_table(&message);
    for mut row in rows {
        row.status = "FAILED".into();
        row.error_code = code.clone();
        row.error_message = message.clone();
        row.retry_count += 1;
        row.updated_at = Utc::now().to_rfc3339();
        store.update_row_result(&row)?;
        store.audit_event(
            batch_id,
            &row.row_id,
            "INITIAL_STOCKTAKE",
            table,
            "",
            "FAILED",
            Value::Object(row.normalized_data.clone()),
            Value::Null,
            &message,
            operator_id,
            trace_id,
        )?;
    }
    Ok(())
}

fn finish_inventory_batch(
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
        .filter(|row| matches!(row.status.as_str(), "FAILED" | "INVALID"))
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
        "FINISH_INITIAL_STOCKTAKE",
        "migration_batch",
        batch_id,
        status,
        Value::Null,
        json!({"success":success,"skipped":skipped,"failed":failed,"pending":valid}),
        &format!("首次盘点库存迁移完成：成功{success}组，跳过{skipped}组，失败{failed}组"),
        operator_id,
        &new_object_id(),
    )
}

fn normalized_text(row: &MigrationRow, key: &str) -> String {
    row.normalized_data
        .get(key)
        .map(|value| match value {
            Value::String(text) => text.clone(),
            Value::Null => String::new(),
            other => other.to_string().trim_matches('"').to_string(),
        })
        .unwrap_or_default()
        .trim()
        .to_string()
}

fn required_normalized(row: &MigrationRow, key: &str) -> Result<String, String> {
    let value = normalized_text(row, key);
    if value.is_empty() {
        Err(format!("库存行 {} 缺少必要字段 {key}", row.row_no))
    } else {
        Ok(value)
    }
}

fn required_decimal(row: &MigrationRow, key: &str) -> Result<String, String> {
    let value = required_normalized(row, key)?;
    parse_decimal(&value, key).map(|value| value.to_string())
}

fn validate_effective_date(value: &str) -> Result<Option<NaiveDate>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    NaiveDate::parse_from_str(value.trim(), "%Y-%m-%d")
        .map(Some)
        .map_err(|_| format!("药品效期“{value}”不是 yyyy-MM-dd 格式"))
}

fn next_check_number(prefix: &str, latest: Option<&str>) -> Result<String, String> {
    if prefix.len() != 8 || !prefix.chars().all(|character| character.is_ascii_digit()) {
        return Err("无法根据当前日期生成盘点单号".into());
    }
    let sequence = latest
        .filter(|value| value.len() == 11 && value.starts_with(prefix))
        .and_then(|value| value[8..].parse::<u16>().ok())
        .unwrap_or(0)
        + 1;
    if sequence > 999 {
        return Err(format!("{prefix} 当天盘点单流水已超过 999，无法继续生成"));
    }
    Ok(format!("{prefix}{sequence:03}"))
}

fn inventory_write_error(table: &str, stage: &str, error: String) -> String {
    format!("目标表 {table} 在“{stage}”时写入失败：{error}")
}

fn inventory_error_code(error: &str) -> String {
    let upper = error.to_ascii_uppercase();
    if let Some(start) = upper.find("ORA-") {
        let code = upper[start..].chars().take(9).collect::<String>();
        if code.len() == 9 {
            return code;
        }
    }
    if error.contains("已经存在") || error.contains("不允许") {
        "INITIAL_STOCKTAKE_BLOCKED".into()
    } else {
        "INVENTORY_WRITE_ERROR".into()
    }
}

fn failed_inventory_table(error: &str) -> &str {
    [
        "hi_sto_inv_log",
        "hi_sto_check_sub",
        "hi_sto_check",
        "hi_sto_inv",
        "hi_sto_med",
        "hi_sto_dept",
        "hi_bd_med_pro",
    ]
    .into_iter()
    .find(|table| error.contains(table))
    .unwrap_or("inventory_precondition")
}
