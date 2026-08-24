export function targetTrialIdentity(profile = {}) {
  return {
    kind: `${profile.kind || ""}`.trim().toLowerCase(),
    host: `${profile.host || ""}`.trim().toLowerCase(),
    port: Number(profile.port) || 0,
    database: `${profile.database || ""}`.trim().toLowerCase(),
    schema: `${profile.schema || ""}`.trim().toLowerCase(),
    serviceName: `${profile.serviceName || ""}`.trim().toLowerCase(),
    username: `${profile.username || ""}`.trim().toLowerCase(),
  };
}

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

export function latestTargetTrialByRow(audits = [], profile = {}) {
  const expected = targetIdentityKey(targetTrialIdentity(profile));
  const latest = new Map();
  for (const audit of audits) {
    if (
      audit.operation !== "TRIAL_ROLLBACK" ||
      targetIdentityKey(audit.afterData?.targetIdentity) !== expected
    ) {
      continue;
    }
    const previous = latest.get(audit.rowId);
    if (
      !previous ||
      `${audit.operatedAt || ""}` >= `${previous.operatedAt || ""}`
    ) {
      latest.set(audit.rowId, audit);
    }
  }
  return latest;
}

export function hasSuccessfulTargetTrial(audits = [], profile = {}) {
  return [...latestTargetTrialByRow(audits, profile).values()].some(
    (audit) => audit.result === "SUCCESS" && audit.afterData?.rolledBack === true,
  );
}
