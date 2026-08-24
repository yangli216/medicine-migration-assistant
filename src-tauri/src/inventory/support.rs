pub fn load_saved_mappings(
    store: &LocalStore,
    tenant_id: &str,
    source_name: &str,
    target: &ConnectionProfile,
) -> Result<Vec<InventoryLocationMapping>, String> {
    store.load_inventory_location_mappings(
        tenant_id,
        source_name,
        &target_identity(target).to_string(),
    )
}

pub fn load_saved_organization_mappings(
    store: &LocalStore,
    tenant_id: &str,
    source_name: &str,
) -> Result<Vec<InventoryOrganizationMapping>, String> {
    store.load_inventory_organization_mappings(tenant_id, source_name)
}

fn validate_mappings(
    organization_mappings: &[InventoryOrganizationMapping],
    mappings: &[InventoryLocationMapping],
    storages: &HashMap<String, &TargetStorage>,
) -> Result<(), String> {
    let organizations = organization_mappings
        .iter()
        .map(|mapping| {
            (
                mapping.source_organization_id.as_str(),
                mapping.target_organization_id.as_str(),
            )
        })
        .collect::<HashMap<_, _>>();
    for mapping in mappings {
        let mapped_organization = organizations
            .get(mapping.source_organization_id.as_str())
            .ok_or_else(|| {
                format!(
                    "老系统机构 {} 尚未映射到新系统机构",
                    mapping.source_organization_id
                )
            })?;
        if *mapped_organization != mapping.target_id_org {
            return Err(format!(
                "{}“{}”选择的目标库房不属于已映射的新系统机构",
                if mapping.source_kind == "WAREHOUSE" {
                    "药库"
                } else {
                    "药房"
                },
                mapping.source_location_name
            ));
        }
        if mapping.source_kind == "WAREHOUSE"
            && mapping.source_location_key.starts_with("YKORG:")
            && mapping.resolved_source_location_key.trim().is_empty()
        {
            return Err(format!(
                "机构 {} 的 YK_KCMX 未保存药库主键，请先指定这批库存属于哪个老系统药库",
                mapping.source_organization_id
            ));
        }
        let storage = storages
            .get(&mapping.target_id_sto)
            .ok_or_else(|| format!("目标仓储 {} 已停用或不存在", mapping.target_name))?;
        let expected_type = if mapping.source_kind == "WAREHOUSE" {
            "1"
        } else {
            "2"
        };
        if storage.storage_type != expected_type {
            return Err(format!(
                "{}“{}”只能映射到新系统{}",
                if mapping.source_kind == "WAREHOUSE" {
                    "药库"
                } else {
                    "药房"
                },
                mapping.source_location_name,
                storage_type_name(expected_type)
            ));
        }
        if storage.organization_id != mapping.target_id_org {
            return Err(format!(
                "目标仓储“{}”的机构信息已变化，请重新选择",
                storage.name
            ));
        }
    }
    Ok(())
}

fn load_target_storages_odbc(
    profile: &ConnectionProfile,
    tenant_id: &str,
) -> Result<Vec<TargetStorage>, String> {
    with_connection(profile, |connection| {
        configure_target_session(connection, profile)?;
        connection
            .execute(
                "SELECT id_sto,na_sto,sd_sto,id_org,id_tet,fg_active FROM hi_sto_dept WHERE 1=0",
                (),
                Some(30),
            )
            .map_err(|error| format!("目标仓储表结构检查失败：{error}"))?;
        let rows = query_rows_strings(
            connection,
            "SELECT id_sto,na_sto,sd_sto,id_org FROM hi_sto_dept \
             WHERE id_tet=? AND fg_active='1' ORDER BY sd_sto,na_sto",
            vec![tenant_id.into()],
            2_000,
        )?;
        Ok(rows
            .into_iter()
            .map(|row| {
                let value = |index: usize| {
                    row.get(index)
                        .and_then(Clone::clone)
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
            .collect())
    })
}

async fn target_duplicate_flags(
    profile: &ConnectionProfile,
    tenant_id: &str,
    groups: &[InventoryGroup],
    mappings: &HashMap<String, &InventoryLocationMapping>,
) -> Result<Vec<bool>, String> {
    if crate::odbc::is_odbc_kind(&profile.kind) {
        return with_connection(profile, |connection| {
            configure_target_session(connection, profile)?;
            let date_text_sql = inventory_date_text_sql(&profile.kind, "dt_effect");
            for (table, columns) in [
                ("hi_sto_med", "id_sto_med,id_med_pro,id_sto,id_org,id_tet"),
                ("hi_sto_inv", "id_sto_inv,id_med_pro,id_sto,id_org,id_tet"),
            ] {
                connection
                    .execute(
                        &format!("SELECT {columns} FROM {table} WHERE 1=0"),
                        (),
                        Some(30),
                    )
                    .map_err(|error| format!("目标库存表结构检查失败（{table}）：{error}"))?;
            }
            groups
                .iter()
                .map(|group| {
                    let Some(mapping) = mappings.get(&group.source_location_key) else {
                        return Ok(false);
                    };
                    query_optional_string(
                        connection,
                        &format!("SELECT id_sto_inv FROM hi_sto_inv WHERE id_tet=? AND id_org=? AND id_sto=? \
                         AND id_med_pro IN (SELECT id_med_pro FROM hi_bd_med_pro WHERE cd_med_pro=? AND id_tet=? AND fg_active='1') \
                         AND price_sale=? AND price_pur=? AND COALESCE(cd_batch,'')=? \
                         AND {date_text_sql}=? AND fg_active='1'"),
                        vec![
                            tenant_id.into(), mapping.target_id_org.clone(), mapping.target_id_sto.clone(),
                            group.source_product_key.clone(), tenant_id.into(), group.price_sale.to_string(),
                            group.price_pur.to_string(), group.batch_code.clone(), group.effective_date.clone(),
                        ],
                    )
                    .map(|value| value.is_some())
                })
                .collect()
        });
    }
    let pool = connect_mysql(profile).await?;
    let mut flags = Vec::with_capacity(groups.len());
    for group in groups {
        let Some(mapping) = mappings.get(&group.source_location_key) else {
            flags.push(false);
            continue;
        };
        let existing = query_scalar::<MySql, String>(
            "SELECT i.id_sto_inv FROM hi_sto_inv i INNER JOIN hi_bd_med_pro p ON p.id_med_pro=i.id_med_pro \
             WHERE i.id_tet=? AND i.id_org=? AND i.id_sto=? AND p.cd_med_pro=? AND p.id_tet=? \
             AND p.fg_active='1' AND i.price_sale=? AND i.price_pur=? AND COALESCE(i.cd_batch,'')=? \
             AND COALESCE(DATE_FORMAT(i.dt_effect,'%Y-%m-%d'),'')=? AND i.fg_active='1' LIMIT 1",
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
    pool.close().await;
    Ok(flags)
}

fn inventory_date_text_sql(kind: &str, column: &str) -> String {
    match crate::odbc::normalize_kind(kind).as_str() {
        "oracle" | "dameng" => {
            format!("COALESCE(TO_CHAR({column},'YYYY-MM-DD'),'')")
        }
        "gbase8s" => format!("COALESCE(TO_CHAR({column},'%Y-%m-%d'),'')"),
        "gbase8a" => format!("COALESCE(DATE_FORMAT({column},'%Y-%m-%d'),'')"),
        _ => format!("COALESCE(CAST({column} AS VARCHAR(10)),'')"),
    }
}

fn group_inventory(items: Vec<Phis27InventoryStockItem>) -> Result<Vec<InventoryGroup>, String> {
    let mut groups = BTreeMap::<String, InventoryGroup>::new();
    for item in items {
        let amount = parse_decimal(&item.amount, "库存数量")?;
        let price_pur = parse_decimal(&item.price_pur, "进货价格")?;
        let price_sale = parse_decimal(&item.price_sale, "零售价格")?;
        let purchase_total = parse_optional_decimal(&item.purchase_total, "进货金额")?;
        let retail_total = parse_optional_decimal(&item.retail_total, "零售金额")?;
        let key = format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            item.source_kind,
            item.source_location_key,
            item.source_product_key,
            item.sale_unit,
            item.sale_specification,
            item.unit_sale_factor,
            price_pur,
            price_sale,
            item.batch_code,
            item.effective_date
        );
        if let Some(group) = groups.get_mut(&key) {
            group.amount += amount;
            group.purchase_total = sum_optional_decimal(group.purchase_total, purchase_total);
            group.retail_total = sum_optional_decimal(group.retail_total, retail_total);
            group.source_record_ids.push(item.source_record_id);
        } else {
            groups.insert(
                key,
                InventoryGroup {
                    source_kind: item.source_kind,
                    source_stock_key: String::new(),
                    source_location_key: item.source_location_key,
                    source_location_name: item.source_location_name,
                    source_product_key: item.source_product_key,
                    source_record_ids: vec![item.source_record_id],
                    drug_name: item.drug_name,
                    specification: item.specification,
                    dosage_form: item.dosage_form,
                    minimum_unit: item.minimum_unit,
                    sale_unit: item.sale_unit,
                    sale_specification: item.sale_specification,
                    unit_sale_factor: item.unit_sale_factor,
                    product_sale_unit: item.product_sale_unit,
                    product_unit_sale_factor: item.product_unit_sale_factor,
                    factory_name: item.factory_name,
                    product_name: item.product_name,
                    amount,
                    price_pur,
                    price_sale,
                    purchase_total,
                    retail_total,
                    batch_code: item.batch_code,
                    effective_date: item.effective_date,
                },
            );
        }
    }
    let mut result = groups.into_values().collect::<Vec<_>>();
    for group in &mut result {
        group.source_record_ids.sort();
        group.source_stock_key = format!(
            "{}:{}",
            if group.source_kind == "WAREHOUSE" {
                "YK"
            } else {
                "YF"
            },
            group.source_record_ids.first().cloned().unwrap_or_default()
        );
    }
    Ok(result)
}

fn select_inventory_items(
    items: Vec<Phis27InventoryStockItem>,
    selected_organization_ids: &HashSet<String>,
) -> Vec<Phis27InventoryStockItem> {
    items
        .into_iter()
        .filter(|item| selected_organization_ids.contains(&item.source_organization_id))
        .collect()
}

fn parse_decimal(value: &str, label: &str) -> Result<Decimal, String> {
    let normalized = value.trim().replace(',', ".");
    if normalized.is_empty() {
        return Err(format!("{label}为空"));
    }
    Decimal::from_str(&normalized).map_err(|_| format!("{label}“{value}”不是有效数值"))
}

fn parse_optional_decimal(value: &str, label: &str) -> Result<Option<Decimal>, String> {
    if value.trim().is_empty() {
        return Ok(None);
    }
    parse_decimal(value, label).map(Some)
}

fn sum_optional_decimal(left: Option<Decimal>, right: Option<Decimal>) -> Option<Decimal> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left + right),
        _ => None,
    }
}

fn decimal_distance(left: Decimal, right: Decimal) -> Decimal {
    if left >= right {
        left - right
    } else {
        right - left
    }
}

fn is_single_minimum_unit_package(
    specification: &str,
    minimum_unit: &str,
    sale_unit: &str,
) -> bool {
    let minimum_unit = minimum_unit.trim();
    let sale_unit = sale_unit.trim();
    if specification.trim().is_empty() || minimum_unit.is_empty() || sale_unit.is_empty() {
        return false;
    }
    let normalized = specification
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>()
        .replace('×', "*")
        .replace('／', "/");
    [
        format!("1{minimum_unit}/{sale_unit}"),
        format!("1{minimum_unit}装/{sale_unit}"),
        format!("每{sale_unit}1{minimum_unit}"),
        format!("1{minimum_unit}*1{sale_unit}"),
    ]
    .iter()
    .any(|pattern| {
        normalized.match_indices(pattern).any(|(index, _)| {
            normalized[..index]
                .chars()
                .next_back()
                .is_none_or(|character| !character.is_ascii_digit() && character != '.')
        })
    })
}

fn inventory_source_hash(group: &InventoryGroup) -> String {
    format!(
        "{:x}",
        Sha256::digest(
            json!({
                "sourceRecordIds":group.source_record_ids,
                "location":group.source_location_key,
                "product":group.source_product_key,
                "amount":group.amount.to_string(),
                "pricePur":group.price_pur.to_string(),
                "priceSale":group.price_sale.to_string(),
                "unitSale":group.sale_unit,
                "specSale":group.sale_specification,
                "unitSaleFactor":group.unit_sale_factor,
                "purchaseTotal":group.purchase_total.map(|value| value.to_string()),
                "retailTotal":group.retail_total.map(|value| value.to_string()),
                "batch":group.batch_code,
                "effectiveDate":group.effective_date
            })
            .to_string()
            .as_bytes()
        )
    )
}

fn storage_type_name(value: &str) -> String {
    match value.trim() {
        "1" => "药库",
        "2" => "药房",
        "3" => "库房",
        "4" => "科室库房",
        _ => "其他仓储",
    }
    .into()
}

#[cfg(test)]
mod tests {
    use super::{
        group_inventory, has_successful_inventory_trials, inventory_date_parameter_sql,
        inventory_date_text_sql, inventory_storage_hash, inventory_undo_preview,
        is_single_minimum_unit_package, next_check_number, select_inventory_items,
        storage_type_name,
        InventoryUndoRow, InventoryUndoStorage, Phis27InventoryStockItem,
    };
    use crate::model::{BatchDetail, ConnectionProfile};
    use rust_decimal::Decimal;
    use serde_json::json;
    use std::collections::HashSet;

    fn item(record: &str, amount: &str) -> Phis27InventoryStockItem {
        Phis27InventoryStockItem {
            source_kind: "WAREHOUSE".into(),
            source_record_id: record.into(),
            source_location_key: "YK:ORG".into(),
            source_location_name: "中心药库".into(),
            source_organization_id: "ORG".into(),
            source_product_key: "100:200".into(),
            drug_name: "测试药品".into(),
            specification: "10mg".into(),
            dosage_form: "1".into(),
            minimum_unit: "片".into(),
            sale_unit: "盒".into(),
            sale_specification: "10mg*12片/盒".into(),
            unit_sale_factor: "12".into(),
            product_sale_unit: "盒".into(),
            product_unit_sale_factor: "12".into(),
            factory_name: "测试药厂".into(),
            product_name: "测试商品名".into(),
            amount: amount.into(),
            price_pur: "1.20".into(),
            price_sale: "2.30".into(),
            purchase_total: (amount.parse::<Decimal>().unwrap() * Decimal::new(120, 2)).to_string(),
            retail_total: (amount.parse::<Decimal>().unwrap() * Decimal::new(230, 2)).to_string(),
            batch_code: "B01".into(),
            effective_date: "2027-12-31".into(),
        }
    }

    #[test]
    fn identical_inventory_identity_is_combined_before_duplicate_check() {
        let groups = group_inventory(vec![item("2", "3"), item("1", "4.5")]).unwrap();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].amount.to_string(), "7.5");
        assert_eq!(groups[0].source_stock_key, "YK:1");
        assert_eq!(groups[0].source_record_ids, vec!["1", "2"]);
    }

    #[test]
    fn different_storage_packaging_is_never_combined() {
        let boxed = item("1", "4");
        let mut split = item("2", "48");
        split.sale_unit = "片".into();
        split.unit_sale_factor = "1".into();
        split.purchase_total = "57.60".into();
        split.retail_total = "110.40".into();

        let groups = group_inventory(vec![boxed, split]).unwrap();

        assert_eq!(groups.len(), 2);
        assert!(groups.iter().any(|group| group.unit_sale_factor == "12"));
        assert!(groups.iter().any(|group| group.unit_sale_factor == "1"));
    }

    #[test]
    fn one_bottle_or_capsule_per_box_is_a_valid_factor_one_package() {
        assert!(is_single_minimum_unit_package("100ml×1瓶/盒", "瓶", "盒"));
        assert!(is_single_minimum_unit_package("1粒/盒", "粒", "盒"));
        assert!(is_single_minimum_unit_package("每盒1支", "支", "盒"));
    }

    #[test]
    fn multiple_minimum_units_cannot_be_misread_as_a_single_unit_package() {
        assert!(!is_single_minimum_unit_package("11粒/盒", "粒", "盒"));
        assert!(!is_single_minimum_unit_package("10ml×12支/盒", "支", "盒"));
    }

    #[test]
    fn target_storage_type_uses_hi_sto_dept_dictionary_codes() {
        assert_eq!(storage_type_name("1"), "药库");
        assert_eq!(storage_type_name("2"), "药房");
        assert_eq!(storage_type_name("3"), "库房");
        assert_eq!(storage_type_name("4"), "科室库房");
        assert_eq!(storage_type_name(" 2 "), "药房");
    }

    #[test]
    fn inventory_batch_can_select_one_completed_organization() {
        let selected = HashSet::from(["ORG".to_string()]);
        let first = item("1", "4");
        let mut deferred = item("2", "5");
        deferred.source_organization_id = "ORG_LATER".into();
        deferred.source_location_key = "YF:2001".into();

        let items = select_inventory_items(vec![first, deferred], &selected);

        assert_eq!(items.len(), 1);
        assert_eq!(items[0].source_organization_id, "ORG");
    }

    #[test]
    fn first_stocktake_number_starts_at_001_and_increments_for_the_day() {
        assert_eq!(next_check_number("20260810", None).unwrap(), "20260810001");
        assert_eq!(
            next_check_number("20260810", Some("20260810009")).unwrap(),
            "20260810010"
        );
        assert_eq!(
            next_check_number("20260810", Some("20260809077")).unwrap(),
            "20260810001"
        );
    }

    #[test]
    fn first_stocktake_number_rejects_daily_overflow() {
        assert!(next_check_number("20260810", Some("20260810999")).is_err());
    }

    #[test]
    fn first_stocktake_sequence_is_scoped_and_locked_by_target_storage() {
        let source = concat!(include_str!("write.rs"), include_str!("undo.rs"));
        assert!(source.contains("id_tet=? AND id_sto=? AND cd_sto_check LIKE ?"));
        assert!(source.contains("sd_sto IN ('1','2') FOR UPDATE"));
    }

    #[test]
    fn inventory_date_binding_uses_the_selected_database_dialect() {
        assert!(inventory_date_parameter_sql("oracle").starts_with("TO_DATE"));
        assert!(inventory_date_parameter_sql("DM8").starts_with("TO_DATE"));
        assert!(inventory_date_parameter_sql("postgresql").starts_with("CAST"));
        assert!(inventory_date_parameter_sql("GBase-8a").starts_with("CAST"));
        assert!(inventory_date_parameter_sql("GBase-8s").contains("%Y-%m-%d"));
    }

    #[test]
    fn inventory_duplicate_check_formats_dates_with_the_selected_odbc_dialect() {
        assert!(inventory_date_text_sql("oracle", "dt_effect")
            .contains("TO_CHAR(dt_effect,'YYYY-MM-DD')"));
        assert!(inventory_date_text_sql("DM8", "dt_effect")
            .contains("TO_CHAR(dt_effect,'YYYY-MM-DD')"));
        assert!(inventory_date_text_sql("GBase-8a", "dt_effect")
            .contains("DATE_FORMAT(dt_effect,'%Y-%m-%d')"));
        assert!(inventory_date_text_sql("GBase-8s", "dt_effect")
            .contains("TO_CHAR(dt_effect,'%Y-%m-%d')"));
        assert_eq!(
            inventory_date_text_sql("postgresql", "dt_effect"),
            "COALESCE(CAST(dt_effect AS VARCHAR(10)),'')"
        );
    }

    #[test]
    fn inventory_undo_preview_blocks_the_entire_batch_when_one_storage_changed() {
        let storages = vec![InventoryUndoStorage {
            id_sto: "sto-1".into(),
            name: "中心药库".into(),
            id_sto_check: "check-1".into(),
            cd_sto_check: "20260810001".into(),
            rows: vec![InventoryUndoRow {
                row_id: "row-1".into(),
                id_sto_inv: "inv-1".into(),
                id_inv_log: "log-1".into(),
                id_check_sub: "sub-1".into(),
                expected_amount: Decimal::new(10, 0),
            }],
        }];
        let preview = inventory_undo_preview(
            "batch-1",
            &storages,
            vec![vec!["已产生后续库存变动".into()]],
        );
        assert!(!preview.can_undo);
        assert_eq!(preview.blocker_count, 1);
        assert_eq!(preview.inventory_count, 1);
        assert!(!preview.storages[0].can_undo);
    }

    #[test]
    fn inventory_undo_uses_reverse_dependency_order_and_retains_storage_medicine() {
        let source = concat!(
            include_str!("write.rs"),
            include_str!("undo.rs"),
            include_str!("pg.rs")
        );
        let log = source.find("DELETE FROM hi_sto_inv_log").unwrap();
        let detail = source.find("DELETE FROM hi_sto_check_sub").unwrap();
        let inventory = source.find("DELETE FROM hi_sto_inv WHERE").unwrap();
        let header = source.find("DELETE FROM hi_sto_check WHERE").unwrap();
        assert!(log < detail && detail < inventory && inventory < header);
        assert!(source.contains("\"retainedTable\":\"hi_sto_med\""));
    }

    #[test]
    fn inventory_contract_covers_every_business_write_and_undo_table() {
        let contract = super::INVENTORY_TARGET_TABLE_PROJECTIONS;
        for table in [
            "hi_sto_dept",
            "hi_bd_med_unit",
            "hi_bd_med_pro",
            "hi_sto_med",
            "hi_sto_check",
            "hi_sto_check_sub",
            "hi_sto_inv",
            "hi_sto_inv_log",
        ] {
            assert!(contract.iter().any(|(candidate, _)| *candidate == table));
        }
        let inventory = contract
            .iter()
            .find(|(table, _)| *table == "hi_sto_inv")
            .unwrap()
            .1;
        assert!(inventory.contains("price_sale"));
        assert!(inventory.contains("dt_effect"));
        assert!(!inventory.contains("memo"));
    }

    #[test]
    fn native_pg_inventory_uses_pg_placeholders_and_safe_undo_order() {
        let source = include_str!("pg.rs");
        assert!(source.contains("id_tet=$1"));
        assert!(!source.contains("id_tet=?"));
        let log = source.find("DELETE FROM hi_sto_inv_log").unwrap();
        let detail = source.find("DELETE FROM hi_sto_check_sub").unwrap();
        let inventory = source.find("DELETE FROM hi_sto_inv WHERE").unwrap();
        let header = source.find("DELETE FROM hi_sto_check WHERE").unwrap();
        assert!(log < detail && detail < inventory && inventory < header);
    }

    #[test]
    fn every_target_storage_needs_a_current_successful_rolled_back_trial() {
        let profile: ConnectionProfile = serde_json::from_value(json!({
            "kind":"postgresql","host":"db.example","port":5432,"database":"phis",
            "username":"writer","password":"","schema":"public","serviceName":"",
            "driver":"","connectionString":""
        }))
        .unwrap();
        let mut detail: BatchDetail = serde_json::from_value(json!({
            "batch":{
                "batchId":"batch-1","batchName":"库存","sourceType":"PHIS27_INVENTORY",
                "sourceName":"source","sourceDescription":"inventory","conflictStrategy":"FAIL",
                "allowCreateFactory":false,"idempotencyKey":"key","status":"VALIDATED",
                "totalCount":2,"validCount":2,"successCount":0,"failCount":0,"skipCount":0,
                "createdAt":"2026-08-21T00:00:00Z","updatedAt":"2026-08-21T00:00:00Z","finishedAt":null
            },
            "rows":[
                {
                    "rowId":"row-1","batchId":"batch-1","rowNo":1,"sourceKey":"1:1",
                    "sourceHash":"hash-1","status":"VALIDATED","rawData":{},
                    "normalizedData":{"idSto":"sto-1","idOrg":"org-1"},"errorCode":"",
                    "errorMessage":"","idMed":"","idMedUnit":"","idFac":"","idMedPro":"",
                    "retryCount":0,"updatedAt":"2026-08-21T00:00:00Z"
                },
                {
                    "rowId":"row-2","batchId":"batch-1","rowNo":2,"sourceKey":"2:2",
                    "sourceHash":"hash-2","status":"VALIDATED","rawData":{},
                    "normalizedData":{"idSto":"sto-2","idOrg":"org-1"},"errorCode":"",
                    "errorMessage":"","idMed":"","idMedUnit":"","idFac":"","idMedPro":"",
                    "retryCount":0,"updatedAt":"2026-08-21T00:00:00Z"
                }
            ],
            "audits":[]
        }))
        .unwrap();
        for (index, id_sto) in ["sto-1", "sto-2"].into_iter().enumerate() {
            let storage_hash = inventory_storage_hash(&detail.rows, id_sto);
            detail
                .audits
                .push(serde_json::from_value(json!({
                    "auditId":format!("audit-{index}"),"batchId":"batch-1",
                    "rowId":format!("row-{}",index+1),"traceId":format!("trace-{index}"),
                    "operation":"INVENTORY_TRIAL_ROLLBACK","targetTable":"hi_sto_check",
                    "targetId":id_sto,"result":"SUCCESS","beforeData":null,
                    "afterData":{
                        "targetIdentity":super::target_identity(&profile),"idSto":id_sto,
                        "storageHash":storage_hash,"rolledBack":true
                    },
                    "message":"已回滚","operatorId":"operator",
                    "operatedAt":format!("2026-08-21T0{}:00:00Z",index+1)
                }))
                .unwrap());
        }
        assert!(has_successful_inventory_trials(&detail, &profile));

        detail.audits.push(
            serde_json::from_value(json!({
                "auditId":"audit-failed","batchId":"batch-1","rowId":"row-2","traceId":"trace-failed",
                "operation":"INVENTORY_TRIAL_ROLLBACK","targetTable":"hi_sto_check","targetId":"sto-2",
                "result":"FAILED","beforeData":null,
                "afterData":{"targetIdentity":super::target_identity(&profile),"idSto":"sto-2","rolledBack":true},
                "message":"验证失败但已回滚","operatorId":"operator","operatedAt":"2026-08-21T09:00:00Z"
            }))
            .unwrap(),
        );
        assert!(!has_successful_inventory_trials(&detail, &profile));
    }
}
