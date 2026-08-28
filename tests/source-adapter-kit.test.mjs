import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, mkdtemp, readFile, unlink, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";
import {
  analyzeSourceObjectDiagnostic,
  analyzeSourceAdapterSupportPackage,
  analyzeAdapterMaintenanceHandoff,
  buildAdapterFixtureDraft,
  buildAdapterMaintenanceHandoff,
  buildSourceStructureContractDraft,
  createAdapterPackage,
  adapterReadiness,
  evidenceExecutionPassed,
  fixtureExecutionPassed,
  formatSourceAdapterSupportAnalysis,
  formatAdapterFixtureDraftSummary,
  formatAdapterMaintenanceHandoffSummary,
  formatAdapterMaintenanceHandoffVerification,
  formatAdapterReadiness,
  formatSourceStructureContractDraftSummary,
  formatSourceObjectDiagnosticAnalysis,
  listAdapterFixtureCases,
  maintenanceHandoffChecksum,
  sourceAdapterSupportChecksum,
  sourceObjectDiagnosticChecksum,
  validateAdapterPackage,
  validateAdapterExamples,
  validateAdapterFixture,
  validateAdapterRegistry,
  validateAcceptanceEvidence,
  validationSummary,
} from "../scripts/source-adapter-kit.mjs";

function sourceObjectDiagnostic() {
  const value = {
    formatVersion: 1,
    packageType: "GENERIC_SOURCE_OBJECT_DIAGNOSTIC",
    generatedAt: "2026-08-27T08:00:00.000Z",
    source: {
      databaseFamily: "oracle",
      schema: "PHIS",
      objectName: "YK_TYPK",
    },
    preview: {
      columnCount: 2,
      sampledRowCount: 2,
      truncated: true,
      elapsedMs: 25,
      metadataMessage: "已读取字段注释",
      rowCountProbe: {
        status: "LOWER_BOUND",
        rowCount: 10001,
        probeLimit: 10001,
        elapsedMs: 18,
      },
    },
    medicineFieldAssessment: {
      status: "warning",
      title: "仅识别到 1 项常用药品字段",
      description: "仍需核对",
      foundCount: 1,
      clues: [
        {
          key: "naMed",
          label: "药品名称",
          column: "YPMC",
          hasNonEmptySample: true,
        },
        {
          key: "spec",
          label: "规格",
          column: "",
          hasNonEmptySample: false,
        },
      ],
    },
    columns: [
      {
        name: "YPXH",
        comment: "药品序号",
        sourceTable: "YK_TYPK",
        sourceColumn: "",
        mappingEligible: true,
        sampleProfile: {
          nonEmptyCount: 2,
          maximumTextLength: 3,
          observedKinds: ["integer"],
        },
      },
      {
        name: "YPMC",
        comment: "药品名称",
        sourceTable: "YK_TYPK",
        sourceColumn: "",
        mappingEligible: true,
        sampleProfile: {
          nonEmptyCount: 2,
          maximumTextLength: 8,
          observedKinds: ["text"],
        },
      },
    ],
    privacy: {
      containsRawSampleValues: false,
      excluded: [
        "数据库主机与端口",
        "连接账号与密码",
        "高级连接串",
        "目标租户",
        "原始样例值",
      ],
    },
    checksum: "",
  };
  value.checksum = sourceObjectDiagnosticChecksum(value);
  return value;
}

function supportPackage() {
  const value = {
    format: "medicine-migration-adapter-support",
    version: 2,
    adapterId: "VENDOR_HIS",
    sourceFingerprint: "1234567890abcdef1234",
    diagnostics: [
      {
        diagnosticId: "64b2fca10a1f2e3d4c5b7102",
        adapterId: "VENDOR_HIS",
        adapterVersion: 2,
        sourceFingerprint: "1234567890abcdef1234",
        databaseFamily: "oracle",
        schema: "HIS",
        detected: true,
        checkedObjects: ["DRUG_MASTER", "DRUG_PRODUCT"],
        missingObjects: [],
        objectStructures: [
          {
            name: "DRUG_MASTER",
            columns: [
              { name: "ID", dataType: "NUMBER" },
              { name: "NAME", dataType: "NVARCHAR2" },
              { name: "PACK_FACTOR", dataType: "NUMBER" },
            ],
          },
          {
            name: "DRUG_PRODUCT",
            columns: [{ name: "PRODUCT_ID", dataType: "NUMBER" }],
          },
        ],
        metrics: [{ id: "MEDICINES", label: "药品", value: 12 }],
        warnings: [],
        message: "结构通过",
        structureHash: "b".repeat(64),
        recordedAt: "2026-08-27T08:00:00Z",
      },
      {
        diagnosticId: "64b2fca10a1f2e3d4c5b7101",
        adapterId: "VENDOR_HIS",
        adapterVersion: 1,
        sourceFingerprint: "1234567890abcdef1234",
        databaseFamily: "oracle",
        schema: "HIS",
        detected: false,
        checkedObjects: ["DRUG_MASTER"],
        missingObjects: ["DRUG_PRODUCT"],
        objectStructures: [
          {
            name: "DRUG_MASTER",
            columns: [
              { name: "ID", dataType: "NUMBER" },
              { name: "NAME", dataType: "VARCHAR2" },
            ],
          },
        ],
        metrics: [{ id: "MEDICINES", label: "药品", value: 10 }],
        warnings: ["缺少商品表"],
        message: "结构未通过",
        structureHash: "a".repeat(64),
        recordedAt: "2026-08-26T08:00:00Z",
      },
    ],
    exportedAt: "2026-08-27T08:05:00Z",
    checksum: "",
  };
  value.checksum = sourceAdapterSupportChecksum(value);
  return value;
}

function supportAcceptance() {
  return {
    acceptanceSchemaVersion: 2,
    adapterId: "VENDOR_HIS",
    sourceStructure: {
      objects: [
        {
          name: "DRUG_MASTER",
          requirement: "REQUIRED",
          usedBy: ["药品主档"],
          columns: [
            {
              name: "ID",
              requirement: "REQUIRED",
              usedBy: ["药品来源身份"],
              acceptedTypeFamilies: ["NUMERIC"],
            },
            {
              name: "NAME",
              requirement: "REQUIRED",
              usedBy: ["药品名称"],
              acceptedTypeFamilies: ["CHARACTER"],
            },
            {
              name: "PACK_FACTOR",
              requirement: "CONDITIONAL",
              appliesToTasks: ["INVENTORY"],
              requiredForTasks: ["INVENTORY"],
              usedBy: ["库存包装"],
              fallback: "没有库存任务时不影响药品主档",
              acceptedTypeFamilies: ["NUMERIC"],
            },
            {
              name: "APPROVAL_NO",
              requirement: "OPTIONAL",
              appliesToTasks: ["MEDICINE_BASE"],
              usedBy: ["批准文号"],
              fallback: "留空并进入人工核对",
              acceptedTypeFamilies: ["CHARACTER"],
            },
          ],
        },
        {
          name: "DRUG_PRODUCT",
          requirement: "REQUIRED",
          appliesToTasks: ["MEDICINE_BASE"],
          usedBy: ["厂家商品"],
          columns: [
            {
              name: "PRODUCT_ID",
              requirement: "REQUIRED",
              usedBy: ["商品来源身份"],
              acceptedTypeFamilies: ["NUMERIC"],
            },
            {
              name: "FACTORY_ID",
              requirement: "CONDITIONAL",
              usedBy: ["生产厂家关联"],
              fallback: "项目确认替代厂家字段",
              acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
            },
          ],
        },
      ],
    },
  };
}

test("fixture runner rejects a successful cargo command that executed no test", () => {
  assert.equal(
    fixtureExecutionPassed(
      0,
      "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored",
    ),
    false,
  );
  assert.equal(
    fixtureExecutionPassed(
      0,
      "running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored",
    ),
    true,
  );
});

test("acceptance evidence must bind a real test function and exact executable path", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "adapter-evidence-"));
  await mkdir(path.join(root, "src-tauri", "src"), { recursive: true });
  const sourceFile = path.join(root, "src-tauri", "src", "vendor.rs");
  await writeFile(
    sourceFile,
    "// fn business_rule_is_proven() {}\nfn unrelated() {}\n",
  );
  const manifest = {
    automaticDetection: true,
    migrationTasks: ["MEDICINE_BASE"],
  };
  const acceptance = {
    evidence: [
      {
        requirement: "业务规则",
        tasks: ["MEDICINE_BASE"],
        file: "src-tauri/src/vendor.rs",
        contains: "business_rule_is_proven",
        rustTest: "vendor::tests::business_rule_is_proven",
        execution: "REQUIRED",
      },
      {
        requirement: "第二项证据",
        tasks: ["MEDICINE_BASE"],
        file: "src-tauri/src/vendor.rs",
        contains: "business_rule_is_proven",
        rustTest: "vendor::tests::business_rule_is_proven",
        execution: "REQUIRED",
      },
    ],
  };
  const report = { errors: [], warnings: [] };
  await validateAcceptanceEvidence(report, acceptance, root, manifest);
  assert.match(report.errors.join("\n"), /未在.*找到测试函数|不是 #\[test\]/);

  await writeFile(
    sourceFile,
    "#[test]\nfn business_rule_is_proven() { assert!(true); }\n",
  );
  const valid = { errors: [], warnings: [] };
  await validateAcceptanceEvidence(valid, acceptance, root, manifest);
  assert.deepEqual(valid.errors, []);
  acceptance.evidence[0].rustTest = "vendor::tests::different_name";
  const mismatch = { errors: [], warnings: [] };
  await validateAcceptanceEvidence(mismatch, acceptance, root, manifest);
  assert.match(mismatch.errors.join("\n"), /完整 Rust 测试路径/);

  const missingInventory = { errors: [], warnings: [] };
  await validateAcceptanceEvidence(
    missingInventory,
    acceptance,
    root,
    { automaticDetection: true, migrationTasks: ["MEDICINE_BASE", "INVENTORY"] },
  );
  assert.match(missingInventory.errors.join("\n"), /缺少 INVENTORY 任务/);
});

test("evidence runner rejects zero tests and distinguishes optional live tests", () => {
  assert.equal(
    evidenceExecutionPassed(
      0,
      "running 1 test\ntest result: ok. 1 passed; 0 failed; 0 ignored",
      "REQUIRED",
    ),
    true,
  );
  assert.equal(
    evidenceExecutionPassed(
      0,
      "running 0 tests\ntest result: ok. 0 passed; 0 failed; 0 ignored",
      "REQUIRED",
    ),
    false,
  );
  assert.equal(
    evidenceExecutionPassed(
      0,
      "running 1 test\ntest result: ok. 0 passed; 0 failed; 1 ignored",
      "REQUIRED",
    ),
    false,
  );
  assert.equal(
    evidenceExecutionPassed(
      0,
      "running 1 test\ntest result: ok. 0 passed; 0 failed; 1 ignored",
      "OPTIONAL_LIVE",
    ),
    true,
  );
});

test("current source adapter packages pass the acceptance gate", async () => {
  const reports = await validateAdapterRegistry(process.cwd());
  const ids = reports.map((report) => report.id);
  for (const required of ["GENERIC_DATABASE", "GENERIC_FILE", "PHIS27"])
    assert.ok(ids.includes(required));
  assert.equal(new Set(ids).size, ids.length);
  assert.equal(validationSummary(reports).ok, true);
  const fixtures = await listAdapterFixtureCases(process.cwd());
  assert.ok(
    fixtures.some((item) => item.fixture.tasks.includes("MEDICINE_BASE")),
  );
  assert.ok(fixtures.some((item) => item.fixture.tasks.includes("INVENTORY")));
});

test("teaching adapter is independently valid but excluded from production registry", async () => {
  const production = await validateAdapterRegistry(process.cwd());
  const examples = await validateAdapterExamples(process.cwd());
  assert.equal(validationSummary(examples).ok, true);
  assert.deepEqual(examples.map((report) => report.id), ["STANDARD_HIS"]);
  assert.equal(
    production.some((report) => report.id === "STANDARD_HIS"),
    false,
  );
  const manifest = JSON.parse(
    await readFile(
      path.resolve(
        "src-tauri/adapters/_examples/standard_his/manifest.json",
      ),
      "utf8",
    ),
  );
  assert.deepEqual(manifest.migrationTasks, ["MEDICINE_BASE", "INVENTORY"]);
  assert.equal(
    manifest.inventoryWorkflow.sourceStockKeyMode,
    "ADAPTER_SCOPED_V1",
  );
  assert.equal(manifest.inventoryWorkflow.writeMode, "FIRST_STOCKTAKE");
  const source = await readFile(
    path.resolve(
      "src-tauri/adapters/_examples/standard_his/adapter.rs",
    ),
    "utf8",
  );
  assert.match(source, /use super::sdk::\*/);
  assert.doesNotMatch(source, /crate::(?:odbc|pg_protocol|datasource)/);
});

test("new automatic adapters cannot bypass the public SDK boundary", async () => {
  const root = path.resolve(
    "src-tauri/adapters/_examples/standard_his",
  );
  const manifest = JSON.parse(
    await readFile(path.join(root, "manifest.json"), "utf8"),
  );
  const acceptance = JSON.parse(
    await readFile(path.join(root, "acceptance.json"), "utf8"),
  );
  const maintenanceSource = await readFile(
    path.join(root, "MAINTENANCE.md"),
    "utf8",
  );
  const implementationSource = `
use super::sdk::*;
use crate::odbc;
pub(super) fn binding() -> SourceAdapterBinding {
  SourceAdapterBinding::new("STANDARD_HIS_EXAMPLE_BUILTIN", None, None)
}`;
  const report = validateAdapterPackage({
    packageName: "standard_his",
    manifest,
    acceptance,
    sourceAdapterRust: implementationSource,
    implementationSource,
    maintenanceSource,
  });
  assert.match(report.errors.join("\n"), /不能直接依赖 crate 内部模块/);
});

test("support package is checksum protected, credential free, and summarizes changes", () => {
  assert.equal(
    sourceAdapterSupportChecksum({
      format: "medicine-migration-adapter-support",
      version: 1,
      adapterId: "VENDOR_HIS",
      sourceFingerprint: "1234567890abcdef1234",
      diagnostics: [],
      exportedAt: "2026-08-27T08:05:00Z",
      checksum: "",
    }),
    "2ddf9b830548daa8f6845cce4a10d75c10b03fc51ccced2174531d173934aa31",
  );
  const value = supportPackage();
  const analysis = analyzeSourceAdapterSupportPackage(JSON.stringify(value));
  assert.equal(analysis.ok, true);
  assert.equal(analysis.summary.diagnosticCount, 2);
  assert.deepEqual(analysis.summary.currentMissingObjects, []);
  assert.deepEqual(analysis.summary.transitions[0].newlyCheckedObjects, [
    "DRUG_PRODUCT",
  ]);
  assert.deepEqual(analysis.summary.transitions[0].resolvedMissingObjects, [
    "DRUG_PRODUCT",
  ]);
  assert.deepEqual(analysis.summary.transitions[0].addedColumns, [
    "DRUG_MASTER.PACK_FACTOR",
  ]);
  assert.deepEqual(analysis.summary.transitions[0].changedColumnTypes, [
    {
      column: "DRUG_MASTER.NAME",
      from: "VARCHAR2",
      to: "NVARCHAR2",
    },
  ]);
  assert.equal(analysis.summary.currentStructuredObjectCount, 2);
  assert.equal(analysis.summary.currentStructuredColumnCount, 4);
  assert.match(
    formatSourceAdapterSupportAnalysis(analysis),
    /已恢复 DRUG_PRODUCT/,
  );
  assert.match(
    formatSourceAdapterSupportAnalysis(analysis),
    /新增字段 DRUG_MASTER.PACK_FACTOR/,
  );
  assert.match(
    formatSourceAdapterSupportAnalysis(analysis),
    /DRUG_MASTER.NAME VARCHAR2→NVARCHAR2/,
  );

  const tampered = structuredClone(value);
  tampered.diagnostics[0].metrics[0].value = 99;
  const tamperedAnalysis = analyzeSourceAdapterSupportPackage(tampered);
  assert.equal(tamperedAnalysis.ok, false);
  assert.match(tamperedAnalysis.errors.join("\n"), /完整性校验失败/);

  const unsafe = structuredClone(value);
  unsafe.password = "secret";
  unsafe.checksum = sourceAdapterSupportChecksum(unsafe);
  const unsafeAnalysis = analyzeSourceAdapterSupportPackage(unsafe);
  assert.equal(unsafeAnalysis.ok, false);
  assert.match(unsafeAnalysis.errors.join("\n"), /连接、凭据或租户字段/);

  const malformed = structuredClone(value);
  malformed.diagnostics[0].message = { note: "not plain text" };
  malformed.checksum = sourceAdapterSupportChecksum(malformed);
  const malformedAnalysis = analyzeSourceAdapterSupportPackage(malformed);
  assert.equal(malformedAnalysis.ok, false);
  assert.match(malformedAnalysis.errors.join("\n"), /message 必须是/);

  const leakedSample = structuredClone(value);
  leakedSample.diagnostics[0].objectStructures[0].columns[0].sampleValue =
    "业务值";
  leakedSample.checksum = sourceAdapterSupportChecksum(leakedSample);
  const leakedSampleAnalysis = analyzeSourceAdapterSupportPackage(leakedSample);
  assert.equal(leakedSampleAnalysis.ok, false);
  assert.match(leakedSampleAnalysis.errors.join("\n"), /sampleValue/);

  const legacy = structuredClone(value);
  legacy.version = 1;
  for (const diagnostic of legacy.diagnostics)
    delete diagnostic.objectStructures;
  legacy.checksum = sourceAdapterSupportChecksum(legacy);
  assert.equal(analyzeSourceAdapterSupportPackage(legacy).ok, true);
});

test("task-scoped support packages select their own contract and reject cross-task analysis", () => {
  const value = supportPackage();
  value.migrationTask = "INVENTORY";
  for (const diagnostic of value.diagnostics)
    diagnostic.migrationTask = "INVENTORY";
  value.checksum = sourceAdapterSupportChecksum(value);

  const analysis = analyzeSourceAdapterSupportPackage(
    value,
    supportAcceptance(),
  );
  assert.equal(analysis.ok, true);
  assert.equal(analysis.summary.migrationTask, "INVENTORY");
  assert.deepEqual(analysis.summary.contractImpact.evaluatedTasks, [
    "INVENTORY",
  ]);
  assert.match(
    formatSourceAdapterSupportAnalysis(analysis),
    /支持包任务：机构库存/,
  );

  const mismatched = analyzeSourceAdapterSupportPackage(
    value,
    supportAcceptance(),
    { tasks: ["MEDICINE_BASE"] },
  );
  assert.equal(mismatched.ok, false);
  assert.match(mismatched.errors.join("\n"), /不能按 MEDICINE_BASE 任务分析/);
});

test("support package classifies field changes by declared business impact", () => {
  const acceptance = supportAcceptance();
  const analysis = analyzeSourceAdapterSupportPackage(
    supportPackage(),
    acceptance,
    { tasks: ["MEDICINE_BASE"] },
  );
  assert.equal(analysis.ok, true);
  assert.equal(analysis.summary.contractImpact.status, "REVIEW");
  assert.deepEqual(
    analysis.summary.contractImpact.reviews.map((item) => item.path),
    ["DRUG_PRODUCT.FACTORY_ID"],
  );
  assert.deepEqual(
    analysis.summary.contractImpact.compatibleFallbacks.map(
      (item) => item.path,
    ),
    ["DRUG_MASTER.APPROVAL_NO"],
  );
  const summary = formatSourceAdapterSupportAnalysis(analysis);
  assert.match(summary, /需核对：[\s\S]*DRUG_PRODUCT\.FACTORY_ID/);
  assert.match(summary, /可兼容降级：[\s\S]*DRUG_MASTER\.APPROVAL_NO/);
  assert.match(summary, /评估任务：药品基础数据/);

  const inventoryOnly = analyzeSourceAdapterSupportPackage(
    supportPackage(),
    acceptance,
    { tasks: ["INVENTORY"] },
  );
  assert.equal(inventoryOnly.summary.contractImpact.status, "COMPATIBLE");
  assert.deepEqual(inventoryOnly.summary.contractImpact.reviews, []);
  assert.deepEqual(
    inventoryOnly.summary.contractImpact.compatibleFallbacks,
    [],
  );

  const missingInventoryField = supportPackage();
  missingInventoryField.diagnostics[0].objectStructures[0].columns =
    missingInventoryField.diagnostics[0].objectStructures[0].columns.filter(
      (column) => column.name !== "PACK_FACTOR",
    );
  missingInventoryField.checksum = sourceAdapterSupportChecksum(
    missingInventoryField,
  );
  const blockedInventory = analyzeSourceAdapterSupportPackage(
    missingInventoryField,
    acceptance,
    { tasks: ["INVENTORY"] },
  );
  assert.equal(blockedInventory.summary.contractImpact.status, "BLOCKED");
  assert.deepEqual(
    blockedInventory.summary.contractImpact.blockers.map((item) => item.path),
    ["DRUG_MASTER.PACK_FACTOR"],
  );

  const incompatible = structuredClone(acceptance);
  incompatible.sourceStructure.objects[0].columns.push({
    name: "MEDICINE_CODE",
    requirement: "REQUIRED",
    usedBy: ["药品稳定编码"],
    acceptedTypeFamilies: ["CHARACTER"],
  });
  const blocked = analyzeSourceAdapterSupportPackage(
    supportPackage(),
    incompatible,
    { tasks: ["MEDICINE_BASE"] },
  );
  assert.equal(blocked.summary.contractImpact.status, "BLOCKED");
  assert.deepEqual(
    blocked.summary.contractImpact.blockers.map((item) => item.path),
    ["DRUG_MASTER.MEDICINE_CODE"],
  );

  const invalidTask = analyzeSourceAdapterSupportPackage(
    supportPackage(),
    acceptance,
    { tasks: ["UNSUPPORTED_TASK"] },
  );
  assert.equal(invalidTask.ok, false);
  assert.match(invalidTask.errors.join("\n"), /迁移任务只支持/);
});

test("contract draft turns verified field differences into review-only proposals", () => {
  const value = supportPackage();
  value.migrationTask = "MEDICINE_BASE";
  for (const diagnostic of value.diagnostics)
    diagnostic.migrationTask = "MEDICINE_BASE";
  value.diagnostics[0].checkedObjects.push("DRUG_ALIAS");
  value.diagnostics[0].objectStructures[0].columns.push({
    name: "SEARCH_CODE",
    dataType: "VARCHAR2(40)",
  });
  value.diagnostics[0].objectStructures.push({
    name: "DRUG_ALIAS",
    columns: [
      { name: "DRUG_ID", dataType: "NUMBER" },
      { name: "ALIAS", dataType: "NVARCHAR2(100)" },
    ],
  });
  value.checksum = sourceAdapterSupportChecksum(value);

  const draft = buildSourceStructureContractDraft(
    value,
    supportAcceptance(),
    { generatedAt: "2026-08-27T09:00:00.000Z" },
  );
  assert.equal(draft.migrationTask, "MEDICINE_BASE");
  assert.equal(draft.summary.proposalCount, 2);
  assert.deepEqual(
    draft.proposals.map((proposal) => proposal.action),
    ["ADD_COLUMN", "ADD_OBJECT"],
  );
  assert.equal(
    draft.proposals[0].path,
    "sourceStructure.objects.DRUG_MASTER.columns.SEARCH_CODE",
  );
  assert.deepEqual(draft.proposals[0].draft.acceptedTypeFamilies, [
    "CHARACTER",
  ]);
  assert.deepEqual(
    draft.proposals[1].draft.columns.map((column) =>
      column.acceptedTypeFamilies.join("/"),
    ),
    ["NUMERIC", "CHARACTER"],
  );
  assert.equal(draft.sourceFingerprint, undefined);
  assert.equal(draft.schema, undefined);
  assert.equal(JSON.stringify(draft).includes("1234567890abcdef1234"), false);
  assert.equal(JSON.stringify(draft).includes('"HIS"'), false);
  assert.equal(draft.privacy.containsRawSampleValues, false);
  assert.match(draft.proposals[0].draft.usedBy[0], /TODO/);
  assert.match(
    formatSourceStructureContractDraftSummary(draft, "/tmp/draft.json"),
    /不会自动修改 acceptance\.json/,
  );
});

test("contract draft requires one task, a matching contract, and a v2 field snapshot", () => {
  assert.throws(
    () =>
      buildSourceStructureContractDraft(
        supportPackage(),
        supportAcceptance(),
      ),
    /必须限定一个迁移任务/,
  );
  assert.throws(
    () =>
      buildSourceStructureContractDraft(
        supportPackage(),
        { ...supportAcceptance(), adapterId: "OTHER_HIS" },
        { tasks: ["MEDICINE_BASE"] },
      ),
    /适配器.*不一致/,
  );
  const legacy = supportPackage();
  legacy.version = 1;
  for (const diagnostic of legacy.diagnostics)
    delete diagnostic.objectStructures;
  legacy.checksum = sourceAdapterSupportChecksum(legacy);
  assert.throws(
    () =>
      buildSourceStructureContractDraft(legacy, supportAcceptance(), {
        tasks: ["MEDICINE_BASE"],
      }),
    /没有字段结构快照/,
  );
});

test("fixture draft converts a verified support snapshot without leaking project identity", () => {
  const draft = buildAdapterFixtureDraft(
    supportPackage(),
    supportAcceptance(),
    { tasks: ["MEDICINE_BASE"] },
  );
  assert.equal(draft.id, "medicine-bbbbbbbbbbbb");
  assert.equal(draft.expectations.outcome, "REVIEW");
  assert.deepEqual(
    draft.sourceObjects.map((object) => object.name),
    ["DRUG_MASTER", "DRUG_PRODUCT"],
  );
  assert.deepEqual(draft.sourceObjects[0].columns, [
    { name: "ID", dataType: "NUMBER" },
    { name: "NAME", dataType: "NVARCHAR2" },
    { name: "PACK_FACTOR", dataType: "NUMBER" },
  ]);
  const serialized = JSON.stringify(draft);
  assert.equal(serialized.includes("1234567890abcdef1234"), false);
  assert.equal(serialized.includes('"HIS"'), false);
  assert.equal(serialized.includes("NVARCHAR2"), true);
  assert.match(draft.rustTest, /TODO/);
  assert.match(
    formatAdapterFixtureDraftSummary(draft, "/tmp/fixture.draft.json"),
    /不要移入 fixtures 目录/,
  );
  const errors = validateAdapterFixture({
    fixture: draft,
    manifest: {
      id: "VENDOR_HIS",
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE"],
    },
    file: "fixture.draft.json",
  });
  assert.match(errors.join("\n"), /outcome 必须为 PASS 或 BLOCK/);
  assert.match(errors.join("\n"), /rustTest/);
});

test("fixture draft preserves a contract blocker as a BLOCK recommendation", () => {
  const value = supportPackage();
  value.diagnostics[0].objectStructures[0].columns =
    value.diagnostics[0].objectStructures[0].columns.filter(
      (column) => column.name !== "PACK_FACTOR",
    );
  value.checksum = sourceAdapterSupportChecksum(value);
  const draft = buildAdapterFixtureDraft(value, supportAcceptance(), {
    tasks: ["INVENTORY"],
  });
  assert.equal(draft.expectations.outcome, "BLOCK");
  assert.match(draft.expectations.checks.join("\n"), /DRUG_MASTER\.PACK_FACTOR/);
});

test("maintenance handoff turns one support package into a task-ordered privacy-safe plan", () => {
  const bundle = buildAdapterMaintenanceHandoff(
    supportPackage(),
    supportAcceptance(),
    {
      tasks: ["MEDICINE_BASE"],
      generatedAt: "2026-08-27T10:00:00.000Z",
    },
  );
  assert.equal(
    bundle.manifest.format,
    "medicine-migration-adapter-maintenance-handoff",
  );
  assert.equal(bundle.manifest.migrationTask, "MEDICINE_BASE");
  assert.equal(bundle.manifest.summary.contractStatus, "REVIEW");
  assert.deepEqual(
    bundle.manifest.workItems.map((item) => item.id),
    [
      "SOURCE_CONTRACT",
      "SOURCE_IMPLEMENTATION",
      "PROJECT_FIXTURE",
      "EXECUTABLE_EVIDENCE",
      "DELIVERY_GATE",
    ],
  );
  assert.match(bundle.planMarkdown, /MedicineSourceAdapter::inspect/);
  assert.match(bundle.planMarkdown, /MedicineSourceAdapter::load/);
  assert.match(bundle.planMarkdown, /不会自动修改 acceptance\.json/);
  assert.match(
    formatAdapterMaintenanceHandoffSummary(bundle, "/tmp/handoff"),
    /维护交接包已生成/,
  );
  const serialized = JSON.stringify(bundle);
  assert.equal(serialized.includes("1234567890abcdef1234"), false);
  assert.equal(serialized.includes('"schema":"HIS"'), false);
  assert.equal(bundle.manifest.privacy.containsRawSampleValues, false);
  const artifactContents = {
    "contract-draft.json": `${JSON.stringify(bundle.contractDraft, null, 2)}\n`,
    "fixture.draft.json": `${JSON.stringify(bundle.fixtureDraft, null, 2)}\n`,
    "IMPLEMENTATION_PLAN.md": bundle.planMarkdown,
  };
  bundle.manifest.artifacts = {
    "contract-draft.json": {
      role: "REVIEW_ONLY_CONTRACT_DRAFT",
      sha256: createHash("sha256")
        .update(artifactContents["contract-draft.json"], "utf8")
        .digest("hex"),
    },
    "fixture.draft.json": {
      role: "REVIEW_ONLY_FIXTURE_DRAFT",
      sha256: createHash("sha256")
        .update(artifactContents["fixture.draft.json"], "utf8")
        .digest("hex"),
    },
    "IMPLEMENTATION_PLAN.md": {
      role: "MAINTAINER_IMPLEMENTATION_PLAN",
      sha256: createHash("sha256")
        .update(artifactContents["IMPLEMENTATION_PLAN.md"], "utf8")
        .digest("hex"),
    },
  };
  bundle.manifest.checksum = maintenanceHandoffChecksum(bundle.manifest);
  const verified = analyzeAdapterMaintenanceHandoff(
    bundle.manifest,
    artifactContents,
  );
  assert.equal(verified.ok, true, verified.errors.join("\n"));
  assert.match(
    formatAdapterMaintenanceHandoffVerification(verified, "/tmp/handoff"),
    /验收通过/,
  );
  const tamperedManifest = structuredClone(bundle.manifest);
  tamperedManifest.nextAction = "被修改";
  const tampered = analyzeAdapterMaintenanceHandoff(
    tamperedManifest,
    artifactContents,
  );
  assert.equal(tampered.ok, false);
  assert.match(tampered.errors.join("\n"), /handoff\.json 完整性校验失败/);
  const leakedArtifacts = { ...artifactContents };
  const leakedContract = JSON.parse(leakedArtifacts["contract-draft.json"]);
  leakedContract.sourceFingerprint = "1234567890abcdef1234";
  leakedArtifacts["contract-draft.json"] = `${JSON.stringify(leakedContract, null, 2)}\n`;
  const leakedManifest = structuredClone(bundle.manifest);
  leakedManifest.artifacts["contract-draft.json"].sha256 = createHash("sha256")
    .update(leakedArtifacts["contract-draft.json"], "utf8")
    .digest("hex");
  leakedManifest.checksum = maintenanceHandoffChecksum(leakedManifest);
  const leaked = analyzeAdapterMaintenanceHandoff(
    leakedManifest,
    leakedArtifacts,
  );
  assert.equal(leaked.ok, false);
  assert.match(leaked.errors.join("\n"), /sourceFingerprint/);

  const inventoryPackage = supportPackage();
  inventoryPackage.diagnostics[0].objectStructures[0].columns =
    inventoryPackage.diagnostics[0].objectStructures[0].columns.filter(
      (column) => column.name !== "PACK_FACTOR",
    );
  inventoryPackage.checksum = sourceAdapterSupportChecksum(inventoryPackage);
  const inventory = buildAdapterMaintenanceHandoff(
    inventoryPackage,
    supportAcceptance(),
    { tasks: ["INVENTORY"] },
  );
  assert.equal(inventory.manifest.summary.contractStatus, "BLOCKED");
  assert.equal(
    inventory.manifest.workItems.find(
      (item) => item.id === "SOURCE_IMPLEMENTATION",
    ).status,
    "WAITING",
  );
  assert.match(inventory.planMarkdown, /InventorySourceAdapter::load_stock_items/);
});

test("inventory contract activates only the source branch that is present", () => {
  const acceptance = supportAcceptance();
  acceptance.sourceStructure.objects.push(
    {
      name: "WAREHOUSE_STOCK",
      requirement: "OPTIONAL",
      appliesToTasks: ["INVENTORY"],
      usedBy: ["药库库存"],
      fallback: "没有药库库存时不读取该分支",
      columns: [
        {
          name: "QUANTITY",
          requirement: "REQUIRED",
          usedBy: ["库存数量"],
          acceptedTypeFamilies: ["NUMERIC"],
        },
      ],
    },
    {
      name: "PHARMACY_STOCK",
      requirement: "OPTIONAL",
      appliesToTasks: ["INVENTORY"],
      usedBy: ["药房库存"],
      fallback: "没有药房库存时不读取该分支",
      columns: [
        {
          name: "QUANTITY",
          requirement: "REQUIRED",
          usedBy: ["库存数量"],
          acceptedTypeFamilies: ["NUMERIC"],
        },
      ],
    },
    {
      name: "PHARMACY_PACK",
      requirement: "CONDITIONAL",
      appliesToTasks: ["INVENTORY"],
      requiredWhenObjectsPresent: ["PHARMACY_STOCK"],
      usedBy: ["药房包装"],
      fallback: "没有药房库存时不读取包装分支",
      columns: [
        {
          name: "FACTOR",
          requirement: "REQUIRED",
          usedBy: ["药房包装系数"],
          acceptedTypeFamilies: ["NUMERIC"],
        },
      ],
    },
  );
  acceptance.sourceStructure.objectGroups = [
    {
      name: "库存明细来源",
      objects: ["WAREHOUSE_STOCK", "PHARMACY_STOCK"],
      requirement: "CONDITIONAL",
      appliesToTasks: ["INVENTORY"],
      requiredForTasks: ["INVENTORY"],
      usedBy: ["库存明细"],
      fallback: "至少需要一个库存明细来源",
    },
  ];

  const noStock = analyzeSourceAdapterSupportPackage(
    supportPackage(),
    acceptance,
    { tasks: ["INVENTORY"] },
  );
  assert.equal(noStock.summary.contractImpact.status, "BLOCKED");
  assert.ok(
    noStock.summary.contractImpact.blockers.some(
      (item) => item.path === "{WAREHOUSE_STOCK|PHARMACY_STOCK}",
    ),
  );

  const warehouseOnlyPackage = supportPackage();
  warehouseOnlyPackage.diagnostics[0].checkedObjects.push("WAREHOUSE_STOCK");
  warehouseOnlyPackage.diagnostics[0].objectStructures.push({
    name: "WAREHOUSE_STOCK",
    columns: [{ name: "QUANTITY", dataType: "NUMBER" }],
  });
  warehouseOnlyPackage.checksum = sourceAdapterSupportChecksum(
    warehouseOnlyPackage,
  );
  const warehouseOnly = analyzeSourceAdapterSupportPackage(
    warehouseOnlyPackage,
    acceptance,
    { tasks: ["INVENTORY"] },
  );
  assert.equal(warehouseOnly.summary.contractImpact.status, "COMPATIBLE");
  assert.ok(
    warehouseOnly.summary.contractImpact.compatibleFallbacks.some(
      (item) => item.path === "PHARMACY_PACK",
    ),
  );

  const pharmacyWithoutPackPackage = supportPackage();
  pharmacyWithoutPackPackage.diagnostics[0].checkedObjects.push(
    "PHARMACY_STOCK",
  );
  pharmacyWithoutPackPackage.diagnostics[0].objectStructures.push({
    name: "PHARMACY_STOCK",
    columns: [{ name: "QUANTITY", dataType: "NUMBER" }],
  });
  pharmacyWithoutPackPackage.checksum = sourceAdapterSupportChecksum(
    pharmacyWithoutPackPackage,
  );
  const pharmacyWithoutPack = analyzeSourceAdapterSupportPackage(
    pharmacyWithoutPackPackage,
    acceptance,
    { tasks: ["INVENTORY"] },
  );
  assert.equal(pharmacyWithoutPack.summary.contractImpact.status, "BLOCKED");
  assert.ok(
    pharmacyWithoutPack.summary.contractImpact.blockers.some(
      (item) => item.path === "PHARMACY_PACK",
    ),
  );
});

test("support package CLI prints a maintainer-ready summary", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "adapter-support-"));
  const file = path.join(root, "support.json");
  const acceptanceFile = path.join(root, "acceptance.json");
  await writeFile(file, `${JSON.stringify(supportPackage(), null, 2)}\n`);
  await writeFile(
    acceptanceFile,
    `${JSON.stringify(supportAcceptance(), null, 2)}\n`,
  );
  const result = spawnSync(
    process.execPath,
    [
      path.resolve("scripts/source-adapter-kit.mjs"),
      "support",
      "--file",
      file,
      "--acceptance",
      acceptanceFile,
      "--task",
      "MEDICINE_BASE",
    ],
    { cwd: process.cwd(), encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /三方 HIS 适配器支持包摘要/);
  assert.match(result.stdout, /当前结论：结构通过/);
  assert.match(result.stdout, /已恢复 DRUG_PRODUCT/);
  assert.match(result.stdout, /DRUG_MASTER.PACK_FACTOR/);
  assert.match(result.stdout, /契约影响：/);
  assert.match(result.stdout, /评估任务：药品基础数据/);
  assert.match(result.stdout, /DRUG_PRODUCT.FACTORY_ID/);
});

test("contract draft CLI writes a separate maintainer review artifact", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "adapter-contract-draft-"));
  const file = path.join(root, "support.json");
  const acceptanceFile = path.join(root, "acceptance.json");
  const outputFile = path.join(root, "draft.json");
  const value = supportPackage();
  value.diagnostics[0].objectStructures[0].columns.push({
    name: "SEARCH_CODE",
    dataType: "VARCHAR2",
  });
  value.checksum = sourceAdapterSupportChecksum(value);
  await writeFile(file, `${JSON.stringify(value, null, 2)}\n`);
  await writeFile(
    acceptanceFile,
    `${JSON.stringify(supportAcceptance(), null, 2)}\n`,
  );
  const result = spawnSync(
    process.execPath,
    [
      path.resolve("scripts/source-adapter-kit.mjs"),
      "contract-draft",
      "--file",
      file,
      "--acceptance",
      acceptanceFile,
      "--task",
      "MEDICINE_BASE",
      "--output",
      outputFile,
    ],
    { cwd: process.cwd(), encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /结构契约差异草稿已生成/);
  const draft = JSON.parse(await readFile(outputFile, "utf8"));
  assert.equal(draft.format, "medicine-migration-source-contract-draft");
  assert.equal(draft.proposals[0].action, "ADD_COLUMN");
  assert.equal(draft.proposals[0].draft.name, "SEARCH_CODE");
  assert.equal(JSON.stringify(draft).includes("1234567890abcdef1234"), false);
});

test("fixture draft CLI writes a deterministic draft outside the fixtures gate", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "adapter-fixture-draft-"));
  const file = path.join(root, "support.json");
  const acceptanceFile = path.join(root, "acceptance.json");
  const outputFile = path.join(root, "fixture.draft.json");
  await writeFile(file, `${JSON.stringify(supportPackage(), null, 2)}\n`);
  await writeFile(
    acceptanceFile,
    `${JSON.stringify(supportAcceptance(), null, 2)}\n`,
  );
  const result = spawnSync(
    process.execPath,
    [
      path.resolve("scripts/source-adapter-kit.mjs"),
      "fixture-draft",
      "--file",
      file,
      "--acceptance",
      acceptanceFile,
      "--task",
      "MEDICINE_BASE",
      "--output",
      outputFile,
    ],
    { cwd: process.cwd(), encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /项目变体夹具草稿已生成/);
  const draft = JSON.parse(await readFile(outputFile, "utf8"));
  assert.equal(draft.id, "medicine-bbbbbbbbbbbb");
  assert.equal(draft.sourceObjects.length, 2);
  assert.equal(JSON.stringify(draft).includes("1234567890abcdef1234"), false);
});

test("maintenance handoff CLI writes one integrity-listed review bundle without overwriting", async () => {
  const root = await mkdtemp(path.join(os.tmpdir(), "adapter-handoff-"));
  const file = path.join(root, "support.json");
  const acceptanceFile = path.join(root, "acceptance.json");
  const outputDir = path.join(root, "handoff");
  await writeFile(file, `${JSON.stringify(supportPackage(), null, 2)}\n`);
  await writeFile(
    acceptanceFile,
    `${JSON.stringify(supportAcceptance(), null, 2)}\n`,
  );
  const args = [
    path.resolve("scripts/source-adapter-kit.mjs"),
    "handoff",
    "--file",
    file,
    "--acceptance",
    acceptanceFile,
    "--task",
    "MEDICINE_BASE",
    "--output",
    outputDir,
  ];
  const result = spawnSync(process.execPath, args, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /维护交接包已生成/);
  const manifest = JSON.parse(
    await readFile(path.join(outputDir, "handoff.json"), "utf8"),
  );
  assert.equal(
    manifest.format,
    "medicine-migration-adapter-maintenance-handoff",
  );
  assert.equal(manifest.checksum, maintenanceHandoffChecksum(manifest));
  const generatedArtifacts = {};
  for (const name of [
    "contract-draft.json",
    "fixture.draft.json",
    "IMPLEMENTATION_PLAN.md",
  ]) {
    const content = await readFile(path.join(outputDir, name), "utf8");
    generatedArtifacts[name] = content;
    assert.equal(
      manifest.artifacts[name].sha256,
      createHash("sha256").update(content, "utf8").digest("hex"),
    );
    assert.equal(content.includes("1234567890abcdef1234"), false);
  }
  const plan = await readFile(
    path.join(outputDir, "IMPLEMENTATION_PLAN.md"),
    "utf8",
  );
  assert.match(plan, /## 完成定义/);
  assert.match(plan, /adapter:verify/);

  const verifyArgs = [
    path.resolve("scripts/source-adapter-kit.mjs"),
    "handoff-verify",
    "--dir",
    outputDir,
  ];
  const verified = spawnSync(process.execPath, verifyArgs, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.equal(verified.status, 0, verified.stdout + verified.stderr);
  assert.match(verified.stdout, /维护交接包验收通过/);
  assert.match(verified.stdout, /文件 SHA-256/);

  const repeated = spawnSync(process.execPath, args, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.notEqual(repeated.status, 0);
  assert.match(repeated.stderr, /为避免覆盖既有审核记录/);

  await writeFile(
    path.join(outputDir, "IMPLEMENTATION_PLAN.md"),
    `${generatedArtifacts["IMPLEMENTATION_PLAN.md"]}\n被修改`,
  );
  const tampered = spawnSync(process.execPath, verifyArgs, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.notEqual(tampered.status, 0);
  assert.match(tampered.stdout, /IMPLEMENTATION_PLAN\.md 完整性校验失败/);
  await writeFile(
    path.join(outputDir, "IMPLEMENTATION_PLAN.md"),
    generatedArtifacts["IMPLEMENTATION_PLAN.md"],
  );

  await unlink(path.join(outputDir, "fixture.draft.json"));
  const missing = spawnSync(process.execPath, verifyArgs, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.notEqual(missing.status, 0);
  assert.match(missing.stdout, /交接目录缺少文件：fixture\.draft\.json/);
  await writeFile(
    path.join(outputDir, "fixture.draft.json"),
    generatedArtifacts["fixture.draft.json"],
  );

  await writeFile(path.join(outputDir, "notes.txt"), "未声明文件\n");
  const unexpected = spawnSync(process.execPath, verifyArgs, {
    cwd: process.cwd(),
    encoding: "utf8",
  });
  assert.notEqual(unexpected.status, 0);
  assert.match(unexpected.stdout, /交接目录含未声明文件：notes\.txt/);
});

test("generic source object diagnostics are integrity checked and produce actionable advice", () => {
  const value = sourceObjectDiagnostic();
  const analysis = analyzeSourceObjectDiagnostic(value);
  assert.equal(analysis.ok, true);
  assert.equal(analysis.summary.columnCount, 2);
  assert.match(analysis.summary.recommendations.join("\n"), /稳定业务键拆分/);
  assert.match(analysis.summary.recommendations.join("\n"), /规格/);
  const summary = formatSourceObjectDiagnosticAnalysis(analysis);
  assert.match(summary, /通用来源结构诊断摘要/);
  assert.match(summary, /样例：2 行（仅保留形态统计）/);
  assert.match(summary, /SHA-256 校验通过/);

  const tampered = structuredClone(value);
  tampered.columns[1].comment = "被修改";
  const tamperedAnalysis = analyzeSourceObjectDiagnostic(tampered);
  assert.equal(tamperedAnalysis.ok, false);
  assert.match(tamperedAnalysis.errors.join("\n"), /完整性校验失败/);

  const unsafe = structuredClone(value);
  unsafe.columns[0].sampleValues = ["敏感药品值"];
  unsafe.checksum = sourceObjectDiagnosticChecksum(unsafe);
  const unsafeAnalysis = analyzeSourceObjectDiagnostic(unsafe);
  assert.equal(unsafeAnalysis.ok, false);
  assert.match(unsafeAnalysis.errors.join("\n"), /原始值字段|未声明字段/);
});

test("source object diagnostic CLI validates and summarizes a field handoff", async () => {
  const root = await mkdtemp(
    path.join(os.tmpdir(), "source-object-diagnostic-"),
  );
  const file = path.join(root, "diagnostic.json");
  await writeFile(
    file,
    `${JSON.stringify(sourceObjectDiagnostic(), null, 2)}\n`,
  );
  const result = spawnSync(
    process.execPath,
    [
      path.resolve("scripts/source-adapter-kit.mjs"),
      "source-object",
      "--file",
      file,
    ],
    { cwd: process.cwd(), encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /通用来源结构诊断摘要/);
  assert.match(result.stdout, /来源对象：YK_TYPK/);
  assert.match(result.stdout, /建议下一步/);
});

test("project variant fixtures reject credentials and undeclared tasks", () => {
  const manifest = {
    id: "VENDOR_HIS",
    databaseFamilies: ["oracle"],
    migrationTasks: ["MEDICINE_BASE"],
  };
  const errors = validateAdapterFixture({
    manifest,
    fixture: {
      fixtureSchemaVersion: 1,
      adapterId: "VENDOR_HIS",
      id: "unsafe-project",
      title: "不安全项目夹具",
      databaseFamily: "oracle",
      tasks: ["INVENTORY"],
      sourceObjects: [{ name: "DRUG_MASTER", columns: ["ID"] }],
      expectations: { outcome: "PASS", checks: ["读取成功"] },
      rustTest: "vendor_his::tests::unsafe_project",
      password: "secret",
    },
  });
  assert.match(errors.join("\n"), /INVENTORY.*未在 manifest/);
  assert.match(errors.join("\n"), /password/);
});

test("typed v2 fixtures reject samples and require physical data types", () => {
  const errors = validateAdapterFixture({
    manifest: {
      id: "VENDOR_HIS",
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE"],
    },
    fixture: {
      fixtureSchemaVersion: 2,
      adapterId: "VENDOR_HIS",
      id: "typed-project",
      title: "类型化项目夹具",
      databaseFamily: "oracle",
      tasks: ["MEDICINE_BASE"],
      sourceObjects: [{
        name: "DRUG_MASTER",
        rows: [{ ID: 1 }],
        columns: [
          { name: "ID", dataType: "NUMBER" },
          { name: "NAME", dataType: "", sampleValue: "敏感样例" },
        ],
      }],
      expectations: { outcome: "PASS", checks: ["读取成功"] },
      rustTest: "vendor_his::tests::typed_project",
      notes: "现场结构说明",
    },
  });
  assert.match(errors.join("\n"), /dataType/);
  assert.match(errors.join("\n"), /sampleValue/);
  assert.match(errors.join("\n"), /rows/);
  assert.match(errors.join("\n"), /notes/);
});

test("legacy v1 fixtures remain valid while rejecting typed columns", () => {
  const manifest = {
    id: "VENDOR_HIS",
    databaseFamilies: ["oracle"],
    migrationTasks: ["MEDICINE_BASE"],
  };
  const fixture = {
    fixtureSchemaVersion: 1,
    adapterId: "VENDOR_HIS",
    id: "legacy-project",
    title: "旧版项目夹具",
    databaseFamily: "oracle",
    tasks: ["MEDICINE_BASE"],
    sourceObjects: [{ name: "DRUG_MASTER", columns: ["ID", "NAME"] }],
    expectations: { outcome: "PASS", checks: ["读取成功"] },
    rustTest: "vendor_his::tests::legacy_project",
  };
  assert.deepEqual(validateAdapterFixture({ fixture, manifest }), []);
  fixture.sourceObjects[0].columns[0] = { name: "ID", dataType: "NUMBER" };
  assert.match(validateAdapterFixture({ fixture, manifest }).join("\n"), /v1.*字段名文本/);
});

test("adapter manifest rejects project connections and unbound implementations", () => {
  const report = validateAdapterPackage({
    packageName: "vendor_his",
    manifest: {
      packageSchemaVersion: 1,
      id: "VENDOR_HIS",
      name: "厂商 HIS",
      version: 1,
      templateCompatibleFromVersion: 1,
      changes: [{ version: 1, summary: "初始版本" }],
      summary: "测试",
      sourceModes: ["database"],
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE"],
      automaticDetection: true,
      reusableMappingProfiles: true,
      builtIn: true,
      implementation: "VENDOR_HIS_BUILTIN",
      password: "secret",
    },
    acceptance: {
      acceptanceSchemaVersion: 1,
      adapterId: "VENDOR_HIS",
      medicine: {
        requiredObjects: ["DRUG_MASTER"],
        sourceIdentityFields: ["DRUG_MASTER.ID"],
        scopes: ["ALL"],
        testCases: ["标准结构", "缺失可选列"],
      },
    },
    sourceAdapterRust: "",
  });
  assert.match(report.errors.join("\n"), /password/);
  assert.match(report.errors.join("\n"), /尚未绑定/);
  assert.match(report.errors.join("\n"), /缺少 MAINTENANCE\.md/);
});

test("adapter manifest requires a valid compatibility range and current change note", () => {
  const report = validateAdapterPackage({
    packageName: "vendor_guided",
    manifest: {
      packageSchemaVersion: 1,
      id: "VENDOR_GUIDED",
      name: "厂商引导适配器",
      version: 2,
      templateCompatibleFromVersion: 3,
      changes: [{ version: 1, summary: "初始版本" }],
      summary: "测试",
      sourceModes: ["database"],
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE"],
      automaticDetection: false,
      reusableMappingProfiles: true,
      builtIn: true,
      implementation: "GUIDED_MAPPING",
    },
    sourceAdapterRust: "",
  });
  assert.match(
    report.errors.join("\n"),
    /templateCompatibleFromVersion 必须在 1 到当前 version 之间/,
  );
  assert.match(report.errors.join("\n"), /changes 必须包含当前 version/);
});

test("automatic adapter must declare an evidence-based HIS compatibility policy", () => {
  const base = {
    packageSchemaVersion: 1,
    id: "VENDOR_HIS",
    name: "厂商 HIS",
    version: 1,
    templateCompatibleFromVersion: 1,
    changes: [{ version: 1, summary: "初始版本" }],
    summary: "测试",
    sourceModes: ["database"],
    databaseFamilies: ["oracle"],
    migrationTasks: ["MEDICINE_BASE"],
    automaticDetection: true,
    reusableMappingProfiles: true,
    builtIn: true,
    implementation: "VENDOR_HIS_BUILTIN",
    medicineWorkflow: {
      sourceKeyMode: "ADAPTER_PROVIDED",
      sourceKeyField: "SOURCE_KEY",
      sourceKeyLabel: "稳定来源键",
      mappingPreset: "GUIDED",
      batchSourceType: "GENERIC",
      factoryPolicy: "OPTIONAL_CREATE",
      legacyProfileKind: "NONE",
    },
  };
  const missing = validateAdapterPackage({
    packageName: "vendor_his",
    manifest: structuredClone(base),
  });
  assert.match(missing.errors.join("\n"), /必须声明 sourceCompatibility/);

  const inventedRange = validateAdapterPackage({
    packageName: "vendor_his",
    manifest: {
      ...structuredClone(base),
      sourceCompatibility: {
        product: "厂商 HIS",
        mode: "DECLARED_VERSION_RANGE",
        declaredVersions: [],
        gate: "通过版本号放行",
      },
    },
  });
  assert.match(inventedRange.errors.join("\n"), /必须明确列出已验证版本范围/);

  const structureWithVersions = validateAdapterPackage({
    packageName: "vendor_his",
    manifest: {
      ...structuredClone(base),
      sourceCompatibility: {
        product: "厂商 HIS",
        mode: "STRUCTURE_CONTRACT",
        declaredVersions: ["V1"],
        gate: "按结构契约验收",
      },
    },
  });
  assert.match(structureWithVersions.errors.join("\n"), /不能声明厂商版本范围/);
});

test("adapter manifest rejects unregistered medicine workflow semantics", () => {
  const report = validateAdapterPackage({
    packageName: "vendor_guided",
    manifest: {
      packageSchemaVersion: 1,
      id: "VENDOR_GUIDED",
      name: "厂商引导适配器",
      version: 1,
      templateCompatibleFromVersion: 1,
      changes: [{ version: 1, summary: "初始版本" }],
      summary: "测试",
      sourceModes: ["database"],
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE"],
      automaticDetection: false,
      reusableMappingProfiles: true,
      builtIn: true,
      implementation: "GUIDED_MAPPING",
      medicineWorkflow: {
        sourceKeyMode: "ADAPTER_PROVIDED",
        sourceKeyField: "DROP TABLE",
        sourceKeyLabel: "伪来源键",
        mappingPreset: "UNKNOWN_PRESET",
        batchSourceType: "UNKNOWN_BATCH",
        factoryPolicy: "OPTIONAL_CREATE",
        legacyProfileKind: "UNKNOWN_LEGACY",
      },
    },
  });
  const errors = report.errors.join("\n");
  assert.match(errors, /合法的 sourceKeyField/);
  assert.match(errors, /mappingPreset 尚未注册/);
  assert.match(errors, /batchSourceType 尚未注册/);
  assert.match(errors, /legacyProfileKind 尚未注册/);
});

test("inventory workflow cannot disable shared first-stocktake safety gates", () => {
  const report = validateAdapterPackage({
    packageName: "vendor_inventory",
    manifest: {
      packageSchemaVersion: 1,
      id: "VENDOR_INVENTORY",
      name: "厂商库存适配器",
      version: 1,
      templateCompatibleFromVersion: 1,
      changes: [{ version: 1, summary: "初始版本" }],
      summary: "测试",
      sourceModes: ["database"],
      databaseFamilies: ["oracle"],
      migrationTasks: ["MEDICINE_BASE", "INVENTORY"],
      automaticDetection: true,
      reusableMappingProfiles: true,
      builtIn: true,
      implementation: "VENDOR_INVENTORY_BUILTIN",
      medicineWorkflow: {
        sourceKeyMode: "ADAPTER_PROVIDED",
        sourceKeyField: "SOURCE_KEY",
        sourceKeyLabel: "稳定来源键",
        mappingPreset: "GUIDED",
        batchSourceType: "GENERIC",
        factoryPolicy: "ADAPTER_MANAGED",
        legacyProfileKind: "NONE",
      },
      inventoryWorkflow: {
        normalizedLocationKinds: ["WAREHOUSE", "UNSAFE_KIND"],
        sourceProductKeyLabel: "来源商品键",
        sourceStockKeyMode: "UNSAFE_MODE",
        writeMode: "DIRECT_INSERT",
        organizationMapping: "OPTIONAL",
        medicineLedger: "OPTIONAL",
        trialPolicy: "NONE",
        undoPolicy: "UNRESTRICTED",
      },
    },
  });
  const errors = report.errors.join("\n");
  assert.match(errors, /normalizedLocationKinds/);
  assert.match(errors, /sourceStockKeyMode/);
  assert.match(errors, /writeMode 必须为 FIRST_STOCKTAKE/);
  assert.match(errors, /trialPolicy 必须为 PER_TARGET_STORAGE/);
  assert.match(errors, /undoPolicy 必须为 VERIFIED_BATCH_ONLY/);
});

async function temporaryProject() {
  const root = await mkdtemp(path.join(os.tmpdir(), "source-adapter-kit-"));
  await mkdir(path.join(root, "src-tauri", "adapters"), { recursive: true });
  await mkdir(path.join(root, "src-tauri", "src"), { recursive: true });
  await writeFile(path.join(root, "src-tauri", "src", "source_adapter.rs"), "");
  return root;
}

test("guided adapter scaffold is immediately valid and contains no credentials", async () => {
  const root = await temporaryProject();
  const created = await createAdapterPackage(root, {
    id: "VENDOR_GUIDED",
    name: "厂商引导模板",
    database: "oracle,mysql",
    guided: true,
  });
  const manifestText = await readFile(
    path.join(created.packageRoot, "manifest.json"),
    "utf8",
  );
  const maintenanceText = await readFile(
    path.join(created.packageRoot, "MAINTENANCE.md"),
    "utf8",
  );
  assert.doesNotMatch(manifestText, /password|tenant|host/i);
  const manifest = JSON.parse(manifestText);
  assert.equal(manifest.medicineWorkflow.sourceKeyMode, "USER_SELECTED");
  assert.equal(manifest.medicineWorkflow.mappingPreset, "GUIDED");
  assert.match(maintenanceText, /templateCompatibleFromVersion/);
  assert.match(maintenanceText, /npm run adapter:support/);
  assert.match(maintenanceText, /npm run adapter:verify/);
  const reports = await validateAdapterRegistry(root);
  assert.equal(validationSummary(reports).ok, true);
});

test("automatic inventory scaffold remains blocked until evidence and binding are completed", async () => {
  const root = await temporaryProject();
  const created = await createAdapterPackage(root, {
    id: "VENDOR_AUTO",
    name: "厂商自动适配器",
    database: "oracle",
    tasks: "MEDICINE_BASE,INVENTORY",
  });
  const implementationTemplate = await readFile(
    path.join(created.packageRoot, "adapter.rs.template"),
    "utf8",
  );
  const intake = await readFile(
    path.join(created.packageRoot, "PROJECT_INTAKE.md"),
    "utf8",
  );
  assert.match(implementationTemplate, /impl MedicineSourceAdapter/);
  assert.match(implementationTemplate, /impl InventorySourceAdapter/);
  assert.match(implementationTemplate, /use super::sdk::\*/);
  assert.match(implementationTemplate, /source_qualified_object/);
  assert.match(implementationTemplate, /source_text_filter_list/);
  assert.match(
    implementationTemplate,
    /InventorySourceStockItem::new\(kind, record_id/,
  );
  assert.match(implementationTemplate, /InventorySourceCatalog::new/);
  assert.match(implementationTemplate, /InventorySourceReadiness::new/);
  assert.match(implementationTemplate, /InventorySourceGuidance::info/);
  assert.match(implementationTemplate, /with_storage_packaging/);
  assert.match(implementationTemplate, /with_single_minimum_package_evidence/);
  assert.match(implementationTemplate, /SourceAdapterBinding::new/);
  assert.match(implementationTemplate, /"VENDOR_AUTO_BUILTIN"/);
  assert.match(intake, /稳定来源键/);
  assert.match(intake, /轻量范围读取/);
  assert.match(intake, /选择后下推过滤/);
  assert.match(intake, /管理包装/);
  assert.match(intake, /首次盘点边界/);
  assert.match(intake, /v2 夹具只含表名、字段名、数据库物理类型/);
  assert.doesNotMatch(intake, /password|host|tenant/i);
  const maintenance = await readFile(
    path.join(created.packageRoot, "MAINTENANCE.md"),
    "utf8",
  );
  assert.match(maintenance, /adapter:status -- --adapter VENDOR_AUTO/);
  assert.match(maintenance, /npm run adapter:handoff/);
  assert.match(maintenance, /npm run adapter:handoff-verify/);
  assert.match(maintenance, /一次生成维护交接计划/);
  const manifest = JSON.parse(
    await readFile(path.join(created.packageRoot, "manifest.json"), "utf8"),
  );
  const acceptance = JSON.parse(
    await readFile(path.join(created.packageRoot, "acceptance.json"), "utf8"),
  );
  assert.equal(acceptance.acceptanceSchemaVersion, 2);
  assert.ok(acceptance.evidence.every((item) => item.rustTest.includes("TODO")));
  assert.ok(acceptance.evidence.every((item) => item.execution === "REQUIRED"));
  assert.ok(acceptance.evidence.some((item) => item.tasks.includes("MEDICINE_BASE")));
  assert.ok(acceptance.evidence.some((item) => item.tasks.includes("INVENTORY")));
  assert.ok(acceptance.sourceStructure.objects.length > 0);
  assert.deepEqual(
    acceptance.sourceStructure.objects.map((object) => object.name),
    ["TODO: 核心药品表", "TODO: 机构表", "TODO: 药库药房表", "TODO: 库存表"],
  );
  assert.equal(
    acceptance.sourceStructure.objects[0].columns[0].requirement,
    "REQUIRED",
  );
  assert.equal(manifest.medicineWorkflow.sourceKeyMode, "ADAPTER_PROVIDED");
  assert.equal(manifest.medicineWorkflow.sourceKeyField, "SOURCE_KEY");
  assert.equal(manifest.sourceCompatibility.mode, "STRUCTURE_CONTRACT");
  assert.deepEqual(manifest.sourceCompatibility.declaredVersions, []);
  assert.deepEqual(manifest.inventoryWorkflow.normalizedLocationKinds, [
    "WAREHOUSE",
    "PHARMACY",
  ]);
  assert.equal(manifest.inventoryWorkflow.writeMode, "FIRST_STOCKTAKE");
  assert.equal(
    manifest.inventoryWorkflow.sourceStockKeyMode,
    "ADAPTER_SCOPED_V1",
  );
  assert.equal(manifest.inventoryWorkflow.trialPolicy, "PER_TARGET_STORAGE");
  const medicineFixture = await readFile(
    path.join(created.packageRoot, "fixtures", "medicine-project-variant.json"),
    "utf8",
  );
  const inventoryFixture = await readFile(
    path.join(
      created.packageRoot,
      "fixtures",
      "inventory-project-variant.json",
    ),
    "utf8",
  );
  assert.match(medicineFixture, /MEDICINE_BASE/);
  assert.match(inventoryFixture, /INVENTORY/);
  assert.match(inventoryFixture, /稳定库存行键/);
  assert.match(inventoryFixture, /药品商品键/);
  assert.match(inventoryFixture, /管理包装系数/);
  assert.match(inventoryFixture, /药库药房表/);
  const reports = await validateAdapterRegistry(root);
  assert.equal(validationSummary(reports).ok, false);
  assert.match(reports[0].errors.join("\n"), /TODO|尚未绑定/);
  const readiness = adapterReadiness(reports[0]);
  assert.equal(readiness.ready, false);
  assert.equal(readiness.currentStage.id, "CONTRACT");
  assert.ok(readiness.stages.find((stage) => stage.id === "IMPLEMENTATION").issues.length > 0);
  assert.ok(readiness.stages.find((stage) => stage.id === "FIXTURES").issues.length > 0);
  assert.ok(readiness.stages.find((stage) => stage.id === "EVIDENCE").issues.length > 0);
  const summary = formatAdapterReadiness(reports);
  assert.match(summary, /来源结构与业务口径/);
  assert.match(summary, /其余 \d+ 项由严格验收保留/);
  assert.match(summary, /PROJECT_INTAKE\.md/);
  assert.doesNotMatch(summary, /sourceObjects\[0\]\.columns\[7\]/);
  const status = spawnSync(
    process.execPath,
    [
      "scripts/source-adapter-kit.mjs",
      "status",
      "--root",
      root,
      "--adapter",
      "VENDOR_AUTO",
    ],
    { cwd: process.cwd(), encoding: "utf8" },
  );
  assert.equal(status.status, 1);
  assert.match(status.stdout, /三方 HIS 适配器就绪诊断/);
  assert.match(status.stdout, /需要完整错误明细.*adapter:validate/);
});
