import { targetTrialIdentity } from "./trialMigration.js";

function targetIdentityKey(identity = {}) {
  return JSON.stringify([
    identity.kind || "",
    identity.host || "",
    Number(identity.port) || 0,
    identity.database || "",
    identity.schema || "",
    identity.serviceName || "",
    identity.username || "",
  ]);
}

export function latestInventoryTrialByStorage(audits = [], profile = {}) {
  const expected = targetIdentityKey(targetTrialIdentity(profile));
  const latest = new Map();
  for (const audit of audits) {
    const idSto = `${audit.afterData?.idSto || ""}`.trim();
    if (
      audit.operation !== "INVENTORY_TRIAL_ROLLBACK" ||
      !idSto ||
      targetIdentityKey(audit.afterData?.targetIdentity) !== expected
    ) {
      continue;
    }
    const previous = latest.get(idSto);
    if (
      !previous ||
      `${audit.operatedAt || ""}` >= `${previous.operatedAt || ""}`
    ) {
      latest.set(idSto, audit);
    }
  }
  return latest;
}

export function inventoryTrialCoverage(detail, profile = {}) {
  const rows = detail?.rows || [];
  const requiredStorageIds = [
    ...new Set(
      rows
        .filter((row) => ["VALIDATED", "FAILED"].includes(row.status))
        .map((row) => `${row.normalizedData?.idSto || ""}`.trim())
        .filter(Boolean),
    ),
  ];
  const latest = latestInventoryTrialByStorage(detail?.audits || [], profile);
  const passedStorageIds = requiredStorageIds.filter((idSto) => {
    const audit = latest.get(idSto);
    return audit?.result === "SUCCESS" && audit.afterData?.rolledBack === true;
  });
  return {
    requiredStorageIds,
    passedStorageIds,
    pendingStorageIds: requiredStorageIds.filter(
      (idSto) => !passedStorageIds.includes(idSto),
    ),
    complete:
      requiredStorageIds.length > 0 &&
      passedStorageIds.length === requiredStorageIds.length,
    latest,
  };
}
