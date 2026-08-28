import assert from "node:assert/strict";
import test from "node:test";
import { buildSourceObjectDiagnostic } from "../src/sourceObjectDiagnostics.js";
import { analyzeSourceObjectDiagnostic } from "../scripts/source-adapter-kit.mjs";

test("generic source object diagnostics preserve structure without leaking source values or credentials", async () => {
  const exported = await buildSourceObjectDiagnostic({
    databaseFamily: "oracle",
    schema: "PHIS",
    objectName: "YK_TYPK",
    preview: {
      columns: ["YPXH", "YPMC", "JX"],
      columnMetadata: [
        { name: "YPXH", comment: "药品序号", sourceTable: "YK_TYPK" },
        { name: "YPMC", comment: "药品名称", sourceTable: "YK_TYPK" },
      ],
      rows: [
        { YPXH: 101, YPMC: "绝密测试药品", JX: "注射剂" },
        { YPXH: 102, YPMC: "另一项敏感样例", JX: "" },
      ],
      truncated: true,
      elapsedMs: 26,
    },
    metadataMessage: "已读取数据库字段注释",
    rowCount: { rowCount: 10001, isExact: false, probeLimit: 10001, elapsedMs: 18 },
    suitability: {
      tone: "ready",
      title: "已识别药品字段",
      description: "仍需核对",
      foundCount: 2,
      clues: [
        { key: "naMed", label: "药品名称", column: "YPMC", hasSample: true },
      ],
    },
    generatedAt: "2026-08-27T08:00:00.000Z",
    connection: {
      host: "10.20.30.40",
      username: "secret_user",
      password: "secret_password",
    },
    targetTenantId: "secret_tenant",
  });

  assert.equal(exported.diagnostic.preview.rowCountProbe.status, "LOWER_BOUND");
  assert.equal(exported.diagnostic.columns[1].sampleProfile.nonEmptyCount, 2);
  assert.deepEqual(exported.diagnostic.columns[2].sampleProfile.observedKinds, [
    "text",
    "empty",
  ]);
  assert.match(exported.content, /YK_TYPK/);
  assert.match(exported.content, /药品名称/);
  assert.doesNotMatch(exported.content, /绝密测试药品|另一项敏感样例/);
  assert.doesNotMatch(
    exported.content,
    /10\.20\.30\.40|secret_user|secret_password|secret_tenant/,
  );
  assert.equal(exported.diagnostic.privacy.containsRawSampleValues, false);
  assert.match(exported.diagnostic.checksum, /^[a-f0-9]{64}$/);
  const maintainerAnalysis = analyzeSourceObjectDiagnostic(exported.content);
  assert.equal(maintainerAnalysis.ok, true, maintainerAnalysis.errors?.join("\n"));
  assert.equal(maintainerAnalysis.summary.objectName, "YK_TYPK");
});

test("diagnostic row-count failures use a controlled message instead of driver text", async () => {
  const exported = await buildSourceObjectDiagnostic({
    databaseFamily: "vastbase",
    objectName: "V_DRUG",
    preview: { columns: ["NAME"], rows: [], truncated: false },
    rowCount: { error: "password=leaked ORA replacement text" },
  });
  assert.equal(exported.diagnostic.preview.rowCountProbe.status, "UNAVAILABLE");
  assert.doesNotMatch(exported.content, /password=leaked|ORA replacement/);
});
