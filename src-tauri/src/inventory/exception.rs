pub fn confirm_validation_exception(
    store: &LocalStore,
    operator_id: &str,
    request: ConfirmInventoryExceptionRequest,
) -> Result<BatchDetail, String> {
    let reason = request.reason.trim();
    if reason.chars().count() < 2 {
        return Err("请填写至少 2 个字的人工确认原因".into());
    }
    if reason.chars().count() > 300 {
        return Err("人工确认原因不能超过 300 个字".into());
    }

    let mut detail = store.load_batch(&request.batch_id)?;
    if !is_inventory_batch_source_type(&detail.batch.source_type) {
        return Err("当前批次不是机构库存首次盘点批次".into());
    }
    if detail.batch.success_count > 0
        || matches!(detail.batch.status.as_str(), "SUCCESS" | "PARTIAL" | "UNDONE")
    {
        return Err("库存批次已经开始或完成目标写入，不能再确认预检例外".into());
    }

    let expected_target = store
        .load_batch_mapping(&request.batch_id)?
        .get("targetIdentity")
        .cloned()
        .unwrap_or(Value::Null);
    let actual_target = target_identity(&request.target);
    if expected_target != actual_target {
        return Err("目标数据库已变化，请重新读取并校验本批库存".into());
    }

    let row_index = detail
        .rows
        .iter()
        .position(|row| row.row_id == request.row_id)
        .ok_or_else(|| "未找到需要确认的库存明细".to_string())?;
    let row = &mut detail.rows[row_index];
    if row.status != "INVALID" || row.error_code != "INVENTORY_PREFLIGHT" {
        return Err("只有尚未写入且标记为可人工复核的库存预检失败项才能确认例外".into());
    }
    let issues = row
        .raw_data
        .get("inventoryValidationIssues")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if issues.is_empty()
        || issues
            .iter()
            .any(|issue| issue.get("reviewable").and_then(Value::as_bool) != Some(true))
    {
        return Err("该库存组包含药品、库房、数量、金额或防重等硬性错误，不能人工放行".into());
    }

    let original_error_code = row.error_code.clone();
    let original_error_message = row.error_message.clone();
    let source_hash = row.source_hash.clone();
    let id_sto = normalized_text(row, "idSto");
    let now = Utc::now().to_rfc3339();
    row.status = "VALIDATED".into();
    row.error_code = "INVENTORY_EXCEPTION_CONFIRMED".into();
    row.error_message = format!("人工确认例外：{reason}；原校验：{original_error_message}");
    row.updated_at = now.clone();
    let confirmed_row = row.clone();

    detail.batch.valid_count = detail
        .rows
        .iter()
        .filter(|row| row.status == "VALIDATED")
        .count();
    detail.batch.fail_count = detail
        .rows
        .iter()
        .filter(|row| matches!(row.status.as_str(), "INVALID" | "FAILED"))
        .count();
    detail.batch.skip_count = detail
        .rows
        .iter()
        .filter(|row| row.status == "SKIPPED")
        .count();
    detail.batch.status = if detail.batch.valid_count > 0 || detail.batch.skip_count > 0 {
        "VALIDATED".into()
    } else {
        "INVALID".into()
    };
    detail.batch.updated_at = now.clone();
    detail.batch.finished_at = None;

    let trace_id = new_object_id();
    let mut audits = vec![MigrationAudit {
        audit_id: new_object_id(),
        batch_id: request.batch_id.clone(),
        row_id: confirmed_row.row_id.clone(),
        trace_id: trace_id.clone(),
        operation: "INVENTORY_VALIDATION_EXCEPTION".into(),
        target_table: "migration_row".into(),
        target_id: confirmed_row.row_id.clone(),
        result: "CONFIRMED".into(),
        before_data: json!({
            "status":"INVALID",
            "errorCode":original_error_code,
            "errorMessage":original_error_message,
            "sourceHash":source_hash,
            "issues":issues,
            "targetIdentity":actual_target
        }),
        after_data: json!({
            "status":"VALIDATED",
            "errorCode":"INVENTORY_EXCEPTION_CONFIRMED",
            "reason":reason,
            "sourceHash":source_hash,
            "targetIdentity":actual_target
        }),
        message: format!("人工确认库存预检例外：{reason}"),
        operator_id: operator_id.into(),
        operated_at: now.clone(),
    }];
    if !id_sto.is_empty() {
        audits.push(MigrationAudit {
            audit_id: new_object_id(),
            batch_id: request.batch_id.clone(),
            row_id: confirmed_row.row_id.clone(),
            trace_id,
            operation: "INVENTORY_TRIAL_ROLLBACK".into(),
            target_table: "migration_batch".into(),
            target_id: id_sto.clone(),
            result: "INVALIDATED".into(),
            before_data: Value::Null,
            after_data: json!({
                "targetIdentity":actual_target,
                "idSto":id_sto,
                "rolledBack":false,
                "invalidatedBy":"INVENTORY_VALIDATION_EXCEPTION"
            }),
            message: "人工确认库存例外后，原目标库房试迁移结果已失效，请重新试迁移".into(),
            operator_id: operator_id.into(),
            operated_at: now,
        });
    }

    store.commit_inventory_exception_confirmation(&confirmed_row, &detail.batch, &audits)?;
    store.load_batch(&request.batch_id)
}

#[cfg(test)]
mod exception_tests {
    use super::*;
    use std::path::Path;

    fn profile(host: &str) -> ConnectionProfile {
        serde_json::from_value(json!({
            "kind":"postgresql","host":host,"port":5432,"database":"medicine",
            "username":"writer","password":"","schema":"public","serviceName":"",
            "driver":"","connectionString":""
        }))
        .unwrap()
    }

    fn store_batch(store: &LocalStore, profile: &ConnectionProfile, reviewable: bool) {
        let issue = if reviewable {
            json!({
                "code":"PHARMACY_SINGLE_PACKAGE_UNCONFIRMED",
                "message":"包装关系待人工核对",
                "reviewable":true
            })
        } else {
            json!({
                "code":"MEDICINE_LEDGER_REQUIRED",
                "message":"药品台账缺失",
                "reviewable":false
            })
        };
        let detail: BatchDetail = serde_json::from_value(json!({
            "batch":{
                "batchId":"batch-1","batchName":"库存预检","sourceType":"PHIS27_INVENTORY",
                "sourceName":"source","sourceDescription":"inventory","conflictStrategy":"FAIL",
                "allowCreateFactory":false,"idempotencyKey":"key-1","status":"INVALID",
                "totalCount":1,"validCount":0,"successCount":0,"failCount":1,"skipCount":0,
                "createdAt":"2026-08-26T00:00:00Z","updatedAt":"2026-08-26T00:00:00Z",
                "finishedAt":null
            },
            "rows":[{
                "rowId":"row-1","batchId":"batch-1","rowNo":1,"sourceKey":"YF:1",
                "sourceHash":"source-hash","status":"INVALID",
                "rawData":{"inventoryValidationIssues":[issue]},
                "normalizedData":{"idSto":"sto-1","idOrg":"org-1"},
                "errorCode":"INVENTORY_PREFLIGHT","errorMessage":"预检失败",
                "idMed":"med-1","idMedUnit":"unit-1","idFac":"fac-1","idMedPro":"pro-1",
                "retryCount":0,"updatedAt":"2026-08-26T00:00:00Z"
            }],
            "audits":[]
        }))
        .unwrap();
        store
            .insert_batch(
                &detail.batch,
                &json!({"targetIdentity":target_identity(profile)}).to_string(),
            )
            .unwrap();
        store.insert_row(&detail.rows[0]).unwrap();
    }

    #[test]
    fn reviewable_inventory_exception_becomes_validated_and_audited() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let target = profile("db.example");
        store_batch(&store, &target, true);

        let detail = confirm_validation_exception(
            &store,
            "operator-1",
            ConfirmInventoryExceptionRequest {
                batch_id: "batch-1".into(),
                row_id: "row-1".into(),
                target,
                reason: "已与药房负责人核对现场包装".into(),
            },
        )
        .unwrap();

        assert_eq!(detail.rows[0].status, "VALIDATED");
        assert_eq!(
            detail.rows[0].error_code,
            "INVENTORY_EXCEPTION_CONFIRMED"
        );
        assert_eq!(detail.batch.valid_count, 1);
        assert_eq!(detail.batch.fail_count, 0);
        assert!(detail.audits.iter().any(|audit| {
            audit.operation == "INVENTORY_VALIDATION_EXCEPTION"
                && audit.result == "CONFIRMED"
        }));
        assert!(detail.audits.iter().any(|audit| {
            audit.operation == "INVENTORY_TRIAL_ROLLBACK"
                && audit.result == "INVALIDATED"
                && audit.after_data.get("idSto").and_then(Value::as_str) == Some("sto-1")
        }));
    }

    #[test]
    fn hard_inventory_error_cannot_be_confirmed() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let target = profile("db.example");
        store_batch(&store, &target, false);

        let error = confirm_validation_exception(
            &store,
            "operator-1",
            ConfirmInventoryExceptionRequest {
                batch_id: "batch-1".into(),
                row_id: "row-1".into(),
                target,
                reason: "尝试人工放行".into(),
            },
        )
        .unwrap_err();

        assert!(error.contains("硬性错误"));
        let detail = store.load_batch("batch-1").unwrap();
        assert_eq!(detail.rows[0].status, "INVALID");
        assert!(detail.audits.is_empty());
    }

    #[test]
    fn target_change_invalidates_the_confirmation_request() {
        let store = LocalStore::open(Path::new(":memory:")).unwrap();
        let target = profile("db.example");
        store_batch(&store, &target, true);

        let error = confirm_validation_exception(
            &store,
            "operator-1",
            ConfirmInventoryExceptionRequest {
                batch_id: "batch-1".into(),
                row_id: "row-1".into(),
                target: profile("another-db.example"),
                reason: "已核对现场包装".into(),
            },
        )
        .unwrap_err();

        assert!(error.contains("目标数据库已变化"));
    }
}
