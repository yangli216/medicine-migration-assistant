#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct InventoryValidationIssue {
    code: String,
    message: String,
    reviewable: bool,
}

impl InventoryValidationIssue {
    fn hard(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            reviewable: false,
        }
    }

    fn reviewable(code: &str, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            reviewable: true,
        }
    }
}

pub async fn load_target_storages(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<TargetStorageCatalog, String> {
    let storages = match inventory_target_backend(profile) {
        InventoryTargetBackend::PostgreSqlWire => {
            load_target_storages_pg(profile, tenant_id).await?
        }
        InventoryTargetBackend::Odbc => load_target_storages_odbc(profile, tenant_id)?,
        InventoryTargetBackend::MySql => {
            let pool = connect_mysql(profile).await?;
            let rows = query::<MySql>(
                "SELECT id_sto,na_sto,sd_sto,id_org FROM hi_sto_dept \
                 WHERE id_tet=? AND fg_active='1' ORDER BY sd_sto,na_sto",
            )
            .bind(tenant_id)
            .fetch_all(&pool)
            .await
            .map_err(|error| format!("读取新系统仓储失败：{error}"))?;
            pool.close().await;
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
                .collect()
        }
    };
    let medicine_storages = storages
        .iter()
        .filter(|storage| matches!(storage.storage_type.as_str(), "1" | "2"))
        .collect::<Vec<_>>();
    let organization_count = medicine_storages
        .iter()
        .map(|storage| storage.organization_id.as_str())
        .filter(|organization_id| !organization_id.is_empty())
        .collect::<HashSet<_>>()
        .len();
    Ok(TargetStorageCatalog {
        message: format!(
            "已读取 {} 个有效新系统仓储，其中 {} 个药库/药房，涉及 {} 个机构",
            storages.len(),
            medicine_storages.len(),
            organization_count
        ),
        storages,
    })
}

pub async fn prepare(
    store: &LocalStore,
    tenant_id: &str,
    request: PrepareInventoryRequest,
) -> Result<BatchDetail, String> {
    if request.source_name.trim().is_empty() {
        return Err("缺少库存来源数据库身份".into());
    }
    let adapter_id = source_adapter::inventory_adapter_id(&request.adapter_id)?;
    let adapter_name = source_adapter::inventory_adapter_name(adapter_id)?;
    let inventory_workflow = source_adapter::inventory_workflow(adapter_id)?;
    let catalog = load_target_storages(&request.target, tenant_id).await?;
    let storages = catalog
        .storages
        .iter()
        .map(|storage| (storage.id_sto.clone(), storage))
        .collect::<HashMap<_, _>>();
    let mapping_by_location = request
        .mappings
        .iter()
        .map(|mapping| (mapping.source_location_key.clone(), mapping))
        .collect::<HashMap<_, _>>();
    let target_identity_value = target_identity(&request.target);
    let target_identity_text = target_identity_value.to_string();
    validate_mappings(&request.organization_mappings, &request.mappings, &storages)?;
    let selected_organization_ids = request
        .organization_mappings
        .iter()
        .map(|mapping| mapping.source_organization_id.trim().to_string())
        .filter(|organization_id| !organization_id.is_empty())
        .collect::<HashSet<_>>();
    if selected_organization_ids.is_empty() {
        return Err("请至少选择并完整映射一个机构下的一个药库/药房".into());
    }
    let selected_location_keys = request
        .mappings
        .iter()
        .map(|mapping| mapping.source_location_key.trim().to_string())
        .filter(|location_key| !location_key.is_empty())
        .collect::<HashSet<_>>();
    if selected_location_keys.is_empty() {
        return Err("请至少选择一个本批需要迁移的药库或药房".into());
    }
    store.save_inventory_organization_mappings(
        tenant_id,
        request.source_name.trim(),
        &request.organization_mappings,
    )?;
    store.save_inventory_location_mappings(
        tenant_id,
        request.source_name.trim(),
        &target_identity_text,
        &request.mappings,
    )?;
    let source_items = select_inventory_items(
        source_adapter::load_inventory_stock_items(
            adapter_id,
            &request.source,
            &selected_organization_ids,
            &selected_location_keys,
        )?,
        &selected_organization_ids,
        &selected_location_keys,
    );
    if source_items.is_empty() {
        return Err("已选择的机构没有需要初始化的非零库存".into());
    }
    let mapped_location_keys = request
        .mappings
        .iter()
        .map(|mapping| mapping.source_location_key.as_str())
        .collect::<HashSet<_>>();
    let mut missing_location_names = source_items
        .iter()
        .filter(|item| !mapped_location_keys.contains(item.source_location_key.as_str()))
        .map(|item| item.source_location_name.clone())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if !missing_location_names.is_empty() {
        missing_location_names.sort();
        return Err(format!(
            "本批所选机构仍有未映射的药库/药房：{}",
            missing_location_names.join("、")
        ));
    }
    let groups = group_inventory(
        source_items,
        adapter_id,
        &inventory_workflow.source_stock_key_mode,
    )?;
    if groups.is_empty() {
        return Err("老系统没有需要初始化的非零库存".into());
    }
    let resolved_medicines = groups
        .iter()
        .map(|group| {
            let source_link = store.find_source_link(
                tenant_id,
                adapter_id,
                request.source_name.trim(),
                &group.source_product_key,
            )?;
            if let Some(link) = source_link.filter(|link| {
                !link.id_med.trim().is_empty() && !link.id_med_pro.trim().is_empty()
            }) {
                return Ok(ResolvedInventoryMedicine {
                    id_med: link.id_med,
                    id_med_unit: link.id_med_unit,
                    id_fac: link.id_fac,
                    id_med_pro: link.id_med_pro,
                });
            }
            let target_organization_id = mapping_by_location
                .get(&group.source_location_key)
                .map(|mapping| mapping.target_id_org.trim())
                .unwrap_or_default();
            if target_organization_id.is_empty() {
                return Ok(ResolvedInventoryMedicine::default());
            }
            let manual = store.find_inventory_medicine_mapping(
                tenant_id,
                request.source_name.trim(),
                &group.source_product_key,
                &target_identity_text,
                target_organization_id,
            )?;
            Ok(manual
                .map(|mapping| ResolvedInventoryMedicine {
                    id_med: mapping.id_med,
                    id_med_pro: mapping.id_med_pro,
                    ..ResolvedInventoryMedicine::default()
                })
                .unwrap_or_default())
        })
        .collect::<Result<Vec<_>, String>>()?;
    let duplicate_flags = target_duplicate_flags(
        &request.target,
        tenant_id,
        &groups,
        &resolved_medicines,
        &mapping_by_location,
    )
    .await?;
    let now = Utc::now().to_rfc3339();
    let batch_id = new_object_id();
    let mut rows = Vec::with_capacity(groups.len());
    let mut valid_count = 0usize;
    let mut fail_count = 0usize;
    let mut skip_count = 0usize;
    for (index, group) in groups.into_iter().enumerate() {
        let source_hash = inventory_source_hash(&group);
        let mapping = mapping_by_location.get(&group.source_location_key).copied();
        let medicine = resolved_medicines.get(index).cloned().unwrap_or_default();
        let previous = store.find_inventory_link(
            tenant_id,
            request.source_name.trim(),
            &group.source_stock_key,
            &target_identity_text,
        )?;
        let mut issues = Vec::new();
        if mapping.is_none() {
            issues.push(InventoryValidationIssue::hard(
                "STORAGE_MAPPING_REQUIRED",
                "库存位置尚未映射到新系统仓储",
            ));
        }
        if !medicine.complete() {
            issues.push(InventoryValidationIssue::hard(
                "MEDICINE_LEDGER_REQUIRED",
                format!(
                    "药品 {} 缺少完整的 id_med/id_med_pro 基础迁移台账",
                    group.source_product_key
                ),
            ));
        }
        if group.amount < Decimal::ZERO {
            issues.push(InventoryValidationIssue::hard(
                "NEGATIVE_INVENTORY",
                "库存数量小于 0，新系统不允许初始化负库存",
            ));
        }
        let unit_sale_factor = group.unit_sale_factor.trim().parse::<i64>().ok();
        if group.sale_unit.trim().is_empty() {
            issues.push(InventoryValidationIssue::hard(
                "INVENTORY_UNIT_REQUIRED",
                "未读取到库房/药房实际库存单位，不能确定数量和单价口径",
            ));
        }
        if unit_sale_factor.is_none_or(|factor| factor <= 0) {
            issues.push(InventoryValidationIssue::hard(
                "INVENTORY_FACTOR_INVALID",
                format!(
                    "库房/药房包装系数“{}”无效，不能确定库存数量和单价口径",
                    group.unit_sale_factor
                ),
            ));
        }
        let mut packaging_notes = Vec::new();
        let evidence_confirms_single_unit =
            structured_evidence_confirms_single_minimum_unit_package(
                &group.single_minimum_package_factor,
            );
        let package_evidence_source = if group
            .single_minimum_package_factor_source
            .trim()
            .is_empty()
        {
            "来源药品主档的一单位包装依据"
        } else {
            group.single_minimum_package_factor_source.trim()
        };
        if group.source_kind == "PHARMACY"
            && unit_sale_factor == Some(1)
            && !group.minimum_unit.trim().is_empty()
            && !group
                .sale_unit
                .trim()
                .eq_ignore_ascii_case(group.minimum_unit.trim())
        {
            if evidence_confirms_single_unit {
                packaging_notes.push(format!(
                    "{}=1，确认每{}仅含 1{}，药房包装系数 1 合法",
                    package_evidence_source, group.sale_unit, group.minimum_unit,
                ));
            } else {
                let evidence_factor = if group.single_minimum_package_factor.trim().is_empty() {
                    "未读取".to_string()
                } else {
                    group.single_minimum_package_factor.clone()
                };
                issues.push(InventoryValidationIssue::reviewable(
                    "PHARMACY_SINGLE_PACKAGE_UNCONFIRMED",
                    format!(
                        "药房包装系数为 1，但库存单位“{}”与最小单位“{}”不一致，而结构化依据“{}”当前值为“{}”（需为 1）；请核实库存位置包装与药品主档包装关系",
                        group.sale_unit,
                        group.minimum_unit,
                        package_evidence_source,
                        evidence_factor
                    ),
                ));
            }
        }
        if group.sale_unit != group.product_sale_unit
            || group.unit_sale_factor != group.product_unit_sale_factor
        {
            packaging_notes.push(format!(
                "已采用当前{}的实际包装：{}×{}；商品主档为 {}×{}",
                if group.source_kind == "PHARMACY" {
                    "药房"
                } else {
                    "药库"
                },
                group.sale_unit,
                group.unit_sale_factor,
                group.product_sale_unit,
                group.product_unit_sale_factor
            ));
        }
        for (label, source_total, calculated_total) in [
            ("进货", group.purchase_total, group.amount * group.price_pur),
            ("零售", group.retail_total, group.amount * group.price_sale),
        ] {
            if let Some(source_total) = source_total {
                let difference = decimal_distance(source_total, calculated_total);
                if difference > Decimal::new(5, 2) {
                    issues.push(InventoryValidationIssue::hard(
                        if label == "进货" {
                            "PURCHASE_AMOUNT_MISMATCH"
                        } else {
                            "RETAIL_AMOUNT_MISMATCH"
                        },
                        format!(
                            "来源{label}金额 {} 与库存数量×单价 {} 不一致（差额 {}）",
                            source_total, calculated_total, difference
                        ),
                    ));
                } else if difference > Decimal::new(1, 2) {
                    packaging_notes.push(format!(
                        "来源{label}金额存在 {} 的累计舍入差异，已保留来源数量与单价",
                        difference
                    ));
                }
            }
        }
        if previous
            .as_ref()
            .is_some_and(|link| link.source_hash != source_hash)
        {
            issues.push(InventoryValidationIssue::hard(
                "SOURCE_CHANGED_AFTER_MIGRATION",
                "该库存来源组已迁移但数量、价格、批号或效期发生变化，禁止重复覆盖",
            ));
        }
        let unchanged = previous
            .as_ref()
            .is_some_and(|link| link.source_hash == source_hash);
        if !unchanged && duplicate_flags.get(index).copied().unwrap_or(false) {
            issues.push(InventoryValidationIssue::hard(
                "TARGET_INVENTORY_DUPLICATE",
                "目标库已存在相同仓储、商品、价格、批号和效期的有效库存，已阻止重复初始化",
            ));
        }
        let status = if unchanged {
            skip_count += 1;
            "SKIPPED"
        } else if issues.is_empty() {
            valid_count += 1;
            "VALIDATED"
        } else {
            fail_count += 1;
            "INVALID"
        };
        let mut raw_data = Map::new();
        raw_data.insert(
            "sourceKind".into(),
            Value::String(group.source_kind.clone()),
        );
        raw_data.insert(
            "sourceRecordIds".into(),
            Value::Array(
                group
                    .source_record_ids
                    .iter()
                    .cloned()
                    .map(Value::String)
                    .collect(),
            ),
        );
        raw_data.insert(
            "sourceLocationKey".into(),
            Value::String(group.source_location_key.clone()),
        );
        raw_data.insert(
            "sourceLocationName".into(),
            Value::String(group.source_location_name.clone()),
        );
        if let Some(mapping) = mapping {
            raw_data.insert(
                "sourceOrganizationId".into(),
                Value::String(mapping.source_organization_id.clone()),
            );
            raw_data.insert(
                "resolvedSourceLocationKey".into(),
                Value::String(mapping.resolved_source_location_key.clone()),
            );
        }
        raw_data.insert(
            "sourceProductKey".into(),
            Value::String(group.source_product_key.clone()),
        );
        raw_data.insert("drugName".into(), Value::String(group.drug_name.clone()));
        raw_data.insert(
            "specification".into(),
            Value::String(group.specification.clone()),
        );
        raw_data.insert(
            "dosageForm".into(),
            Value::String(group.dosage_form.clone()),
        );
        raw_data.insert(
            "minimumUnit".into(),
            Value::String(group.minimum_unit.clone()),
        );
        raw_data.insert("saleUnit".into(), Value::String(group.sale_unit.clone()));
        raw_data.insert(
            "saleSpecification".into(),
            Value::String(group.sale_specification.clone()),
        );
        raw_data.insert(
            "unitSaleFactor".into(),
            Value::String(group.unit_sale_factor.clone()),
        );
        raw_data.insert(
            "singleMinimumPackageFactor".into(),
            Value::String(group.single_minimum_package_factor.clone()),
        );
        raw_data.insert(
            "singleMinimumPackageFactorSource".into(),
            Value::String(group.single_minimum_package_factor_source.clone()),
        );
        if adapter_id == "PHIS27" {
            raw_data.insert(
                "typkUnitSaleFactor".into(),
                Value::String(group.single_minimum_package_factor.clone()),
            );
        }
        raw_data.insert(
            "productSaleUnit".into(),
            Value::String(group.product_sale_unit.clone()),
        );
        raw_data.insert(
            "productUnitSaleFactor".into(),
            Value::String(group.product_unit_sale_factor.clone()),
        );
        raw_data.insert(
            "packagingNotes".into(),
            Value::Array(packaging_notes.into_iter().map(Value::String).collect()),
        );
        raw_data.insert(
            "inventoryValidationIssues".into(),
            serde_json::to_value(&issues).unwrap_or_else(|_| Value::Array(Vec::new())),
        );
        raw_data.insert(
            "factoryName".into(),
            Value::String(group.factory_name.clone()),
        );
        raw_data.insert(
            "productName".into(),
            Value::String(group.product_name.clone()),
        );
        let mut normalized_data = Map::new();
        normalized_data.insert("amount".into(), Value::String(group.amount.to_string()));
        normalized_data.insert(
            "pricePur".into(),
            Value::String(group.price_pur.to_string()),
        );
        normalized_data.insert(
            "priceSale".into(),
            Value::String(group.price_sale.to_string()),
        );
        normalized_data.insert("unitSale".into(), Value::String(group.sale_unit.clone()));
        normalized_data.insert(
            "unitSaleFactor".into(),
            Value::String(group.unit_sale_factor.clone()),
        );
        normalized_data.insert(
            "specSale".into(),
            Value::String(group.sale_specification.clone()),
        );
        normalized_data.insert(
            "purchaseTotal".into(),
            Value::String(
                group
                    .purchase_total
                    .unwrap_or(group.amount * group.price_pur)
                    .to_string(),
            ),
        );
        normalized_data.insert(
            "retailTotal".into(),
            Value::String(
                group
                    .retail_total
                    .unwrap_or(group.amount * group.price_sale)
                    .to_string(),
            ),
        );
        normalized_data.insert("batchCode".into(), Value::String(group.batch_code.clone()));
        normalized_data.insert(
            "effectiveDate".into(),
            Value::String(group.effective_date.clone()),
        );
        if let Some(mapping) = mapping {
            normalized_data.insert("idSto".into(), Value::String(mapping.target_id_sto.clone()));
            normalized_data.insert("idOrg".into(), Value::String(mapping.target_id_org.clone()));
            normalized_data.insert("naSto".into(), Value::String(mapping.target_name.clone()));
        }
        if medicine.complete() {
            normalized_data.insert("idMed".into(), Value::String(medicine.id_med.clone()));
            normalized_data.insert(
                "idMedUnit".into(),
                Value::String(medicine.id_med_unit.clone()),
            );
            normalized_data.insert(
                "idMedPro".into(),
                Value::String(medicine.id_med_pro.clone()),
            );
        }
        rows.push(MigrationRow {
            row_id: new_object_id(),
            batch_id: batch_id.clone(),
            row_no: index + 1,
            source_key: group.source_stock_key,
            source_hash,
            status: status.into(),
            raw_data,
            normalized_data,
            error_code: if issues.is_empty() {
                String::new()
            } else {
                "INVENTORY_PREFLIGHT".into()
            },
            error_message: issues
                .iter()
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join("；"),
            id_med: medicine.id_med,
            id_med_unit: medicine.id_med_unit,
            id_fac: medicine.id_fac,
            id_med_pro: medicine.id_med_pro,
            retry_count: 0,
            updated_at: now.clone(),
        });
    }
    let batch = MigrationBatch {
        batch_id: batch_id.clone(),
        batch_name: format!(
            "{}库存预检-{}机构-{}",
            adapter_name,
            selected_organization_ids.len(),
            Utc::now().format("%Y%m%d-%H%M")
        ),
        source_type: INSTITUTION_INVENTORY_SOURCE_TYPE.into(),
        source_name: request.source_name.trim().into(),
        source_description: format!(
            "{}非零库存；本批选择 {} 个机构；按仓储、商品、价格、批号、效期合并",
            adapter_name,
            selected_organization_ids.len()
        ),
        conflict_strategy: "FAIL".into(),
        allow_create_factory: false,
        idempotency_key: new_object_id(),
        status: if valid_count > 0 || skip_count > 0 {
            "VALIDATED".into()
        } else {
            "INVALID".into()
        },
        total_count: rows.len(),
        valid_count,
        success_count: 0,
        fail_count,
        skip_count,
        created_at: now.clone(),
        updated_at: now,
        finished_at: None,
    };
    store.insert_batch(
        &batch,
        &json!({
            "task":INSTITUTION_INVENTORY_SOURCE_TYPE,
            "targetIdentity":target_identity_value,
            "selectedOrganizationIds":selected_organization_ids,
            "organizationMappings":request.organization_mappings,
            "locationMappings":request.mappings
        })
        .to_string(),
    )?;
    for row in &rows {
        store.insert_row(row)?;
    }
    store.load_batch(&batch_id)
}
