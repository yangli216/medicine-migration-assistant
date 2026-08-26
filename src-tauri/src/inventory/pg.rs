async fn validate_inventory_schema_pg(pool: &PgPool) -> Result<(), String> {
    for (table, columns) in INVENTORY_TARGET_TABLE_PROJECTIONS {
        query::<Postgres>(&format!("SELECT {columns} FROM {table} WHERE 1=0"))
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

async fn load_target_storages_pg(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<TargetStorage>, String> {
    let pool = crate::pg_protocol::connect(profile).await?;
    let result = async {
        query::<Postgres>(
            "SELECT id_sto,na_sto,sd_sto,id_org FROM hi_sto_dept \
             WHERE id_tet=$1 AND fg_active='1' ORDER BY sd_sto,na_sto",
        )
        .bind(tenant_id)
        .fetch_all(&pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    let value = |index| {
                        row.try_get::<String, _>(index)
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    };
                    let storage_type = value(2);
                    TargetStorage {
                        id_sto: value(0),
                        name: value(1),
                        storage_type_name: storage_type_name(&storage_type),
                        storage_type,
                        product_types: String::new(),
                        organization_id: value(3),
                    }
                })
                .collect::<Vec<_>>()
        })
        .map_err(|error| format!("读取新系统仓储失败：{error}"))
    }
    .await;
    pool.close().await;
    result
}

async fn target_duplicate_flags_pg(
    profile: &ConnectionProfile,
    tenant_id: &str,
    groups: &[InventoryGroup],
    mappings: &HashMap<String, &InventoryLocationMapping>,
) -> Result<Vec<bool>, String> {
    let pool = crate::pg_protocol::connect(profile).await?;
    let result = async {
        let mut flags = Vec::with_capacity(groups.len());
        for group in groups {
            let Some(mapping) = mappings.get(&group.source_location_key) else {
                flags.push(false);
                continue;
            };
            let existing = query_scalar::<Postgres, String>(
                "SELECT i.id_sto_inv FROM hi_sto_inv i INNER JOIN hi_bd_med_pro p ON p.id_med_pro=i.id_med_pro \
                 WHERE i.id_tet=$1 AND i.id_org=$2 AND i.id_sto=$3 AND p.cd_med_pro=$4 AND p.id_tet=$5 \
                 AND p.fg_active='1' AND i.price_sale=$6 AND i.price_pur=$7 AND COALESCE(i.cd_batch,'')=$8 \
                 AND COALESCE(CAST(i.dt_effect AS VARCHAR(10)),'')=$9 AND i.fg_active='1' LIMIT 1",
            )
            .bind(tenant_id)
            .bind(&mapping.target_id_org)
            .bind(&mapping.target_id_sto)
            .bind(&group.source_product_key)
            .bind(tenant_id)
            .bind(group.price_sale)
            .bind(group.price_pur)
            .bind(&group.batch_code)
            .bind(&group.effective_date)
            .fetch_optional(&pool)
            .await
            .map_err(|error| format!("检查目标库存重复失败：{error}"))?;
            flags.push(existing.is_some());
        }
        Ok(flags)
    }
    .await;
    pool.close().await;
    result
}

async fn execute_pg_groups(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    detail: &BatchDetail,
    request: &ExecuteInventoryRequest,
    groups: BTreeMap<(String, String), Vec<MigrationRow>>,
) -> Result<(), String> {
    let pool = crate::pg_protocol::connect(&request.target).await?;
    validate_inventory_schema_pg(&pool).await?;
    let target_identity_text = target_identity(&request.target).to_string();
    for ((id_org, id_sto), rows) in groups {
        let trace_id = new_object_id();
        let mut tx = pool
            .begin()
            .await
            .map_err(|error| format!("开启库存事务失败：{error}"))?;
        match write_storage_pg(
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

async fn ensure_inventory_unit_pg(
    tx: &mut PgTransaction<'_>,
    tenant_id: &str,
    id_med: &str,
    unit: &str,
    factor: i64,
) -> Result<String, String> {
    if let Some(id) = query_scalar::<Postgres, String>(
        "SELECT id_med_unit FROM hi_bd_med_unit WHERE id_tet=$1 AND id_med=$2 AND na_unit=$3 AND unit_factor=$4 LIMIT 1",
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
    query::<Postgres>(
        "INSERT INTO hi_bd_med_unit(id_med_unit,id_med,na_unit,unit_factor,id_tet) VALUES($1,$2,$3,$4,$5)",
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
async fn write_storage_pg(
    tx: &mut PgTransaction<'_>,
    tenant_id: &str,
    operator_id: &str,
    batch_id: &str,
    id_org: &str,
    id_sto: &str,
    rows: &[MigrationRow],
) -> Result<StorageWriteResult, String> {
    let storage = query_scalar::<Postgres, String>(
        "SELECT id_sto FROM hi_sto_dept WHERE id_tet=$1 AND id_org=$2 AND id_sto=$3 AND fg_active='1' AND sd_sto IN ('1','2') LIMIT 1 FOR UPDATE",
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
    let check_count = query_scalar::<Postgres, i64>(
        "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=$1 AND id_org=$2 AND id_sto=$3 AND sd_check='1' AND fg_sto_check<>'9'",
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
    let inventory_count = query_scalar::<Postgres, i64>(
        "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=$1 AND id_org=$2 AND id_sto=$3",
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
    let latest = query_scalar::<Postgres, String>(
        "SELECT cd_sto_check FROM hi_sto_check WHERE id_tet=$1 AND id_sto=$2 AND cd_sto_check LIKE $3 AND CHAR_LENGTH(cd_sto_check)=11 ORDER BY cd_sto_check DESC LIMIT 1",
    )
    .bind(tenant_id)
    .bind(id_sto)
    .bind(format!("{prefix}%"))
    .fetch_optional(&mut **tx)
    .await
    .map_err(|error| inventory_write_error("hi_sto_check", "生成盘点单号", error.to_string()))?;
    let cd_sto_check = next_check_number(&prefix, latest.as_deref())?;
    let id_sto_check = new_object_id();
    query::<Postgres>(
        "INSERT INTO hi_sto_check(id_sto_check,id_sto,cd_sto_check,dt_check_begin,dt_check_end,fg_sto_check,sd_pol,sd_check,des_sto_check,id_org,id_tet,revision,insert_user,insert_time) VALUES($1,$2,$3,CURRENT_TIMESTAMP,NULL,'0','1','1',$4,$5,$6,$7,$8,CURRENT_TIMESTAMP)",
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
        let product_id_med = query_scalar::<Postgres, String>(
            "SELECT id_med FROM hi_bd_med_pro WHERE id_tet=$1 AND id_med_pro=$2 AND fg_active='1' LIMIT 1",
        )
        .bind(tenant_id)
        .bind(&id_med_pro)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| inventory_write_error("hi_bd_med_pro", "读取商品属性", error.to_string()))?
        .ok_or_else(|| format!("药品商品 {id_med_pro} 在目标租户中不存在或已停用"))?;
        if product_id_med != id_med {
            return Err(format!("药品商品 {id_med_pro} 的 id_med 与基础迁移台账不一致"));
        }
        let unit_sale = required_normalized(row, "unitSale")?;
        let spec_sale = normalized_text(row, "specSale");
        let unit_sale_factor = required_normalized(row, "unitSaleFactor")?
            .parse::<i64>()
            .ok()
            .filter(|factor| *factor > 0)
            .ok_or_else(|| format!("库存行 {} 的库房包装系数无效", row.row_no))?;
        let id_med_unit = ensure_inventory_unit_pg(
            tx,
            tenant_id,
            &id_med,
            &unit_sale,
            unit_sale_factor,
        )
        .await?;
        let existing_sto_med = query_scalar::<Postgres, String>(
            "SELECT id_sto_med FROM hi_sto_med WHERE id_tet=$1 AND id_org=$2 AND id_sto=$3 AND id_med_pro=$4 LIMIT 1",
        )
        .bind(tenant_id)
        .bind(id_org)
        .bind(id_sto)
        .bind(&id_med_pro)
        .fetch_optional(&mut **tx)
        .await
        .map_err(|error| inventory_write_error("hi_sto_med", "查找库房药品属性", error.to_string()))?;
        let id_sto_med = existing_sto_med.unwrap_or_else(new_object_id);
        let sto_med_count = query_scalar::<Postgres, i64>(
            "SELECT COUNT(*) FROM hi_sto_med WHERE id_sto_med=$1",
        )
        .bind(&id_sto_med)
        .fetch_one(&mut **tx)
        .await
        .map_err(|error| inventory_write_error("hi_sto_med", "确认库房药品属性", error.to_string()))?;
        if sto_med_count > 0 {
            query::<Postgres>("UPDATE hi_sto_med SET id_med=$1,id_med_unit=$2,unit_sale=$3,spec_sale=$4,price_sale=$5,price_pur=$6,unit_sale_factor=$7,fg_active='1' WHERE id_sto_med=$8")
                .bind(&id_med).bind(&id_med_unit).bind(&unit_sale).bind(&spec_sale)
                .bind(price_sale).bind(price_pur).bind(unit_sale_factor).bind(&id_sto_med)
                .execute(&mut **tx).await
                .map_err(|error| inventory_write_error("hi_sto_med", "更新库房药品属性", error.to_string()))?;
        } else {
            query::<Postgres>("INSERT INTO hi_sto_med(id_sto_med,id_med,id_med_pro,unit_sale,spec_sale,price_sale,price_pur,unit_sale_factor,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time,id_med_unit) VALUES($1,$2,$3,$4,$5,$6,$7,$8,'1',$9,$10,$11,'0',$12,CURRENT_TIMESTAMP,$13)")
                .bind(&id_sto_med).bind(&id_med).bind(&id_med_pro).bind(&unit_sale).bind(&spec_sale)
                .bind(price_sale).bind(price_pur).bind(unit_sale_factor).bind(id_sto).bind(id_org)
                .bind(tenant_id).bind(operator_id).bind(&id_med_unit)
                .execute(&mut **tx).await
                .map_err(|error| inventory_write_error("hi_sto_med", "建立库房药品属性", error.to_string()))?;
        }

        let id_sto_inv = new_object_id();
        let id_check_sub = new_object_id();
        let id_inv_log = new_object_id();
        query::<Postgres>("INSERT INTO hi_sto_check_sub(id,id_sto_check,id_med_pro,id_sto_inv,cd_batch,dt_effect,amt_check_bgn,amt_check_end,amt_change,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur,unit_sale,unit_sale_factor) VALUES($1,$2,$3,$4,$5,$6,0,$7,$8,$9,$10,'0',$11,CURRENT_TIMESTAMP,$12,$13,$14,$15)")
            .bind(&id_check_sub).bind(&id_sto_check).bind(&id_med_pro).bind(&id_sto_inv).bind(&batch_code)
            .bind(effective_date).bind(amount).bind(amount).bind(id_org).bind(tenant_id).bind(operator_id)
            .bind(price_sale).bind(price_pur).bind(&unit_sale).bind(unit_sale_factor)
            .execute(&mut **tx).await
            .map_err(|error| inventory_write_error("hi_sto_check_sub", "写入首次盘点明细", error.to_string()))?;
        query::<Postgres>("INSERT INTO hi_sto_inv(id_sto_inv,id_med_pro,amount,price_sale,price_pur,cd_batch,dt_effect,fg_active,id_sto,id_org,id_tet,revision,insert_user,insert_time) VALUES($1,$2,$3,$4,$5,$6,$7,'1',$8,$9,$10,'0',$11,CURRENT_TIMESTAMP)")
            .bind(&id_sto_inv).bind(&id_med_pro).bind(amount).bind(price_sale).bind(price_pur).bind(&batch_code)
            .bind(effective_date).bind(id_sto).bind(id_org).bind(tenant_id).bind(operator_id)
            .execute(&mut **tx).await
            .map_err(|error| inventory_write_error("hi_sto_inv", "建立初始库存", error.to_string()))?;
        query::<Postgres>("INSERT INTO hi_sto_inv_log(id_inv_log,id_sto_inv,id_med_pro,sd_amt_change,des_reason,id_biz_ori,amt_change,amt_before,amt_after,unit_sale,unit_sale_factor,id_sto,id_org,id_tet,revision,insert_user,insert_time,price_sale,price_pur) VALUES($1,$2,$3,'100',$4,$5,$6,0,$7,$8,$9,$10,$11,$12,'0',$13,CURRENT_TIMESTAMP,$14,$15)")
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
    query::<Postgres>("UPDATE hi_sto_check SET fg_sto_check='1',sd_pol='1',dt_check_end=CURRENT_TIMESTAMP WHERE id_sto_check=$1 AND fg_sto_check='0'")
        .bind(&id_sto_check).execute(&mut **tx).await
        .map_err(|error| inventory_write_error("hi_sto_check", "完成首次盘点", error.to_string()))?;
    Ok(StorageWriteResult {
        id_sto_check,
        cd_sto_check,
        rows: outcomes,
    })
}

async fn inspect_inventory_undo_pg(
    pool: &PgPool,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<InventoryUndoPreview, String> {
    let mut checks = Vec::with_capacity(storages.len());
    for storage in storages {
        let mut messages = Vec::new();
        let header = query::<Postgres>(
            "SELECT id_sto,fg_sto_check,sd_check,des_sto_check FROM hi_sto_check WHERE id_tet=$1 AND id_sto_check=$2 LIMIT 1",
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
        let other_checks = query_scalar::<Postgres, i64>(
            "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=$1 AND id_sto=$2 AND id_sto_check<>$3",
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
        let inventory_count = query_scalar::<Postgres, i64>(
            "SELECT COUNT(*) FROM hi_sto_inv WHERE id_tet=$1 AND id_sto=$2",
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
        let detail_count = query_scalar::<Postgres, i64>(
            "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=$1 AND id_sto_check=$2",
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
            let inventory = query::<Postgres>(
                "SELECT amount,id_sto FROM hi_sto_inv WHERE id_tet=$1 AND id_sto_inv=$2 LIMIT 1",
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
            let log_count = query_scalar::<Postgres, i64>(
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=$1 AND id_sto_inv=$2",
            )
            .bind(tenant_id)
            .bind(&undo_row.id_sto_inv)
            .fetch_one(pool)
            .await
            .map_err(|error| format!("检查库存变动日志失败：{error}"))?;
            let own_log = query_scalar::<Postgres, i64>(
                "SELECT COUNT(*) FROM hi_sto_inv_log WHERE id_tet=$1 AND id_inv_log=$2 AND id_sto_inv=$3 AND id_biz_ori=$4 AND sd_amt_change='100'",
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
            let own_detail = query_scalar::<Postgres, i64>(
                "SELECT COUNT(*) FROM hi_sto_check_sub WHERE id_tet=$1 AND id=$2 AND id_sto_check=$3 AND id_sto_inv=$4",
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

async fn undo_inventory_pg(
    profile: &ConnectionProfile,
    tenant_id: &str,
    batch_id: &str,
    storages: &[InventoryUndoStorage],
) -> Result<(), String> {
    let pool = crate::pg_protocol::connect(profile).await?;
    validate_inventory_schema_pg(&pool).await?;
    let mut tx = pool
        .begin()
        .await
        .map_err(|error| format!("开启库存撤销事务失败：{error}"))?;
    let result = async {
        for storage in storages {
            query_scalar::<Postgres, String>(
                "SELECT id_sto FROM hi_sto_dept WHERE id_tet=$1 AND id_sto=$2 FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto)
            .fetch_optional(&mut *tx)
            .await
            .map_err(|error| format!("锁定目标库房失败：{error}"))?
            .ok_or_else(|| format!("目标库房 {} 已不存在", storage.name))?;

            let header = query::<Postgres>(
                "SELECT id_sto,fg_sto_check,sd_check,des_sto_check FROM hi_sto_check WHERE id_tet=$1 AND id_sto_check=$2 FOR UPDATE",
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
            let other_checks = query_scalar::<Postgres, i64>(
                "SELECT COUNT(*) FROM hi_sto_check WHERE id_tet=$1 AND id_sto=$2 AND id_sto_check<>$3",
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
            let inventory_rows = query::<Postgres>(
                "SELECT id_sto_inv,amount FROM hi_sto_inv WHERE id_tet=$1 AND id_sto=$2 FOR UPDATE",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto)
            .fetch_all(&mut *tx)
            .await
            .map_err(|error| format!("锁定当前库存失败：{error}"))?;
            if inventory_rows.len() != storage.rows.len() {
                return Err(format!("{}：当前库存组数已发生变化", storage.name));
            }
            let detail_rows = query::<Postgres>(
                "SELECT id,id_sto_inv FROM hi_sto_check_sub WHERE id_tet=$1 AND id_sto_check=$2 FOR UPDATE",
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
                let inventory = inventory_rows.iter().find(|row| {
                    row.try_get::<String, _>(0).unwrap_or_default() == undo_row.id_sto_inv
                });
                let Some(inventory) = inventory else {
                    return Err(format!("{}：初始库存已被删除", storage.name));
                };
                if inventory.try_get::<Decimal, _>(1).unwrap_or(Decimal::ZERO)
                    != undo_row.expected_amount
                {
                    return Err(format!("{}：初始库存数量已变化", storage.name));
                }
                let detail_matches = detail_rows.iter().any(|row| {
                    row.try_get::<String, _>(0).unwrap_or_default() == undo_row.id_check_sub
                        && row.try_get::<String, _>(1).unwrap_or_default() == undo_row.id_sto_inv
                });
                if !detail_matches {
                    return Err(format!("{}：首次盘点明细已变化", storage.name));
                }
                let logs = query::<Postgres>(
                    "SELECT id_inv_log,id_biz_ori,sd_amt_change FROM hi_sto_inv_log WHERE id_tet=$1 AND id_sto_inv=$2 FOR UPDATE",
                )
                .bind(tenant_id)
                .bind(&undo_row.id_sto_inv)
                .fetch_all(&mut *tx)
                .await
                .map_err(|error| format!("锁定库存日志失败：{error}"))?;
                if logs.len() != 1
                    || logs[0].try_get::<String, _>(0).unwrap_or_default()
                        != undo_row.id_inv_log
                    || logs[0].try_get::<String, _>(1).unwrap_or_default()
                        != undo_row.id_check_sub
                    || logs[0].try_get::<String, _>(2).unwrap_or_default() != "100"
                {
                    return Err(format!("{}：已产生后续库存变动", storage.name));
                }
            }
        }

        for storage in storages {
            for row in &storage.rows {
                query::<Postgres>(
                    "DELETE FROM hi_sto_inv_log WHERE id_tet=$1 AND id_inv_log=$2 AND id_sto_inv=$3",
                )
                .bind(tenant_id)
                .bind(&row.id_inv_log)
                .bind(&row.id_sto_inv)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除初始账簿日志失败：{error}"))?;
                query::<Postgres>(
                    "DELETE FROM hi_sto_check_sub WHERE id_tet=$1 AND id=$2 AND id_sto_check=$3",
                )
                .bind(tenant_id)
                .bind(&row.id_check_sub)
                .bind(&storage.id_sto_check)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除首次盘点明细失败：{error}"))?;
                query::<Postgres>(
                    "DELETE FROM hi_sto_inv WHERE id_tet=$1 AND id_sto_inv=$2 AND id_sto=$3",
                )
                .bind(tenant_id)
                .bind(&row.id_sto_inv)
                .bind(&storage.id_sto)
                .execute(&mut *tx)
                .await
                .map_err(|error| format!("删除初始库存失败：{error}"))?;
            }
            query::<Postgres>(
                "DELETE FROM hi_sto_check WHERE id_tet=$1 AND id_sto_check=$2 AND id_sto=$3",
            )
            .bind(tenant_id)
            .bind(&storage.id_sto_check)
            .bind(&storage.id_sto)
            .execute(&mut *tx)
            .await
            .map_err(|error| format!("删除首次盘点主单失败：{error}"))?;
        }
        Ok::<_, String>(())
    }
    .await;
    match result {
        Ok(()) => tx
            .commit()
            .await
            .map_err(|error| format!("提交库存撤销事务失败：{error}")),
        Err(error) => {
            let rollback = tx.rollback().await.err().map(|value| value.to_string());
            Err(rollback
                .map(|value| format!("{error}；回滚异常：{value}"))
                .unwrap_or(error))
        }
    }?;
    pool.close().await;
    Ok(())
}
