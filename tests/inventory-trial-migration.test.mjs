import assert from "node:assert/strict";
import test from "node:test";
import {
  inventoryTrialCoverage,
  latestInventoryTrialByStorage,
} from "../src/inventoryTrialMigration.js";

const target = {
  kind: "postgresql",
  host: "DB.EXAMPLE",
  port: 5432,
  database: "phis",
  schema: "public",
  username: "writer",
};

function audit(idSto, result, operatedAt = "2026-08-21T08:00:00Z") {
  return {
    operation: "INVENTORY_TRIAL_ROLLBACK",
    result,
    operatedAt,
    afterData: {
      idSto,
      rolledBack: true,
      targetIdentity: {
        kind: "postgresql",
        host: "db.example",
        port: 5432,
        database: "phis",
        schema: "public",
        serviceName: "",
        username: "writer",
      },
    },
  };
}

test("库存试迁移按目标库房保留最新结果", () => {
  const latest = latestInventoryTrialByStorage(
    [
      audit("sto-1", "SUCCESS"),
      audit("sto-1", "FAILED", "2026-08-21T09:00:00Z"),
    ],
    target,
  );
  assert.equal(latest.get("sto-1").result, "FAILED");
});

test("每个目标库房均通过后才完成库存试迁移门禁", () => {
  const detail = {
    rows: [
      { status: "VALIDATED", normalizedData: { idSto: "sto-1" } },
      { status: "VALIDATED", normalizedData: { idSto: "sto-1" } },
      { status: "VALIDATED", normalizedData: { idSto: "sto-2" } },
    ],
    audits: [audit("sto-1", "SUCCESS")],
  };
  const first = inventoryTrialCoverage(detail, target);
  assert.equal(first.complete, false);
  assert.deepEqual(first.pendingStorageIds, ["sto-2"]);
  detail.audits.push(audit("sto-2", "SUCCESS"));
  assert.equal(inventoryTrialCoverage(detail, target).complete, true);
});

test("其它目标库的试迁移不能解锁库存正式执行", () => {
  const detail = {
    rows: [{ status: "VALIDATED", normalizedData: { idSto: "sto-1" } }],
    audits: [audit("sto-1", "SUCCESS")],
  };
  assert.equal(
    inventoryTrialCoverage(detail, { ...target, database: "other" }).complete,
    false,
  );
});

test("人工确认库存例外会使原库房试迁移失效", () => {
  const detail = {
    rows: [{ status: "VALIDATED", normalizedData: { idSto: "sto-1" } }],
    audits: [
      audit("sto-1", "SUCCESS"),
      {
        ...audit("sto-1", "INVALIDATED", "2026-08-21T10:00:00Z"),
        afterData: {
          ...audit("sto-1", "INVALIDATED").afterData,
          rolledBack: false,
          invalidatedBy: "INVENTORY_VALIDATION_EXCEPTION",
        },
      },
    ],
  };

  const coverage = inventoryTrialCoverage(detail, target);
  assert.equal(coverage.complete, false);
  assert.deepEqual(coverage.pendingStorageIds, ["sto-1"]);
});
