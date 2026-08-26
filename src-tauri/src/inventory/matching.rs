fn inventory_target_medicine_from_values(
    value: impl Fn(usize) -> String,
) -> InventoryTargetMedicine {
    let private = value(10).trim() == "1";
    InventoryTargetMedicine {
        id_med_pro: value(0),
        id_med: value(1),
        drug_name: value(2),
        specification: value(3),
        minimum_unit: value(4),
        product_name: value(5),
        sale_specification: value(6),
        factory_name: value(7),
        external_code: value(8),
        approval_code: value(9),
        private,
        organization_id: if private { value(11) } else { String::new() },
    }
}

const INVENTORY_MEDICINE_CATALOG_SQL: &str =
    "SELECT p.id_med_pro,p.id_med,m.na_med,COALESCE(m.spec,''),COALESCE(m.unit_pre,''),\
     COALESCE(p.na_med_pro,''),COALESCE(p.spec_sale,''),COALESCE(f.na_fac,''),\
     COALESCE(p.cd_med_pro,''),COALESCE(p.cd_appr,''),COALESCE(p.fg_pri,'0'),\
     COALESCE(p.id_org_pri,'') FROM hi_bd_med_pro p \
     INNER JOIN hi_bd_med m ON m.id_med=p.id_med AND m.id_tet=p.id_tet \
     LEFT JOIN hi_bd_fac f ON f.id_fac=p.id_fac AND f.id_tet=p.id_tet \
     WHERE p.id_tet=? AND m.id_tet=? AND p.fg_active='1' AND m.fg_active='1'";

fn load_target_medicine_catalog_odbc(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    with_connection(profile, |connection| {
        configure_target_session(connection, profile)?;
        let rows = query_rows_strings(
            connection,
            INVENTORY_MEDICINE_CATALOG_SQL,
            vec![tenant_id.into(), tenant_id.into()],
            50_000,
        )?;
        Ok(rows
            .into_iter()
            .map(|row| {
                inventory_target_medicine_from_values(|index| {
                    row.get(index)
                        .and_then(Clone::clone)
                        .unwrap_or_default()
                        .trim()
                        .to_string()
                })
            })
            .collect())
    })
}

async fn load_target_medicine_catalog_pg(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    let pool = crate::pg_protocol::connect(profile).await?;
    let sql = INVENTORY_MEDICINE_CATALOG_SQL
        .replacen('?', "$1", 1)
        .replacen('?', "$2", 1);
    let result = query::<Postgres>(&sql)
        .bind(tenant_id)
        .bind(tenant_id)
        .fetch_all(&pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    inventory_target_medicine_from_values(|index| {
                        row.try_get::<String, _>(index)
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    })
                })
                .collect::<Vec<_>>()
        })
        .map_err(|error| format!("读取新系统药品目录失败：{error}"));
    pool.close().await;
    result
}

async fn load_target_medicine_catalog_mysql(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<InventoryTargetMedicine>, String> {
    let pool = connect_mysql(profile).await?;
    let result = query::<MySql>(INVENTORY_MEDICINE_CATALOG_SQL)
        .bind(tenant_id)
        .bind(tenant_id)
        .fetch_all(&pool)
        .await
        .map(|rows| {
            rows.into_iter()
                .map(|row| {
                    inventory_target_medicine_from_values(|index| {
                        row.try_get::<String, _>(index)
                            .unwrap_or_default()
                            .trim()
                            .to_string()
                    })
                })
                .collect::<Vec<_>>()
        })
        .map_err(|error| format!("读取新系统药品目录失败：{error}"));
    pool.close().await;
    result
}

pub async fn load_target_medicine_catalog(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<InventoryTargetMedicineCatalog, String> {
    let medicines = match inventory_target_backend(profile) {
        InventoryTargetBackend::PostgreSqlWire => {
            load_target_medicine_catalog_pg(profile, tenant_id).await?
        }
        InventoryTargetBackend::Odbc => load_target_medicine_catalog_odbc(profile, tenant_id)?,
        InventoryTargetBackend::MySql => {
            load_target_medicine_catalog_mysql(profile, tenant_id).await?
        }
    };
    if medicines.len() >= 50_000 {
        return Err(
            "新系统有效药品商品达到 50000 条，已停止目录匹配；请先清理停用或重复商品后重试"
                .into(),
        );
    }
    Ok(InventoryTargetMedicineCatalog {
        message: format!("已读取 {} 个有效新系统药品商品用于目录匹配", medicines.len()),
        medicines,
    })
}

fn inventory_row_text(row: &MigrationRow, field: &str) -> String {
    row.raw_data
        .get(field)
        .map(crate::normalize::value_text)
        .unwrap_or_default()
        .trim()
        .to_string()
}

pub async fn save_inventory_medicine_matches(
    store: &LocalStore,
    tenant_id: &str,
    operator_id: &str,
    request: SaveInventoryMedicineMatchesRequest,
) -> Result<SaveInventoryMedicineMatchesResponse, String> {
    if request.matches.is_empty() {
        return Err("请至少确认一个药品目录匹配结果".into());
    }
    let detail = store.load_batch(&request.batch_id)?;
    if detail.batch.source_type != "PHIS27_INVENTORY" {
        return Err("只能为二系列phis机构库存批次维护药品目录匹配".into());
    }
    let target_identity_value = target_identity(&request.target);
    let target_identity_text = target_identity_value.to_string();
    let batch_mapping = store.load_batch_mapping(&request.batch_id)?;
    if batch_mapping.get("targetIdentity") != Some(&target_identity_value) {
        return Err("目标数据库已变化，请重新读取机构库存后再匹配药品目录".into());
    }
    let catalog = load_target_medicine_catalog(&request.target, tenant_id).await?;
    let candidates = catalog
        .medicines
        .iter()
        .map(|medicine| (medicine.id_med_pro.as_str(), medicine))
        .collect::<HashMap<_, _>>();
    let mut sources = HashMap::<(String, String), &MigrationRow>::new();
    for row in &detail.rows {
        let source_product_key = inventory_row_text(row, "sourceProductKey");
        let organization_id = row
            .normalized_data
            .get("idOrg")
            .map(crate::normalize::value_text)
            .unwrap_or_default();
        if !source_product_key.is_empty() && !organization_id.is_empty() {
            sources
                .entry((source_product_key, organization_id))
                .or_insert(row);
        }
    }
    let mut saved = HashSet::new();
    for selection in &request.matches {
        let source_key = selection.source_product_key.trim();
        let target_org = selection.target_organization_id.trim();
        if !saved.insert((source_key.to_string(), target_org.to_string())) {
            return Err(format!("药品 {source_key} 在同一目标机构中存在重复匹配选择"));
        }
        let source_row = sources
            .get(&(source_key.to_string(), target_org.to_string()))
            .copied()
            .ok_or_else(|| format!("药品 {source_key} 不属于当前库存批次或目标机构已变化"))?;
        let target = candidates
            .get(selection.id_med_pro.trim())
            .copied()
            .ok_or_else(|| format!("目标药品商品 {} 已停用或不存在", selection.id_med_pro))?;
        if target.id_med != selection.id_med {
            return Err(format!(
                "目标药品商品 {} 的基础药品关系已变化，请重新读取候选",
                selection.id_med_pro
            ));
        }
        if target.private && target.organization_id != target_org {
            return Err(format!(
                "目标药品商品 {} 不属于当前目标机构",
                selection.id_med_pro
            ));
        }
        let match_method = match selection.match_method.trim() {
            "EXACT_AUTO" => "EXACT_AUTO",
            _ => "MANUAL",
        };
        let source_snapshot = serde_json::json!({
            "sourceProductKey":source_key,
            "drugName":inventory_row_text(source_row, "drugName"),
            "specification":inventory_row_text(source_row, "specification"),
            "factoryName":inventory_row_text(source_row, "factoryName"),
            "productName":inventory_row_text(source_row, "productName"),
            "targetOrganizationId":target_org
        });
        let target_snapshot = serde_json::to_value(target).map_err(|error| error.to_string())?;
        store.save_inventory_medicine_mapping(
            tenant_id,
            &detail.batch.source_name,
            source_key,
            &target_identity_text,
            target_org,
            &target.id_med,
            &target.id_med_pro,
            match_method,
            &source_snapshot,
            &target_snapshot,
        )?;
        store.audit_event(
            &request.batch_id,
            &source_row.row_id,
            "INVENTORY_MEDICINE_MATCH",
            "migration_inventory_medicine_mapping",
            &target.id_med_pro,
            "SUCCESS",
            Value::Null,
            serde_json::json!({
                "source":source_snapshot,
                "target":target_snapshot,
                "matchMethod":match_method,
                "targetIdentity":target_identity_value
            }),
            "已确认老系统药品与新系统药品商品目录关系",
            operator_id,
            &new_object_id(),
        )?;
    }
    Ok(SaveInventoryMedicineMatchesResponse {
        saved_count: saved.len(),
        message: format!("已保存 {} 项药品目录匹配，可重新核对本批库存", saved.len()),
    })
}
