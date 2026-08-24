import assert from "node:assert/strict";
import test from "node:test";
import {
  hasSuccessfulTargetTrial,
  latestTargetTrialByRow,
  targetTrialIdentity,
} from "../src/trialMigration.js";

const profile = {
  kind: "Oracle",
  host: " DB.EXAMPLE.COM ",
  port: 1521,
  serviceName: "ORCL",
  username: "PHIS",
};

test("target trial identity normalizes connection fields", () => {
  assert.deepEqual(targetTrialIdentity(profile), {
    kind: "oracle",
    host: "db.example.com",
    port: 1521,
    database: "",
    schema: "",
    serviceName: "orcl",
    username: "phis",
  });
});

test("latest trial result remains stable per row", () => {
  const identity = targetTrialIdentity(profile);
  const audits = [
    {
      rowId: "row-1",
      operation: "TRIAL_ROLLBACK",
      result: "FAILED",
      operatedAt: "2026-08-21T08:00:00Z",
      afterData: { targetIdentity: identity, rolledBack: true },
    },
    {
      rowId: "row-1",
      operation: "TRIAL_ROLLBACK",
      result: "SUCCESS",
      operatedAt: "2026-08-21T08:01:00Z",
      afterData: { targetIdentity: identity, rolledBack: true },
    },
  ];
  assert.equal(latestTargetTrialByRow(audits, profile).get("row-1").result, "SUCCESS");
  assert.equal(hasSuccessfulTargetTrial(audits, profile), true);
});

test("trial from another target does not unlock formal migration", () => {
  const audits = [
    {
      rowId: "row-1",
      operation: "TRIAL_ROLLBACK",
      result: "SUCCESS",
      operatedAt: "2026-08-21T08:01:00Z",
      afterData: {
        targetIdentity: targetTrialIdentity({ ...profile, host: "other-db" }),
        rolledBack: true,
      },
    },
  ];
  assert.equal(hasSuccessfulTargetTrial(audits, profile), false);
});
