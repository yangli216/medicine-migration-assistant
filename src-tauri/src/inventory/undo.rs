pub async fn execute(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    request: ExecuteInventoryRequest,
) -> Result<BatchDetail, String> {
    let _active_batch = ActiveBatchGuard::enter(&request.batch_id)?;
    let detail = store.load_batch(&request.batch_id)?;
    if !is_inventory_batch_source_type(&detail.batch.source_type) {
        return Err("当前批次不是机构库存首次盘点批次".into());
    }
    if detail.batch.status == "SUCCESS" {
        return Ok(detail);
    }
    if detail.rows.iter().any(|row| row.status == "INVALID") {
        return Err("本批次仍有库存预检失败项，已阻止所有目标库存写入".into());
    }
    if !has_successful_inventory_trials(&detail, &request.target) {
        return Err(
            "正式执行首次盘点前，请为本批每个目标库房选择一条库存明细完成试迁移；试迁移会完整建立盘点、库存和账簿后自动回滚"
                .into(),
        );
    }
    let executable = detail
        .rows
        .iter()
        .filter(|row| matches!(row.status.as_str(), "VALIDATED" | "FAILED"))
        .cloned()
        .collect::<Vec<_>>();
    if executable.is_empty() {
        return Err("本批次没有可执行的库存明细；请先处理预检失败项".into());
    }
    let mut groups = BTreeMap::<(String, String), Vec<MigrationRow>>::new();
    for row in executable {
        let id_sto = normalized_text(&row, "idSto");
        let id_org = normalized_text(&row, "idOrg");
        if id_sto.is_empty() || id_org.is_empty() {
            return Err(format!("库存行 {} 缺少目标机构或库房映射", row.row_no));
        }
        groups.entry((id_org, id_sto)).or_default().push(row);
    }

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
        "INITIAL_STOCKTAKE_EXECUTE",
        "migration_batch",
        &request.batch_id,
        "RUNNING",
        Value::Null,
        json!({"storageCount":groups.len(),"numberRule":"按库房 yyyyMMdd+3位日流水","targetIdentity":target_identity(&request.target)}),
        "开始按首次盘点逻辑初始化机构库存",
        operator_id,
        &new_object_id(),
    )?;

    if crate::pg_protocol::uses_native_connection(&request.target) {
        execute_pg_groups(store, tenant_id, operator_id, &detail, &request, groups).await?;
    } else if crate::odbc::is_odbc_kind(&request.target.kind) {
        execute_odbc_groups(store, tenant_id, operator_id, &detail, &request, groups)?;
    } else {
        execute_mysql_groups(store, tenant_id, operator_id, &detail, &request, groups).await?;
    }
    finish_inventory_batch(store, &request.batch_id, operator_id)?;
    store.load_batch(&request.batch_id)
}

pub async fn preview_undo(
    store: &LocalStore,
    tenant_id: &str,
    request: UndoInventoryRequest,
) -> Result<InventoryUndoPreview, String> {
    let detail = store.load_batch(&request.batch_id)?;
    let plan = inventory_undo_plan(&detail, &request.target)?;
    if crate::pg_protocol::uses_native_connection(&request.target) {
        let pool = crate::pg_protocol::connect(&request.target).await?;
        validate_inventory_schema_pg(&pool).await?;
        let preview =
            inspect_inventory_undo_pg(&pool, tenant_id, &request.batch_id, &plan).await;
        pool.close().await;
        preview
    } else if crate::odbc::is_odbc_kind(&request.target.kind) {
        with_connection(&request.target, |connection| {
            configure_target_session(connection, &request.target)?;
            inspect_inventory_undo_odbc(connection, tenant_id, &request.batch_id, &plan)
        })
    } else {
        let pool = connect_mysql(&request.target).await?;
        validate_inventory_schema_mysql(&pool).await?;
        let preview =
            inspect_inventory_undo_mysql(&pool, tenant_id, &request.batch_id, &plan).await;
        pool.close().await;
        preview
    }
}

pub async fn undo(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    request: UndoInventoryRequest,
) -> Result<BatchDetail, String> {
    let detail = store.load_batch(&request.batch_id)?;
    let plan = inventory_undo_plan(&detail, &request.target)?;
    let trace_id = new_object_id();
    store.audit_event(
        &request.batch_id,
        "",
        "UNDO_INITIAL_STOCKTAKE_START",
        "migration_batch",
        &request.batch_id,
        "RUNNING",
        Value::Null,
        json!({"targetIdentity":target_identity(&request.target),"storageCount":plan.len()}),
        "开始安全撤销首次盘点；正式删除前将再次检查后续业务引用",
        operator_id,
        &trace_id,
    )?;
    let result = if crate::pg_protocol::uses_native_connection(&request.target) {
        undo_inventory_pg(&request.target, tenant_id, &request.batch_id, &plan).await
    } else if crate::odbc::is_odbc_kind(&request.target.kind) {
        undo_inventory_odbc(&request.target, tenant_id, &request.batch_id, &plan)
    } else {
        undo_inventory_mysql(&request.target, tenant_id, &request.batch_id, &plan).await
    };
    match result {
        Ok(()) => finish_inventory_undo(store, operator_id, &detail, &plan, &trace_id),
        Err(error) => {
            store.audit_event(
                &request.batch_id,
                "",
                "UNDO_INITIAL_STOCKTAKE_FAILED",
                "migration_batch",
                &request.batch_id,
                "FAILED",
                Value::Null,
                json!({"error":error}),
                "安全撤销未执行，目标库存保持原状",
                operator_id,
                &trace_id,
            )?;
            Err(error)
        }
    }
}

fn inventory_undo_plan(
    detail: &BatchDetail,
    profile: &ConnectionProfile,
) -> Result<Vec<InventoryUndoStorage>, String> {
    if !is_inventory_batch_source_type(&detail.batch.source_type) {
        return Err("当前批次不是机构库存首次盘点批次".into());
    }
    match detail.batch.status.as_str() {
        "SUCCESS" | "PARTIAL" => {}
        "UNDONE" => return Err("该首次盘点批次已经撤销，不能重复操作".into()),
        _ => return Err("只有已完成或部分完成的首次盘点批次可以撤销".into()),
    }
    let expected_target = detail
        .audits
        .iter()
        .find(|audit| audit.operation == "INITIAL_STOCKTAKE_EXECUTE")
        .and_then(|audit| audit.after_data.get("targetIdentity"))
        .ok_or_else(|| "该库存批次缺少执行时的目标库身份，不能自动撤销".to_string())?;
    if expected_target != &target_identity(profile) {
        return Err("当前目标库与首次盘点执行时的目标库不一致，已阻止撤销".into());
    }
    let mut storages = BTreeMap::<String, InventoryUndoStorage>::new();
    for row in detail.rows.iter().filter(|row| row.status == "SUCCESS") {
        let audit = detail
            .audits
            .iter()
            .find(|audit| {
                audit.row_id == row.row_id
                    && audit.operation == "INSERT"
                    && audit.target_table == "hi_sto_inv"
            })
            .ok_or_else(|| format!("库存行 {} 缺少首次盘点写入审计，不能安全撤销", row.row_no))?;
        let text = |key: &str| {
            audit
                .after_data
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string()
        };
        let id_sto = normalized_text(row, "idSto");
        let id_sto_check = text("idStoCheck");
        let id_sto_inv = text("idStoInv");
        let id_inv_log = text("idInvLog");
        let id_check_sub = text("idCheckSub");
        if [
            id_sto.as_str(),
            id_sto_check.as_str(),
            id_sto_inv.as_str(),
            id_inv_log.as_str(),
            id_check_sub.as_str(),
        ]
        .iter()
        .any(|value| value.is_empty())
        {
            return Err(format!("库存行 {} 的撤销审计主键不完整", row.row_no));
        }
        let expected_amount = parse_decimal(&normalized_text(row, "amount"), "库存数量")?;
        let storage = storages
            .entry(id_sto.clone())
            .or_insert_with(|| InventoryUndoStorage {
                id_sto,
                name: normalized_text(row, "naSto"),
                id_sto_check: id_sto_check.clone(),
                cd_sto_check: text("cdStoCheck"),
                rows: Vec::new(),
            });
        if storage.id_sto_check != id_sto_check {
            return Err(format!(
                "目标库房 {} 的首次盘点审计出现多个主单",
                storage.name
            ));
        }
        storage.rows.push(InventoryUndoRow {
            row_id: row.row_id.clone(),
            id_sto_inv,
            id_inv_log,
            id_check_sub,
            expected_amount,
        });
    }
    if storages.is_empty() {
        return Err("该批次没有可撤销的成功库存记录".into());
    }
    Ok(storages.into_values().collect())
}

fn inventory_undo_preview(
    batch_id: &str,
    storages: &[InventoryUndoStorage],
    checks: Vec<Vec<String>>,
) -> InventoryUndoPreview {
    let previews = storages
        .iter()
        .zip(checks)
        .map(|(storage, messages)| InventoryUndoStoragePreview {
            id_sto: storage.id_sto.clone(),
            name: storage.name.clone(),
            cd_sto_check: storage.cd_sto_check.clone(),
            inventory_count: storage.rows.len(),
            can_undo: messages.is_empty(),
            messages,
        })
        .collect::<Vec<_>>();
    let blocker_count = previews.iter().map(|item| item.messages.len()).sum();
    let inventory_count = storages.iter().map(|storage| storage.rows.len()).sum();
    InventoryUndoPreview {
        batch_id: batch_id.into(),
        can_undo: blocker_count == 0,
        storage_count: storages.len(),
        inventory_count,
        blocker_count,
        message: if blocker_count == 0 {
            format!(
                "撤销预检通过：{} 个库房、{} 组初始库存均未发生后续业务",
                storages.len(),
                inventory_count
            )
        } else {
            format!("发现 {blocker_count} 项后续业务或数据变化，已阻止撤销")
        },
        storages: previews,
    }
}

fn odbc_count_value(
    connection: &Connection<'_>,
    sql: &str,
    params: Vec<String>,
) -> Result<i64, String> {
    query_optional_string(connection, sql, params)?
        .unwrap_or_default()
        .trim()
        .parse::<i64>()
        .map_err(|_| "目标库返回了无法识别的计数结果".to_string())
}

fn inspect_inventory_undo_odbc(
    connection: &Connection<'_>,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<InventoryUndoPreview, String> {
    let mut checks = Vec::with_capacity(storages.len());
    for storage in storages {
        let mut messages = Vec::new();
        let header = query_optional_row_strings(
            connection,
            "SELECT id_sto,fg_sto_check,sd_check,des_sto_check FROM hi_sto_check WHERE id_tet=? AND id_sto_check=?",
            vec![tenant_id.into(), storage.id_sto_check.clone()],
        )?;
        match header {
            None => messages.push("本批首次盘点主单已不存在".into()),
            Some(values) => {
                let value =
                    |index: usize| values.get(index).and_then(Clone::clone).unwrap_or_default();
                if value(0) != storage.id_sto || value(1) != "1" || value(2) != "1" {
                    messages.push("首次盘点主单状态或所属库房已发生变化".into());
                }
                if !value(3).contains(batch_id) {
                    messages.push("首次盘点主单无法确认属于当前迁移批次".into());
                }
            }
        }
        let other_checks = odbc_count_value(
            connection,
            "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=? AND id_sto=? AND id_sto_check<>?",
            vec![
                tenant_id.into(),
                storage.id_sto.clone(),
                storage.id_sto_check.clone(),
            ],
        )?;
        if other_checks > 0 {
            messages.push(format!("该库房已产生 {other_checks} 张后续盘点单"));
        }
        let inventory_count = odbc_count_value(
            connection,
            "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=? AND id_sto=?",
            vec![tenant_id.into(), storage.id_sto.clone()],
        )?;
        if inventory_count != storage.rows.len() as i64 {
            messages.push(format!(
                "该库房现有 {inventory_count} 组库存，与本批 {} 组初始库存不一致",
                storage.rows.len()
            ));
        }
        let detail_count = odbc_count_value(
            connection,
            "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=? AND id_sto_check=?",
            vec![tenant_id.into(), storage.id_sto_check.clone()],
        )?;
        if detail_count != storage.rows.len() as i64 {
            messages.push("首次盘点明细数量已发生变化".into());
        }
        for row in &storage.rows {
            let inventory = query_optional_row_strings(
                connection,
                "SELECT amount,id_sto FROM hi_sto_inv WHERE id_tet=? AND id_sto_inv=?",
                vec![tenant_id.into(), row.id_sto_inv.clone()],
            )?;
            match inventory {
                None => messages.push(format!("初始库存 {} 已不存在", row.id_sto_inv)),
                Some(values) => {
                    let amount = values.first().and_then(Clone::clone).unwrap_or_default();
                    let id_sto = values.get(1).and_then(Clone::clone).unwrap_or_default();
                    if parse_decimal(&amount, "当前库存数量")? != row.expected_amount
                        || id_sto != storage.id_sto
                    {
                        messages.push(format!("初始库存 {} 的数量或库房已变化", row.id_sto_inv));
                    }
                }
            }
            let log_count = odbc_count_value(
                connection,
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=? AND id_sto_inv=?",
                vec![tenant_id.into(), row.id_sto_inv.clone()],
            )?;
            let own_log = odbc_count_value(
                connection,
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=? AND id_inv_log=? AND id_sto_inv=? AND id_biz_ori=? AND sd_amt_change='100'",
                vec![tenant_id.into(), row.id_inv_log.clone(), row.id_sto_inv.clone(), row.id_check_sub.clone()],
            )?;
            if log_count != 1 || own_log != 1 {
                messages.push(format!(
                    "初始库存 {} 已产生后续库存变动或日志被修改",
                    row.id_sto_inv
                ));
            }
            let own_detail = odbc_count_value(
                connection,
                "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=? AND id=? AND id_sto_check=? AND id_sto_inv=?",
                vec![tenant_id.into(), row.id_check_sub.clone(), storage.id_sto_check.clone(), row.id_sto_inv.clone()],
            )?;
            if own_detail != 1 {
                messages.push(format!("盘点明细 {} 已被修改或删除", row.id_check_sub));
            }
        }
        messages.sort();
        messages.dedup();
        checks.push(messages);
    }
    Ok(inventory_undo_preview(batch_id, storages, checks))
}

async fn inspect_inventory_undo_mysql(
    pool: &MySqlPool,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<InventoryUndoPreview, String> {
    let mut checks = Vec::with_capacity(storages.len());
    for storage in storages {
        let mut messages = Vec::new();
        let header = query::<MySql>(
            "SELECT id_sto,fg_sto_check,sd_check,des_sto_check FROM hi_sto_check WHERE id_tet=? AND id_sto_check=? LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&storage.id_sto_check)
        .fetch_optional(pool)
        .await
        .map_err(|error| format!("检查首次盘点主单失败：{error}"))?;
        match header {
            None => messages.push("本批首次盘点主单已不存在".into()),
            Some(row) => {
                let value = |index| row.try_get::<String, _>(index).unwrap_or_default();
                if value(0) != storage.id_sto || value(1) != "1" || value(2) != "1" {
                    messages.push("首次盘点主单状态或所属库房已发生变化".into());
                }
                if !value(3).contains(batch_id) {
                    messages.push("首次盘点主单无法确认属于当前迁移批次".into());
                }
            }
        }
        let other_checks = query_scalar::<MySql, i64>(
            "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=? AND id_sto=? AND id_sto_check<>?",
        )
        .bind(tenant_id)
        .bind(&storage.id_sto)
        .bind(&storage.id_sto_check)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("检查后续盘点失败：{error}"))?;
        if other_checks > 0 {
            messages.push(format!("该库房已产生 {other_checks} 张后续盘点单"));
        }
        let inventory_count = query_scalar::<MySql, i64>(
            "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=? AND id_sto=?",
        )
        .bind(tenant_id)
        .bind(&storage.id_sto)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("检查当前库存失败：{error}"))?;
        if inventory_count != storage.rows.len() as i64 {
            messages.push(format!(
                "该库房现有 {inventory_count} 组库存，与本批 {} 组初始库存不一致",
                storage.rows.len()
            ));
        }
        let detail_count = query_scalar::<MySql, i64>(
            "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=? AND id_sto_check=?",
        )
        .bind(tenant_id)
        .bind(&storage.id_sto_check)
        .fetch_one(pool)
        .await
        .map_err(|error| format!("检查盘点明细失败：{error}"))?;
        if detail_count != storage.rows.len() as i64 {
            messages.push("首次盘点明细数量已发生变化".into());
        }
        for undo_row in &storage.rows {
            let inventory = query::<MySql>(
                "SELECT amount,id_sto FROM hi_sto_inv WHERE id_tet=? AND id_sto_inv=? LIMIT 1",
            )
            .bind(tenant_id)
            .bind(&undo_row.id_sto_inv)
            .fetch_optional(pool)
            .await
            .map_err(|error| format!("检查初始库存失败：{error}"))?;
            match inventory {
                None => messages.push(format!("初始库存 {} 已不存在", undo_row.id_sto_inv)),
                Some(row) => {
                    let amount = row.try_get::<Decimal, _>(0).unwrap_or(Decimal::ZERO);
                    let id_sto = row.try_get::<String, _>(1).unwrap_or_default();
                    if amount != undo_row.expected_amount || id_sto != storage.id_sto {
                        messages.push(format!(
                            "初始库存 {} 的数量或库房已变化",
                            undo_row.id_sto_inv
                        ));
                    }
                }
            }
            let log_count = query_scalar::<MySql, i64>(
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=? AND id_sto_inv=?",
            )
            .bind(tenant_id)
            .bind(&undo_row.id_sto_inv)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("检查库存变动日志失败：{error}"))?;
            let own_log = query_scalar::<MySql, i64>(
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=? AND id_inv_log=? AND id_sto_inv=? AND id_biz_ori=? AND sd_amt_change='100'",
            )
            .bind(tenant_id)
            .bind(&undo_row.id_inv_log)
            .bind(&undo_row.id_sto_inv)
            .bind(&undo_row.id_check_sub)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("核对初始账簿日志失败：{error}"))?;
            if log_count != 1 || own_log != 1 {
                messages.push(format!(
                    "初始库存 {} 已产生后续库存变动或日志被修改",
                    undo_row.id_sto_inv
                ));
            }
            let own_detail = query_scalar::<MySql, i64>(
                "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=? AND id=? AND id_sto_check=? AND id_sto_inv=?",
            )
            .bind(tenant_id)
            .bind(&undo_row.id_check_sub)
            .bind(&storage.id_sto_check)
            .bind(&undo_row.id_sto_inv)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("核对首次盘点明细失败：{error}"))?;
            if own_detail != 1 {
                messages.push(format!("盘点明细 {} 已被修改或删除", undo_row.id_check_sub));
            }
        }
        messages.sort();
        messages.dedup();
        checks.push(messages);
    }
    Ok(inventory_undo_preview(batch_id, storages, checks))
}

fn blocked_undo_message(preview: &InventoryUndoPreview) -> String {
    let details = preview
        .storages
        .iter()
        .flat_map(|storage| {
            storage
                .messages
                .iter()
                .map(move |message| format!("{}：{}", storage.name, message))
        })
        .collect::<Vec<_>>();
    format!("安全撤销预检未通过：{}", details.join("；"))
}

fn undo_inventory_odbc(
    profile: &ConnectionProfile,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<(), String> {
    with_connection(profile, |connection| {
        configure_target_session(connection, profile)?;
        validate_inventory_schema_odbc(connection)?;
        connection
            .set_autocommit(false)
            .map_err(|error| format!("开启库存撤销事务失败：{error}"))?;
        let result = (|| {
            for storage in storages {
                query_optional_string(
                    connection,
                    "SELECT id_sto FROM hi_sto_dept WHERE id_tet=? AND id_sto=? FOR UPDATE",
                    vec![tenant_id.into(), storage.id_sto.clone()],
                )?
                .ok_or_else(|| format!("目标库房 {} 已不存在", storage.name))?;
            }
            let preview = inspect_inventory_undo_odbc(connection, tenant_id, batch_id, storages)?;
            if !preview.can_undo {
                return Err(blocked_undo_message(&preview));
            }
            for storage in storages {
                for row in &storage.rows {
                    execute_strings(
                        connection,
                        "DELETE FROM hi_sto_inv_log WHERE id_tet=? AND id_inv_log=? AND id_sto_inv=?",
                        vec![tenant_id.into(), row.id_inv_log.clone(), row.id_sto_inv.clone()],
                    )?;
                    execute_strings(
                        connection,
                        "DELETE FROM hi_sto_check_sub WHERE id_tet=? AND id=? AND id_sto_check=?",
                        vec![
                            tenant_id.into(),
                            row.id_check_sub.clone(),
                            storage.id_sto_check.clone(),
                        ],
                    )?;
                    execute_strings(
                        connection,
                        "DELETE FROM hi_sto_inv WHERE id_tet=? AND id_sto_inv=? AND id_sto=?",
                        vec![
                            tenant_id.into(),
                            row.id_sto_inv.clone(),
                            storage.id_sto.clone(),
                        ],
                    )?;
                }
                execute_strings(
                    connection,
                    "DELETE FROM hi_sto_check WHERE id_tet=? AND id_sto_check=? AND id_sto=?",
                    vec![
                        tenant_id.into(),
                        storage.id_sto_check.clone(),
                        storage.id_sto.clone(),
                    ],
                )?;
            }
            Ok(())
        })();
        match result {
            Ok(()) => {
                connection
                    .commit()
                    .map_err(|error| format!("提交库存撤销事务失败：{error}"))?;
                connection
                    .set_autocommit(true)
                    .map_err(|error| format!("恢复自动提交失败：{error}"))?;
                Ok(())
            }
            Err(error) => {
                let rollback = connection.rollback().err().map(|value| value.to_string());
                let _ = connection.set_autocommit(true);
                Err(rollback
                    .map(|value| format!("{error}；回滚异常：{value}"))
                    .unwrap_or(error))
            }
        }
    })
}

async fn undo_inventory_mysql(
    profile: &ConnectionProfile,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<(), String> {
    let pool = connect_mysql(profile).await?;
    validate_inventory_schema_mysql(&pool).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("开启库存撤销事务失败：{error}"))?;
    let result = async {
        for storage in storages {
            query_scalar::<MySql, String>(
                "SELECT id_sto FROM hi_sto_dept WHERE id_tet=? AND id_sto=? FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| format!("锁定目标库房失败：{error}"))?
            .ok_or_else(|| format!("目标库房 {} 已不存在", storage.name))?;

            let header = query::<MySql>(
                "SELECT id_sto,fg_sto_check,sd_check,des_sto_check FROM hi_sto_check WHERE id_tet=? AND id_sto_check=? FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto_check)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| format!("锁定首次盘点主单失败：{error}"))?
            .ok_or_else(|| format!("{}：本批首次盘点主单已不存在", storage.name))?;
            let header_value = |index| header.try_get::<String, _>(index).unwrap_or_default();
            if header_value(0) != storage.id_sto
                || header_value(1) != "1"
                || header_value(2) != "1"
                || !header_value(3).contains(batch_id)
            {
                return Err(format!("{}：首次盘点主单状态或归属已变化", storage.name));
            }
            let other_checks = query_scalar::<MySql, i64>(
                "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=? AND id_sto=? AND id_sto_check<>?",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto)
            .bind(&storage.id_sto_check)
            .fetch_one(&mut *tx)
            .await
            .map_err(|error| format!("检查后续盘点失败：{error}"))?;
            if other_checks > 0 {
                return Err(format!("{}：已经产生后续盘点单", storage.name));
            }
            let inventory_rows = query::<MySql>(
                "SELECT id_sto_inv,amount FROM hi_sto_inv WHERE id_tet=? AND id_sto=? FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| format!("锁定当前库存失败：{error}"))?;
            if inventory_rows.len() != storage.rows.len() {
                return Err(format!("{}：当前库存组数已发生变化", storage.name));
            }
            let detail_rows = query::<MySql>(
                "SELECT id,id_sto_inv FROM hi_sto_check_sub WHERE id_tet=? AND id_sto_check=? FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto_check)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| format!("锁定盘点明细失败：{error}"))?;
            if detail_rows.len() != storage.rows.len() {
                return Err(format!("{}：首次盘点明细数量已变化", storage.name));
            }
            for undo_row in &storage.rows {
                let inventory = inventory_rows
                    .iter()
                    .find(|row| {
                        row.try_get::<String, _>(0).unwrap_or_default()
                            == undo_row.id_sto_inv
                    })
                    .ok_or_else(|| format!("初始库存 {} 已不存在", undo_row.id_sto_inv))?;
                if inventory
                    .try_get::<Decimal, _>(1)
                    .unwrap_or(Decimal::ZERO)
                    != undo_row.expected_amount
                {
                    return Err(format!("初始库存 {} 的数量已变化", undo_row.id_sto_inv));
                }
                if !detail_rows.iter().any(|row| {
                    row.try_get::<String, _>(0).unwrap_or_default() == undo_row.id_check_sub
                        && row.try_get::<String, _>(1).unwrap_or_default()
                            == undo_row.id_sto_inv
                }) {
                    return Err(format!("盘点明细 {} 已被修改", undo_row.id_check_sub));
                }
                let logs = query::<MySql>(
                    "SELECT id_inv_log,id_biz_ori,sd_amt_change FROM hi_sto_inv_log WHERE id_tet=? AND id_sto_inv=? FOR UPDATE",
                )
                .bind(tenant_id)
                .bind(&undo_row.id_sto_inv)
                .fetch_all(&mut *tx)
                .await
                .map_err(|error| format!("锁定库存变动日志失败：{error}"))?;
                if logs.len() != 1
                    || logs[0].try_get::<String, _>(0).unwrap_or_default()
                        != undo_row.id_inv_log
                    || logs[0].try_get::<String, _>(1).unwrap_or_default()
                        != undo_row.id_check_sub
                    || logs[0].try_get::<String, _>(2).unwrap_or_default() != "100"
                {
                    return Err(format!(
                        "初始库存 {} 已产生后续库存变动或日志被修改",
                        undo_row.id_sto_inv
                    ));
                }
            }
        }

        for storage in storages {
            for undo_row in &storage.rows {
                query::<MySql>(
                    "DELETE FROM hi_sto_inv_log WHERE id_tet=? AND id_inv_log=? AND id_sto_inv=?",
                )
                .bind(tenant_id)
                .bind(&undo_row.id_inv_log)
                .bind(&undo_row.id_sto_inv)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除初始账簿日志失败：{error}"))?;
                query::<MySql>(
                    "DELETE FROM hi_sto_check_sub WHERE id_tet=? AND id=? AND id_sto_check=?",
                )
                .bind(tenant_id)
                .bind(&undo_row.id_check_sub)
                .bind(&storage.id_sto_check)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除首次盘点明细失败：{error}"))?;
                query::<MySql>(
                    "DELETE FROM hi_sto_inv WHERE id_tet=? AND id_sto_inv=? AND id_sto=?",
                )
                .bind(tenant_id)
                .bind(&undo_row.id_sto_inv)
                .bind(&storage.id_sto)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除初始库存失败：{error}"))?;
            }
            query::<MySql>(
                "DELETE FROM hi_sto_check WHERE id_tet=? AND id_sto_check=? AND id_sto=?",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto_check)
            .bind(&storage.id_sto)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("删除首次盘点主单失败：{error}"))?;
        }
        Ok::<(), String>(())
    }
    .await;
    match result {
        Ok(()) => tx
            .commit()
            .await
            .map_err(|error| format!("提交库存撤销事务失败：{error}"))?,
        Err(error) => {
            let rollback = tx.rollback().await.err().map(|value| value.to_string());
            pool.close().await;
            return Err(rollback
                .map(|value| format!("{error}；回滚异常：{value}"))
                .unwrap_or(error));
        }
    }
    pool.close().await;
    Ok(())
}

fn finish_inventory_undo(
    store: &LocalStore,
    operator_id: &str,
    detail: &BatchDetail,
    storages: &[InventoryUndoStorage],
    trace_id: &str,
) -> Result<BatchDetail, String> {
    for storage in storages {
        for row in &storage.rows {
            for (table, target_id) in [
                ("hi_sto_inv_log", row.id_inv_log.as_str()),
                ("hi_sto_check_sub", row.id_check_sub.as_str()),
                ("hi_sto_inv", row.id_sto_inv.as_str()),
            ] {
                store.audit_event(
                    &detail.batch.batch_id,
                    &row.row_id,
                    "UNDO_DELETE",
                    table,
                    target_id,
                    "SUCCESS",
                    json!({"table":table,"targetId":target_id}),
                    Value::Null,
                    "首次盘点未发生后续业务，已按撤销事务删除",
                    operator_id,
                    trace_id,
                )?;
            }
        }
        store.audit_event(
            &detail.batch.batch_id,
            "",
            "UNDO_DELETE",
            "hi_sto_check",
            &storage.id_sto_check,
            "SUCCESS",
            json!({"idSto":storage.id_sto,"cdStoCheck":storage.cd_sto_check}),
            Value::Null,
            "首次盘点主单已安全撤销",
            operator_id,
            trace_id,
        )?;
    }
    for mut row in detail
        .rows
        .iter()
        .filter(|row| row.status == "SUCCESS")
        .cloned()
    {
        row.status = "UNDONE".into();
        row.error_code.clear();
        row.error_message = "本批首次盘点和初始账簿已安全撤销".into();
        row.updated_at = Utc::now().to_rfc3339();
        store.update_row_result(&row)?;
    }
    let deactivated = store.deactivate_inventory_links_for_batch(&detail.batch.batch_id)?;
    store.update_batch_status(&detail.batch.batch_id, "UNDONE")?;
    store.audit_event(
        &detail.batch.batch_id,
        "",
        "UNDO_INITIAL_STOCKTAKE",
        "migration_batch",
        &detail.batch.batch_id,
        "UNDONE",
        Value::Null,
        json!({
            "storageCount":storages.len(),
            "inventoryCount":storages.iter().map(|storage| storage.rows.len()).sum::<usize>(),
            "deactivatedSourceLinks":deactivated,
            "retainedTable":"hi_sto_med"
        }),
        "安全撤销完成；首次盘点、初始库存和初始账簿日志已删除，库房药品配置保留",
        operator_id,
        trace_id,
    )?;
    store.load_batch(&detail.batch.batch_id)
}
