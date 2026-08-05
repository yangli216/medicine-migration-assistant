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
        apply_cost_merge_mapping(&mut normalized, &request.cost_merge_mappings);
        let mut errors = validate(&normalized);
        errors.extend(crate::target_dictionary::validate_values(
            &normalized,
            dictionary_values,
        ));
        if source_duplicate_count(source) > 1 {
            errors.push(
                "老库中存在相同通用名、物品类型和规格的不同 YPXH；为防止错误合并，请人工确认后再迁移"
                    .into(),
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
        allow_create_factory: request.allow_create_factory,
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
    store.insert_batch(
        &batch,
        &serde_json::to_string(&json!({
            "fieldMappings": request.mappings,
            "costMergeMappings": request.cost_merge_mappings
        }))
        .map_err(|error| error.to_string())?,
    )?;
    for row in &rows {
        store.insert_row(row)?;
    }
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

#[cfg(test)]
mod tests {
    use super::{prepare_batch, source_duplicate_count};
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
    fn overwrite_prepare_reuses_tool_managed_target_ids() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let old_batch = MigrationBatch {
            batch_id: "old-batch".into(),
            batch_name: "old".into(),
            source_type: "PHIS27".into(),
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
                "PHIS27",
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
                source_type: "PHIS27".into(),
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
