use crate::id::new_object_id;
use crate::local_store::LocalStore;
use crate::model::{BatchDetail, MigrationBatch, MigrationRow, PrepareBatchRequest};
use crate::normalize::{normalize, validate};
use chrono::Utc;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

const MAX_BATCH_ROWS: usize = 10_000;

pub fn prepare_batch(
    store: &LocalStore,
    request: PrepareBatchRequest,
) -> Result<BatchDetail, String> {
    if request.rows.is_empty() {
        return Err("没有可迁移的三方药品数据".into());
    }
    if request.rows.len() > MAX_BATCH_ROWS {
        return Err(format!("单批最多迁移 {} 行，请拆分批次", MAX_BATCH_ROWS));
    }
    if let Some(batch_id) = store.find_by_idempotency_key(request.idempotency_key.trim())? {
        return store.load_batch(&batch_id);
    }
    let now = Utc::now().to_rfc3339();
    let batch_id = new_object_id();
    let mut rows = Vec::with_capacity(request.rows.len());
    let mut valid_count = 0;
    for (index, source) in request.rows.iter().enumerate() {
        let normalized = normalize(source, &request.mappings);
        let errors = validate(&normalized);
        if errors.is_empty() {
            valid_count += 1;
        }
        let raw_json = Value::Object(source.clone()).to_string();
        let source_hash = format!("{:x}", Sha256::digest(raw_json.as_bytes()));
        rows.push(MigrationRow {
            row_id: new_object_id(),
            batch_id: batch_id.clone(),
            row_no: index + 1,
            source_key: source
                .get("_sourceKey")
                .map(crate::normalize::value_text)
                .filter(|text| !text.is_empty())
                .unwrap_or_else(|| (index + 1).to_string()),
            source_hash,
            status: if errors.is_empty() {
                "VALIDATED".into()
            } else {
                "INVALID".into()
            },
            raw_data: source.clone(),
            normalized_data: normalized,
            error_code: if errors.is_empty() {
                String::new()
            } else {
                "VALIDATION_ERROR".into()
            },
            error_message: errors.join("；"),
            id_med: String::new(),
            id_med_unit: String::new(),
            id_fac: String::new(),
            id_med_pro: String::new(),
            retry_count: 0,
            updated_at: now.clone(),
        });
    }
    let invalid_count = rows.len() - valid_count;
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
        conflict_strategy: if request.conflict_strategy.is_empty() {
            "REUSE".into()
        } else {
            request.conflict_strategy
        },
        allow_create_factory: request.allow_create_factory,
        idempotency_key: request.idempotency_key,
        status: if valid_count > 0 {
            "VALIDATED".into()
        } else {
            "INVALID".into()
        },
        total_count: rows.len(),
        valid_count,
        success_count: 0,
        fail_count: invalid_count,
        skip_count: 0,
        created_at: now.clone(),
        updated_at: now,
        finished_at: None,
    };
    store.insert_batch(
        &batch,
        &serde_json::to_string(&request.mappings).map_err(|error| error.to_string())?,
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
            "暂存并校验完成：有效{}行，无效{}行",
            valid_count, invalid_count
        ),
        "",
        &trace_id,
    )?;
    store.load_batch(&batch_id)
}
