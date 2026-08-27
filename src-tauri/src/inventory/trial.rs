const INVENTORY_TRIAL_TABLES: &[&str] = &[
    "hi_bd_med_unit",
    "hi_sto_med",
    "hi_sto_check",
    "hi_sto_check_sub",
    "hi_sto_inv",
    "hi_sto_inv_log",
];

fn inventory_storage_hash(rows: &[MigrationRow], id_sto: &str) -> String {
    let mut values = rows
        .iter()
        .filter(|row| normalized_text(row, "idSto") == id_sto)
        .map(|row| format!("{}:{}", row.row_id, row.source_hash))
        .collect::<Vec<_>>();
    values.sort();
    let mut digest = Sha256::new();
    digest.update(id_sto.as_bytes());
    for value in values {
        digest.update([0]);
        digest.update(value.as_bytes());
    }
    format!("{:x}", digest.finalize())
}

pub(crate) fn has_successful_inventory_trials(
    detail: &BatchDetail,
    profile: &ConnectionProfile,
) -> bool {
    let executable = detail
        .rows
        .iter()
        .filter(|row| matches!(row.status.as_str(), "VALIDATED" | "FAILED"))
        .collect::<Vec<_>>();
    let required_storages = executable
        .iter()
        .filter_map(|row| {
            let id_sto = normalized_text(row, "idSto");
            (!id_sto.is_empty()).then_some(id_sto)
        })
        .collect::<HashSet<_>>();
    if required_storages.is_empty() {
        return false;
    }
    let expected_target = target_identity(profile);
    required_storages.into_iter().all(|id_sto| {
        let expected_hash = inventory_storage_hash(&detail.rows, &id_sto);
        detail
            .audits
            .iter()
            .filter(|audit| {
                audit.operation == "INVENTORY_TRIAL_ROLLBACK"
                    && audit.after_data.get("targetIdentity") == Some(&expected_target)
                    && audit.after_data.get("idSto").and_then(Value::as_str)
                        == Some(id_sto.as_str())
            })
            .max_by(|left, right| left.operated_at.cmp(&right.operated_at))
            .is_some_and(|audit| {
                audit.result == "SUCCESS"
                    && audit
                        .after_data
                        .get("storageHash")
                        .and_then(Value::as_str)
                        == Some(expected_hash.as_str())
                    && audit.after_data.get("rolledBack").and_then(Value::as_bool) == Some(true)
            })
    })
}

pub async fn trial(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    request: TrialInventoryRequest,
) -> Result<TrialInventoryResponse, String> {
    let _active_batch = ActiveBatchGuard::enter(&request.batch_id)?;
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.source_type != "PHIS27_INVENTORY" {
        return Err("当前批次不是二系列phis机构库存批次".into());
    }
    if detail.batch.status == "SUCCESS" {
        return Err("本批首次盘点已经完成，无需再执行试迁移".into());
    }
    let row = detail
        .rows
        .iter()
        .find(|row| row.row_id == request.row_id)
        .cloned()
        .ok_or_else(|| "未找到需要试迁移的库存明细".to_string())?;
    if !matches!(row.status.as_str(), "VALIDATED" | "FAILED") {
        return Err("只有预检通过或上次写入失败的库存明细可以试迁移".into());
    }
    let id_sto = required_normalized(&row, "idSto")?;
    let id_org = required_normalized(&row, "idOrg")?;
    let storage_name = normalized_text(&row, "naSto");
    let storage_hash = inventory_storage_hash(&detail.rows, &id_sto);
    let sample = vec![row.clone()];

    let write_result = if crate::pg_protocol::uses_native_connection(&request.target) {
        trial_inventory_pg(
            tenant_id,
            operator_id,
            &request,
            &id_org,
            &id_sto,
            &sample,
        )
        .await
    } else if crate::odbc::is_odbc_kind(&request.target.kind) {
        trial_inventory_odbc(
            tenant_id,
            operator_id,
            &request,
            &id_org,
            &id_sto,
            &sample,
        )
    } else {
        trial_inventory_mysql(
            tenant_id,
            operator_id,
            &request,
            &id_org,
            &id_sto,
            &sample,
        )
        .await
    };

    let trace_id = new_object_id();
    let target = target_identity(&request.target);
    let checked_tables = INVENTORY_TRIAL_TABLES
        .iter()
        .map(|table| (*table).to_string())
        .collect::<Vec<_>>();
    let result = match write_result {
        Ok(outcome) => {
            let message = format!(
                "库存试迁移通过：已用该明细完整建立首次盘点 {}、初始库存和账簿；目标事务已自动回滚",
                outcome.cd_sto_check
            );
            store.audit_event(
                &request.batch_id,
                &row.row_id,
                "INVENTORY_TRIAL_ROLLBACK",
                "hi_sto_check",
                &id_sto,
                "SUCCESS",
                Value::Object(row.normalized_data.clone()),
                json!({
                    "targetIdentity": target,
                    "idSto": id_sto,
                    "storageName": storage_name,
                    "sourceHash": row.source_hash,
                    "storageHash": storage_hash,
                    "candidateCheckNumber": outcome.cd_sto_check,
                    "checkedTables": checked_tables,
                    "rolledBack": true
                }),
                &message,
                operator_id,
                &trace_id,
            )?;
            TrialInventoryResult {
                ok: true,
                row_id: row.row_id.clone(),
                row_no: row.row_no,
                source_key: row.source_key.clone(),
                id_sto: id_sto.clone(),
                storage_name: storage_name.clone(),
                message,
                checked_tables: checked_tables.clone(),
            }
        }
        Err(error) => {
            let rollback_confirmed = !error.contains("库存试迁移回滚失败");
            let message = if rollback_confirmed {
                format!("库存试迁移未通过：{error}；目标事务未提交，已自动回滚")
            } else {
                format!(
                    "库存试迁移未通过，且无法确认目标事务已回滚：{error}；请停止正式迁移并立即核对目标库"
                )
            };
            store.audit_event(
                &request.batch_id,
                &row.row_id,
                "INVENTORY_TRIAL_ROLLBACK",
                failed_inventory_table(&error),
                &id_sto,
                "FAILED",
                Value::Object(row.normalized_data.clone()),
                json!({
                    "targetIdentity": target,
                    "idSto": id_sto,
                    "storageName": storage_name,
                    "sourceHash": row.source_hash,
                    "storageHash": storage_hash,
                    "checkedTables": [],
                    "rolledBack": rollback_confirmed
                }),
                &message,
                operator_id,
                &trace_id,
            )?;
            TrialInventoryResult {
                ok: false,
                row_id: row.row_id.clone(),
                row_no: row.row_no,
                source_key: row.source_key.clone(),
                id_sto: id_sto.clone(),
                storage_name: storage_name.clone(),
                message,
                checked_tables: Vec::new(),
            }
        }
    };
    Ok(TrialInventoryResponse {
        result,
        detail: store.load_batch(&request.batch_id)?,
    })
}

async fn trial_inventory_mysql(
    tenant_id: &str,
    operator_id: &str,
    request: &TrialInventoryRequest,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
) -> Result<StorageWriteResult, String> {
    let pool = connect_mysql(&request.target).await?;
    let result = async {
        validate_inventory_schema_mysql(&pool).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("开启库存试迁移事务失败：{error}"))?;
        match write_storage_mysql(
            &mut tx,
            tenant_id,
            operator_id,
            &request.batch_id,
            id_org,
            id_sto,
            rows,
        )
        .await
        {
            Ok(outcome) => tx
                .rollback()
                .await
                .map(|_| outcome)
                .map_err(|error| format!("库存试迁移回滚失败：{error}")),
            Err(error) => match tx.rollback().await {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!("{error}；库存试迁移回滚失败：{rollback}")),
            },
        }
    }
    .await;
    pool.close().await;
    result
}

async fn trial_inventory_pg(
    tenant_id: &str,
    operator_id: &str,
    request: &TrialInventoryRequest,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
) -> Result<StorageWriteResult, String> {
    let pool = crate::pg_protocol::connect(&request.target).await?;
    let result = async {
        validate_inventory_schema_pg(&pool).await?;
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("开启库存试迁移事务失败：{error}"))?;
        match write_storage_pg(
            &mut tx,
            tenant_id,
            operator_id,
            &request.batch_id,
            id_org,
            id_sto,
            rows,
        )
        .await
        {
            Ok(outcome) => tx
                .rollback()
                .await
                .map(|_| outcome)
                .map_err(|error| format!("库存试迁移回滚失败：{error}")),
            Err(error) => match tx.rollback().await {
                Ok(()) => Err(error),
                Err(rollback) => Err(format!("{error}；库存试迁移回滚失败：{rollback}")),
            },
        }
    }
    .await;
    pool.close().await;
    result
}

fn trial_inventory_odbc(
    tenant_id: &str,
    operator_id: &str,
    request: &TrialInventoryRequest,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
) -> Result<StorageWriteResult, String> {
    with_connection(&request.target, |connection| {
        configure_target_session(connection, &request.target)?;
        validate_inventory_schema_odbc(connection)?;
        connection
            .set_autocommit(false)
            .map_err(|error| format!("开启库存试迁移事务失败：{error}"))?;
        let write_result = write_storage_odbc(
            connection,
            tenant_id,
            operator_id,
            &request.batch_id,
            id_org,
            id_sto,
            rows,
            inventory_date_parameter_sql(&request.target.kind),
            inventory_current_timestamp_sql(&request.target.kind),
        );
        let rollback_result = connection.rollback();
        let restore_result = connection.set_autocommit(true);
        match (write_result, rollback_result, restore_result) {
            (_, Err(error), _) => Err(format!("库存试迁移回滚失败：{error}")),
            (_, Ok(()), Err(error)) => Err(format!("恢复数据库自动提交失败：{error}")),
            (Ok(outcome), Ok(()), Ok(())) => Ok(outcome),
            (Err(error), Ok(()), Ok(())) => Err(error),
        }
    })
}
