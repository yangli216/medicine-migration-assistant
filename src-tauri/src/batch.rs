use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{BatchDetail, MigrationBatch, MigrationRow, PrepareBatchRequest};
use crate::normalize::{apply_cost_merge_mapping, normalize, validate};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};

const MAX_BATCH_ROWS: usize = 10_000;

pub fn prepare_batch(
    store: &LocalStore,
    request: PrepareBatchRequest,
    dictionary_values: &HashMap<String, HashSet<String>>,
    tenant_id: &str,
) -> Result<BatchDetail, String> {
    let is_phis27 = request.source_type.eq_ignore_ascii_case("PHIS27");
    if request.rows.is_empty() {
        return Err("没有可迁移的三方药品数据".into());
    }
    if request.rows.len() > MAX_BATCH_ROWS {
        return Err(format!("单批最多迁移 {} 行，请拆分批次", MAX_BATCH_ROWS));
    }
    let strategy = request.conflict_strategy.trim().to_ascii_uppercase();
    if !["INCREMENTAL", "OVERWRITE", "REUSE", "FAIL"].contains(&strategy.as_str()) {
        return Err("迁移模式无效".into());
    }
    if let Some(batch_id) = store.find_by_idempotency_key(request.idempotency_key.trim())? {
        return store.load_batch(&batch_id);
    }
    let now = Utc::now().to_rfc3339();
    let batch_id = new_object_id();
    let mut rows = Vec::with_capacity(request.rows.len());
    let mut valid_count = 0;
    let mut skip_count = 0;
    for (index, source) in request.rows.iter().enumerate() {
        let mut normalized = normalize(source, &request.mappings);
        if is_phis27 {
            if let Some(source_factory_id) = source.get("SOURCE_FACTORY_ID") {
                let source_factory_key = crate::normalize::value_text(source_factory_id);
                if !source_factory_key.is_empty() {
                    normalized.insert(
                        "_sourceFactoryKey".into(),
                        serde_json::Value::String(source_factory_key),
                    );
                }
            }
        }
        apply_cost_merge_mapping(&mut normalized, &request.cost_merge_mappings);
        let mut errors = validate(&normalized);
        errors.extend(crate::target_dictionary::validate_values(
            &normalized,
            dictionary_values,
        ));
        if is_phis27 {
            errors.extend(phis27_identity_errors(source, &normalized));
        }
        if is_phis27 && source_duplicate_count(source) > 1 {
            normalized.insert(
                "_baseMergeGroupSize".into(),
                Value::String(source_duplicate_count(source).to_string()),
            );
            normalized.insert(
                "_baseMergeKey".into(),
                Value::String(phis27_base_merge_key(source)),
            );
        }
        let raw_json = Value::Object(source.clone()).to_string();
        let source_hash = format!("{:x}", Sha256::digest(raw_json.as_bytes()));
        let source_key = source
            .get("_sourceKey")
            .map(crate::normalize::value_text)
            .filter(|text| !text.is_empty())
            .unwrap_or_else(|| (index + 1).to_string());
        let prior_link = if strategy == "INCREMENTAL" || strategy == "OVERWRITE" {
            store.find_source_link(
                tenant_id,
                request.source_type.trim(),
                request.source_name.trim(),
                &source_key,
            )?
        } else {
            None
        };
        let unchanged = errors.is_empty()
            && prior_link.as_ref().map(|link| link.source_hash.as_str())
                == Some(source_hash.as_str());
        if strategy == "INCREMENTAL" && errors.is_empty() && prior_link.is_some() && !unchanged {
            errors.push("该来源主键已迁移但源数据发生变化；请确认差异后选择覆盖迁移".into());
        }
        let mut previous_ids = (String::new(), String::new(), String::new(), String::new());
        if strategy == "OVERWRITE" && !unchanged {
            if let Some(link) = &prior_link {
                previous_ids = (
                    link.id_med.clone(),
                    link.id_med_unit.clone(),
                    link.id_fac.clone(),
                    link.id_med_pro.clone(),
                );
                if link.id_med.is_empty() || !link.manages("hi_bd_med", &link.id_med) {
                    errors.push("原药品记录不是由本工具创建或覆盖，禁止直接改写共享主数据".into());
                }
                let previous_has_product = !link.id_med_pro.is_empty();
                let current_has_product = crate::normalize::has_product_data(&normalized);
                if previous_has_product != current_has_product {
                    errors.push(
                        "覆盖迁移暂不允许在“仅基础药品”和“含厂家商品”之间改变数据层级".into(),
                    );
                }
                if previous_has_product && !link.manages("hi_bd_med_pro", &link.id_med_pro) {
                    errors.push("原药品商品不是由本工具创建或覆盖，禁止直接改写".into());
                }
            }
        }
        let status = if unchanged {
            skip_count += 1;
            "SKIPPED"
        } else if errors.is_empty() {
            valid_count += 1;
            "VALIDATED"
        } else {
            "INVALID"
        };
        rows.push(MigrationRow {
            row_id: new_object_id(),
            batch_id: batch_id.clone(),
            row_no: index + 1,
            source_key,
            source_hash,
            status: status.into(),
            raw_data: source.clone(),
            normalized_data: normalized,
            error_code: if errors.is_empty() {
                String::new()
            } else {
                "VALIDATION_ERROR".into()
            },
            error_message: errors.join("；"),
            id_med: previous_ids.0,
            id_med_unit: previous_ids.1,
            id_fac: previous_ids.2,
            id_med_pro: previous_ids.3,
            retry_count: 0,
            updated_at: now.clone(),
        });
    }
    let invalid_count = rows.len() - valid_count - skip_count;
    let batch = MigrationBatch {
        batch_id: batch_id.clone(),
        batch_name: if request.batch_name.trim().is_empty() {
            format!("药品迁移-{}", Utc::now().format("%Y%m%d-%H%M"))
        } else {
            request.batch_name
        },
        source_type: if request.source_type.is_empty() {
            "FILE".into()
        } else {
            request.source_type
        },
        source_name: if request.source_name.is_empty() {
            "第三方数据源".into()
        } else {
            request.source_name
        },
        source_description: request.source_description,
        conflict_strategy: strategy,
        allow_create_factory: request.allow_create_factory || is_phis27,
        idempotency_key: request.idempotency_key,
        status: if valid_count > 0 || skip_count > 0 {
            "VALIDATED".into()
        } else {
            "INVALID".into()
        },
        total_count: rows.len(),
        valid_count,
        success_count: 0,
        fail_count: invalid_count,
        skip_count,
        created_at: now.clone(),
        updated_at: now,
        finished_at: None,
    };
    store.insert_prepared_batch(
        &batch,
        &serde_json::to_string(&json!({
            "fieldMappings": request.mappings,
            "costMergeMappings": request.cost_merge_mappings
        }))
        .map_err(|error| error.to_string())?,
        &rows,
    )?;
    let trace_id = new_object_id();
    store.audit_event(
        &batch_id,
        "",
        "PREPARE",
        "migration_batch",
        &batch_id,
        &batch.status,
        Value::Null,
        json!(batch),
        &format!(
            "暂存并校验完成：有效{}行，增量跳过{}行，无效{}行",
            valid_count, skip_count, invalid_count
        ),
        "",
        &trace_id,
    )?;
    store.load_batch(&batch_id)
}

fn source_duplicate_count(source: &serde_json::Map<String, Value>) -> usize {
    source
        .get("SOURCE_DUPLICATE_COUNT")
        .or_else(|| source.get("sourceDuplicateCount"))
        .map(crate::normalize::value_text)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(0)
}

fn phis27_base_merge_key(source: &serde_json::Map<String, Value>) -> String {
    ["DRUG_NAME", "SPEC", "PRE_UNIT"]
        .iter()
        .map(|key| {
            source
                .get(*key)
                .map(crate::normalize::value_text)
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("|")
}

fn phis27_identity_errors(
    source: &serde_json::Map<String, Value>,
    normalized: &serde_json::Map<String, Value>,
) -> Vec<String> {
    let mut errors = Vec::new();
    let source_med_id = source
        .get("SOURCE_MED_ID")
        .map(crate::normalize::value_text)
        .unwrap_or_default();
    let source_factory_id = source
        .get("SOURCE_FACTORY_ID")
        .map(crate::normalize::value_text)
        .unwrap_or_default();
    if source_med_id.is_empty() {
        return vec!["二系列phis记录缺少 YPXH，无法建立来源药品对照".into()];
    }
    let expected = if source_factory_id.is_empty() {
        format!("{source_med_id}:BASE")
    } else {
        format!("{source_med_id}:{source_factory_id}")
    };
    for (field, label) in [("SOURCE_KEY", "查询来源键"), ("_sourceKey", "批次来源键")] {
        let actual = source
            .get(field)
            .map(crate::normalize::value_text)
            .unwrap_or_default();
        if actual != expected {
            errors.push(format!(
                "二系列phis{label}必须是 YPXH:YPCD；期望“{expected}”，实际“{actual}”"
            ));
        }
    }
    if source_factory_id.is_empty() {
        return errors;
    }
    for (field, label) in [
        ("SOURCE_MED_PRO_KEY", "商品来源键"),
        ("CD_MED_PRO", "商品货品码"),
    ] {
        let actual = source
            .get(field)
            .map(crate::normalize::value_text)
            .unwrap_or_default();
        if actual != expected {
            errors.push(format!(
                "二系列phis{label}必须是 YPXH:YPCD；期望“{expected}”，实际“{actual}”"
            ));
        }
    }
    let normalized_key = normalized
        .get("cdMedPro")
        .map(crate::normalize::value_text)
        .unwrap_or_default();
    if normalized_key != expected {
        errors.push(format!(
            "二系列phis货品码必须映射为 YPXH:YPCD；期望“{expected}”，实际“{normalized_key}”"
        ));
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::{
        phis27_base_merge_key, phis27_identity_errors, prepare_batch, source_duplicate_count,
    };
    use crate::local_store::LocalStore;
    use crate::model::{MigrationBatch, MigrationRow, PrepareBatchRequest};
    use serde_json::json;
    use serde_json::Map;
    use std::collections::HashMap;
    use std::path::Path;

    #[test]
    fn phis27_duplicate_marker_is_read_safely() {
        let duplicate = json!({"SOURCE_DUPLICATE_COUNT":"2"})
            .as_object()
            .unwrap()
            .clone();
        let ordinary = json!({"SOURCE_DUPLICATE_COUNT":"1"})
            .as_object()
            .unwrap()
            .clone();
        assert_eq!(source_duplicate_count(&duplicate), 2);
        assert_eq!(source_duplicate_count(&ordinary), 1);
    }

    #[test]
    fn phis27_base_merge_key_uses_name_spec_and_minimum_unit() {
        let source = json!({
            "DRUG_NAME":"阿莫西林胶囊",
            "SPEC":"0.25g*24粒",
            "PRE_UNIT":"粒",
            "DRUG_TYPE":"1"
        })
        .as_object()
        .unwrap()
        .clone();
        assert_eq!(phis27_base_merge_key(&source), "阿莫西林胶囊|0.25g*24粒|粒");
    }

    #[test]
    fn phis27_duplicate_base_group_is_mergeable_and_audited() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let source = json!({
            "SOURCE_KEY":"1001:BASE",
            "SOURCE_MED_ID":"1001",
            "SOURCE_DUPLICATE_COUNT":"2",
            "DRUG_NAME":"阿莫西林胶囊",
            "SPEC":"0.25g*24粒",
            "PRE_UNIT":"粒",
            "_sourceKey":"1001:BASE",
            "naMed":"阿莫西林胶囊",
            "sdMed":"1",
            "idCstmg":"63aa8b1b3c6f491981ba4221",
            "unitPre":"粒",
            "spec":"0.25g*24粒",
            "dose":"0.25",
            "unitDose":"g",
            "sdDose":"1",
            "dftUsage":"100"
        })
        .as_object()
        .unwrap()
        .clone();
        let detail = prepare_batch(
            &store,
            PrepareBatchRequest {
                batch_name: "auto-merge".into(),
                source_type: "PHIS27".into(),
                source_name: "source".into(),
                source_description: String::new(),
                conflict_strategy: "INCREMENTAL".into(),
                allow_create_factory: false,
                idempotency_key: "auto-merge-key".into(),
                mappings: Vec::new(),
                cost_merge_mappings: Map::new(),
                rows: vec![source],
            },
            &HashMap::new(),
            "tenant",
        )
        .unwrap();
        assert_eq!(detail.rows[0].status, "VALIDATED");
        assert_eq!(detail.rows[0].normalized_data["_baseMergeGroupSize"], "2");
        assert_eq!(
            detail.rows[0].normalized_data["_baseMergeKey"],
            "阿莫西林胶囊|0.25g*24粒|粒"
        );
    }

    #[test]
    fn one_medicine_with_two_factories_keeps_two_product_identities() {
        let first_source = json!({
            "SOURCE_MED_ID":"1001","SOURCE_FACTORY_ID":"2001",
            "SOURCE_KEY":"1001:2001","SOURCE_MED_PRO_KEY":"1001:2001",
            "CD_MED_PRO":"1001:2001","_sourceKey":"1001:2001"
        })
        .as_object()
        .unwrap()
        .clone();
        let second_source = json!({
            "SOURCE_MED_ID":"1001","SOURCE_FACTORY_ID":"2002",
            "SOURCE_KEY":"1001:2002","SOURCE_MED_PRO_KEY":"1001:2002",
            "CD_MED_PRO":"1001:2002","_sourceKey":"1001:2002"
        })
        .as_object()
        .unwrap()
        .clone();
        let first_normalized = json!({"cdMedPro":"1001:2001"}).as_object().unwrap().clone();
        let second_normalized = json!({"cdMedPro":"1001:2002"}).as_object().unwrap().clone();
        assert!(phis27_identity_errors(&first_source, &first_normalized).is_empty());
        assert!(phis27_identity_errors(&second_source, &second_normalized).is_empty());
        assert_eq!(
            first_source["SOURCE_MED_ID"],
            second_source["SOURCE_MED_ID"]
        );
        assert_ne!(first_source["SOURCE_KEY"], second_source["SOURCE_KEY"]);
        assert_ne!(first_normalized["cdMedPro"], second_normalized["cdMedPro"]);
    }

    #[test]
    fn yplsh_cannot_replace_the_composite_product_identity() {
        let source = json!({
            "SOURCE_MED_ID":"1001","SOURCE_FACTORY_ID":"2001",
            "SOURCE_KEY":"1001:2001","SOURCE_MED_PRO_KEY":"1001:2001",
            "CD_MED_PRO":"3001","SOURCE_PRODUCT_ID":"3001","_sourceKey":"1001:2001"
        })
        .as_object()
        .unwrap()
        .clone();
        let normalized = json!({"cdMedPro":"3001"}).as_object().unwrap().clone();
        let errors = phis27_identity_errors(&source, &normalized);
        assert!(errors.iter().any(|error| error.contains("1001:2001")));
    }

    #[test]
    fn overwrite_prepare_reuses_tool_managed_target_ids() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let old_batch = MigrationBatch {
            batch_id: "old-batch".into(),
            batch_name: "old".into(),
            source_type: "DATABASE".into(),
            source_name: "source".into(),
            source_description: String::new(),
            conflict_strategy: "INCREMENTAL".into(),
            allow_create_factory: false,
            idempotency_key: String::new(),
            status: "SUCCESS".into(),
            total_count: 1,
            valid_count: 0,
            success_count: 1,
            fail_count: 0,
            skip_count: 0,
            created_at: "now".into(),
            updated_at: "now".into(),
            finished_at: Some("now".into()),
        };
        store.insert_batch(&old_batch, "{}").unwrap();
        let old_row = MigrationRow {
            row_id: "old-row".into(),
            batch_id: old_batch.batch_id.clone(),
            row_no: 1,
            source_key: "YPXH-1".into(),
            source_hash: "old-hash".into(),
            status: "SUCCESS".into(),
            raw_data: Map::new(),
            normalized_data: Map::new(),
            error_code: String::new(),
            error_message: String::new(),
            id_med: "66aa10244f0d4826ac110001".into(),
            id_med_unit: "66aa10244f0d4826ac110002".into(),
            id_fac: String::new(),
            id_med_pro: String::new(),
            retry_count: 0,
            updated_at: "now".into(),
        };
        store.insert_row(&old_row).unwrap();
        store
            .record_source_link_upsert(
                "tenant",
                "DATABASE",
                "source",
                &old_row,
                json!([{
                    "operation":"INSERT","table":"hi_bd_med",
                    "targetId":"66aa10244f0d4826ac110001"
                }]),
                "operator",
                "trace",
            )
            .unwrap();
        let changed = json!({
            "_sourceKey":"YPXH-1","naMed":"更新药品","sdMed":"1",
            "idCstmg":"63aa8b1b3c6f491981ba4221","unitPre":"片","spec":"10mg",
            "dose":"10","unitDose":"mg","sdDose":"1","dftUsage":"1","dftFreq":"1"
        })
        .as_object()
        .unwrap()
        .clone();
        let detail = prepare_batch(
            &store,
            PrepareBatchRequest {
                batch_name: "overwrite".into(),
                source_type: "DATABASE".into(),
                source_name: "source".into(),
                source_description: String::new(),
                conflict_strategy: "OVERWRITE".into(),
                allow_create_factory: false,
                idempotency_key: String::new(),
                mappings: Vec::new(),
                cost_merge_mappings: Map::new(),
                rows: vec![changed],
            },
            &HashMap::new(),
            "tenant",
        )
        .unwrap();
        assert_eq!(detail.rows[0].status, "VALIDATED");
        assert_eq!(detail.rows[0].id_med, "66aa10244f0d4826ac110001");
    }
}
