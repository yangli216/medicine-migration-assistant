import { mkdir, readFile, readdir, writeFile } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const PACKAGE_SCHEMA_VERSION = 1;
export const ACCEPTANCE_SCHEMA_VERSION = 2;
export const FIXTURE_SCHEMA_VERSION = 2;
const supportedFixtureSchemaVersions = new Set([1, 2]);
export const SUPPORT_PACKAGE_FORMAT = "medicine-migration-adapter-support";
export const SUPPORT_PACKAGE_VERSION = 2;
export const CONTRACT_DRAFT_FORMAT =
  "medicine-migration-source-contract-draft";
export const CONTRACT_DRAFT_VERSION = 1;
export const MAINTENANCE_HANDOFF_FORMAT =
  "medicine-migration-adapter-maintenance-handoff";
export const MAINTENANCE_HANDOFF_VERSION = 1;
const supportedSupportPackageVersions = new Set([1, 2]);
export const SOURCE_OBJECT_DIAGNOSTIC_TYPE = "GENERIC_SOURCE_OBJECT_DIAGNOSTIC";
export const SOURCE_OBJECT_DIAGNOSTIC_VERSION = 1;

const databaseFamilies = new Set([
  "mysql",
  "oracle",
  "dameng",
  "opengauss",
  "vastbase",
  "gbase8c",
  "gbase8a",
  "gbase8s",
  "kingbase",
  "postgresql",
]);
const migrationTasks = new Set(["MEDICINE_BASE", "INVENTORY"]);
const sourceModes = new Set(["database", "file"]);
const manifestKeys = new Set([
  "packageSchemaVersion",
  "id",
  "name",
  "version",
  "templateCompatibleFromVersion",
  "changes",
  "summary",
  "sourceModes",
  "databaseFamilies",
  "migrationTasks",
  "automaticDetection",
  "reusableMappingProfiles",
  "builtIn",
  "implementation",
  "sourceCompatibility",
  "medicineWorkflow",
  "inventoryWorkflow",
]);
const forbiddenManifestKey =
  /^(host|port|username|password|tenant|schema|service(name)?|connection(string)?|query|sql|path)$/i;
const placeholder = /TODO|待填写|REPLACE_ME/i;
const sensitiveSupportKey =
  /^(?:.*(?:password|passwd|pwd).*|username|userName|.*tenant.*|sourceIdentity|targetIdentity|.*connectionString.*|host|hostname|port|serviceName|databaseName|url|path)$/i;
const sensitiveHandoffKey =
  /^(?:.*(?:password|passwd|pwd).*|username|userName|.*tenant.*|sourceFingerprint|sourceIdentity|targetIdentity|.*connectionString.*|host|hostname|port|serviceName|databaseName|url|schema|query|sql|.*sampleValue.*|.*rawSample.*|connection)$/i;
const supportTopLevelKeys = new Set([
  "format",
  "version",
  "adapterId",
  "migrationTask",
  "sourceFingerprint",
  "diagnostics",
  "exportedAt",
  "checksum",
]);
const supportDiagnosticKeys = new Set([
  "diagnosticId",
  "adapterId",
  "migrationTask",
  "adapterVersion",
  "sourceFingerprint",
  "databaseFamily",
  "schema",
  "detected",
  "checkedObjects",
  "missingObjects",
  "objectStructures",
  "metrics",
  "warnings",
  "message",
  "structureHash",
  "recordedAt",
]);
const supportMetricKeys = new Set(["id", "label", "value"]);
const supportObjectStructureKeys = new Set(["name", "columns"]);
const supportObjectColumnKeys = new Set(["name", "dataType"]);
const handoffTopLevelKeys = new Set([
  "format",
  "version",
  "generatedAt",
  "adapterId",
  "adapterVersion",
  "migrationTask",
  "databaseFamily",
  "sourceSupport",
  "summary",
  "nextAction",
  "workItems",
  "artifacts",
  "privacy",
  "checksum",
]);
const handoffSourceSupportKeys = new Set(["checksum", "exportedAt"]);
const handoffSummaryKeys = new Set([
  "contractStatus",
  "observedObjectCount",
  "observedColumnCount",
  "proposalCount",
  "blockerCount",
  "reviewCount",
  "compatibleFallbackCount",
  "declaredEvidenceCount",
]);
const handoffWorkItemKeys = new Set(["id", "status", "title", "details"]);
const handoffArtifactKeys = new Set(["role", "sha256"]);
const handoffPrivacyKeys = new Set([
  "containsSourceFingerprint",
  "containsSchema",
  "containsRawSampleValues",
  "containsSqlOrComments",
  "containsConnectionOrTenantData",
]);
const handoffArtifactRoles = new Map([
  ["contract-draft.json", "REVIEW_ONLY_CONTRACT_DRAFT"],
  ["fixture.draft.json", "REVIEW_ONLY_FIXTURE_DRAFT"],
  ["IMPLEMENTATION_PLAN.md", "MAINTAINER_IMPLEMENTATION_PLAN"],
]);
const fixtureTopLevelKeys = new Set([
  "fixtureSchemaVersion",
  "adapterId",
  "id",
  "title",
  "databaseFamily",
  "tasks",
  "sourceObjects",
  "expectations",
  "rustTest",
]);
const fixtureSourceObjectKeys = new Set(["name", "columns"]);
const fixtureColumnKeys = new Set(["name", "dataType"]);
const fixtureExpectationKeys = new Set(["outcome", "checks"]);
const structureRequirements = new Set(["REQUIRED", "CONDITIONAL", "OPTIONAL"]);
const structureTypeFamilies = new Set([
  "CHARACTER",
  "NUMERIC",
  "TEMPORAL",
  "BINARY",
  "BOOLEAN",
  "OTHER",
]);
const acceptanceStructureKeys = new Set(["objects", "objectGroups"]);
const acceptanceStructureObjectKeys = new Set([
  "name",
  "requirement",
  "appliesToTasks",
  "requiredForTasks",
  "requiredWhenObjectsPresent",
  "usedBy",
  "fallback",
  "columns",
  "columnGroups",
]);
const acceptanceStructureColumnKeys = new Set([
  "name",
  "requirement",
  "appliesToTasks",
  "requiredForTasks",
  "requiredWhenObjectsPresent",
  "usedBy",
  "fallback",
  "acceptedTypeFamilies",
]);
const acceptanceStructureGroupKeys = new Set([
  "name",
  "columns",
  "requirement",
  "appliesToTasks",
  "requiredForTasks",
  "requiredWhenObjectsPresent",
  "usedBy",
  "fallback",
]);
const acceptanceStructureObjectGroupKeys = new Set([
  "name",
  "objects",
  "requirement",
  "appliesToTasks",
  "requiredForTasks",
  "usedBy",
  "fallback",
]);
const objectDiagnosticTopLevelKeys = new Set([
  "formatVersion",
  "packageType",
  "generatedAt",
  "source",
  "preview",
  "medicineFieldAssessment",
  "columns",
  "privacy",
  "checksum",
]);
const objectDiagnosticSourceKeys = new Set([
  "databaseFamily",
  "schema",
  "objectName",
]);
const objectDiagnosticPreviewKeys = new Set([
  "columnCount",
  "sampledRowCount",
  "truncated",
  "elapsedMs",
  "metadataMessage",
  "rowCountProbe",
]);
const objectDiagnosticCountKeys = new Set([
  "status",
  "message",
  "rowCount",
  "probeLimit",
  "elapsedMs",
]);
const objectDiagnosticAssessmentKeys = new Set([
  "status",
  "title",
  "description",
  "foundCount",
  "clues",
]);
const objectDiagnosticClueKeys = new Set([
  "key",
  "label",
  "column",
  "hasNonEmptySample",
]);
const objectDiagnosticColumnKeys = new Set([
  "name",
  "comment",
  "sourceTable",
  "sourceColumn",
  "mappingEligible",
  "sampleProfile",
]);
const objectDiagnosticSampleKeys = new Set([
  "nonEmptyCount",
  "maximumTextLength",
  "observedKinds",
]);
const objectDiagnosticPrivacyKeys = new Set([
  "containsRawSampleValues",
  "excluded",
]);
const objectDiagnosticSensitiveKey =
  /(?:password|passwd|pwd|username|userName|tenant|connectionString|hostname|host|port|serviceName|databaseName|url|rawSample|sampleValues?|rows|originalValues?)/i;
const objectDiagnosticValueKinds = new Set([
  "empty",
  "boolean",
  "integer",
  "decimal",
  "array",
  "object",
  "integer-text",
  "decimal-text",
  "date-text",
  "text",
]);

function unique(values) {
  return [...new Set(values)];
}

function escapedPattern(value) {
  return `${value}`.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function hasImplementationBinding(source, implementation) {
  const escaped = escapedPattern(implementation);
  return new RegExp(
    `(?:implementation\\s*:\\s*|SourceAdapterBinding::new\\s*\\(\\s*)"${escaped}"`,
  ).test(source);
}

function stringList(value) {
  return Array.isArray(value)
    ? value.map((item) => `${item ?? ""}`.trim()).filter(Boolean)
    : [];
}

function collectForbiddenKeys(value, prefix = "") {
  if (!value || typeof value !== "object") return [];
  if (Array.isArray(value)) {
    return value.flatMap((item, index) =>
      collectForbiddenKeys(item, `${prefix}[${index}]`),
    );
  }
  return Object.entries(value).flatMap(([key, item]) => {
    const current = prefix ? `${prefix}.${key}` : key;
    return [
      ...(forbiddenManifestKey.test(key) ? [current] : []),
      ...collectForbiddenKeys(item, current),
    ];
  });
}

function canonicalJson(value) {
  if (Array.isArray(value)) return value.map(canonicalJson);
  if (value && typeof value === "object") {
    return Object.fromEntries(
      Object.keys(value)
        .sort()
        .map((key) => [key, canonicalJson(value[key])]),
    );
  }
  return value;
}

function unknownKeys(value, allowed) {
  return value && typeof value === "object" && !Array.isArray(value)
    ? Object.keys(value).filter((key) => !allowed.has(key))
    : [];
}

function collectSensitiveSupportKeys(value, prefix = "") {
  if (!value || typeof value !== "object") return [];
  if (Array.isArray(value)) {
    return value.flatMap((item, index) =>
      collectSensitiveSupportKeys(item, `${prefix}[${index}]`),
    );
  }
  return Object.entries(value).flatMap(([key, item]) => {
    const current = prefix ? `${prefix}.${key}` : key;
    return [
      ...(sensitiveSupportKey.test(key) ? [current] : []),
      ...collectSensitiveSupportKeys(item, current),
    ];
  });
}

export function sourceAdapterSupportChecksum(value) {
  const unsigned = structuredClone(value);
  delete unsigned.checksum;
  return createHash("sha256")
    .update(JSON.stringify(canonicalJson(unsigned)))
    .digest("hex");
}

export function sourceObjectDiagnosticChecksum(value) {
  const unsigned = structuredClone(value);
  unsigned.checksum = "";
  return createHash("sha256")
    .update(JSON.stringify(canonicalJson(unsigned)))
    .digest("hex");
}

function artifactChecksum(content) {
  return createHash("sha256").update(content, "utf8").digest("hex");
}

export function maintenanceHandoffChecksum(value) {
  const unsigned = structuredClone(value);
  delete unsigned.checksum;
  return createHash("sha256")
    .update(JSON.stringify(canonicalJson(unsigned)))
    .digest("hex");
}

function collectSensitiveHandoffKeys(value, prefix = "") {
  if (!value || typeof value !== "object") return [];
  if (Array.isArray(value))
    return value.flatMap((item, index) =>
      collectSensitiveHandoffKeys(item, `${prefix}[${index}]`),
    );
  return Object.entries(value).flatMap(([key, item]) => {
    const current = prefix ? `${prefix}.${key}` : key;
    if (current === "privacy" || current.startsWith("privacy.")) return [];
    return [
      ...(sensitiveHandoffKey.test(key) ? [current] : []),
      ...collectSensitiveHandoffKeys(item, current),
    ];
  });
}

function validDate(value) {
  return (
    typeof value === "string" &&
    value.trim() &&
    !Number.isNaN(Date.parse(value))
  );
}

function supportList(value, field, errors, maximum = 200, maximumLength = 500) {
  if (!Array.isArray(value)) {
    errors.push(`${field} 必须是数组`);
    return [];
  }
  if (value.length > maximum) errors.push(`${field} 超过 ${maximum} 项上限`);
  if (
    value.some(
      (item) =>
        typeof item !== "string" || !item.trim() || item.length > maximumLength,
    )
  )
    errors.push(`${field} 只能包含 1–${maximumLength} 字符的非空文本`);
  return value.filter((item) => typeof item === "string");
}

function validateSupportObjectStructures(value, field, checkedObjects, errors) {
  if (value === undefined) return [];
  if (!Array.isArray(value)) {
    errors.push(`${field} 必须是数组`);
    return [];
  }
  if (value.length > 80) errors.push(`${field} 超过 80 张表上限`);
  const checked = new Set(
    (checkedObjects || []).map((name) => `${name}`.trim().toUpperCase()),
  );
  const seenObjects = new Set();
  let totalColumns = 0;
  value.forEach((structure, objectIndex) => {
    const objectField = `${field}[${objectIndex}]`;
    if (!structure || typeof structure !== "object" || Array.isArray(structure)) {
      errors.push(`${objectField} 必须是对象`);
      return;
    }
    const unknown = unknownKeys(structure, supportObjectStructureKeys);
    if (unknown.length)
      errors.push(`${objectField} 含未识别字段：${unknown.join("、")}`);
    const name = `${structure.name || ""}`.trim();
    if (!name || name.length > 120)
      errors.push(`${objectField}.name 必须是 1–120 字符文本`);
    const identity = name.toUpperCase();
    if (seenObjects.has(identity)) errors.push(`${objectField}.name 重复`);
    seenObjects.add(identity);
    if (name && !checked.has(identity))
      errors.push(`${objectField}.name 不在 checkedObjects 中`);
    if (!Array.isArray(structure.columns)) {
      errors.push(`${objectField}.columns 必须是数组`);
      return;
    }
    if (structure.columns.length > 500)
      errors.push(`${objectField}.columns 超过 500 个字段上限`);
    totalColumns += structure.columns.length;
    const seenColumns = new Set();
    structure.columns.forEach((column, columnIndex) => {
      const columnField = `${objectField}.columns[${columnIndex}]`;
      if (!column || typeof column !== "object" || Array.isArray(column)) {
        errors.push(`${columnField} 必须是对象`);
        return;
      }
      const unknownColumn = unknownKeys(column, supportObjectColumnKeys);
      if (unknownColumn.length)
        errors.push(`${columnField} 含未识别字段：${unknownColumn.join("、")}`);
      const columnName = `${column.name || ""}`.trim();
      if (!columnName || columnName.length > 120)
        errors.push(`${columnField}.name 必须是 1–120 字符文本`);
      const columnIdentity = columnName.toUpperCase();
      if (seenColumns.has(columnIdentity))
        errors.push(`${columnField}.name 重复`);
      seenColumns.add(columnIdentity);
      if (
        typeof column.dataType !== "string" ||
        column.dataType.length > 120
      )
        errors.push(`${columnField}.dataType 必须是不超过 120 字符的文本`);
    });
  });
  if (totalColumns > 5_000) errors.push(`${field} 总字段数超过 5,000 上限`);
  return value;
}

function setDifference(left, right) {
  const rightSet = new Set(right);
  return left.filter((item) => !rightSet.has(item));
}

function diagnosticTransition(previous, current) {
  const previousMetrics = new Map(
    (previous.metrics || []).map((metric) => [metric.id, metric]),
  );
  const previousStructures = new Map(
    (previous.objectStructures || []).map((structure) => [
      `${structure.name}`.toUpperCase(),
      structure,
    ]),
  );
  const currentStructures = new Map(
    (current.objectStructures || []).map((structure) => [
      `${structure.name}`.toUpperCase(),
      structure,
    ]),
  );
  const newlyStructuredObjects = [...currentStructures.keys()]
    .filter((name) => !previousStructures.has(name))
    .map((name) => currentStructures.get(name).name);
  const removedStructuredObjects = [...previousStructures.keys()]
    .filter((name) => !currentStructures.has(name))
    .map((name) => previousStructures.get(name).name);
  const addedColumns = [];
  const removedColumns = [];
  const changedColumnTypes = [];
  for (const [identity, structure] of currentStructures) {
    const previousStructure = previousStructures.get(identity);
    if (!previousStructure) continue;
    const previousColumns = new Map(
      (previousStructure.columns || []).map((column) => [
        `${column.name}`.toUpperCase(),
        column,
      ]),
    );
    const currentColumns = new Map(
      (structure.columns || []).map((column) => [
        `${column.name}`.toUpperCase(),
        column,
      ]),
    );
    for (const [columnIdentity, column] of currentColumns) {
      const oldColumn = previousColumns.get(columnIdentity);
      if (!oldColumn) {
        addedColumns.push(`${structure.name}.${column.name}`);
      } else if (
        `${oldColumn.dataType || ""}`.trim().toUpperCase() !==
        `${column.dataType || ""}`.trim().toUpperCase()
      ) {
        changedColumnTypes.push({
          column: `${structure.name}.${column.name}`,
          from: oldColumn.dataType || "未记录",
          to: column.dataType || "未记录",
        });
      }
    }
    for (const [columnIdentity, column] of previousColumns) {
      if (!currentColumns.has(columnIdentity))
        removedColumns.push(`${previousStructure.name}.${column.name}`);
    }
  }
  return {
    recordedAt: current.recordedAt,
    adapterVersion: current.adapterVersion,
    detected: current.detected,
    newlyCheckedObjects: setDifference(
      current.checkedObjects || [],
      previous.checkedObjects || [],
    ),
    removedCheckedObjects: setDifference(
      previous.checkedObjects || [],
      current.checkedObjects || [],
    ),
    newMissingObjects: setDifference(
      current.missingObjects || [],
      previous.missingObjects || [],
    ),
    resolvedMissingObjects: setDifference(
      previous.missingObjects || [],
      current.missingObjects || [],
    ),
    newlyStructuredObjects,
    removedStructuredObjects,
    addedColumns,
    removedColumns,
    changedColumnTypes,
    metricChanges: (current.metrics || []).flatMap((metric) => {
      const oldValue = previousMetrics.get(metric.id)?.value;
      return oldValue === metric.value
        ? []
        : [
            {
              id: metric.id,
              label: metric.label,
              from: oldValue,
              to: metric.value,
            },
          ];
    }),
  };
}

function sourceTypeFamily(dataType) {
  const value = `${dataType || ""}`.trim().toUpperCase();
  if (!value) return "";
  if (/BOOL|\bBIT\b/.test(value)) return "BOOLEAN";
  if (/DATE|TIME|INTERVAL|YEAR/.test(value)) return "TEMPORAL";
  if (/BLOB|BINARY|VARBINARY|BYTEA|\bRAW\b|IMAGE/.test(value)) return "BINARY";
  if (
    /CHAR|TEXT|CLOB|STRING|JSON|XML|UUID|ENUM|SET|NCHAR|VARCHAR/.test(value)
  )
    return "CHARACTER";
  if (/NUMBER|NUMERIC|DECIMAL|INT|FLOAT|DOUBLE|REAL|MONEY/.test(value))
    return "NUMERIC";
  return "OTHER";
}

function normalizedMigrationTasks(value) {
  const values = Array.isArray(value)
    ? value
    : `${value || ""}`.split(",");
  return unique(
    values.map((item) => `${item || ""}`.trim().toUpperCase()).filter(Boolean),
  );
}

function appliesToSelectedTasks(item, selectedTasks) {
  const itemTasks = normalizedMigrationTasks(item?.appliesToTasks);
  return (
    !selectedTasks.length ||
    !itemTasks.length ||
    selectedTasks.some((task) => itemTasks.includes(task))
  );
}

function effectiveStructureRequirement(item, selectedTasks, checkedObjects = null) {
  const requirement = `${item?.requirement || ""}`.trim().toUpperCase();
  const requiredForTasks = normalizedMigrationTasks(item?.requiredForTasks);
  if (
    requirement === "CONDITIONAL" &&
    selectedTasks.some((task) => requiredForTasks.includes(task))
  )
    return "REQUIRED";
  const dependencies = stringList(item?.requiredWhenObjectsPresent).map(
    (name) => name.toUpperCase(),
  );
  if (requirement === "CONDITIONAL" && dependencies.length && checkedObjects)
    return dependencies.some((name) => checkedObjects.has(name))
      ? "REQUIRED"
      : "OPTIONAL";
  return requirement;
}

function evaluateSourceStructureContract(
  acceptance,
  diagnostic,
  selectedTasks = [],
) {
  const contract = acceptance?.sourceStructure;
  if (!contract || !Array.isArray(contract.objects))
    return {
      status: "UNAVAILABLE",
      blockers: [],
      reviews: [],
      compatibleFallbacks: [],
      message: "未找到当前适配器的机器可读表字段契约",
    };
  const checked = new Set(
    (diagnostic.checkedObjects || []).map((name) =>
      `${name}`.trim().toUpperCase(),
    ),
  );
  const structures = new Map(
    (diagnostic.objectStructures || []).map((structure) => [
      `${structure.name}`.trim().toUpperCase(),
      structure,
    ]),
  );
  const blockers = [];
  const reviews = [];
  const compatibleFallbacks = [];
  const addMissing = ({ requirement, path, usedBy, fallback, subject }) => {
    const item = {
      path,
      usedBy: usedBy || [],
      fallback: `${fallback || ""}`.trim(),
      message: `${subject}缺失`,
    };
    if (requirement === "REQUIRED") blockers.push(item);
    else if (requirement === "CONDITIONAL") reviews.push(item);
    else compatibleFallbacks.push(item);
  };
  for (const object of contract.objects) {
    if (!appliesToSelectedTasks(object, selectedTasks)) continue;
    const objectName = `${object.name || ""}`.trim().toUpperCase();
    if (!checked.has(objectName)) {
      addMissing({
        requirement: effectiveStructureRequirement(
          object,
          selectedTasks,
          checked,
        ),
        path: objectName,
        usedBy: object.usedBy,
        fallback: object.fallback,
        subject: "来源表",
      });
      continue;
    }
    const structure = structures.get(objectName);
    if (!structure) {
      reviews.push({
        path: objectName,
        usedBy: object.usedBy || [],
        fallback: "重新执行当前版本结构识别，或核对元数据读取权限",
        message: "来源表可读，但支持包没有字段快照，无法自动判断 SQL 影响",
      });
      continue;
    }
    const columns = new Map(
      (structure.columns || []).map((column) => [
        `${column.name}`.trim().toUpperCase(),
        column,
      ]),
    );
    for (const column of object.columns || []) {
      if (!appliesToSelectedTasks(column, selectedTasks)) continue;
      const columnName = `${column.name || ""}`.trim().toUpperCase();
      const path = `${objectName}.${columnName}`;
      const actual = columns.get(columnName);
      if (!actual) {
        addMissing({
          requirement: effectiveStructureRequirement(
            column,
            selectedTasks,
            checked,
          ),
          path,
          usedBy: column.usedBy,
          fallback: column.fallback,
          subject: "字段",
        });
        continue;
      }
      const accepted = stringList(column.acceptedTypeFamilies).map((item) =>
        item.toUpperCase(),
      );
      if (!accepted.length) continue;
      const family = sourceTypeFamily(actual.dataType);
      if (!family || !accepted.includes(family)) {
        const item = {
          path,
          usedBy: column.usedBy || [],
          fallback: `${column.fallback || ""}`.trim(),
          message: family
            ? `数据库类型 ${actual.dataType || "未记录"} 属于 ${family}，契约接受 ${accepted.join("/")}`
            : `数据库类型未记录，契约接受 ${accepted.join("/")}`,
        };
        if (
          effectiveStructureRequirement(column, selectedTasks, checked) ===
            "REQUIRED" &&
          family
        )
          blockers.push(item);
        else reviews.push(item);
      }
    }
    for (const group of object.columnGroups || []) {
      if (!appliesToSelectedTasks(group, selectedTasks)) continue;
      const candidates = stringList(group.columns).map((name) =>
        name.toUpperCase(),
      );
      if (candidates.some((name) => columns.has(name))) continue;
      addMissing({
        requirement: effectiveStructureRequirement(
          group,
          selectedTasks,
          checked,
        ),
        path: `${objectName}.{${candidates.join("|")}}`,
        usedBy: group.usedBy,
        fallback: group.fallback,
        subject: `字段组“${group.name}”`,
      });
    }
  }
  for (const group of contract.objectGroups || []) {
    if (!appliesToSelectedTasks(group, selectedTasks)) continue;
    const candidates = stringList(group.objects).map((name) =>
      name.toUpperCase(),
    );
    if (candidates.some((name) => checked.has(name))) continue;
    addMissing({
      requirement: effectiveStructureRequirement(group, selectedTasks, checked),
      path: `{${candidates.join("|")}}`,
      usedBy: group.usedBy,
      fallback: group.fallback,
      subject: `来源表组“${group.name}”`,
    });
  }
  const status = blockers.length
    ? "BLOCKED"
    : reviews.length
      ? "REVIEW"
      : "COMPATIBLE";
  return {
    status,
    blockers,
    reviews,
    compatibleFallbacks,
    evaluatedTasks: selectedTasks,
    message:
      status === "BLOCKED"
        ? `发现 ${blockers.length} 个阻断项，当前结构不能直接套用适配器 SQL`
        : status === "REVIEW"
          ? `没有已证实的阻断项，但有 ${reviews.length} 项需要项目核对`
          : "当前字段结构满足核心契约，可按已声明降级规则处理可选差异",
  };
}

function contractDraftTasks(packageTask, requestedTasks) {
  const tasks = requestedTasks.length
    ? requestedTasks
    : packageTask
      ? [packageTask]
      : [];
  if (tasks.length !== 1)
    throw new Error(
      "契约差异草稿必须限定一个迁移任务，请使用 --task MEDICINE_BASE 或 --task INVENTORY",
    );
  return tasks;
}

function contractDraftColumn(column, selectedTasks) {
  const family = sourceTypeFamily(column.dataType);
  const name = `${column.name || ""}`.trim().toUpperCase();
  return {
    name,
    observedDataType: `${column.dataType || ""}`.trim(),
    draft: {
      name,
      requirement: "OPTIONAL",
      appliesToTasks: selectedTasks,
      usedBy: ["TODO: 确认业务用途"],
      fallback: "TODO: 确认忽略或安全降级方式",
      acceptedTypeFamilies: family ? [family] : ["OTHER"],
    },
  };
}

/**
 * Build a review-only acceptance.json difference draft from an already
 * verified support package. Source identity, schema, samples, SQL, comments,
 * credentials and tenant information are deliberately omitted.
 */
export function buildSourceStructureContractDraft(
  content,
  acceptance,
  options = {},
) {
  if (!acceptance?.sourceStructure?.objects)
    throw new Error("未找到 acceptance.json 的 sourceStructure，无法生成差异草稿");
  const requestedTasks = normalizedMigrationTasks(options.tasks);
  const initial = analyzeSourceAdapterSupportPackage(content);
  if (!initial.ok)
    throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
  const selectedTasks = contractDraftTasks(
    initial.summary.migrationTask,
    requestedTasks,
  );
  const analysis = analyzeSourceAdapterSupportPackage(content, acceptance, {
    tasks: selectedTasks,
  });
  if (!analysis.ok)
    throw new Error(`支持包分析失败：${analysis.errors.join("；")}`);
  if (!analysis.summary.currentObjectStructures.length)
    throw new Error(
      "支持包没有字段结构快照，请使用当前版本应用重新执行结构识别并导出 v2 支持包",
    );
  const adapterId = analysis.summary.adapterId;
  const acceptanceAdapterId = `${acceptance.adapterId || ""}`
    .trim()
    .toUpperCase();
  if (acceptanceAdapterId && acceptanceAdapterId !== adapterId)
    throw new Error(
      `支持包适配器 ${adapterId} 与 acceptance.json 的 ${acceptanceAdapterId} 不一致`,
    );

  const contractObjects = new Map(
    acceptance.sourceStructure.objects
      .map((object) => [`${object.name || ""}`.trim().toUpperCase(), object]),
  );
  const proposals = [];
  for (const structure of analysis.summary.currentObjectStructures) {
    const objectName = `${structure.name || ""}`.trim().toUpperCase();
    const contractObject = contractObjects.get(objectName);
    if (
      contractObject &&
      !appliesToSelectedTasks(contractObject, selectedTasks)
    )
      continue;
    if (!contractObject) {
      const observedColumns = (structure.columns || []).map((column) =>
        contractDraftColumn(column, selectedTasks),
      );
      proposals.push({
        action: "ADD_OBJECT",
        path: `sourceStructure.objects.${objectName}`,
        observedColumns,
        draft: {
          name: objectName,
          requirement: "OPTIONAL",
          appliesToTasks: selectedTasks,
          usedBy: ["TODO: 确认业务用途"],
          fallback: "TODO: 确认忽略或安全降级方式",
          columns: observedColumns.map((column) => column.draft),
        },
        reviewStatus: "TODO",
      });
      continue;
    }
    const contractColumns = new Set(
      (contractObject.columns || [])
        .map((column) => `${column.name || ""}`.trim().toUpperCase()),
    );
    for (const column of structure.columns || []) {
      const columnName = `${column.name || ""}`.trim().toUpperCase();
      if (contractColumns.has(columnName)) continue;
      const proposal = contractDraftColumn(column, selectedTasks);
      proposals.push({
        action: "ADD_COLUMN",
        path: `sourceStructure.objects.${objectName}.columns.${columnName}`,
        observedDataType: proposal.observedDataType,
        draft: proposal.draft,
        reviewStatus: "TODO",
      });
    }
  }

  const impact = analysis.summary.contractImpact;
  const contractReviews = [
    ...impact.blockers.map((item) => ({ severity: "BLOCKER", ...item })),
    ...impact.reviews.map((item) => ({ severity: "REVIEW", ...item })),
    ...impact.compatibleFallbacks.map((item) => ({
      severity: "COMPATIBLE_FALLBACK",
      ...item,
    })),
  ];
  return {
    format: CONTRACT_DRAFT_FORMAT,
    version: CONTRACT_DRAFT_VERSION,
    generatedAt: options.generatedAt || new Date().toISOString(),
    adapterId,
    adapterVersion: analysis.summary.currentAdapterVersion,
    migrationTask: selectedTasks[0],
    databaseFamily: analysis.summary.currentDatabaseFamily,
    sourceSupport: {
      checksum: analysis.package.checksum,
      exportedAt: analysis.package.exportedAt,
      diagnosticRecordedAt: analysis.summary.lastRecordedAt,
    },
    contractTarget: {
      acceptanceSchemaVersion:
        acceptance.acceptanceSchemaVersion || ACCEPTANCE_SCHEMA_VERSION,
      adapterId,
      section: "sourceStructure",
    },
    summary: {
      contractStatus: impact.status,
      proposalCount: proposals.length,
      blockerCount: impact.blockers.length,
      reviewCount: impact.reviews.length,
      compatibleFallbackCount: impact.compatibleFallbacks.length,
    },
    proposals,
    contractReviews,
    reviewChecklist: [
      "逐项确认新增表和字段是否属于当前迁移任务，删除无关项",
      "把 OPTIONAL 调整为真实的 REQUIRED、CONDITIONAL 或 OPTIONAL",
      "补齐 usedBy 和 fallback，禁止将 TODO 内容提交到 acceptance.json",
      "将确认后的结构差异加入脱敏夹具和可执行回归测试",
      "运行 npm run adapter:verify 后再进入桌面构建",
    ],
    privacy: {
      containsSourceFingerprint: false,
      containsSchema: false,
      containsRawSampleValues: false,
      containsSqlOrComments: false,
      containsConnectionOrTenantData: false,
    },
    checksum: "",
  };
}

export function formatSourceStructureContractDraftSummary(draft, outputFile) {
  const labels = {
    MEDICINE_BASE: "药品基础数据",
    INVENTORY: "机构库存",
  };
  return [
    "三方 HIS 结构契约差异草稿已生成",
    "",
    `适配器：${draft.adapterId} v${draft.adapterVersion}`,
    `迁移任务：${labels[draft.migrationTask] || draft.migrationTask}`,
    `新增候选：${draft.summary.proposalCount} 项`,
    `现有阻断/核对/兼容降级：${draft.summary.blockerCount}/${draft.summary.reviewCount}/${draft.summary.compatibleFallbackCount}`,
    `输出文件：${outputFile}`,
    "提醒：这是待审核草稿，不会自动修改 acceptance.json；请补齐 TODO 并完成夹具回归。",
  ].join("\n");
}

export function buildAdapterFixtureDraft(content, acceptance, options = {}) {
  if (!acceptance?.sourceStructure?.objects)
    throw new Error("未找到 acceptance.json 的 sourceStructure，无法生成项目变体夹具草稿");
  const initial = analyzeSourceAdapterSupportPackage(content);
  if (!initial.ok)
    throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
  const selectedTasks = contractDraftTasks(
    initial.summary.migrationTask,
    normalizedMigrationTasks(options.tasks),
  );
  const analysis = analyzeSourceAdapterSupportPackage(content, acceptance, {
    tasks: selectedTasks,
  });
  if (!analysis.ok)
    throw new Error(`支持包分析失败：${analysis.errors.join("；")}`);
  const adapterId = analysis.summary.adapterId;
  const acceptanceAdapterId = `${acceptance.adapterId || ""}`
    .trim()
    .toUpperCase();
  if (acceptanceAdapterId && acceptanceAdapterId !== adapterId)
    throw new Error(
      `支持包适配器 ${adapterId} 与 acceptance.json 的 ${acceptanceAdapterId} 不一致`,
    );
  if (!analysis.summary.currentObjectStructures.length)
    throw new Error(
      "支持包没有字段结构快照，请使用当前版本应用重新执行结构识别并导出 v2 支持包",
    );
  const current = [...analysis.package.diagnostics]
    .sort((left, right) => Date.parse(left.recordedAt) - Date.parse(right.recordedAt))
    .at(-1);
  const task = selectedTasks[0];
  const impact = analysis.summary.contractImpact;
  const outcome =
    impact.status === "BLOCKED"
      ? "BLOCK"
      : impact.status === "COMPATIBLE"
        ? "PASS"
        : "REVIEW";
  const impactChecks = [
    ...impact.blockers.map((item) => ({ label: "阻断", item })),
    ...impact.reviews.map((item) => ({ label: "核对", item })),
    ...impact.compatibleFallbacks.map((item) => ({ label: "兼容降级", item })),
  ]
    .slice(0, 20)
    .map(({ label, item }) => {
      const business = item.usedBy?.length
        ? `；影响 ${item.usedBy.join("、")}`
        : "";
      const fallback = item.fallback ? `；处理 ${item.fallback}` : "";
      return `${label} ${item.path}：${item.message}${business}${fallback}`;
    });
  const checks = impactChecks.length
    ? impactChecks
    : [
        "当前字段结构满足已声明契约",
        "请补充本项目变体必须守住的读取、关联或安全降级行为",
      ];
  const structureHash = `${current.structureHash || ""}`.toLowerCase();
  const taskSlug = task === "INVENTORY" ? "inventory" : "medicine";
  const taskLabel = task === "INVENTORY" ? "机构库存" : "药品基础数据";
  return {
    fixtureSchemaVersion: FIXTURE_SCHEMA_VERSION,
    adapterId,
    id: `${taskSlug}-${structureHash.slice(0, 12)}`,
    title: `${taskLabel}结构变体 ${structureHash.slice(0, 8)}（请改为可读特征）`,
    databaseFamily: analysis.summary.currentDatabaseFamily.toLowerCase(),
    tasks: [task],
    sourceObjects: analysis.summary.currentObjectStructures.map((structure) => ({
      name: `${structure.name || ""}`.trim().toUpperCase(),
      columns: (structure.columns || []).map((column) => ({
        name: `${column.name || ""}`.trim().toUpperCase(),
        dataType: `${column.dataType || ""}`.trim().toUpperCase(),
      })),
    })),
    expectations: {
      outcome,
      checks,
    },
    rustTest: "TODO::replace::with::exact_test_path",
  };
}

export function formatAdapterFixtureDraftSummary(draft, outputFile) {
  const outcomeLabels = {
    PASS: "建议通过",
    BLOCK: "建议阻断",
    REVIEW: "需要人工决定 PASS 或 BLOCK",
  };
  return [
    "三方 HIS 项目变体夹具草稿已生成",
    "",
    `适配器：${draft.adapterId}`,
    `迁移任务：${draft.tasks[0] === "INVENTORY" ? "机构库存" : "药品基础数据"}`,
    `来源结构：${draft.sourceObjects.length} 张表 / ${draft.sourceObjects.reduce((total, object) => total + object.columns.length, 0)} 个字段`,
    `契约结论：${outcomeLabels[draft.expectations.outcome] || draft.expectations.outcome}`,
    `输出文件：${outputFile}`,
    "提醒：请改写标题和业务检查项，填写可执行 rustTest；完成前不要移入 fixtures 目录。",
  ].join("\n");
}

function handoffTaskLabel(task) {
  return task === "INVENTORY" ? "机构库存" : "药品基础数据";
}

function handoffInlineCode(value) {
  return `\`${`${value || ""}`.replace(/[`\r\n]/g, "")}\``;
}

function handoffDetailLines(items, emptyMessage) {
  if (!items.length) return [`- ${emptyMessage}`];
  return items.map((item) => `- ${item}`);
}

/**
 * Turn one verified, task-scoped support package into a privacy-safe
 * maintainer handoff. The result intentionally remains review-only: it never
 * modifies acceptance.json, fixtures, adapter code, or executable evidence.
 */
export function buildAdapterMaintenanceHandoff(
  content,
  acceptance,
  options = {},
) {
  if (!acceptance?.sourceStructure?.objects)
    throw new Error(
      "未找到 acceptance.json 的 sourceStructure，无法生成维护交接包",
    );
  const initial = analyzeSourceAdapterSupportPackage(content);
  if (!initial.ok)
    throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
  const selectedTasks = contractDraftTasks(
    initial.summary.migrationTask,
    normalizedMigrationTasks(options.tasks),
  );
  const analysis = analyzeSourceAdapterSupportPackage(content, acceptance, {
    tasks: selectedTasks,
  });
  if (!analysis.ok)
    throw new Error(`支持包分析失败：${analysis.errors.join("；")}`);
  const generatedAt = options.generatedAt || new Date().toISOString();
  const contractDraft = buildSourceStructureContractDraft(content, acceptance, {
    tasks: selectedTasks,
    generatedAt,
  });
  const fixtureDraft = buildAdapterFixtureDraft(content, acceptance, {
    tasks: selectedTasks,
  });
  const task = selectedTasks[0];
  const taskLabel = handoffTaskLabel(task);
  const impact = analysis.summary.contractImpact;
  const evidence = (acceptance.evidence || []).filter((item) => {
    const tasks = normalizedMigrationTasks(item.tasks);
    return tasks.includes(task);
  });
  const contractDetails = [
    ...impact.blockers.map(
      (item) => `阻断 ${handoffInlineCode(item.path)}：${item.message}`,
    ),
    ...impact.reviews.map(
      (item) => `核对 ${handoffInlineCode(item.path)}：${item.message}`,
    ),
    ...contractDraft.proposals.map(
      (item) => `新增候选 ${handoffInlineCode(item.path)}`,
    ),
  ].slice(0, 30);
  const implementationDetails =
    task === "INVENTORY"
      ? [
          "实现并核对 InventorySourceAdapter::load_catalog，只读取机构与库房目录",
          "实现并核对 InventorySourceAdapter::inspect，选择范围前只返回轻量汇总",
          "实现并核对 InventorySourceAdapter::load_stock_items，把已选机构和库房过滤下推到来源 SQL",
          "通过 InventorySourceStockItem 标准模型保留来源稳定键、药品商品键、管理包装、数量价格和批号效期",
          "空库检查、逐库试迁移、首次盘点和安全撤销继续由公共流程控制",
        ]
      : [
          "实现并核对 MedicineSourceAdapter::inspect，只读取结构与识别摘要",
          "实现并核对 MedicineSourceAdapter::load，提供整批稳定来源键和有上限的只读数据",
          "通过公共 SDK 处理 Schema/对象限定、只读查询和来源过滤，不直接依赖数据库内部模块",
          "核对名称、规格、剂型、最小单位、厂家和商品关系的物理来源与降级语义",
        ];
  const contractStatus = impact.blockers.length
    ? "BLOCKED"
    : impact.reviews.length || contractDraft.proposals.length
      ? "REVIEW"
      : "READY";
  const workItems = [
    {
      id: "SOURCE_CONTRACT",
      status: contractStatus,
      title: "确认来源结构与业务口径",
      details: contractDetails.length
        ? contractDetails
        : ["当前没有阻断、核对或新增结构候选"],
    },
    {
      id: "SOURCE_IMPLEMENTATION",
      status: contractStatus === "BLOCKED" ? "WAITING" : "VERIFY",
      title: "实现或调整来源读取",
      details: implementationDetails,
    },
    {
      id: "PROJECT_FIXTURE",
      status: "PENDING",
      title: "完成脱敏项目变体夹具",
      details: [
        `把 ${handoffInlineCode("fixture.draft.json")} 改为可读标题和明确 PASS/BLOCK 业务预期`,
        "填写真实可执行 rustTest，并确认测试确实断言该结构差异",
        "草稿完成前不得移入正式 fixtures 目录",
      ],
    },
    {
      id: "EXECUTABLE_EVIDENCE",
      status: evidence.length ? "VERIFY" : "PENDING",
      title: "补齐可执行业务证据",
      details: [
        `当前 acceptance.json 对本任务声明 ${evidence.length} 条证据`,
        "每条证据必须绑定真实 #[test] 函数和完整 Rust 测试路径",
        "真实数据库证据只能显式标为 OPTIONAL_LIVE 且测试必须 #[ignore]",
      ],
    },
    {
      id: "DELIVERY_GATE",
      status: "PENDING",
      title: "执行交付门禁",
      details: [
        "运行 npm run adapter:status 查看阶段性问题",
        "运行 npm run adapter:verify 执行清单、证据、夹具和教学适配器回归",
        "运行 Rust 全量测试和桌面构建后才允许交付",
      ],
    },
  ];
  const nextAction =
    contractStatus === "BLOCKED"
      ? "先处理来源契约阻断项；在阻断消除前不要修改正式写入流程。"
      : contractStatus === "REVIEW"
        ? "先完成结构候选和业务语义核对，再修改适配器代码与正式契约。"
        : "当前结构满足已声明契约，可进入来源读取复核和回归证据补齐。";
  const manifest = {
    format: MAINTENANCE_HANDOFF_FORMAT,
    version: MAINTENANCE_HANDOFF_VERSION,
    generatedAt,
    adapterId: analysis.summary.adapterId,
    adapterVersion: analysis.summary.currentAdapterVersion,
    migrationTask: task,
    databaseFamily: analysis.summary.currentDatabaseFamily,
    sourceSupport: {
      checksum: analysis.package.checksum,
      exportedAt: analysis.package.exportedAt,
    },
    summary: {
      contractStatus,
      observedObjectCount: analysis.summary.currentStructuredObjectCount,
      observedColumnCount: analysis.summary.currentStructuredColumnCount,
      proposalCount: contractDraft.summary.proposalCount,
      blockerCount: contractDraft.summary.blockerCount,
      reviewCount: contractDraft.summary.reviewCount,
      compatibleFallbackCount:
        contractDraft.summary.compatibleFallbackCount,
      declaredEvidenceCount: evidence.length,
    },
    nextAction,
    workItems,
    artifacts: {},
    privacy: {
      containsSourceFingerprint: false,
      containsSchema: false,
      containsRawSampleValues: false,
      containsSqlOrComments: false,
      containsConnectionOrTenantData: false,
    },
  };
  const impactLines = contractDetails.map((item) => item);
  const planMarkdown = [
    `# ${manifest.adapterId} ${taskLabel}维护交接计划`,
    "",
    "本交接包由已校验的脱敏支持包生成，只包含表名、字段名、数据库类型、契约影响和待办；不包含来源指纹、Schema、样例值、SQL、连接信息或租户信息。",
    "",
    "## 当前判断",
    "",
    `- 适配器：${handoffInlineCode(manifest.adapterId)} v${manifest.adapterVersion}`,
    `- 迁移任务：${taskLabel}`,
    `- 数据库家族：${handoffInlineCode(manifest.databaseFamily)}`,
    `- 来源结构：${manifest.summary.observedObjectCount} 张表 / ${manifest.summary.observedColumnCount} 个字段`,
    `- 契约状态：${handoffInlineCode(contractStatus)}`,
    `- 新增候选：${manifest.summary.proposalCount} 项；阻断/核对/兼容降级：${manifest.summary.blockerCount}/${manifest.summary.reviewCount}/${manifest.summary.compatibleFallbackCount}`,
    "",
    `**下一动作：${nextAction}**`,
    "",
    "## 结构与业务核对",
    "",
    ...handoffDetailLines(impactLines, "当前没有阻断、核对或新增结构候选。"),
    "",
    "## 来源实现核对",
    "",
    ...implementationDetails.map((item) => `- ${item}`),
    "",
    "## 本包文件",
    "",
    "- `contract-draft.json`：待审核结构契约差异；不会自动修改 acceptance.json。",
    "- `fixture.draft.json`：待补业务预期和真实 rustTest 的项目变体草稿。",
    "- `handoff.json`：阶段、待办和各文件 SHA-256 清单。",
    "- `IMPLEMENTATION_PLAN.md`：本文件，按维护顺序给出下一步。",
    "",
    "## 完成定义",
    "",
    "- [ ] 每个结构候选已经确认是否属于当前任务，并补齐 usedBy、必要性和安全降级。",
    "- [ ] 来源读取只使用公共 SDK，稳定键、范围、关联、包装和金额口径均有代码与测试证据。",
    "- [ ] fixture.draft.json 已改成明确 PASS 或 BLOCK，且绑定真实可执行 Rust 测试。",
    "- [ ] acceptance.evidence 覆盖当前任务，adapter:verify 没有错误或提醒。",
    "- [ ] Rust 全量测试和目标桌面构建通过。",
    "",
    "> 这是一份维护交接计划，不是可直接合并的正式契约或夹具。",
    "",
  ].join("\n");
  return { manifest, contractDraft, fixtureDraft, planMarkdown };
}

export function formatAdapterMaintenanceHandoffSummary(bundle, outputDir) {
  return [
    "三方 HIS 适配器维护交接包已生成",
    "",
    `适配器：${bundle.manifest.adapterId} v${bundle.manifest.adapterVersion}`,
    `迁移任务：${handoffTaskLabel(bundle.manifest.migrationTask)}`,
    `契约状态：${bundle.manifest.summary.contractStatus}`,
    `来源结构：${bundle.manifest.summary.observedObjectCount} 张表 / ${bundle.manifest.summary.observedColumnCount} 个字段`,
    `输出目录：${outputDir}`,
    `下一步：${bundle.manifest.nextAction}`,
    "提醒：交接包不修改正式契约、夹具或适配器代码，所有草稿仍需人工业务确认。",
  ].join("\n");
}

export function analyzeAdapterMaintenanceHandoff(
  manifestContent,
  artifactContents = {},
) {
  const errors = [];
  const warnings = [];
  let manifest;
  try {
    manifest =
      typeof manifestContent === "string"
        ? JSON.parse(manifestContent)
        : structuredClone(manifestContent);
  } catch (error) {
    return {
      ok: false,
      errors: [`handoff.json 解析失败：${error.message}`],
      warnings,
    };
  }
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest))
    return {
      ok: false,
      errors: ["handoff.json 必须是 JSON 对象"],
      warnings,
    };
  const unknown = unknownKeys(manifest, handoffTopLevelKeys);
  if (unknown.length)
    errors.push(`handoff.json 含未识别字段：${unknown.join("、")}`);
  const sensitive = collectSensitiveHandoffKeys(manifest);
  if (sensitive.length)
    errors.push(`交接清单含连接、凭据、来源身份或租户字段：${sensitive.join("、")}`);
  if (manifest.format !== MAINTENANCE_HANDOFF_FORMAT)
    errors.push(`format 必须为 ${MAINTENANCE_HANDOFF_FORMAT}`);
  if (manifest.version !== MAINTENANCE_HANDOFF_VERSION)
    errors.push(`version 必须为 ${MAINTENANCE_HANDOFF_VERSION}`);
  if (!validDate(manifest.generatedAt)) errors.push("generatedAt 不是有效时间");
  const adapterId = `${manifest.adapterId || ""}`.trim().toUpperCase();
  if (!/^[A-Z][A-Z0-9_]{1,79}$/.test(adapterId)) errors.push("adapterId 无效");
  if (!Number.isInteger(manifest.adapterVersion) || manifest.adapterVersion < 1)
    errors.push("adapterVersion 必须是正整数");
  const task = `${manifest.migrationTask || ""}`.trim().toUpperCase();
  if (!migrationTasks.has(task)) errors.push(`migrationTask 不支持 ${task}`);
  const family = `${manifest.databaseFamily || ""}`.trim().toLowerCase();
  if (!databaseFamilies.has(family))
    errors.push(`databaseFamily 不支持 ${family || "空值"}`);
  const sourceSupport = manifest.sourceSupport;
  if (!sourceSupport || typeof sourceSupport !== "object" || Array.isArray(sourceSupport)) {
    errors.push("sourceSupport 必须是对象");
  } else {
    const sourceUnknown = unknownKeys(sourceSupport, handoffSourceSupportKeys);
    if (sourceUnknown.length)
      errors.push(`sourceSupport 含未识别字段：${sourceUnknown.join("、")}`);
    if (!/^[a-f0-9]{64}$/.test(`${sourceSupport.checksum || ""}`))
      errors.push("sourceSupport.checksum 必须是 64 位小写 SHA-256");
    if (!validDate(sourceSupport.exportedAt))
      errors.push("sourceSupport.exportedAt 不是有效时间");
  }
  const summary = manifest.summary;
  if (!summary || typeof summary !== "object" || Array.isArray(summary)) {
    errors.push("summary 必须是对象");
  } else {
    const summaryUnknown = unknownKeys(summary, handoffSummaryKeys);
    if (summaryUnknown.length)
      errors.push(`summary 含未识别字段：${summaryUnknown.join("、")}`);
    if (!new Set(["READY", "REVIEW", "BLOCKED"]).has(summary.contractStatus))
      errors.push("summary.contractStatus 必须为 READY、REVIEW 或 BLOCKED");
    for (const key of [...handoffSummaryKeys].filter(
      (name) => name !== "contractStatus",
    )) {
      if (!Number.isInteger(summary[key]) || summary[key] < 0)
        errors.push(`summary.${key} 必须是非负整数`);
    }
  }
  if (typeof manifest.nextAction !== "string" || !manifest.nextAction.trim())
    errors.push("nextAction 不能为空");
  const expectedWorkItems = [
    "SOURCE_CONTRACT",
    "SOURCE_IMPLEMENTATION",
    "PROJECT_FIXTURE",
    "EXECUTABLE_EVIDENCE",
    "DELIVERY_GATE",
  ];
  const workItems = Array.isArray(manifest.workItems) ? manifest.workItems : [];
  if (!Array.isArray(manifest.workItems)) errors.push("workItems 必须是数组");
  const workIds = [];
  for (const [index, item] of workItems.entries()) {
    const field = `workItems[${index}]`;
    if (!item || typeof item !== "object" || Array.isArray(item)) {
      errors.push(`${field} 必须是对象`);
      continue;
    }
    const itemUnknown = unknownKeys(item, handoffWorkItemKeys);
    if (itemUnknown.length)
      errors.push(`${field} 含未识别字段：${itemUnknown.join("、")}`);
    workIds.push(item.id);
    if (!expectedWorkItems.includes(item.id)) errors.push(`${field}.id 无效`);
    if (!new Set(["READY", "REVIEW", "BLOCKED", "WAITING", "VERIFY", "PENDING"]).has(item.status))
      errors.push(`${field}.status 无效`);
    if (typeof item.title !== "string" || !item.title.trim())
      errors.push(`${field}.title 不能为空`);
    if (
      !Array.isArray(item.details) ||
      !item.details.length ||
      item.details.some((detail) => typeof detail !== "string" || !detail.trim())
    )
      errors.push(`${field}.details 至少需要一条非空说明`);
  }
  if (
    workIds.length !== expectedWorkItems.length ||
    expectedWorkItems.some((id, index) => workIds[index] !== id)
  )
    errors.push("workItems 必须按契约、实现、夹具、证据、交付门禁顺序完整声明");
  const declaredArtifacts = manifest.artifacts;
  if (
    !declaredArtifacts ||
    typeof declaredArtifacts !== "object" ||
    Array.isArray(declaredArtifacts)
  ) {
    errors.push("artifacts 必须是对象");
  } else {
    const artifactNames = Object.keys(declaredArtifacts);
    const expectedNames = [...handoffArtifactRoles.keys()];
    const unexpected = artifactNames.filter((name) => !handoffArtifactRoles.has(name));
    const missing = expectedNames.filter((name) => !artifactNames.includes(name));
    if (unexpected.length)
      errors.push(`artifacts 含未声明文件：${unexpected.join("、")}`);
    if (missing.length) errors.push(`artifacts 缺少文件：${missing.join("、")}`);
    for (const name of expectedNames) {
      const artifact = declaredArtifacts[name];
      if (!artifact || typeof artifact !== "object" || Array.isArray(artifact))
        continue;
      const artifactUnknown = unknownKeys(artifact, handoffArtifactKeys);
      if (artifactUnknown.length)
        errors.push(`artifacts.${name} 含未识别字段：${artifactUnknown.join("、")}`);
      if (artifact.role !== handoffArtifactRoles.get(name))
        errors.push(`artifacts.${name}.role 无效`);
      if (!/^[a-f0-9]{64}$/.test(`${artifact.sha256 || ""}`))
        errors.push(`artifacts.${name}.sha256 必须是 64 位小写 SHA-256`);
    }
  }
  const privacy = manifest.privacy;
  if (!privacy || typeof privacy !== "object" || Array.isArray(privacy)) {
    errors.push("privacy 必须是对象");
  } else {
    const privacyUnknown = unknownKeys(privacy, handoffPrivacyKeys);
    if (privacyUnknown.length)
      errors.push(`privacy 含未识别字段：${privacyUnknown.join("、")}`);
    for (const key of handoffPrivacyKeys)
      if (privacy[key] !== false) errors.push(`privacy.${key} 必须明确为 false`);
  }
  if (!/^[a-f0-9]{64}$/.test(`${manifest.checksum || ""}`)) {
    errors.push("checksum 必须是 64 位小写 SHA-256");
  } else if (maintenanceHandoffChecksum(manifest) !== manifest.checksum) {
    errors.push("handoff.json 完整性校验失败，清单可能被修改");
  }
  const artifactNames = [...handoffArtifactRoles.keys()];
  const unexpectedContents = Object.keys(artifactContents).filter(
    (name) => !handoffArtifactRoles.has(name),
  );
  if (unexpectedContents.length)
    errors.push(`交接目录含未声明内容：${unexpectedContents.join("、")}`);
  for (const name of artifactNames) {
    const content = artifactContents[name];
    if (typeof content !== "string") {
      errors.push(`交接目录缺少 ${name}`);
      continue;
    }
    const declared = declaredArtifacts?.[name]?.sha256;
    if (declared && artifactChecksum(content) !== declared)
      errors.push(`${name} 完整性校验失败，文件可能被修改`);
  }
  let contractDraft;
  let fixtureDraft;
  try {
    contractDraft = JSON.parse(artifactContents["contract-draft.json"] || "");
  } catch (error) {
    errors.push(`contract-draft.json 解析失败：${error.message}`);
  }
  try {
    fixtureDraft = JSON.parse(artifactContents["fixture.draft.json"] || "");
  } catch (error) {
    errors.push(`fixture.draft.json 解析失败：${error.message}`);
  }
  if (contractDraft) {
    if (contractDraft.format !== CONTRACT_DRAFT_FORMAT)
      errors.push("contract-draft.json format 无效");
    if (`${contractDraft.adapterId || ""}`.toUpperCase() !== adapterId)
      errors.push("contract-draft.json adapterId 与交接清单不一致");
    if (`${contractDraft.migrationTask || ""}`.toUpperCase() !== task)
      errors.push("contract-draft.json migrationTask 与交接清单不一致");
    const leaked = collectSensitiveHandoffKeys(contractDraft);
    if (leaked.length)
      errors.push(`contract-draft.json 含敏感字段：${leaked.join("、")}`);
  }
  if (fixtureDraft) {
    if (`${fixtureDraft.adapterId || ""}`.toUpperCase() !== adapterId)
      errors.push("fixture.draft.json adapterId 与交接清单不一致");
    if (!Array.isArray(fixtureDraft.tasks) || fixtureDraft.tasks.length !== 1 || fixtureDraft.tasks[0] !== task)
      errors.push("fixture.draft.json tasks 与交接清单不一致");
    const leaked = collectSensitiveHandoffKeys(fixtureDraft);
    if (leaked.length)
      errors.push(`fixture.draft.json 含敏感字段：${leaked.join("、")}`);
  }
  const plan = artifactContents["IMPLEMENTATION_PLAN.md"];
  if (
    typeof plan === "string" &&
    (!plan.includes("维护交接计划") || !plan.includes("## 完成定义"))
  )
    errors.push("IMPLEMENTATION_PLAN.md 缺少维护计划或完成定义");
  return {
    ok: errors.length === 0,
    errors,
    warnings,
    manifest,
    summary: {
      adapterId,
      adapterVersion: manifest.adapterVersion,
      migrationTask: task,
      databaseFamily: family,
      contractStatus: summary?.contractStatus || "",
      artifactCount: artifactNames.length,
    },
  };
}

export async function verifyAdapterMaintenanceHandoffDirectory(directory) {
  const errors = [];
  let entries;
  try {
    entries = await readdir(directory, { withFileTypes: true });
  } catch (error) {
    return {
      ok: false,
      errors: [`无法读取交接目录：${error.message}`],
      warnings: [],
    };
  }
  const expected = new Set(["handoff.json", ...handoffArtifactRoles.keys()]);
  for (const entry of entries) {
    if (!expected.has(entry.name)) errors.push(`交接目录含未声明文件：${entry.name}`);
    if (!entry.isFile()) errors.push(`交接目录只允许普通文件：${entry.name}`);
  }
  for (const name of expected)
    if (!entries.some((entry) => entry.name === name))
      errors.push(`交接目录缺少文件：${name}`);
  const contents = {};
  for (const name of expected) {
    if (!entries.some((entry) => entry.name === name && entry.isFile())) continue;
    const content = await readFile(path.join(directory, name), "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      errors.push(`${name} 超过 5 MB 上限`);
    contents[name] = content;
  }
  if (!contents["handoff.json"])
    return { ok: false, errors, warnings: [] };
  const analysis = analyzeAdapterMaintenanceHandoff(
    contents["handoff.json"],
    Object.fromEntries(
      [...handoffArtifactRoles.keys()]
        .filter((name) => contents[name] !== undefined)
        .map((name) => [name, contents[name]]),
    ),
  );
  const combinedErrors = [...errors, ...analysis.errors];
  return { ...analysis, ok: combinedErrors.length === 0, errors: combinedErrors };
}

export function formatAdapterMaintenanceHandoffVerification(analysis, directory) {
  if (!analysis.ok)
    return [
      "三方 HIS 适配器维护交接包验收失败",
      "",
      ...analysis.errors.map((error) => `✗ ${error}`),
      "",
      "请重新从原始脱敏支持包生成交接包，不要继续使用完整性失败的文件。",
    ].join("\n");
  return [
    "三方 HIS 适配器维护交接包验收通过",
    "",
    `适配器：${analysis.summary.adapterId} v${analysis.summary.adapterVersion}`,
    `迁移任务：${handoffTaskLabel(analysis.summary.migrationTask)}`,
    `契约状态：${analysis.summary.contractStatus}`,
    `文件：${analysis.summary.artifactCount} 个交接产物 + handoff.json`,
    `目录：${directory}`,
    "✓ 清单自校验、文件 SHA-256、任务一致性和隐私字段检查全部通过",
  ].join("\n");
}

export function analyzeSourceAdapterSupportPackage(
  content,
  acceptance = null,
  options = {},
) {
  const errors = [];
  const warnings = [];
  const requestedTasks = normalizedMigrationTasks(options.tasks);
  const invalidTasks = requestedTasks.filter(
    (task) => !migrationTasks.has(task),
  );
  if (invalidTasks.length)
    errors.push(
      `迁移任务只支持 ${[...migrationTasks].join("、")}，收到 ${invalidTasks.join("、")}`,
    );
  let value;
  try {
    value =
      typeof content === "string"
        ? JSON.parse(content)
        : structuredClone(content);
  } catch (error) {
    return {
      ok: false,
      errors: [`支持包 JSON 解析失败：${error.message}`],
      warnings,
    };
  }
  if (!value || typeof value !== "object" || Array.isArray(value))
    return { ok: false, errors: ["支持包必须是 JSON 对象"], warnings };
  const topUnknown = unknownKeys(value, supportTopLevelKeys);
  if (topUnknown.length)
    errors.push(`支持包含未识别字段：${topUnknown.join("、")}`);
  const sensitive = collectSensitiveSupportKeys(value);
  if (sensitive.length)
    errors.push(`支持包含连接、凭据或租户字段：${sensitive.join("、")}`);
  if (value.format !== SUPPORT_PACKAGE_FORMAT)
    errors.push(`format 必须为 ${SUPPORT_PACKAGE_FORMAT}`);
  if (!supportedSupportPackageVersions.has(value.version))
    errors.push(
      `version 必须为受支持版本 ${[...supportedSupportPackageVersions].join(" 或 ")}`,
    );
  const adapterId = `${value.adapterId || ""}`.trim().toUpperCase();
  if (!/^[A-Z][A-Z0-9_]{1,79}$/.test(adapterId)) errors.push("adapterId 无效");
  const packageTask = `${value.migrationTask || ""}`.trim().toUpperCase();
  if (value.version === 1 && value.migrationTask !== undefined)
    errors.push("migrationTask 需要支持包 version 2，请使用当前应用重新导出");
  if (packageTask && !migrationTasks.has(packageTask))
    errors.push(`migrationTask 不支持 ${packageTask}`);
  if (
    packageTask &&
    requestedTasks.length &&
    !requestedTasks.includes(packageTask)
  )
    errors.push(
      `支持包属于 ${packageTask}，不能按 ${requestedTasks.join("、")} 任务分析`,
    );
  const selectedTasks = requestedTasks.length
    ? requestedTasks
    : packageTask
      ? [packageTask]
      : [];
  const sourceFingerprint = `${value.sourceFingerprint || ""}`
    .trim()
    .toLowerCase();
  if (!/^[0-9a-f]{20}$/.test(sourceFingerprint))
    errors.push("sourceFingerprint 必须是 20 位小写十六进制摘要");
  if (!validDate(value.exportedAt)) errors.push("exportedAt 不是有效时间");
  if (!/^[0-9a-f]{64}$/i.test(`${value.checksum || ""}`)) {
    errors.push("checksum 必须是 64 位 SHA-256");
  } else if (
    sourceAdapterSupportChecksum(value) !== `${value.checksum}`.toLowerCase()
  ) {
    errors.push("支持包完整性校验失败，文件可能被修改或传输不完整");
  }
  const diagnostics = Array.isArray(value.diagnostics) ? value.diagnostics : [];
  if (!Array.isArray(value.diagnostics) || !diagnostics.length)
    errors.push("diagnostics 至少需要 1 条结构诊断");
  if (diagnostics.length > 20) errors.push("diagnostics 超过 20 条历史上限");
  const seenDiagnosticIds = new Set();
  let lastTimestamp = Number.POSITIVE_INFINITY;
  diagnostics.forEach((diagnostic, index) => {
    const field = `diagnostics[${index}]`;
    if (
      !diagnostic ||
      typeof diagnostic !== "object" ||
      Array.isArray(diagnostic)
    ) {
      errors.push(`${field} 必须是对象`);
      return;
    }
    const unknown = unknownKeys(diagnostic, supportDiagnosticKeys);
    if (unknown.length)
      errors.push(`${field} 含未识别字段：${unknown.join("、")}`);
    if (!/^[0-9a-f]{24}$/i.test(`${diagnostic.diagnosticId || ""}`))
      errors.push(`${field}.diagnosticId 必须是 24 位十六进制标识`);
    if (seenDiagnosticIds.has(diagnostic.diagnosticId))
      errors.push(`${field}.diagnosticId 重复`);
    seenDiagnosticIds.add(diagnostic.diagnosticId);
    if (`${diagnostic.adapterId || ""}`.trim().toUpperCase() !== adapterId)
      errors.push(`${field}.adapterId 与支持包不一致`);
    const diagnosticTask = `${diagnostic.migrationTask || ""}`
      .trim()
      .toUpperCase();
    if (diagnosticTask && !migrationTasks.has(diagnosticTask))
      errors.push(`${field}.migrationTask 不受支持`);
    if (packageTask && diagnosticTask !== packageTask)
      errors.push(`${field}.migrationTask 与支持包不一致`);
    if (value.version === 1 && diagnostic.migrationTask !== undefined)
      errors.push(
        `${field}.migrationTask 需要支持包 version 2，请使用当前应用重新导出`,
      );
    if (
      !Number.isInteger(diagnostic.adapterVersion) ||
      diagnostic.adapterVersion < 1
    )
      errors.push(`${field}.adapterVersion 必须是正整数`);
    if (
      `${diagnostic.sourceFingerprint || ""}`.trim().toLowerCase() !==
      sourceFingerprint
    )
      errors.push(`${field}.sourceFingerprint 与支持包不一致`);
    if (
      typeof diagnostic.databaseFamily !== "string" ||
      !diagnostic.databaseFamily.trim() ||
      diagnostic.databaseFamily.length > 40
    )
      errors.push(`${field}.databaseFamily 必须是 1–40 字符文本`);
    if (typeof diagnostic.schema !== "string" || diagnostic.schema.length > 128)
      errors.push(`${field}.schema 必须是最多 128 字符文本`);
    if (typeof diagnostic.detected !== "boolean")
      errors.push(`${field}.detected 必须是布尔值`);
    supportList(
      diagnostic.checkedObjects,
      `${field}.checkedObjects`,
      errors,
      160,
      120,
    );
    supportList(
      diagnostic.missingObjects,
      `${field}.missingObjects`,
      errors,
      160,
      120,
    );
    validateSupportObjectStructures(
      diagnostic.objectStructures,
      `${field}.objectStructures`,
      diagnostic.checkedObjects,
      errors,
    );
    if (value.version === 1 && diagnostic.objectStructures !== undefined)
      errors.push(
        `${field}.objectStructures 需要支持包 version 2，请使用当前应用重新导出`,
      );
    supportList(diagnostic.warnings, `${field}.warnings`, errors, 40, 500);
    if (!Array.isArray(diagnostic.metrics)) {
      errors.push(`${field}.metrics 必须是数组`);
    } else {
      if (diagnostic.metrics.length > 80)
        errors.push(`${field}.metrics 超过 80 项上限`);
      diagnostic.metrics.forEach((metric, metricIndex) => {
        const metricField = `${field}.metrics[${metricIndex}]`;
        const unknownMetric = unknownKeys(metric, supportMetricKeys);
        if (unknownMetric.length)
          errors.push(
            `${metricField} 含未识别字段：${unknownMetric.join("、")}`,
          );
        if (
          typeof metric?.id !== "string" ||
          !metric.id.trim() ||
          metric.id.length > 80 ||
          typeof metric?.label !== "string" ||
          !metric.label.trim() ||
          metric.label.length > 120
        )
          errors.push(`${metricField} 的 id/label 必须是非空短文本`);
        if (!Number.isSafeInteger(metric?.value) || metric.value < 0)
          errors.push(`${metricField}.value 必须是非负整数`);
      });
    }
    if (!/^[0-9a-f]{64}$/i.test(`${diagnostic.structureHash || ""}`))
      errors.push(`${field}.structureHash 必须是 64 位 SHA-256`);
    if (
      typeof diagnostic.message !== "string" ||
      diagnostic.message.length > 800
    )
      errors.push(`${field}.message 必须是最多 800 字符文本`);
    if (!validDate(diagnostic.recordedAt)) {
      errors.push(`${field}.recordedAt 不是有效时间`);
    } else {
      const timestamp = Date.parse(diagnostic.recordedAt);
      if (timestamp > lastTimestamp)
        warnings.push("诊断历史不是按最新到最早排序，摘要已按时间重新排列");
      lastTimestamp = timestamp;
    }
  });
  if (errors.length)
    return { ok: false, errors: unique(errors), warnings: unique(warnings) };
  const chronological = [...diagnostics].sort(
    (left, right) => Date.parse(left.recordedAt) - Date.parse(right.recordedAt),
  );
  const transitions = chronological
    .slice(1)
    .map((current, index) =>
      diagnosticTransition(chronological[index], current),
    );
  const current = chronological.at(-1);
  const currentObjectStructures = current.objectStructures || [];
  const contractImpact = evaluateSourceStructureContract(
    acceptance,
    current,
    selectedTasks,
  );
  return {
    ok: true,
    errors: [],
    warnings: unique(warnings),
    package: value,
    summary: {
      adapterId,
      migrationTask: packageTask,
      sourceFingerprint,
      diagnosticCount: diagnostics.length,
      firstRecordedAt: chronological[0].recordedAt,
      lastRecordedAt: current.recordedAt,
      currentAdapterVersion: current.adapterVersion,
      currentDetected: current.detected,
      currentDatabaseFamily: current.databaseFamily,
      currentSchema: current.schema || "",
      currentCheckedObjects: current.checkedObjects || [],
      currentMissingObjects: current.missingObjects || [],
      currentObjectStructures,
      currentStructuredObjectCount: currentObjectStructures.length,
      currentStructuredColumnCount: currentObjectStructures.reduce(
        (total, structure) => total + (structure.columns || []).length,
        0,
      ),
      currentWarnings: current.warnings || [],
      contractImpact,
      transitions,
    },
  };
}

export function formatSourceAdapterSupportAnalysis(result) {
  if (!result.ok) {
    return [
      "三方 HIS 适配器支持包校验失败",
      "",
      ...result.errors.map((error) => `错误：${error}`),
      ...result.warnings.map((warning) => `提醒：${warning}`),
    ].join("\n");
  }
  const { summary } = result;
  const taskLabels = {
    MEDICINE_BASE: "药品基础数据",
    INVENTORY: "机构库存",
  };
  const evaluatedTasks = summary.contractImpact.evaluatedTasks || [];
  const lines = [
    "三方 HIS 适配器支持包摘要",
    "",
    `适配器：${summary.adapterId} v${summary.currentAdapterVersion}`,
    `支持包任务：${summary.migrationTask ? taskLabels[summary.migrationTask] || summary.migrationTask : "旧版未标注"}`,
    `来源指纹：${summary.sourceFingerprint}`,
    `数据库：${summary.currentDatabaseFamily}${summary.currentSchema ? ` · Schema ${summary.currentSchema}` : ""}`,
    `当前结论：${summary.currentDetected ? "结构通过" : "结构未通过"}`,
    `诊断历史：${summary.diagnosticCount} 次结构变化`,
    `时间范围：${summary.firstRecordedAt} → ${summary.lastRecordedAt}`,
    `当前缺失对象：${summary.currentMissingObjects.join("、") || "无"}`,
    `当前字段结构：${summary.currentStructuredObjectCount} 张表 / ${summary.currentStructuredColumnCount} 个字段（仅名称与类型）`,
    `评估任务：${evaluatedTasks.length ? evaluatedTasks.map((task) => taskLabels[task] || task).join("、") : "未限定（检查全部契约）"}`,
    `契约影响：${summary.contractImpact.message}`,
    `当前提醒：${summary.currentWarnings.join("；") || "无"}`,
  ];
  const formatImpact = (item) =>
    `${item.path}：${item.message}；影响 ${item.usedBy.join("、") || "未标注"}${item.fallback ? `；处理方式 ${item.fallback}` : ""}`;
  const appendImpact = (title, items) => {
    if (!items.length) return;
    const maximum = 30;
    lines.push(
      "",
      `${title}：`,
      ...items.slice(0, maximum).map((item) => `- ${formatImpact(item)}`),
    );
    if (items.length > maximum)
      lines.push(`- 其余 ${items.length - maximum} 项请结合 acceptance.json 核对`);
  };
  appendImpact("阻断项", summary.contractImpact.blockers);
  appendImpact("需核对", summary.contractImpact.reviews);
  appendImpact("可兼容降级", summary.contractImpact.compatibleFallbacks);
  if (summary.transitions.length) {
    lines.push("", "结构变化：");
    for (const transition of summary.transitions) {
      const changes = [
        transition.newlyCheckedObjects.length
          ? `新增可见 ${transition.newlyCheckedObjects.join("、")}`
          : "",
        transition.removedCheckedObjects.length
          ? `不再可见 ${transition.removedCheckedObjects.join("、")}`
          : "",
        transition.newMissingObjects.length
          ? `新增缺失 ${transition.newMissingObjects.join("、")}`
          : "",
        transition.resolvedMissingObjects.length
          ? `已恢复 ${transition.resolvedMissingObjects.join("、")}`
          : "",
        transition.newlyStructuredObjects.length
          ? `新增字段快照 ${compactSupportItems(transition.newlyStructuredObjects)}`
          : "",
        transition.removedStructuredObjects.length
          ? `字段快照缺失 ${compactSupportItems(transition.removedStructuredObjects)}`
          : "",
        transition.addedColumns.length
          ? `新增字段 ${compactSupportItems(transition.addedColumns)}`
          : "",
        transition.removedColumns.length
          ? `缺失字段 ${compactSupportItems(transition.removedColumns)}`
          : "",
        transition.changedColumnTypes.length
          ? `类型变化 ${compactSupportItems(
              transition.changedColumnTypes.map(
                (change) => `${change.column} ${change.from}→${change.to}`,
              ),
            )}`
          : "",
        ...transition.metricChanges.map(
          (metric) => `${metric.label} ${metric.from ?? "无"}→${metric.to}`,
        ),
      ].filter(Boolean);
      lines.push(
        `- ${transition.recordedAt} · v${transition.adapterVersion} · ${transition.detected ? "通过" : "未通过"}：${changes.join("；") || "检查结论或提醒发生变化"}`,
      );
    }
  }
  for (const warning of result.warnings) lines.push(`提醒：${warning}`);
  return lines.join("\n");
}

function compactSupportItems(items, limit = 12) {
  const visible = items.slice(0, limit);
  return `${visible.join("、")}${items.length > limit ? ` 等 ${items.length} 项` : ""}`;
}

function objectDiagnosticUnknownKeys(value, allowed, field, errors) {
  for (const key of unknownKeys(value, allowed))
    errors.push(`${field} 含有未声明字段 ${key}`);
}

function objectDiagnosticText(
  value,
  field,
  errors,
  maximum = 500,
  required = false,
) {
  if (
    typeof value !== "string" ||
    (required && !value.trim()) ||
    value.length > maximum
  )
    errors.push(
      `${field} 必须是${required ? "非空" : ""}且不超过 ${maximum} 字符的文本`,
    );
  return typeof value === "string" ? value.trim() : "";
}

function objectDiagnosticInteger(
  value,
  field,
  errors,
  maximum = Number.MAX_SAFE_INTEGER,
) {
  if (!Number.isInteger(value) || value < 0 || value > maximum)
    errors.push(`${field} 必须是 0–${maximum} 的整数`);
  return Number.isInteger(value) ? value : 0;
}

function collectObjectDiagnosticSensitiveKeys(value, prefix = "") {
  if (!value || typeof value !== "object") return [];
  if (Array.isArray(value))
    return value.flatMap((item, index) =>
      collectObjectDiagnosticSensitiveKeys(item, `${prefix}[${index}]`),
    );
  return Object.entries(value).flatMap(([key, item]) => {
    const current = prefix ? `${prefix}.${key}` : key;
    return [
      ...(key !== "containsRawSampleValues" &&
      objectDiagnosticSensitiveKey.test(key)
        ? [current]
        : []),
      ...collectObjectDiagnosticSensitiveKeys(item, current),
    ];
  });
}

export function analyzeSourceObjectDiagnostic(content) {
  const errors = [];
  const warnings = [];
  let value;
  try {
    value =
      typeof content === "string"
        ? JSON.parse(content)
        : structuredClone(content);
  } catch (error) {
    return {
      ok: false,
      errors: [`结构诊断 JSON 解析失败：${error.message}`],
      warnings,
    };
  }
  if (!value || typeof value !== "object" || Array.isArray(value))
    return { ok: false, errors: ["结构诊断必须是 JSON 对象"], warnings };
  objectDiagnosticUnknownKeys(
    value,
    objectDiagnosticTopLevelKeys,
    "根对象",
    errors,
  );
  const sensitive = collectObjectDiagnosticSensitiveKeys(value);
  if (sensitive.length)
    errors.push(
      `结构诊断含有禁止的连接、凭据、租户或原始值字段：${sensitive.join("、")}`,
    );
  if (value.formatVersion !== SOURCE_OBJECT_DIAGNOSTIC_VERSION)
    errors.push(`formatVersion 必须为 ${SOURCE_OBJECT_DIAGNOSTIC_VERSION}`);
  if (value.packageType !== SOURCE_OBJECT_DIAGNOSTIC_TYPE)
    errors.push(`packageType 必须为 ${SOURCE_OBJECT_DIAGNOSTIC_TYPE}`);
  if (!validDate(value.generatedAt)) errors.push("generatedAt 不是有效时间");
  if (
    typeof value.checksum !== "string" ||
    !/^[a-f0-9]{64}$/.test(value.checksum)
  )
    errors.push("checksum 必须是 64 位小写 SHA-256");
  else if (sourceObjectDiagnosticChecksum(value) !== value.checksum)
    errors.push("结构诊断完整性校验失败，文件可能已损坏或被修改");

  const source = value.source;
  if (!source || typeof source !== "object" || Array.isArray(source)) {
    errors.push("source 必须是对象");
  } else {
    objectDiagnosticUnknownKeys(
      source,
      objectDiagnosticSourceKeys,
      "source",
      errors,
    );
    const family = objectDiagnosticText(
      source.databaseFamily,
      "source.databaseFamily",
      errors,
      64,
      true,
    );
    if (family && !databaseFamilies.has(family))
      errors.push(`source.databaseFamily 不支持 ${family}`);
    objectDiagnosticText(source.schema, "source.schema", errors, 256);
    objectDiagnosticText(
      source.objectName,
      "source.objectName",
      errors,
      256,
      true,
    );
  }

  const preview = value.preview;
  let sampledRowCount = 0;
  if (!preview || typeof preview !== "object" || Array.isArray(preview)) {
    errors.push("preview 必须是对象");
  } else {
    objectDiagnosticUnknownKeys(
      preview,
      objectDiagnosticPreviewKeys,
      "preview",
      errors,
    );
    objectDiagnosticInteger(
      preview.columnCount,
      "preview.columnCount",
      errors,
      5_000,
    );
    sampledRowCount = objectDiagnosticInteger(
      preview.sampledRowCount,
      "preview.sampledRowCount",
      errors,
      100,
    );
    if (typeof preview.truncated !== "boolean")
      errors.push("preview.truncated 必须是布尔值");
    objectDiagnosticInteger(
      preview.elapsedMs,
      "preview.elapsedMs",
      errors,
      3_600_000,
    );
    objectDiagnosticText(
      preview.metadataMessage,
      "preview.metadataMessage",
      errors,
    );
    const probe = preview.rowCountProbe;
    if (!probe || typeof probe !== "object" || Array.isArray(probe)) {
      errors.push("preview.rowCountProbe 必须是对象");
    } else {
      objectDiagnosticUnknownKeys(
        probe,
        objectDiagnosticCountKeys,
        "preview.rowCountProbe",
        errors,
      );
      if (
        !["PENDING", "UNAVAILABLE", "LOWER_BOUND", "EXACT"].includes(
          probe.status,
        )
      )
        errors.push("preview.rowCountProbe.status 无效");
      if (["PENDING", "UNAVAILABLE"].includes(probe.status))
        objectDiagnosticText(
          probe.message,
          "preview.rowCountProbe.message",
          errors,
          500,
          true,
        );
      else {
        objectDiagnosticInteger(
          probe.rowCount,
          "preview.rowCountProbe.rowCount",
          errors,
        );
        objectDiagnosticInteger(
          probe.probeLimit,
          "preview.rowCountProbe.probeLimit",
          errors,
        );
        objectDiagnosticInteger(
          probe.elapsedMs,
          "preview.rowCountProbe.elapsedMs",
          errors,
          3_600_000,
        );
      }
    }
  }

  const columns = value.columns;
  const columnNames = [];
  if (!Array.isArray(columns) || !columns.length || columns.length > 5_000) {
    errors.push("columns 必须包含 1–5000 个字段");
  } else {
    columns.forEach((column, index) => {
      const field = `columns[${index}]`;
      if (!column || typeof column !== "object" || Array.isArray(column)) {
        errors.push(`${field} 必须是对象`);
        return;
      }
      objectDiagnosticUnknownKeys(
        column,
        objectDiagnosticColumnKeys,
        field,
        errors,
      );
      const name = objectDiagnosticText(
        column.name,
        `${field}.name`,
        errors,
        256,
        true,
      );
      if (name) columnNames.push(name);
      objectDiagnosticText(column.comment, `${field}.comment`, errors);
      objectDiagnosticText(
        column.sourceTable,
        `${field}.sourceTable`,
        errors,
        256,
      );
      objectDiagnosticText(
        column.sourceColumn,
        `${field}.sourceColumn`,
        errors,
        256,
      );
      if (typeof column.mappingEligible !== "boolean")
        errors.push(`${field}.mappingEligible 必须是布尔值`);
      const sample = column.sampleProfile;
      if (!sample || typeof sample !== "object" || Array.isArray(sample)) {
        errors.push(`${field}.sampleProfile 必须是对象`);
        return;
      }
      objectDiagnosticUnknownKeys(
        sample,
        objectDiagnosticSampleKeys,
        `${field}.sampleProfile`,
        errors,
      );
      const nonEmpty = objectDiagnosticInteger(
        sample.nonEmptyCount,
        `${field}.sampleProfile.nonEmptyCount`,
        errors,
        100,
      );
      if (nonEmpty > sampledRowCount)
        errors.push(`${field}.sampleProfile.nonEmptyCount 不能大于样例行数`);
      objectDiagnosticInteger(
        sample.maximumTextLength,
        `${field}.sampleProfile.maximumTextLength`,
        errors,
        1_000_000,
      );
      if (
        !Array.isArray(sample.observedKinds) ||
        sample.observedKinds.some(
          (kind) => !objectDiagnosticValueKinds.has(kind),
        )
      )
        errors.push(`${field}.sampleProfile.observedKinds 含有未知类型`);
    });
  }
  if (new Set(columnNames).size !== columnNames.length)
    errors.push("columns.name 不能重复");
  if (preview?.columnCount !== columns?.length)
    errors.push("preview.columnCount 与 columns 数量不一致");

  const assessment = value.medicineFieldAssessment;
  if (assessment !== null) {
    if (
      !assessment ||
      typeof assessment !== "object" ||
      Array.isArray(assessment)
    ) {
      errors.push("medicineFieldAssessment 必须是对象或 null");
    } else {
      objectDiagnosticUnknownKeys(
        assessment,
        objectDiagnosticAssessmentKeys,
        "medicineFieldAssessment",
        errors,
      );
      if (!["ready", "warning", "danger"].includes(assessment.status))
        errors.push("medicineFieldAssessment.status 无效");
      objectDiagnosticText(
        assessment.title,
        "medicineFieldAssessment.title",
        errors,
        500,
        true,
      );
      objectDiagnosticText(
        assessment.description,
        "medicineFieldAssessment.description",
        errors,
      );
      objectDiagnosticInteger(
        assessment.foundCount,
        "medicineFieldAssessment.foundCount",
        errors,
        100,
      );
      if (!Array.isArray(assessment.clues) || assessment.clues.length > 100) {
        errors.push("medicineFieldAssessment.clues 必须是不超过 100 项的数组");
      } else {
        assessment.clues.forEach((clue, index) => {
          const field = `medicineFieldAssessment.clues[${index}]`;
          objectDiagnosticUnknownKeys(
            clue,
            objectDiagnosticClueKeys,
            field,
            errors,
          );
          objectDiagnosticText(clue?.key, `${field}.key`, errors, 64, true);
          objectDiagnosticText(
            clue?.label,
            `${field}.label`,
            errors,
            128,
            true,
          );
          const column = objectDiagnosticText(
            clue?.column,
            `${field}.column`,
            errors,
            256,
          );
          if (column && !columnNames.includes(column))
            errors.push(`${field}.column 不在字段清单中`);
          if (typeof clue?.hasNonEmptySample !== "boolean")
            errors.push(`${field}.hasNonEmptySample 必须是布尔值`);
        });
      }
    }
  }

  const privacy = value.privacy;
  if (!privacy || typeof privacy !== "object" || Array.isArray(privacy)) {
    errors.push("privacy 必须是对象");
  } else {
    objectDiagnosticUnknownKeys(
      privacy,
      objectDiagnosticPrivacyKeys,
      "privacy",
      errors,
    );
    if (privacy.containsRawSampleValues !== false)
      errors.push("privacy.containsRawSampleValues 必须明确为 false");
    const excluded = supportList(
      privacy.excluded,
      "privacy.excluded",
      errors,
      20,
      100,
    );
    if (!excluded.some((item) => item.includes("原始样例值")))
      errors.push("privacy.excluded 必须声明排除原始样例值");
  }

  if (errors.length)
    return { ok: false, errors: unique(errors), warnings: unique(warnings) };
  const assessmentClues = assessment?.clues || [];
  const missingClues = assessmentClues.filter((clue) => !clue.column);
  const foundClues = assessmentClues.filter((clue) => clue.column);
  const commentedColumns = columns.filter((column) => column.comment).length;
  const recommendations = [];
  if (
    !foundClues.some((clue) => clue.key === "naMed" && clue.hasNonEmptySample)
  )
    recommendations.push(
      "先确认药品名称字段；当前对象不适合直接建立药品迁移模板",
    );
  if (missingClues.length)
    recommendations.push(
      `补充核对未识别字段：${missingClues.map((clue) => clue.label).join("、")}`,
    );
  if (!commentedColumns)
    recommendations.push(
      "当前没有字段注释，建议向 HIS 厂商索取数据字典或提供已整理业务视图",
    );
  if (
    preview.rowCountProbe.status === "LOWER_BOUND" ||
    Number(preview.rowCountProbe.rowCount || 0) > 10_000
  )
    recommendations.push(
      "来源超过单批上限，请按稳定业务键拆分只读查询或准备已筛选业务视图",
    );
  if (!recommendations.length)
    recommendations.push(
      "结构线索较完整，可进入字段映射并通过样例与正式校验确认业务含义",
    );
  return {
    ok: true,
    errors: [],
    warnings: unique(warnings),
    package: value,
    summary: {
      databaseFamily: source.databaseFamily,
      schema: source.schema,
      objectName: source.objectName,
      generatedAt: value.generatedAt,
      columnCount: columns.length,
      commentedColumnCount: commentedColumns,
      sampledRowCount,
      rowCountProbe: preview.rowCountProbe,
      assessmentTitle: assessment?.title || "未提供药品字段评估",
      foundClues,
      missingClues,
      recommendations,
    },
  };
}

export function formatSourceObjectDiagnosticAnalysis(result) {
  if (!result.ok)
    return [
      "通用来源结构诊断校验失败",
      "",
      ...result.errors.map((error) => `错误：${error}`),
      ...result.warnings.map((warning) => `提醒：${warning}`),
    ].join("\n");
  const { summary } = result;
  const probe = summary.rowCountProbe;
  const rowCount =
    probe.status === "EXACT"
      ? `${probe.rowCount} 行（准确）`
      : probe.status === "LOWER_BOUND"
        ? `至少 ${probe.rowCount} 行`
        : "短时探测未完成";
  return [
    "通用来源结构诊断摘要",
    "",
    `数据库：${summary.databaseFamily}${summary.schema ? ` · Schema ${summary.schema}` : ""}`,
    `来源对象：${summary.objectName}`,
    `生成时间：${summary.generatedAt}`,
    `字段：${summary.columnCount} 个，其中 ${summary.commentedColumnCount} 个带注释`,
    `样例：${summary.sampledRowCount} 行（仅保留形态统计）`,
    `数据规模：${rowCount}`,
    `药品字段判断：${summary.assessmentTitle}`,
    `已识别：${summary.foundClues.map((clue) => `${clue.label}←${clue.column}`).join("、") || "无"}`,
    `未识别：${summary.missingClues.map((clue) => clue.label).join("、") || "无"}`,
    "",
    "建议下一步：",
    ...summary.recommendations.map((item) => `- ${item}`),
    "",
    "隐私与完整性：SHA-256 校验通过；未发现连接、凭据、租户或原始样例值字段。",
    ...result.warnings.map((warning) => `提醒：${warning}`),
  ].join("\n");
}

function requireList(report, value, field, minimum = 1) {
  const values = stringList(value);
  if (values.length < minimum)
    report.errors.push(`${field} 至少需要 ${minimum} 项`);
  if (values.some((item) => placeholder.test(item)))
    report.errors.push(`${field} 仍含有 TODO/待填写占位内容`);
  return values;
}

function validateStructureRequirement(report, value, field, fallback) {
  const requirement = `${value || ""}`.trim().toUpperCase();
  if (!structureRequirements.has(requirement))
    report.errors.push(
      `${field} 必须为 REQUIRED、CONDITIONAL 或 OPTIONAL`,
    );
  if (
    requirement !== "REQUIRED" &&
    (!`${fallback || ""}`.trim() || placeholder.test(`${fallback || ""}`))
  )
    report.errors.push(`${field} 为 ${requirement || "非必需"} 时必须说明 fallback`);
  return requirement;
}

function validateStructureTasks(
  report,
  value,
  field,
  manifest,
  parentTasks = null,
) {
  if (value === undefined) return parentTasks;
  if (!Array.isArray(value) || !value.length) {
    report.errors.push(`${field} 必须是至少包含 1 个迁移任务的数组`);
    return new Set();
  }
  const tasks = normalizedMigrationTasks(value);
  if (tasks.length !== value.length)
    report.errors.push(`${field} 不能包含空值或重复任务`);
  for (const task of tasks) {
    if (!migrationTasks.has(task))
      report.errors.push(`${field} 含不支持的迁移任务 ${task}`);
    else if (!manifest.migrationTasks.includes(task))
      report.errors.push(`${field} 的 ${task} 未在 manifest.json 声明`);
    if (parentTasks && !parentTasks.has(task))
      report.errors.push(`${field} 的 ${task} 超出上级对象任务范围`);
  }
  return new Set(tasks);
}

function validateRequiredForTasks(
  report,
  value,
  requirement,
  field,
  manifest,
  applicableTasks,
) {
  if (value === undefined) return;
  if (requirement !== "CONDITIONAL")
    report.errors.push(`${field} 仅能用于 CONDITIONAL 表、字段或字段组`);
  validateStructureTasks(
    report,
    value,
    field,
    manifest,
    applicableTasks,
  );
}

function validateRequiredWhenObjectsPresent(
  report,
  value,
  requirement,
  field,
  references,
) {
  if (value === undefined) return;
  if (requirement !== "CONDITIONAL")
    report.errors.push(`${field} 仅能用于 CONDITIONAL 表、字段或字段组`);
  const objects = requireList(report, value, field).map((name) =>
    name.toUpperCase(),
  );
  if (new Set(objects).size !== objects.length)
    report.errors.push(`${field} 不能包含重复来源表`);
  references.push({ field, objects });
}

function validateSourceStructureContract(report, acceptance, manifest) {
  const contract = acceptance?.sourceStructure;
  if (!contract || typeof contract !== "object" || Array.isArray(contract)) {
    report.errors.push("acceptance.json 缺少 sourceStructure 表字段契约");
    return;
  }
  const unknownContractKeys = Object.keys(contract).filter(
    (key) => !acceptanceStructureKeys.has(key),
  );
  if (unknownContractKeys.length)
    report.errors.push(
      `sourceStructure 含未识别字段：${unknownContractKeys.join("、")}`,
    );
  const objects = Array.isArray(contract.objects) ? contract.objects : [];
  if (!objects.length) {
    report.errors.push("sourceStructure.objects 至少需要 1 张表");
    return;
  }
  if (objects.length > 80)
    report.errors.push("sourceStructure.objects 超过 80 张表上限");
  const seenObjects = new Set();
  const objectColumns = new Map();
  const objectColumnRequirements = new Map();
  const requiredObjectReferences = [];
  for (const [objectIndex, object] of objects.entries()) {
    const field = `sourceStructure.objects[${objectIndex}]`;
    if (!object || typeof object !== "object" || Array.isArray(object)) {
      report.errors.push(`${field} 必须是对象`);
      continue;
    }
    const unknown = Object.keys(object).filter(
      (key) => !acceptanceStructureObjectKeys.has(key),
    );
    if (unknown.length)
      report.errors.push(`${field} 含未识别字段：${unknown.join("、")}`);
    const name = `${object.name || ""}`.trim().toUpperCase();
    if (!name || name.length > 120 || placeholder.test(name))
      report.errors.push(`${field}.name 必须是真实表名`);
    if (seenObjects.has(name)) report.errors.push(`${field}.name ${name} 重复`);
    seenObjects.add(name);
    const objectRequirement = validateStructureRequirement(
      report,
      object.requirement,
      `${field}.requirement`,
      object.fallback,
    );
    const objectTasks = validateStructureTasks(
      report,
      object.appliesToTasks,
      `${field}.appliesToTasks`,
      manifest,
    );
    validateRequiredForTasks(
      report,
      object.requiredForTasks,
      objectRequirement,
      `${field}.requiredForTasks`,
      manifest,
      objectTasks,
    );
    validateRequiredWhenObjectsPresent(
      report,
      object.requiredWhenObjectsPresent,
      objectRequirement,
      `${field}.requiredWhenObjectsPresent`,
      requiredObjectReferences,
    );
    requireList(report, object.usedBy, `${field}.usedBy`);
    const columns = Array.isArray(object.columns) ? object.columns : [];
    if (!columns.length) report.errors.push(`${field}.columns 至少需要 1 个字段`);
    if (columns.length > 500)
      report.errors.push(`${field}.columns 超过 500 个字段上限`);
    const seenColumns = new Set();
    const columnRequirements = new Map();
    for (const [columnIndex, column] of columns.entries()) {
      const columnField = `${field}.columns[${columnIndex}]`;
      if (!column || typeof column !== "object" || Array.isArray(column)) {
        report.errors.push(`${columnField} 必须是对象`);
        continue;
      }
      const unknownColumn = Object.keys(column).filter(
        (key) => !acceptanceStructureColumnKeys.has(key),
      );
      if (unknownColumn.length)
        report.errors.push(
          `${columnField} 含未识别字段：${unknownColumn.join("、")}`,
        );
      const columnName = `${column.name || ""}`.trim().toUpperCase();
      if (!columnName || columnName.length > 120 || placeholder.test(columnName))
        report.errors.push(`${columnField}.name 必须是真实字段名`);
      if (seenColumns.has(columnName))
        report.errors.push(`${columnField}.name ${columnName} 重复`);
      seenColumns.add(columnName);
      const columnRequirement = validateStructureRequirement(
        report,
        column.requirement,
        `${columnField}.requirement`,
        column.fallback,
      );
      validateStructureTasks(
        report,
        column.appliesToTasks,
        `${columnField}.appliesToTasks`,
        manifest,
        objectTasks,
      );
      validateRequiredForTasks(
        report,
        column.requiredForTasks,
        columnRequirement,
        `${columnField}.requiredForTasks`,
        manifest,
        objectTasks,
      );
      validateRequiredWhenObjectsPresent(
        report,
        column.requiredWhenObjectsPresent,
        columnRequirement,
        `${columnField}.requiredWhenObjectsPresent`,
        requiredObjectReferences,
      );
      columnRequirements.set(columnName, columnRequirement);
      requireList(report, column.usedBy, `${columnField}.usedBy`);
      const families = stringList(column.acceptedTypeFamilies).map((item) =>
        item.toUpperCase(),
      );
      if (
        families.some((family) => !structureTypeFamilies.has(family)) ||
        new Set(families).size !== families.length
      )
        report.errors.push(
          `${columnField}.acceptedTypeFamilies 含无效或重复类型族`,
        );
    }
    objectColumns.set(name, seenColumns);
    objectColumnRequirements.set(name, columnRequirements);
    const groups = Array.isArray(object.columnGroups)
      ? object.columnGroups
      : [];
    const seenGroups = new Set();
    for (const [groupIndex, group] of groups.entries()) {
      const groupField = `${field}.columnGroups[${groupIndex}]`;
      if (!group || typeof group !== "object" || Array.isArray(group)) {
        report.errors.push(`${groupField} 必须是对象`);
        continue;
      }
      const unknownGroup = Object.keys(group).filter(
        (key) => !acceptanceStructureGroupKeys.has(key),
      );
      if (unknownGroup.length)
        report.errors.push(
          `${groupField} 含未识别字段：${unknownGroup.join("、")}`,
        );
      const groupName = `${group.name || ""}`.trim();
      if (!groupName || placeholder.test(groupName))
        report.errors.push(`${groupField}.name 不能为空`);
      if (seenGroups.has(groupName))
        report.errors.push(`${groupField}.name ${groupName} 重复`);
      seenGroups.add(groupName);
      const groupColumns = requireList(
        report,
        group.columns,
        `${groupField}.columns`,
        2,
      ).map((item) => item.toUpperCase());
      if (groupColumns.some((column) => !seenColumns.has(column)))
        report.errors.push(`${groupField}.columns 必须引用本表已声明字段`);
      const groupRequirement = validateStructureRequirement(
        report,
        group.requirement,
        `${groupField}.requirement`,
        group.fallback,
      );
      validateStructureTasks(
        report,
        group.appliesToTasks,
        `${groupField}.appliesToTasks`,
        manifest,
        objectTasks,
      );
      validateRequiredForTasks(
        report,
        group.requiredForTasks,
        groupRequirement,
        `${groupField}.requiredForTasks`,
        manifest,
        objectTasks,
      );
      validateRequiredWhenObjectsPresent(
        report,
        group.requiredWhenObjectsPresent,
        groupRequirement,
        `${groupField}.requiredWhenObjectsPresent`,
        requiredObjectReferences,
      );
      requireList(report, group.usedBy, `${groupField}.usedBy`);
    }
  }
  const objectGroups = Array.isArray(contract.objectGroups)
    ? contract.objectGroups
    : [];
  if (objectGroups.length > 40)
    report.errors.push("sourceStructure.objectGroups 超过 40 组上限");
  const seenObjectGroups = new Set();
  for (const [groupIndex, group] of objectGroups.entries()) {
    const field = `sourceStructure.objectGroups[${groupIndex}]`;
    if (!group || typeof group !== "object" || Array.isArray(group)) {
      report.errors.push(`${field} 必须是对象`);
      continue;
    }
    const unknown = Object.keys(group).filter(
      (key) => !acceptanceStructureObjectGroupKeys.has(key),
    );
    if (unknown.length)
      report.errors.push(`${field} 含未识别字段：${unknown.join("、")}`);
    const name = `${group.name || ""}`.trim();
    if (!name || placeholder.test(name))
      report.errors.push(`${field}.name 不能为空`);
    if (seenObjectGroups.has(name))
      report.errors.push(`${field}.name ${name} 重复`);
    seenObjectGroups.add(name);
    const candidates = requireList(
      report,
      group.objects,
      `${field}.objects`,
      2,
    ).map((item) => item.toUpperCase());
    if (new Set(candidates).size !== candidates.length)
      report.errors.push(`${field}.objects 不能包含重复来源表`);
    if (candidates.some((objectName) => !seenObjects.has(objectName)))
      report.errors.push(`${field}.objects 必须引用已声明来源表`);
    const requirement = validateStructureRequirement(
      report,
      group.requirement,
      `${field}.requirement`,
      group.fallback,
    );
    const tasks = validateStructureTasks(
      report,
      group.appliesToTasks,
      `${field}.appliesToTasks`,
      manifest,
    );
    validateRequiredForTasks(
      report,
      group.requiredForTasks,
      requirement,
      `${field}.requiredForTasks`,
      manifest,
      tasks,
    );
    requireList(report, group.usedBy, `${field}.usedBy`);
  }
  for (const reference of requiredObjectReferences) {
    if (reference.objects.some((name) => !seenObjects.has(name)))
      report.errors.push(`${reference.field} 必须引用已声明来源表`);
  }
  const requiredObjects = new Set(
    stringList(acceptance?.medicine?.requiredObjects).map((item) =>
      item.toUpperCase(),
    ),
  );
  for (const name of requiredObjects) {
    const object = objects.find(
      (item) => `${item?.name || ""}`.trim().toUpperCase() === name,
    );
    if (!object || `${object.requirement || ""}`.toUpperCase() !== "REQUIRED")
      report.errors.push(
        `medicine.requiredObjects 中的 ${name} 必须在 sourceStructure 中声明为 REQUIRED`,
      );
    else {
      const tasks = normalizedMigrationTasks(object.appliesToTasks);
      if (tasks.length && !tasks.includes("MEDICINE_BASE"))
        report.errors.push(
          `medicine.requiredObjects 中的 ${name} 必须适用于 MEDICINE_BASE`,
        );
    }
  }
  for (const identity of stringList(acceptance?.medicine?.sourceIdentityFields)) {
    const [objectName, columnName, ...extra] = identity
      .split(".")
      .map((item) => item.trim().toUpperCase());
    const object = objects.find(
      (item) =>
        `${item?.name || ""}`.trim().toUpperCase() === objectName,
    );
    const column = (object?.columns || []).find(
      (item) => `${item?.name || ""}`.trim().toUpperCase() === columnName,
    );
    const objectTasks = normalizedMigrationTasks(object?.appliesToTasks);
    const columnTasks = normalizedMigrationTasks(column?.appliesToTasks);
    if (
      extra.length ||
      !objectName ||
      !columnName ||
      !objectColumns.get(objectName)?.has(columnName) ||
      objectColumnRequirements.get(objectName)?.get(columnName) !== "REQUIRED" ||
      (objectTasks.length && !objectTasks.includes("MEDICINE_BASE")) ||
      (columnTasks.length && !columnTasks.includes("MEDICINE_BASE"))
    )
      report.errors.push(
        `medicine.sourceIdentityFields 中的 ${identity} 必须在 sourceStructure 声明为 REQUIRED`,
      );
  }
}

export function validateAdapterFixture({
  fixture,
  manifest,
  file = "fixture.json",
}) {
  const errors = [];
  const add = (message) => errors.push(`${file}：${message}`);
  if (!fixture || typeof fixture !== "object" || Array.isArray(fixture)) {
    add("必须是 JSON 对象");
    return errors;
  }
  const rootUnknown = unknownKeys(fixture, fixtureTopLevelKeys);
  if (rootUnknown.length)
    add(`含未识别根字段：${rootUnknown.join("、")}`);
  const schemaVersion = fixture.fixtureSchemaVersion;
  if (!supportedFixtureSchemaVersions.has(schemaVersion))
    add(`fixtureSchemaVersion 必须为 1 或 ${FIXTURE_SCHEMA_VERSION}`);
  if (`${fixture.adapterId || ""}`.trim().toUpperCase() !== manifest.id)
    add("adapterId 与 manifest.json 不一致");
  if (!/^[a-z][a-z0-9_-]{1,79}$/.test(`${fixture.id || ""}`.trim()))
    add("id 必须是稳定的小写字母、数字、下划线或短横线标识");
  if (
    !`${fixture.title || ""}`.trim() ||
    placeholder.test(`${fixture.title || ""}`)
  )
    add("title 不能为空或保留 TODO");
  const family = `${fixture.databaseFamily || ""}`.trim().toLowerCase();
  if (!manifest.databaseFamilies.includes(family))
    add(`databaseFamily ${family || "<空>"} 未在 manifest.json 声明`);
  const tasks = unique(
    stringList(fixture.tasks).map((item) => item.toUpperCase()),
  );
  if (!tasks.length) add("tasks 至少需要 1 项");
  for (const task of tasks) {
    if (!manifest.migrationTasks.includes(task))
      add(`tasks 中的 ${task} 未在 manifest.json 声明`);
  }
  const sourceObjects = Array.isArray(fixture.sourceObjects)
    ? fixture.sourceObjects
    : [];
  if (!sourceObjects.length) add("sourceObjects 至少需要 1 项");
  if (sourceObjects.length > 80) add("sourceObjects 超过 80 张表上限");
  const seenObjects = new Set();
  for (const [index, object] of sourceObjects.entries()) {
    if (!object || typeof object !== "object" || Array.isArray(object)) {
      add(`sourceObjects[${index}] 必须是对象`);
      continue;
    }
    const objectUnknown = unknownKeys(object, fixtureSourceObjectKeys);
    if (objectUnknown.length)
      add(`sourceObjects[${index}] 含未识别字段：${objectUnknown.join("、")}`);
    const name = `${object?.name || ""}`.trim().toUpperCase();
    const rawColumns = Array.isArray(object?.columns) ? object.columns : [];
    if (!Array.isArray(object?.columns))
      add(`sourceObjects[${index}].columns 必须是数组`);
    if (rawColumns.length > 500)
      add(`sourceObjects[${index}].columns 超过 500 个字段上限`);
    const columns = rawColumns.map((item, columnIndex) => {
      if (schemaVersion === 1) {
        if (typeof item !== "string") {
          add(`sourceObjects[${index}].columns[${columnIndex}] 在 v1 中必须是字段名文本`);
          return "";
        }
        return item.trim().toUpperCase();
      }
      if (!item || typeof item !== "object" || Array.isArray(item)) {
        add(`sourceObjects[${index}].columns[${columnIndex}] 在 v2 中必须包含 name 和 dataType`);
        return "";
      }
      const extra = unknownKeys(item, fixtureColumnKeys);
      if (extra.length)
        add(`sourceObjects[${index}].columns[${columnIndex}] 含未识别字段：${extra.join("、")}`);
      const columnName = `${item.name || ""}`.trim().toUpperCase();
      const dataType = `${item.dataType || ""}`.trim();
      if (!columnName || columnName.length > 120 || placeholder.test(columnName))
        add(`sourceObjects[${index}].columns[${columnIndex}].name 无效`);
      if (!dataType || dataType.length > 120 || placeholder.test(dataType))
        add(`sourceObjects[${index}].columns[${columnIndex}].dataType 必须是 1–120 字符物理类型`);
      return columnName;
    });
    if (!name || name.length > 120 || placeholder.test(name))
      add(`sourceObjects[${index}].name 无效`);
    if (seenObjects.has(name)) add(`sourceObjects 中对象 ${name} 重复`);
    seenObjects.add(name);
    if (!columns.length || columns.some((column) => !column || column.length > 120 || placeholder.test(column)))
      add(`sourceObjects[${index}].columns 至少需要 1 个真实字段`);
    if (new Set(columns).size !== columns.length)
      add(`sourceObjects[${index}].columns 存在重复字段`);
  }
  if (!fixture.expectations || typeof fixture.expectations !== "object" || Array.isArray(fixture.expectations))
    add("expectations 必须是对象");
  const expectationUnknown = unknownKeys(fixture.expectations, fixtureExpectationKeys);
  if (expectationUnknown.length)
    add(`expectations 含未识别字段：${expectationUnknown.join("、")}`);
  const outcome = `${fixture.expectations?.outcome || ""}`.trim().toUpperCase();
  if (!new Set(["PASS", "BLOCK"]).has(outcome))
    add("expectations.outcome 必须为 PASS 或 BLOCK");
  const checks = stringList(fixture.expectations?.checks);
  if (!Array.isArray(fixture.expectations?.checks))
    add("expectations.checks 必须是数组");
  if ((fixture.expectations?.checks || []).length > 50)
    add("expectations.checks 超过 50 项上限");
  if (!checks.length || checks.some((item) => placeholder.test(item)))
    add("expectations.checks 至少需要 1 项且不能保留 TODO");
  if (checks.some((item) => item.length > 500))
    add("expectations.checks 每项不能超过 500 字符");
  const rustTest = `${fixture.rustTest || ""}`.trim();
  if (
    !/^[a-zA-Z0-9_]+(?:::[a-zA-Z0-9_]+)+$/.test(rustTest) ||
    placeholder.test(rustTest)
  )
    add("rustTest 必须填写不含占位内容的可执行 Rust 测试完整模块路径");
  const forbidden = collectForbiddenKeys(fixture);
  if (forbidden.length)
    add(`含项目连接、凭据或 SQL 字段：${forbidden.join("、")}`);
  return errors;
}

function validateAcceptance(report, acceptance, manifest) {
  if (!acceptance) {
    report.errors.push("自动识别或库存适配器缺少 acceptance.json 验收契约");
    return;
  }
  if (acceptance.acceptanceSchemaVersion !== ACCEPTANCE_SCHEMA_VERSION)
    report.errors.push(
      `acceptanceSchemaVersion 必须为 ${ACCEPTANCE_SCHEMA_VERSION}`,
    );
  if (`${acceptance.adapterId || ""}`.trim().toUpperCase() !== manifest.id)
    report.errors.push("acceptance.json 的 adapterId 与 manifest.json 不一致");
  if (manifest.migrationTasks.includes("MEDICINE_BASE")) {
    const medicine = acceptance.medicine;
    if (!medicine || typeof medicine !== "object") {
      report.errors.push("acceptance.json 缺少 medicine 验收范围");
    } else {
      requireList(report, medicine.requiredObjects, "medicine.requiredObjects");
      requireList(
        report,
        medicine.sourceIdentityFields,
        "medicine.sourceIdentityFields",
      );
      requireList(report, medicine.scopes, "medicine.scopes");
      requireList(report, medicine.testCases, "medicine.testCases", 2);
    }
  }
  if (manifest.migrationTasks.includes("INVENTORY")) {
    const inventory = acceptance.inventory;
    if (!inventory || typeof inventory !== "object") {
      report.errors.push("acceptance.json 缺少 inventory 验收范围");
    } else {
      requireList(
        report,
        inventory.organizationObjects,
        "inventory.organizationObjects",
      );
      requireList(
        report,
        inventory.locationObjects,
        "inventory.locationObjects",
      );
      requireList(report, inventory.stockObjects, "inventory.stockObjects");
      requireList(report, inventory.stableKeys, "inventory.stableKeys", 4);
      requireList(
        report,
        inventory.packagingEvidence,
        "inventory.packagingEvidence",
      );
      requireList(report, inventory.amountEvidence, "inventory.amountEvidence");
      requireList(report, inventory.testCases, "inventory.testCases", 4);
    }
  }
  validateSourceStructureContract(report, acceptance, manifest);
}

export function validateAdapterPackage({
  packageName,
  manifest,
  acceptance = null,
  sourceAdapterRust = "",
  implementationSource = "",
  maintenanceSource = "",
}) {
  const report = { packageName, id: "", errors: [], warnings: [] };
  if (!manifest || typeof manifest !== "object" || Array.isArray(manifest)) {
    report.errors.push("manifest.json 必须是 JSON 对象");
    return report;
  }
  manifest.id = `${manifest.id || ""}`.trim().toUpperCase();
  manifest.sourceModes = unique(
    stringList(manifest.sourceModes).map((item) => item.toLowerCase()),
  );
  manifest.databaseFamilies = unique(
    stringList(manifest.databaseFamilies).map((item) => item.toLowerCase()),
  );
  manifest.migrationTasks = unique(
    stringList(manifest.migrationTasks).map((item) => item.toUpperCase()),
  );
  report.id = manifest.id;
  const unknownKeys = Object.keys(manifest).filter(
    (key) => !manifestKeys.has(key),
  );
  if (unknownKeys.length)
    report.errors.push(`manifest.json 含未识别字段：${unknownKeys.join("、")}`);
  if (manifest.packageSchemaVersion !== PACKAGE_SCHEMA_VERSION)
    report.errors.push(`packageSchemaVersion 必须为 ${PACKAGE_SCHEMA_VERSION}`);
  if (!/^[A-Z][A-Z0-9_]{1,79}$/.test(manifest.id))
    report.errors.push("id 必须是稳定的大写字母、数字或下划线标识");
  if (packageName && packageName !== manifest.id.toLowerCase())
    report.errors.push(
      `目录名应为 ${manifest.id.toLowerCase() || "适配器 ID 的小写形式"}`,
    );
  if (!`${manifest.name || ""}`.trim()) report.errors.push("name 不能为空");
  if (!Number.isInteger(manifest.version) || manifest.version < 1)
    report.errors.push("version 必须是大于 0 的整数");
  if (
    !Number.isInteger(manifest.templateCompatibleFromVersion) ||
    manifest.templateCompatibleFromVersion < 1 ||
    manifest.templateCompatibleFromVersion > manifest.version
  )
    report.errors.push(
      "templateCompatibleFromVersion 必须在 1 到当前 version 之间",
    );
  const changes = Array.isArray(manifest.changes) ? manifest.changes : [];
  if (!changes.length)
    report.errors.push("changes 至少需要当前版本的一条变更说明");
  const changeVersions = new Set();
  for (const [index, change] of changes.entries()) {
    const version = change?.version;
    const summary = `${change?.summary || ""}`.trim();
    if (!Number.isInteger(version) || version < 1 || version > manifest.version)
      report.errors.push(`changes[${index}].version 无效`);
    if (changeVersions.has(version))
      report.errors.push(`changes 中版本 ${version} 重复`);
    changeVersions.add(version);
    if (!summary || placeholder.test(summary))
      report.errors.push(`changes[${index}].summary 不能为空或保留 TODO`);
  }
  if (
    Number.isInteger(manifest.version) &&
    !changeVersions.has(manifest.version)
  )
    report.errors.push("changes 必须包含当前 version 的变更说明");
  if (!`${manifest.summary || ""}`.trim())
    report.errors.push("summary 不能为空");
  if (!manifest.sourceModes.length) report.errors.push("sourceModes 不能为空");
  for (const mode of manifest.sourceModes) {
    if (!sourceModes.has(mode))
      report.errors.push(`不支持的 sourceModes：${mode}`);
  }
  for (const family of manifest.databaseFamilies) {
    if (!databaseFamilies.has(family))
      report.errors.push(`不支持的 databaseFamilies：${family}`);
  }
  for (const task of manifest.migrationTasks) {
    if (!migrationTasks.has(task))
      report.errors.push(`不支持的 migrationTasks：${task}`);
  }
  if (!manifest.migrationTasks.length)
    report.errors.push("migrationTasks 不能为空");
  if (
    manifest.sourceModes.includes("database") &&
    !manifest.databaseFamilies.length
  )
    report.errors.push("数据库适配器必须声明经过验证的 databaseFamilies");
  if (
    manifest.sourceModes.length === 1 &&
    manifest.sourceModes[0] === "file" &&
    manifest.databaseFamilies.length
  )
    report.errors.push("纯文件适配器不能声明 databaseFamilies");
  if (
    manifest.migrationTasks.includes("INVENTORY") &&
    (!manifest.sourceModes.includes("database") || !manifest.automaticDetection)
  )
    report.errors.push("库存适配器必须使用数据库来源并实现自动识别契约");
  if (typeof manifest.automaticDetection !== "boolean")
    report.errors.push("automaticDetection 必须是布尔值");
  if (typeof manifest.reusableMappingProfiles !== "boolean")
    report.errors.push("reusableMappingProfiles 必须是布尔值");
  if (typeof manifest.builtIn !== "boolean")
    report.errors.push("builtIn 必须是布尔值");
  const sourceCompatibility = manifest.sourceCompatibility;
  if (manifest.automaticDetection && !sourceCompatibility) {
    report.errors.push("自动识别适配器必须声明 sourceCompatibility 来源兼容策略");
  }
  if (sourceCompatibility !== undefined) {
    if (
      !sourceCompatibility ||
      typeof sourceCompatibility !== "object" ||
      Array.isArray(sourceCompatibility)
    ) {
      report.errors.push("sourceCompatibility 必须是对象");
    } else {
      const compatibilityKeys = new Set([
        "product",
        "mode",
        "declaredVersions",
        "gate",
      ]);
      const unknownCompatibilityKeys = Object.keys(sourceCompatibility).filter(
        (key) => !compatibilityKeys.has(key),
      );
      if (unknownCompatibilityKeys.length)
        report.errors.push(
          `sourceCompatibility 含未识别字段：${unknownCompatibilityKeys.join("、")}`,
        );
      const product = `${sourceCompatibility.product || ""}`.trim();
      const mode = `${sourceCompatibility.mode || ""}`.trim().toUpperCase();
      const declaredVersions = stringList(sourceCompatibility.declaredVersions).map(
        (version) => version.trim(),
      );
      const gate = `${sourceCompatibility.gate || ""}`.trim();
      if (!product || product.length > 120)
        report.errors.push("sourceCompatibility.product 必须是最多 120 字符的产品名称");
      if (!["STRUCTURE_CONTRACT", "DECLARED_VERSION_RANGE"].includes(mode))
        report.errors.push(
          "sourceCompatibility.mode 必须为 STRUCTURE_CONTRACT 或 DECLARED_VERSION_RANGE",
        );
      if (!Array.isArray(sourceCompatibility.declaredVersions))
        report.errors.push("sourceCompatibility.declaredVersions 必须是数组");
      if (
        declaredVersions.length > 20 ||
        declaredVersions.some((version) => !version || version.length > 80)
      )
        report.errors.push(
          "sourceCompatibility.declaredVersions 最多 20 项，且每项为最多 80 字符的非空版本说明",
        );
      if (new Set(declaredVersions).size !== declaredVersions.length)
        report.errors.push("sourceCompatibility.declaredVersions 不能重复");
      if (mode === "STRUCTURE_CONTRACT" && declaredVersions.length)
        report.errors.push(
          "STRUCTURE_CONTRACT 不能声明厂商版本范围，版本号只能作为项目记录",
        );
      if (mode === "DECLARED_VERSION_RANGE" && !declaredVersions.length)
        report.errors.push(
          "DECLARED_VERSION_RANGE 必须明确列出已验证版本范围",
        );
      if (!gate || gate.length > 300)
        report.errors.push("sourceCompatibility.gate 必须是最多 300 字符的验收门禁说明");
    }
  }
  const workflow = manifest.medicineWorkflow;
  if (!workflow || typeof workflow !== "object" || Array.isArray(workflow)) {
    report.errors.push("medicineWorkflow 必须声明药品来源键、映射和批次工作流");
  } else {
    const workflowKeys = new Set([
      "sourceKeyMode",
      "sourceKeyField",
      "sourceKeyLabel",
      "mappingPreset",
      "batchSourceType",
      "factoryPolicy",
      "legacyProfileKind",
    ]);
    const unknownWorkflowKeys = Object.keys(workflow).filter(
      (key) => !workflowKeys.has(key),
    );
    if (unknownWorkflowKeys.length)
      report.errors.push(
        `medicineWorkflow 含未识别字段：${unknownWorkflowKeys.join("、")}`,
      );
    const sourceKeyMode = `${workflow.sourceKeyMode || ""}`.toUpperCase();
    if (!["USER_SELECTED", "ADAPTER_PROVIDED"].includes(sourceKeyMode))
      report.errors.push(
        "medicineWorkflow.sourceKeyMode 必须为 USER_SELECTED 或 ADAPTER_PROVIDED",
      );
    if (
      sourceKeyMode === "ADAPTER_PROVIDED" &&
      (!`${workflow.sourceKeyField || ""}`.trim() ||
        !`${workflow.sourceKeyLabel || ""}`.trim() ||
        !/^[A-Za-z][A-Za-z0-9_]{0,127}$/.test(
          `${workflow.sourceKeyField || ""}`.trim(),
        ))
    )
      report.errors.push(
        "适配器提供来源键时必须声明合法的 sourceKeyField 和 sourceKeyLabel",
      );
    for (const [key, supported] of [
      ["mappingPreset", ["GUIDED", "PHIS27"]],
      ["batchSourceType", ["GENERIC", "PHIS27"]],
      ["legacyProfileKind", ["NONE", "PHIS27_V2"]],
    ]) {
      if (!supported.includes(`${workflow[key] || ""}`.toUpperCase()))
        report.errors.push(
          `medicineWorkflow.${key} 尚未注册，当前支持 ${supported.join("、")}`,
        );
    }
    if (
      !["OPTIONAL_CREATE", "ADAPTER_MANAGED"].includes(
        `${workflow.factoryPolicy || ""}`.toUpperCase(),
      )
    )
      report.errors.push(
        "medicineWorkflow.factoryPolicy 必须为 OPTIONAL_CREATE 或 ADAPTER_MANAGED",
      );
    if (manifest.automaticDetection && sourceKeyMode !== "ADAPTER_PROVIDED")
      report.errors.push("自动识别适配器必须由适配器提供稳定来源键");
  }
  const inventoryWorkflow = manifest.inventoryWorkflow;
  if (manifest.migrationTasks.includes("INVENTORY")) {
    if (
      !inventoryWorkflow ||
      typeof inventoryWorkflow !== "object" ||
      Array.isArray(inventoryWorkflow)
    ) {
      report.errors.push(
        "库存适配器必须声明 inventoryWorkflow 标准位置和公共安全契约",
      );
    } else {
      const inventoryWorkflowKeys = new Set([
        "normalizedLocationKinds",
        "sourceProductKeyLabel",
        "sourceStockKeyMode",
        "writeMode",
        "organizationMapping",
        "medicineLedger",
        "trialPolicy",
        "undoPolicy",
      ]);
      const unknownInventoryKeys = Object.keys(inventoryWorkflow).filter(
        (key) => !inventoryWorkflowKeys.has(key),
      );
      if (unknownInventoryKeys.length)
        report.errors.push(
          `inventoryWorkflow 含未识别字段：${unknownInventoryKeys.join("、")}`,
        );
      const kinds = unique(
        stringList(inventoryWorkflow.normalizedLocationKinds).map((kind) =>
          kind.toUpperCase(),
        ),
      );
      if (
        !kinds.length ||
        kinds.some((kind) => !["WAREHOUSE", "PHARMACY"].includes(kind))
      )
        report.errors.push(
          "inventoryWorkflow.normalizedLocationKinds 必须声明 WAREHOUSE 和/或 PHARMACY",
        );
      if (!`${inventoryWorkflow.sourceProductKeyLabel || ""}`.trim())
        report.errors.push("inventoryWorkflow.sourceProductKeyLabel 不能为空");
      const sourceStockKeyMode = `${inventoryWorkflow.sourceStockKeyMode || ""}`
        .trim()
        .toUpperCase();
      if (!["ADAPTER_SCOPED_V1", "PHIS27_LEGACY"].includes(sourceStockKeyMode))
        report.errors.push(
          "inventoryWorkflow.sourceStockKeyMode 必须为 ADAPTER_SCOPED_V1 或 PHIS27_LEGACY",
        );
      if (sourceStockKeyMode === "PHIS27_LEGACY" && manifest.id !== "PHIS27")
        report.errors.push(
          "PHIS27_LEGACY 仅用于二系列历史库存台账兼容，新适配器必须使用 ADAPTER_SCOPED_V1",
        );
      for (const [key, required] of [
        ["writeMode", "FIRST_STOCKTAKE"],
        ["organizationMapping", "REQUIRED"],
        ["medicineLedger", "REQUIRED"],
        ["trialPolicy", "PER_TARGET_STORAGE"],
        ["undoPolicy", "VERIFIED_BATCH_ONLY"],
      ]) {
        if (`${inventoryWorkflow[key] || ""}`.toUpperCase() !== required)
          report.errors.push(
            `inventoryWorkflow.${key} 必须为 ${required}，适配器不能绕过公共库存安全门禁`,
          );
      }
    }
  } else if (inventoryWorkflow !== undefined) {
    report.errors.push(
      "未声明 INVENTORY 任务的适配器不能配置 inventoryWorkflow",
    );
  }
  const implementation = `${manifest.implementation || ""}`
    .trim()
    .toUpperCase();
  if (!/^[A-Z][A-Z0-9_]{1,79}$/.test(implementation))
    report.errors.push("implementation 必须是大写字母、数字或下划线标识");
  if (manifest.automaticDetection && implementation === "GUIDED_MAPPING")
    report.errors.push("自动识别适配器不能使用 GUIDED_MAPPING 实现");
  if (manifest.automaticDetection && !implementationSource.trim())
    report.errors.push("自动识别适配器缺少包内 adapter.rs 实现文件");
  if (implementationSource && /TODO|todo!\s*\(/i.test(implementationSource))
    report.errors.push("adapter.rs 仍含有 TODO 占位实现");
  if (
    manifest.automaticDetection &&
    implementationSource.trim() &&
    !/use\s+super::sdk::\*/.test(implementationSource)
  )
    report.errors.push("adapter.rs 必须通过 super::sdk::* 使用公共适配器边界");
  if (
    manifest.automaticDetection &&
    manifest.id !== "PHIS27" &&
    /\bcrate::/.test(implementationSource)
  )
    report.errors.push(
      "新适配器不能直接依赖 crate 内部模块；请使用 super::sdk::* 的只读查询与统一模型",
    );
  if (!manifest.automaticDetection && implementation !== "GUIDED_MAPPING")
    report.warnings.push("非自动识别适配器通常应使用 GUIDED_MAPPING");
  if (
    implementation &&
    implementation !== "GUIDED_MAPPING" &&
    !hasImplementationBinding(sourceAdapterRust, implementation)
  )
    report.errors.push(`后端尚未绑定 implementation：${implementation}`);
  const forbidden = collectForbiddenKeys(manifest);
  if (forbidden.length)
    report.errors.push(
      `manifest.json 含项目连接或 SQL 字段：${forbidden.join("、")}`,
    );
  if (!maintenanceSource.trim()) {
    report.errors.push("缺少 MAINTENANCE.md 适配器维护说明");
  } else {
    if (placeholder.test(maintenanceSource))
      report.errors.push("MAINTENANCE.md 仍含有 TODO/待填写占位内容");
    const requiredMaintenanceSteps = [
      "templateCompatibleFromVersion",
      "npm run adapter:verify",
      "npm run adapter:support",
    ];
    if (manifest.automaticDetection || manifest.migrationTasks.includes("INVENTORY"))
      requiredMaintenanceSteps.push(
        "npm run adapter:contract-draft",
        "npm run adapter:fixture-draft",
        "npm run adapter:handoff",
        "npm run adapter:handoff-verify",
      );
    for (const required of requiredMaintenanceSteps) {
      if (!maintenanceSource.includes(required))
        report.errors.push(`MAINTENANCE.md 缺少维护步骤：${required}`);
    }
  }
  if (
    manifest.automaticDetection ||
    manifest.migrationTasks.includes("INVENTORY")
  )
    validateAcceptance(report, acceptance, manifest);
  return report;
}

async function readJson(file, optional = false) {
  try {
    return JSON.parse(await readFile(file, "utf8"));
  } catch (error) {
    if (optional && error.code === "ENOENT") return null;
    throw new Error(`${file} 解析失败：${error.message}`);
  }
}

async function readAdapterFixtures(packageRoot, manifest) {
  const fixturesRoot = path.join(packageRoot, "fixtures");
  let entries = [];
  try {
    entries = (await readdir(fixturesRoot, { withFileTypes: true }))
      .filter((entry) => entry.isFile() && entry.name.endsWith(".json"))
      .sort((left, right) => left.name.localeCompare(right.name));
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  return Promise.all(
    entries.map(async (entry) => ({
      file: entry.name,
      path: path.join(fixturesRoot, entry.name),
      packageName: path.basename(packageRoot),
      manifest,
      fixture: await readJson(path.join(fixturesRoot, entry.name)),
    })),
  );
}

async function validateAdapterFixtures(report, packageRoot, manifest) {
  if (!manifest.automaticDetection) return [];
  const cases = await readAdapterFixtures(packageRoot, manifest);
  if (!cases.length) {
    report.errors.push("自动识别适配器缺少 fixtures/*.json 项目变体回归夹具");
    return cases;
  }
  for (const item of cases) {
    report.errors.push(
      ...validateAdapterFixture({
        fixture: item.fixture,
        manifest,
        file: `fixtures/${item.file}`,
      }),
    );
  }
  for (const task of manifest.migrationTasks) {
    if (
      !cases.some((item) =>
        stringList(item.fixture.tasks)
          .map((value) => value.toUpperCase())
          .includes(task),
      )
    )
      report.errors.push(`fixtures 至少需要 1 个覆盖 ${task} 的项目变体`);
  }
  return cases;
}

export async function validateAcceptanceEvidence(
  report,
  acceptance,
  projectRoot,
  manifest,
) {
  if (
    !acceptance ||
    (!manifest.automaticDetection &&
      !manifest.migrationTasks.includes("INVENTORY"))
  )
    return;
  const evidence = Array.isArray(acceptance.evidence)
    ? acceptance.evidence
    : [];
  const minimum = manifest.migrationTasks.includes("INVENTORY") ? 4 : 2;
  if (evidence.length < minimum) {
    report.errors.push(`acceptance.evidence 至少需要 ${minimum} 条可执行证据`);
  }
  const coveredTasks = new Set();
  for (const [index, item] of evidence.entries()) {
    const label = `acceptance.evidence[${index}]`;
    const requirement = `${item?.requirement || ""}`.trim();
    const file = `${item?.file || ""}`.trim();
    const contains = `${item?.contains || ""}`.trim();
    const rustTest = `${item?.rustTest || ""}`.trim();
    const execution = `${item?.execution || "REQUIRED"}`.trim().toUpperCase();
    const unknown = unknownKeys(
      item,
      new Set(["requirement", "tasks", "file", "contains", "rustTest", "execution"]),
    );
    if (unknown.length)
      report.errors.push(`${label} 含未识别字段：${unknown.join("、")}`);
    const tasks = unique(stringList(item?.tasks).map((task) => task.toUpperCase()));
    if (!tasks.length) {
      report.errors.push(`${label}.tasks 至少需要声明 1 个迁移任务`);
    } else {
      for (const task of tasks) {
        if (!manifest.migrationTasks.includes(task))
          report.errors.push(`${label}.tasks 的 ${task} 未在 manifest.json 声明`);
        else coveredTasks.add(task);
      }
    }
    if (
      !requirement ||
      !file ||
      !contains ||
      !rustTest ||
      [requirement, file, contains, rustTest].some((value) => placeholder.test(value))
    ) {
      report.errors.push(
        `${label} 必须填写 requirement、file、contains 和 rustTest，且不能保留 TODO`,
      );
      continue;
    }
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(contains)) {
      report.errors.push(`${label}.contains 必须是 Rust 测试函数名`);
      continue;
    }
    if (
      !/^[a-zA-Z0-9_]+(?:::[a-zA-Z0-9_]+)+$/.test(rustTest) ||
      rustTest.split("::").at(-1) !== contains
    ) {
      report.errors.push(`${label}.rustTest 必须是以 ${contains} 结尾的完整 Rust 测试路径`);
      continue;
    }
    if (!new Set(["REQUIRED", "OPTIONAL_LIVE"]).has(execution)) {
      report.errors.push(`${label}.execution 必须为 REQUIRED 或 OPTIONAL_LIVE`);
      continue;
    }
    if (!file.endsWith(".rs")) {
      report.errors.push(`${label}.file 必须指向 Rust 源文件`);
      continue;
    }
    if (path.isAbsolute(file) || file.split(/[\\/]/).includes("..")) {
      report.errors.push(`${label}.file 必须是项目内相对路径`);
      continue;
    }
    try {
      const source = await readFile(path.join(projectRoot, file), "utf8");
      const functionPattern = new RegExp(
        `((?:\\s*#\\s*\\[[^\\]]+\\]\\s*)*)\\s*(?:pub(?:\\([^)]*\\))?\\s+)?(?:async\\s+)?fn\\s+${escapedPattern(contains)}\\s*\\(`,
      );
      const functionMatch = functionPattern.exec(source);
      if (!functionMatch) {
        report.errors.push(`${label} 未在 ${file} 找到测试函数 ${contains}`);
        continue;
      }
      const attributes = functionMatch[1] || "";
      if (!/#\s*\[\s*(?:(?:tokio|async_std)::)?test\b[^\]]*\]/.test(attributes))
        report.errors.push(`${label} 的 ${contains} 不是 #[test] 测试函数`);
      if (execution === "OPTIONAL_LIVE" && !/#\s*\[\s*ignore\b[^\]]*\]/.test(attributes))
        report.errors.push(`${label} 标记 OPTIONAL_LIVE 时测试必须使用 #[ignore]`);
    } catch (error) {
      report.errors.push(`${label} 无法读取 ${file}：${error.message}`);
    }
  }
  for (const task of manifest.migrationTasks) {
    if (!coveredTasks.has(task))
      report.errors.push(`acceptance.evidence 缺少 ${task} 任务的可执行证据`);
  }
}

async function readAdapterEvidenceCases(root, includeHidden = false) {
  let entries = [];
  try {
    entries = (await readdir(root, { withFileTypes: true }))
      .filter((entry) => entry.isDirectory() && (includeHidden || !entry.name.startsWith("_")))
      .sort((left, right) => left.name.localeCompare(right.name));
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  const cases = [];
  for (const entry of entries) {
    const packageRoot = path.join(root, entry.name);
    const manifest = await readJson(path.join(packageRoot, "manifest.json"));
    if (!manifest.automaticDetection) continue;
    const acceptance = await readJson(path.join(packageRoot, "acceptance.json"), true);
    for (const evidence of acceptance?.evidence || []) {
      cases.push({
        adapterId: manifest.id,
        requirement: evidence.requirement,
        tasks: evidence.tasks || [],
        rustTest: evidence.rustTest,
        execution: `${evidence.execution || "REQUIRED"}`.toUpperCase(),
      });
    }
  }
  return cases;
}

export async function listAdapterEvidenceCases(projectRoot) {
  const adaptersRoot = path.join(projectRoot, "src-tauri", "adapters");
  return [
    ...(await readAdapterEvidenceCases(adaptersRoot)),
    ...(await readAdapterEvidenceCases(path.join(adaptersRoot, "_examples"), true)),
  ];
}

export function evidenceExecutionPassed(status, output, execution) {
  if (status !== 0 || !/running 1 test/.test(output)) return false;
  if (/test result: ok\. 1 passed; 0 failed/.test(output)) return true;
  return (
    execution === "OPTIONAL_LIVE" &&
    /test result: ok\. 0 passed; 0 failed; 1 ignored/.test(output)
  );
}

export async function runAdapterEvidenceRegression(projectRoot) {
  const validationReports = [
    ...(await validateAdapterRegistry(projectRoot)),
    ...(await validateAdapterExamples(projectRoot)),
  ];
  if (!validationSummary(validationReports).ok)
    return { ok: false, validationReports, cases: [], results: [] };
  const cases = await listAdapterEvidenceCases(projectRoot);
  const byTest = new Map();
  for (const item of cases) {
    const current = byTest.get(item.rustTest);
    if (!current) {
      byTest.set(item.rustTest, { ...item, tasks: unique(item.tasks || []) });
      continue;
    }
    current.tasks = unique([...(current.tasks || []), ...(item.tasks || [])]);
    current.requirement = unique([current.requirement, item.requirement]).join("；");
    if (item.execution === "REQUIRED") current.execution = "REQUIRED";
  }
  const uniqueCases = [...byTest.values()];
  const results = uniqueCases.map((item) => {
    const executed = spawnSync(
      "cargo",
      ["test", "--lib", item.rustTest, "--", "--exact"],
      {
        cwd: path.join(projectRoot, "src-tauri"),
        encoding: "utf8",
        env: process.env,
      },
    );
    const output = `${executed.stdout || ""}\n${executed.stderr || ""}`;
    return {
      ...item,
      ok: evidenceExecutionPassed(executed.status, output, item.execution),
      status: executed.status,
      stdout: executed.stdout || "",
      stderr: executed.stderr || "",
    };
  });
  return {
    ok: results.every((item) => item.ok),
    validationReports,
    cases,
    results,
  };
}

export async function validateAdapterRegistry(projectRoot) {
  const adaptersRoot = path.join(projectRoot, "src-tauri", "adapters");
  const sourceAdapterRust = await readFile(
    path.join(projectRoot, "src-tauri", "src", "source_adapter.rs"),
    "utf8",
  );
  const entries = (await readdir(adaptersRoot, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory() && !entry.name.startsWith("_"))
    .sort((left, right) => left.name.localeCompare(right.name));
  const reports = [];
  for (const entry of entries) {
    try {
      const packageRoot = path.join(adaptersRoot, entry.name);
      const manifest = await readJson(path.join(packageRoot, "manifest.json"));
      const acceptance = await readJson(
        path.join(packageRoot, "acceptance.json"),
        true,
      );
      let adapterSource = "";
      let maintenanceSource = "";
      try {
        adapterSource = await readFile(
          path.join(packageRoot, "adapter.rs"),
          "utf8",
        );
      } catch (error) {
        if (error.code !== "ENOENT") throw error;
      }
      try {
        maintenanceSource = await readFile(
          path.join(packageRoot, "MAINTENANCE.md"),
          "utf8",
        );
      } catch (error) {
        if (error.code !== "ENOENT") throw error;
      }
      reports.push(
        validateAdapterPackage({
          packageName: entry.name,
          manifest,
          acceptance,
          sourceAdapterRust: `${sourceAdapterRust}\n${adapterSource}`,
          implementationSource: adapterSource,
          maintenanceSource,
        }),
      );
      await validateAcceptanceEvidence(
        reports.at(-1),
        acceptance,
        projectRoot,
        manifest,
      );
      await validateAdapterFixtures(reports.at(-1), packageRoot, manifest);
    } catch (error) {
      reports.push({
        packageName: entry.name,
        id: "",
        errors: [error.message],
        warnings: [],
      });
    }
  }
  const seen = new Map();
  for (const report of reports) {
    if (!report.id) continue;
    if (seen.has(report.id)) {
      report.errors.push(`适配器 ID 与 ${seen.get(report.id)} 重复`);
    } else {
      seen.set(report.id, report.packageName);
    }
  }
  return reports;
}

export async function validateAdapterExamples(projectRoot) {
  const examplesRoot = path.join(
    projectRoot,
    "src-tauri",
    "adapters",
    "_examples",
  );
  const sourceAdapterRust = await readFile(
    path.join(projectRoot, "src-tauri", "src", "source_adapter.rs"),
    "utf8",
  );
  let entries = [];
  try {
    entries = (await readdir(examplesRoot, { withFileTypes: true }))
      .filter((entry) => entry.isDirectory())
      .sort((left, right) => left.name.localeCompare(right.name));
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  const reports = [];
  for (const entry of entries) {
    try {
      const packageRoot = path.join(examplesRoot, entry.name);
      const manifest = await readJson(path.join(packageRoot, "manifest.json"));
      const acceptance = await readJson(
        path.join(packageRoot, "acceptance.json"),
        true,
      );
      const adapterSource = await readFile(
        path.join(packageRoot, "adapter.rs"),
        "utf8",
      );
      const maintenanceSource = await readFile(
        path.join(packageRoot, "MAINTENANCE.md"),
        "utf8",
      );
      const report = validateAdapterPackage({
        packageName: entry.name,
        manifest,
        acceptance,
        sourceAdapterRust: `${sourceAdapterRust}\n${adapterSource}`,
        implementationSource: adapterSource,
        maintenanceSource,
      });
      reports.push(report);
      await validateAcceptanceEvidence(
        report,
        acceptance,
        projectRoot,
        manifest,
      );
      await validateAdapterFixtures(report, packageRoot, manifest);
    } catch (error) {
      reports.push({
        packageName: entry.name,
        id: "",
        errors: [error.message],
        warnings: [],
      });
    }
  }
  return reports;
}

export function validationSummary(reports) {
  const errors = reports.reduce(
    (total, report) => total + report.errors.length,
    0,
  );
  const warnings = reports.reduce(
    (total, report) => total + report.warnings.length,
    0,
  );
  return { packages: reports.length, errors, warnings, ok: errors === 0 };
}

export function formatValidationReport(reports) {
  const lines = ["三方 HIS 来源适配器验收", ""];
  for (const report of reports) {
    lines.push(
      `${report.errors.length ? "✗" : "✓"} ${report.id || report.packageName}（${report.packageName}）`,
    );
    for (const error of report.errors) lines.push(`  错误：${error}`);
    for (const warning of report.warnings) lines.push(`  提醒：${warning}`);
  }
  const summary = validationSummary(reports);
  lines.push(
    "",
    `合计 ${summary.packages} 个适配包，${summary.errors} 个错误，${summary.warnings} 个提醒`,
  );
  return lines.join("\n");
}

const readinessStageDefinitions = [
  {
    id: "PACKAGE",
    label: "适配包声明与任务边界",
    matches: (error) =>
      /manifest|MAINTENANCE|packageSchemaVersion|adapterId 与|数据库类型|迁移任务|版本|清单/i.test(error),
    next: "先修正 manifest.json、维护说明和任务/数据库能力边界。",
  },
  {
    id: "CONTRACT",
    label: "来源结构与业务口径",
    matches: (error) =>
      /medicine\.|inventory\.|sourceStructure|acceptanceSchemaVersion/i.test(error) &&
      !/acceptance\.evidence/i.test(error),
    next: "按 PROJECT_INTAKE.md 核对真实表字段、稳定键、范围、包装和金额口径，再替换 acceptance.json 中的占位内容。",
  },
  {
    id: "IMPLEMENTATION",
    label: "来源读取与公共 SDK",
    matches: (error) =>
      /adapter\.rs|后端尚未绑定|implementation|公共 SDK|super::sdk|crate 内部/i.test(error),
    next: "将 adapter.rs.template 改为 adapter.rs，通过 super::sdk::* 完成只读来源识别和读取绑定。",
  },
  {
    id: "FIXTURES",
    label: "脱敏项目变体夹具",
    matches: (error) => /fixtures\/|项目变体夹具|fixture/i.test(error),
    next: "用现场脱敏支持包生成 v2 夹具草稿，补齐业务预期并移入 fixtures。",
  },
  {
    id: "EVIDENCE",
    label: "可执行测试证据",
    matches: (error) => /acceptance\.evidence|测试证据|可执行 Rust|rustTest/i.test(error),
    next: "为每个业务结论添加真实 Rust 测试，并把文件和断言标记写入 acceptance.evidence。",
  },
];

export function adapterReadiness(report) {
  const remaining = [...(report.errors || [])];
  const stages = readinessStageDefinitions.map((definition) => {
    const issues = remaining.filter(definition.matches);
    for (const issue of issues) remaining.splice(remaining.indexOf(issue), 1);
    return { ...definition, issues, ready: issues.length === 0 };
  });
  if (remaining.length) {
    stages[0].issues.push(...remaining);
    stages[0].ready = false;
  }
  const currentIndex = stages.findIndex((stage) => !stage.ready);
  return {
    packageName: report.packageName,
    id: report.id,
    ready: currentIndex === -1,
    currentStage: currentIndex === -1 ? null : stages[currentIndex],
    stages,
    warnings: report.warnings || [],
    issueCount: (report.errors || []).length,
  };
}

export function formatAdapterReadiness(reports) {
  const readiness = reports.map(adapterReadiness);
  const lines = ["三方 HIS 适配器就绪诊断", ""];
  for (const item of readiness) {
    lines.push(`${item.ready ? "✓" : "→"} ${item.id || item.packageName}（${item.ready ? "可进入交付回归" : `${item.issueCount} 项待处理`}）`);
    const currentIndex = item.stages.findIndex((stage) => !stage.ready);
    item.stages.forEach((stage, index) => {
      const marker = stage.ready ? "✓" : index === currentIndex ? "→" : "○";
      lines.push(`  ${marker} ${index + 1}. ${stage.label}${stage.ready ? "" : `：${stage.issues.length} 项`}`);
      if (index === currentIndex) {
        for (const issue of stage.issues.slice(0, 3)) lines.push(`     - ${issue}`);
        if (stage.issues.length > 3)
          lines.push(`     - 其余 ${stage.issues.length - 3} 项由严格验收保留，不在此重复展开`);
        lines.push(`     下一步：${stage.next}`);
      }
    });
    for (const warning of item.warnings.slice(0, 3)) lines.push(`  提醒：${warning}`);
    lines.push("");
  }
  lines.push(
    readiness.every((item) => item.ready)
      ? "全部适配器已就绪，请运行 npm run adapter:verify 完成交付回归。"
      : "需要完整错误明细时运行 npm run adapter:validate。",
  );
  return lines.join("\n");
}

function parseOptions(args) {
  const options = {};
  for (let index = 0; index < args.length; index += 1) {
    const token = args[index];
    if (!token.startsWith("--")) continue;
    const key = token.slice(2);
    if (key === "guided") options.guided = true;
    else ((options[key] = args[index + 1]), (index += 1));
  }
  return options;
}

export async function createAdapterPackage(projectRoot, options) {
  const id = `${options.id || ""}`.trim().toUpperCase();
  const name = `${options.name || ""}`.trim();
  if (!/^[A-Z][A-Z0-9_]{1,79}$/.test(id))
    throw new Error("--id 必须是大写字母、数字或下划线标识");
  if (!name) throw new Error("缺少 --name");
  const guided = Boolean(options.guided);
  const families = unique(
    `${options.database || "oracle"}`
      .split(",")
      .map((item) => item.trim().toLowerCase())
      .filter(Boolean),
  );
  const tasks = unique(
    `${options.tasks || "MEDICINE_BASE"}`
      .split(",")
      .map((item) => item.trim().toUpperCase())
      .filter(Boolean),
  );
  if (guided && tasks.includes("INVENTORY"))
    throw new Error("引导映射模式不支持库存；库存必须实现自动来源适配器");
  const packageName = id.toLowerCase();
  const packageRoot = path.join(
    projectRoot,
    "src-tauri",
    "adapters",
    packageName,
  );
  await mkdir(packageRoot, { recursive: false });
  const manifest = {
    packageSchemaVersion: PACKAGE_SCHEMA_VERSION,
    id,
    name,
    version: 1,
    templateCompatibleFromVersion: 1,
    changes: [
      {
        version: 1,
        summary: `${name} 初始适配器包。`,
      },
    ],
    summary: options.summary || `${name} 标准来源适配器。`,
    sourceModes: ["database"],
    databaseFamilies: families,
    migrationTasks: tasks,
    automaticDetection: !guided,
    reusableMappingProfiles: true,
    builtIn: true,
    implementation: guided ? "GUIDED_MAPPING" : `${id}_BUILTIN`,
    ...(!guided
      ? {
          sourceCompatibility: {
            product: name,
            mode: "STRUCTURE_CONTRACT",
            declaredVersions: [],
            gate: "版本号仅作项目记录；连接后以任务级来源表字段契约和项目变体回归为准。",
          },
        }
      : {}),
    medicineWorkflow: {
      sourceKeyMode: guided ? "USER_SELECTED" : "ADAPTER_PROVIDED",
      sourceKeyField: guided ? "" : "SOURCE_KEY",
      sourceKeyLabel: guided ? "" : "适配器稳定来源键",
      mappingPreset: "GUIDED",
      batchSourceType: "GENERIC",
      factoryPolicy: "OPTIONAL_CREATE",
      legacyProfileKind: "NONE",
    },
  };
  if (tasks.includes("INVENTORY")) {
    manifest.inventoryWorkflow = {
      normalizedLocationKinds: ["WAREHOUSE", "PHARMACY"],
      sourceProductKeyLabel: "来源药品商品键",
      sourceStockKeyMode: "ADAPTER_SCOPED_V1",
      writeMode: "FIRST_STOCKTAKE",
      organizationMapping: "REQUIRED",
      medicineLedger: "REQUIRED",
      trialPolicy: "PER_TARGET_STORAGE",
      undoPolicy: "VERIFIED_BATCH_ONLY",
    };
  }
  await writeFile(
    path.join(packageRoot, "manifest.json"),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  await writeFile(
    path.join(packageRoot, "MAINTENANCE.md"),
    `# ${name} 适配器维护\n\n` +
      `## 版本与模板兼容\n\n` +
      (!guided
        ? `- \`sourceCompatibility\` 必须明确选择“按结构契约”或“按已验证版本范围”；没有真实验收证据时不得填写或暗示厂商版本已认证。\n`
        : "") +
      `- 修改来源读取、字段含义或范围行为时递增 \`manifest.json\` 的 \`version\`，并在 \`changes\` 增加当前版本说明。\n` +
      `- 仅在旧模板仍可安全复用时保留或下调 \`templateCompatibleFromVersion\`；不兼容变更应将它提高到新的最低兼容版本。\n` +
      `- 来自未来版本的模板不能降级套用。\n\n` +
      `## 项目差异处理\n\n` +
      `1. 请现场人员在来源识别区导出“适配器支持包”。\n` +
      `2. 运行 \`npm run adapter:support -- --file <支持包.json> --task MEDICINE_BASE\`（或 \`INVENTORY\`），先确认本次任务的校验值、脱敏状态、业务影响，以及表字段新增、缺失和类型变化；不传 \`--task\` 时执行全量契约审计。v2 只携带字段名称与数据库类型，不含字段注释、样例值或 SQL。\n` +
      (!guided
        ? `3. 推荐运行 \`npm run adapter:handoff -- --file <支持包.json> --task MEDICINE_BASE\`（或 \`INVENTORY\`）一次生成维护交接计划、契约差异草稿、项目变体夹具草稿和 SHA-256 清单；接收方先运行 \`npm run adapter:handoff-verify -- --dir <交接目录>\` 验证清单自校验、文件完整性、任务一致性和隐私边界。交接包不会修改正式文件。仅需单独重生成时可使用 \`npm run adapter:contract-draft\` 和 \`npm run adapter:fixture-draft\`。\n`
        : "") +
      `${guided ? 3 : 4}. 将确认过的结构差异补入 \`fixtures/*.json\` 和对应可执行 Rust 回归测试；自动生成的 \`fixture.draft.json\` 必须先改写标题、业务预期并填写真实 \`rustTest\`。\n` +
      `${guided ? 4 : 5}. 如涉及表、字段、类型、稳定键、包装或金额证据，同步维护 \`acceptance.json\` 的 \`sourceStructure\`：明确 REQUIRED / CONDITIONAL / OPTIONAL、\`appliesToTasks\`、必要时的 \`requiredForTasks\`、受影响业务、可接受类型族和安全降级方式；库存的多来源分支使用 \`objectGroups\` 声明至少一个可用对象，并用 \`requiredWhenObjectsPresent\` 激活分支专属依赖。\n\n` +
      (tasks.includes("INVENTORY")
        ? `库存目录和预检优先使用 \`InventorySourceCatalog::new\`、\`InventorySourceReadiness::new\`，适配器身份和明细汇总由公共层补齐；库存明细先通过 \`InventorySourceStockItem::new\` 填写稳定身份，再按药品、库房包装、商品包装和金额分组补充字段，避免手工维护完整结构体。\n\n` +
          `库存明细必须使用标准包装字段：\`unit_sale_factor\` 表示库房/药房实际管理包装；\`single_minimum_package_factor\` 及 \`single_minimum_package_factor_source\` 表示可证明一包装仅含一个最小单位的结构化主档依据。没有可靠字段时留空，禁止从规格文本猜测。\n\n`
        : "") +
      `## 交付门禁\n\n` +
      `开发过程中运行 \`npm run adapter:status -- --adapter ${id}\` 查看当前阶段和下一步，需要完整错误时运行 \`npm run adapter:validate\`。每条 \`acceptance.evidence\` 必须声明所属任务并绑定真实 Rust 测试完整路径，每个清单任务都必须有证据；\`npm run adapter:evidence\` 会精确执行并拒绝任务遗漏或 0 项测试，只有带 \`#[ignore]\` 的真实数据库测试可标记 \`OPTIONAL_LIVE\`。最终运行 \`npm run adapter:verify\`；只有清单、实现绑定、维护说明、验收证据、脱敏夹具和可执行回归测试全部通过，才可进入桌面构建。\n`,
  );
  await writeFile(
    path.join(packageRoot, "PROJECT_INTAKE.md"),
    `# ${name} 接入调查清单\n\n` +
      `这份清单只记录表、字段、关系和业务口径，不填写数据库地址、账号密码、患者信息、样例药品或任何业务数据。现场结构证据统一从应用导出的脱敏支持包取得。\n\n` +
      `## 1. 能力与版本\n\n` +
      `- [ ] HIS 产品名称、正式版本和项目补丁范围已确认。\n` +
      `- [ ] 本次任务：${tasks.join("、")}。\n` +
      `- [ ] 数据库家族：${families.join("、")}；已确认只读账号可见对象范围。\n` +
      `- [ ] 已决定使用${guided ? "引导字段映射" : "自动来源适配器"}，并明确不支持的任务或版本。\n\n` +
      `- [ ] 动态 Schema/对象名和本批机构/库房过滤统一使用公共 SDK，不在适配器中复制引号或转义逻辑。\n\n` +
      `## 2. 药品主数据\n\n` +
      `| 必须确认 | 证据要求 | 完成 |\n` +
      `| --- | --- | --- |\n` +
      `| 药品主表与商品/厂家表 | 表名、关联字段、零或多商品时的业务含义 | [ ] |\n` +
      `| 稳定来源键 | 整批非空、唯一、长期不复用；复合键需固定顺序 | [ ] |\n` +
      `| 迁移范围 | 全部、机构配置、机构在用等范围的真实判定表和停用语义 | [ ] |\n` +
      `| 人可识别字段 | 名称、规格、剂型、最小单位、厂家、商品名的物理来源 | [ ] |\n` +
      `| 字典字段 | 来源编码、含义来源、项目覆盖规则及无法读取时的处理 | [ ] |\n` +
      `| 可选字段缺失 | 每个可选表/字段缺失时是安全留空、降级还是阻断 | [ ] |\n\n` +
      (tasks.includes("INVENTORY")
        ? `## 3. 机构库存\n\n` +
          `| 必须确认 | 证据要求 | 完成 |\n` +
          `| --- | --- | --- |\n` +
          `| 机构与库房目录 | 机构键、库房键、药库/药房类型、启停状态和归属关系 | [ ] |\n` +
          `| 轻量范围读取 | 选择范围前只读取机构、库房、药品数和库存行数，不枚举全量明细 | [ ] |\n` +
          `| 选择后下推过滤 | 机构键和库房键进入来源 SQL；未选位置不能进入本批 | [ ] |\n` +
          `| 库存稳定键 | 来源行键、库房键和药品商品键可解释且不会因连接重复 | [ ] |\n` +
          `| 药品商品关系 | 每行库存能解析到唯一来源商品；零个或多个候选时明确阻断/人工选择 | [ ] |\n` +
          `| 管理包装 | 管理单位、换算系数和“1 个最小单位包装”的结构化证据来源 | [ ] |\n` +
          `| 数量与金额 | 数量、进价、售价、进货金额、零售金额的单位和舍入规则 | [ ] |\n` +
          `| 批号和效期 | 物理类型、空值语义和字符日期格式；禁止依赖会话隐式转换 | [ ] |\n` +
          `| 首次盘点边界 | 适配器只标准化来源，空库、试迁移、写入、撤销继续由公共流程控制 | [ ] |\n\n`
        : "") +
      `## ${tasks.includes("INVENTORY") ? 4 : 3}. 可交付证据\n\n` +
      `- [ ] 每个支持任务至少有一个通过夹具和一个关键缺失/歧义夹具。\n` +
      `- [ ] v2 夹具只含表名、字段名、数据库物理类型和业务预期。\n` +
      `- [ ] 每个夹具绑定真实可执行 Rust 测试，且测试确实断言该结构差异。\n` +
      `- [ ] \`acceptance.json\` 对所有必需、条件和可选依赖给出业务用途与安全降级。\n` +
      `- [ ] \`npm run adapter:verify\`、Rust 全量测试和桌面构建均通过。\n`,
  );
  if (!guided) {
    await mkdir(path.join(packageRoot, "fixtures"));
    const acceptance = {
      acceptanceSchemaVersion: ACCEPTANCE_SCHEMA_VERSION,
      adapterId: id,
      medicine: tasks.includes("MEDICINE_BASE")
        ? {
            requiredObjects: ["TODO: 核心药品表"],
            sourceIdentityFields: ["TODO: 稳定药品与商品键"],
            scopes: ["TODO: 迁移范围"],
            testCases: ["TODO: 标准版本", "TODO: 项目变体"],
          }
        : null,
      inventory: tasks.includes("INVENTORY")
        ? {
            organizationObjects: ["TODO: 机构表"],
            locationObjects: ["TODO: 药库药房表"],
            stockObjects: ["TODO: 库存表"],
            stableKeys: [
              "sourceOrganizationId",
              "sourceLocationKey",
              "sourceRecordId",
              "sourceProductKey",
            ],
            packagingEvidence: ["TODO: 库房管理单位与换算系数"],
            amountEvidence: ["TODO: 数量、进价、售价与金额字段"],
            testCases: [
              "TODO: 标准库存",
              "TODO: 重复或歧义库房",
              "TODO: 包装关系异常",
              "TODO: 药品台账未匹配",
            ],
          }
        : null,
      sourceStructure: {
        objects: [
          {
            name: "TODO: 核心药品表",
            requirement: "REQUIRED",
            appliesToTasks: tasks,
            usedBy: ["TODO: 药品主档读取"],
            columns: [
              {
                name: "TODO: 稳定药品键",
                requirement: "REQUIRED",
                usedBy: ["TODO: 来源身份"],
                acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
              },
              {
                name: "TODO: 可选显示字段",
                requirement: "OPTIONAL",
                usedBy: ["TODO: 界面核对"],
                fallback: "TODO: 缺失时如何安全降级",
                acceptedTypeFamilies: ["CHARACTER"],
              },
              ...(tasks.includes("INVENTORY")
                ? [
                    {
                      name: "TODO: 库存任务必需的条件字段",
                      requirement: "CONDITIONAL",
                      appliesToTasks: ["INVENTORY"],
                      requiredForTasks: ["INVENTORY"],
                      usedBy: ["TODO: 库存包装或金额校验"],
                      fallback: "TODO: 非库存任务下的安全降级方式",
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                  ]
                : []),
            ],
          },
          ...(tasks.includes("INVENTORY")
            ? [
                {
                  name: "TODO: 机构表",
                  requirement: "REQUIRED",
                  appliesToTasks: ["INVENTORY"],
                  usedBy: ["库存机构范围"],
                  columns: [
                    {
                      name: "TODO: 稳定机构键",
                      requirement: "REQUIRED",
                      usedBy: ["机构映射"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 机构名称",
                      requirement: "REQUIRED",
                      usedBy: ["机构识别"],
                      acceptedTypeFamilies: ["CHARACTER"],
                    },
                  ],
                },
                {
                  name: "TODO: 药库药房表",
                  requirement: "REQUIRED",
                  appliesToTasks: ["INVENTORY"],
                  usedBy: ["库存位置范围"],
                  columns: [
                    {
                      name: "TODO: 稳定库房键",
                      requirement: "REQUIRED",
                      usedBy: ["库房映射"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 所属机构键",
                      requirement: "REQUIRED",
                      usedBy: ["机构与库房归属"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 库房类型",
                      requirement: "REQUIRED",
                      usedBy: ["药库药房标准化"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                  ],
                },
                {
                  name: "TODO: 库存表",
                  requirement: "REQUIRED",
                  appliesToTasks: ["INVENTORY"],
                  usedBy: ["本批库存明细"],
                  columns: [
                    {
                      name: "TODO: 稳定库存行键",
                      requirement: "REQUIRED",
                      usedBy: ["库存来源身份"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 库房键",
                      requirement: "REQUIRED",
                      usedBy: ["选择范围下推"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 药品商品键",
                      requirement: "REQUIRED",
                      usedBy: ["药品台账解析"],
                      acceptedTypeFamilies: ["NUMERIC", "CHARACTER"],
                    },
                    {
                      name: "TODO: 库存数量",
                      requirement: "REQUIRED",
                      usedBy: ["首次盘点数量"],
                      acceptedTypeFamilies: ["NUMERIC"],
                    },
                    {
                      name: "TODO: 管理单位",
                      requirement: "REQUIRED",
                      usedBy: ["库房包装"],
                      acceptedTypeFamilies: ["CHARACTER"],
                    },
                    {
                      name: "TODO: 管理包装系数",
                      requirement: "REQUIRED",
                      usedBy: ["库房包装换算"],
                      acceptedTypeFamilies: ["NUMERIC"],
                    },
                    {
                      name: "TODO: 进价",
                      requirement: "REQUIRED",
                      usedBy: ["进货金额核对"],
                      acceptedTypeFamilies: ["NUMERIC"],
                    },
                    {
                      name: "TODO: 零售价",
                      requirement: "REQUIRED",
                      usedBy: ["零售金额核对"],
                      acceptedTypeFamilies: ["NUMERIC"],
                    },
                  ],
                },
              ]
            : []),
        ],
      },
      evidence: [
        {
          requirement: "TODO: 药品范围与稳定来源键",
          tasks: ["MEDICINE_BASE"],
          file: "TODO: 测试文件相对路径",
          contains: "TODO: 可执行测试函数或验收标记",
          rustTest: "TODO::完整::Rust::测试路径",
          execution: "REQUIRED",
        },
        {
          requirement: "TODO: 项目变体兼容",
          tasks: ["MEDICINE_BASE"],
          file: "TODO: 测试文件相对路径",
          contains: "TODO: 可执行测试函数或验收标记",
          rustTest: "TODO::完整::Rust::测试路径",
          execution: "REQUIRED",
        },
        ...(tasks.includes("INVENTORY")
          ? [
              {
                requirement: "TODO: 库房归属与包装金额校验",
                tasks: ["INVENTORY"],
                file: "TODO: 测试文件相对路径",
                contains: "TODO: 可执行测试函数或验收标记",
                rustTest: "TODO::完整::Rust::测试路径",
                execution: "REQUIRED",
              },
              {
                requirement: "TODO: 首次盘点门禁与回滚",
                tasks: ["INVENTORY"],
                file: "TODO: 测试文件相对路径",
                contains: "TODO: 可执行测试函数或验收标记",
                rustTest: "TODO::完整::Rust::测试路径",
                execution: "REQUIRED",
              },
            ]
          : []),
      ],
    };
    await writeFile(
      path.join(packageRoot, "acceptance.json"),
      `${JSON.stringify(acceptance, null, 2)}\n`,
    );
    await writeFile(
      path.join(packageRoot, "IMPLEMENTATION.md"),
      `# ${name} 实现清单\n\n` +
        `1. 先阅读 \`src-tauri/adapters/_examples/standard_his\`，其中包含药品、机构、库房、轻量范围和本批库存明细的可执行示例。\n` +
        `2. 将 \`adapter.rs.template\` 改名为 \`adapter.rs\` 并完成其中的来源读取；查询统一使用 SDK 的 \`read_source_select\`，不要直接依赖 crate 内部数据库模块。\n` +
        `3. 实现统一药品/库存来源契约，不写目标数据库；构建会自动绑定 \`${id}_BUILTIN\`。\n` +
        (tasks.includes("INVENTORY")
          ? `   库存对象请使用模板中的 SDK 构造器示例，必填身份与可选业务字段已分开。\n`
          : "") +
        `4. 替换 \`acceptance.json\` 和 \`fixtures/*.json\` 中所有 TODO。\n` +
        `5. 为每个项目变体增加可执行 Rust 回归测试，在夹具中填写完整测试路径。\n` +
        `6. 为每条 \`acceptance.evidence\` 填写所属任务、真实测试函数、完整 \`rustTest\` 路径和 REQUIRED / OPTIONAL_LIVE 策略，确认每个清单任务均被覆盖，再运行 \`npm run adapter:evidence\`。\n` +
        `7. 开发中运行 \`npm run adapter:status -- --adapter ${id}\` 查看当前阶段；完成后运行 \`npm run adapter:verify\`，确认清单、证据、夹具和测试全部通过。\n`,
    );
    if (tasks.includes("MEDICINE_BASE")) {
      await writeFile(
        path.join(packageRoot, "fixtures", "medicine-project-variant.json"),
        `${JSON.stringify(
          {
            fixtureSchemaVersion: FIXTURE_SCHEMA_VERSION,
            adapterId: id,
            id: "medicine-project-variant",
            title: "TODO: 药品项目结构变体",
            databaseFamily: families[0],
            tasks: ["MEDICINE_BASE"],
            sourceObjects: [
              {
                name: "TODO: 核心药品表",
                columns: [{ name: "TODO: 稳定药品键", dataType: "TODO: 物理类型" }],
              },
            ],
            expectations: {
              outcome: "PASS",
              checks: ["TODO: 可选字段缺失时仍能生成安全读取结果"],
            },
            rustTest: "TODO::完整::Rust::测试路径",
          },
          null,
          2,
        )}\n`,
      );
    }
    if (tasks.includes("INVENTORY")) {
      await writeFile(
        path.join(packageRoot, "fixtures", "inventory-project-variant.json"),
        `${JSON.stringify(
          {
            fixtureSchemaVersion: FIXTURE_SCHEMA_VERSION,
            adapterId: id,
            id: "inventory-project-variant",
            title: "TODO: 库存项目结构变体",
            databaseFamily: families[0],
            tasks: ["INVENTORY"],
            sourceObjects: [
              {
                name: "TODO: 库存表",
                columns: [
                  { name: "TODO: 稳定库存行键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 库房键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 药品商品键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 库存数量", dataType: "TODO: 物理类型" },
                  { name: "TODO: 管理单位", dataType: "TODO: 物理类型" },
                  { name: "TODO: 管理包装系数", dataType: "TODO: 物理类型" },
                  { name: "TODO: 进价", dataType: "TODO: 物理类型" },
                  { name: "TODO: 零售价", dataType: "TODO: 物理类型" },
                ],
              },
              {
                name: "TODO: 机构表",
                columns: [
                  { name: "TODO: 稳定机构键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 机构名称", dataType: "TODO: 物理类型" },
                ],
              },
              {
                name: "TODO: 药库药房表",
                columns: [
                  { name: "TODO: 稳定库房键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 所属机构键", dataType: "TODO: 物理类型" },
                  { name: "TODO: 库房类型", dataType: "TODO: 物理类型" },
                ],
              },
            ],
            expectations: {
              outcome: "BLOCK",
              checks: ["TODO: 缺少核心库存字段时给出可定位错误"],
            },
            rustTest: "TODO::完整::Rust::测试路径",
          },
          null,
          2,
        )}\n`,
      );
    }
    const medicineImplementation = tasks.includes("MEDICINE_BASE")
      ? `\n// inspect 返回的 object_structures 只填写已核对表的字段名和数据库类型；禁止放入注释、SQL、样例值或业务数据。\nimpl MedicineSourceAdapter for VendorSourceAdapter {\n    fn id(&self) -> &'static str {\n        "${id}"\n    }\n\n    fn inspect(&self, _profile: &ConnectionProfile) -> Result<MedicineSourceInspection, String> {\n        Err("TODO: 实现来源结构识别".into())\n    }\n\n    fn load(&self, _request: &LoadMedicineSourceAdapterRequest) -> Result<SourcePreview, String> {\n        Err("TODO: 实现药品来源读取".into())\n    }\n}\n`
      : "";
    const inventoryImplementation = tasks.includes("INVENTORY")
      ? `\n// 公共层会自动补适配器身份并从位置明细汇总数量，不要在厂商代码中重复维护。\n// InventorySourceCatalog::new(organizations, locations).with_message("已读取机构和库房");\n// InventorySourceReadiness::new(schema, source_name, readiness_locations)\n//     .with_medicine_ledger(mapped_count, unresolved_count, unresolved_keys)\n//     .with_guidance(vec![InventorySourceGuidance::info("RELATION", "业务关系", "说明")])\n//     .with_message("已读取库存范围");\n// 推荐的库存行映射写法：只在 new 中填写稳定身份，再按来源中确有的业务字段分组补充。\n// InventorySourceStockItem::new(kind, record_id, location_key, location_name, organization_id, product_key)\n//     .with_medicine(name, specification, dosage_form, minimum_unit)\n//     .with_storage_packaging(sale_unit, sale_specification, unit_sale_factor)\n//     .with_single_minimum_package_evidence(factor, "来源表.字段")\n//     .with_product_packaging(product_sale_unit, product_unit_sale_factor)\n//     .with_product(factory_name, product_name)\n//     .with_quantity_and_prices(amount, price_pur, price_sale)\n//     .with_totals(purchase_total, retail_total)\n//     .with_batch(batch, expiry);\n\nimpl InventorySourceAdapter for VendorSourceAdapter {\n    fn id(&self) -> &'static str {\n        "${id}"\n    }\n\n    fn load_catalog(&self, _profile: &ConnectionProfile) -> Result<InventorySourceCatalog, String> {\n        Err("TODO: 实现机构与库房目录读取".into())\n    }\n\n    fn inspect(\n        &self,\n        _store: &LocalStore,\n        _tenant_id: &str,\n        _request: &InspectInventorySourceAdapterRequest,\n    ) -> Result<InventorySourceReadiness, String> {\n        Err("TODO: 实现轻量库存预检".into())\n    }\n\n    fn load_stock_items(\n        &self,\n        _profile: &ConnectionProfile,\n        _selected_organization_ids: &HashSet<String>,\n        _selected_location_keys: &HashSet<String>,\n    ) -> Result<Vec<InventorySourceStockItem>, String> {\n        Err("TODO: 实现已选范围库存明细读取".into())\n    }\n}\n`
      : "";
    await writeFile(
      path.join(packageRoot, "adapter.rs.template"),
      `use super::sdk::*;\n\n// 所有来源 SQL 通过 read_source_select 执行；公共入口会限制 SELECT/WITH 和最多 10,000 行。\n// Schema/表/视图使用 source_qualified_object(&profile.kind, &profile.schema, "TABLE")?，禁止直接拼接。\n// 本批机构和库房过滤使用 source_text_filter_list(selected_ids, "机构")?，统一限制数量并安全转义。\n// 完整药品与库存实现参考 src-tauri/adapters/_examples/standard_his。\nstruct VendorSourceAdapter;\n${medicineImplementation}${inventoryImplementation}\nstatic ADAPTER: VendorSourceAdapter = VendorSourceAdapter;\n\npub(super) fn binding() -> SourceAdapterBinding {\n    SourceAdapterBinding::new(\n        "${id}_BUILTIN",\n        ${tasks.includes("MEDICINE_BASE") ? "Some(&ADAPTER)" : "None"},\n        ${tasks.includes("INVENTORY") ? "Some(&ADAPTER)" : "None"},\n    )\n}\n`,
    );
  }
  return { packageName, packageRoot, manifest };
}

export async function listAdapterFixtureCases(projectRoot) {
  const adaptersRoot = path.join(projectRoot, "src-tauri", "adapters");
  const entries = (await readdir(adaptersRoot, { withFileTypes: true }))
    .filter((entry) => entry.isDirectory() && !entry.name.startsWith("_"))
    .sort((left, right) => left.name.localeCompare(right.name));
  const cases = [];
  for (const entry of entries) {
    const packageRoot = path.join(adaptersRoot, entry.name);
    const manifest = await readJson(path.join(packageRoot, "manifest.json"));
    if (!manifest.automaticDetection) continue;
    cases.push(...(await readAdapterFixtures(packageRoot, manifest)));
  }
  return cases;
}

export async function listAdapterExampleFixtureCases(projectRoot) {
  const examplesRoot = path.join(
    projectRoot,
    "src-tauri",
    "adapters",
    "_examples",
  );
  let entries = [];
  try {
    entries = (await readdir(examplesRoot, { withFileTypes: true }))
      .filter((entry) => entry.isDirectory())
      .sort((left, right) => left.name.localeCompare(right.name));
  } catch (error) {
    if (error.code === "ENOENT") return [];
    throw error;
  }
  const cases = [];
  for (const entry of entries) {
    const packageRoot = path.join(examplesRoot, entry.name);
    const manifest = await readJson(path.join(packageRoot, "manifest.json"));
    if (!manifest.automaticDetection) continue;
    cases.push(...(await readAdapterFixtures(packageRoot, manifest)));
  }
  return cases;
}

export function fixtureExecutionPassed(status, output) {
  return (
    status === 0 &&
    /running 1 test/.test(output) &&
    /test result: ok\. 1 passed; 0 failed/.test(output)
  );
}

export async function runAdapterFixtureRegression(projectRoot) {
  const reports = await validateAdapterRegistry(projectRoot);
  if (!validationSummary(reports).ok) {
    return {
      ok: false,
      validationReports: reports,
      results: [],
    };
  }
  const cases = await listAdapterFixtureCases(projectRoot);
  const tests = unique(cases.map((item) => `${item.fixture.rustTest}`.trim()));
  const results = tests.map((rustTest) => {
    const executed = spawnSync(
      "cargo",
      ["test", "--lib", rustTest, "--", "--exact"],
      {
        cwd: path.join(projectRoot, "src-tauri"),
        encoding: "utf8",
        env: process.env,
      },
    );
    const output = `${executed.stdout || ""}\n${executed.stderr || ""}`;
    return {
      rustTest,
      ok: fixtureExecutionPassed(executed.status, output),
      status: executed.status,
      stdout: executed.stdout || "",
      stderr: executed.stderr || "",
    };
  });
  return {
    ok: results.every((result) => result.ok),
    validationReports: reports,
    cases,
    results,
  };
}

export async function runAdapterExampleRegression(projectRoot) {
  const validationReports = await validateAdapterExamples(projectRoot);
  if (!validationReports.length || !validationSummary(validationReports).ok) {
    return { ok: false, validationReports, results: [], cases: [] };
  }
  const cases = await listAdapterExampleFixtureCases(projectRoot);
  const tests = unique(cases.map((item) => `${item.fixture.rustTest}`.trim()));
  const results = tests.map((rustTest) => {
    const executed = spawnSync(
      "cargo",
      ["test", "--lib", rustTest, "--", "--exact"],
      {
        cwd: path.join(projectRoot, "src-tauri"),
        encoding: "utf8",
        env: process.env,
      },
    );
    const output = `${executed.stdout || ""}\n${executed.stderr || ""}`;
    return {
      rustTest,
      ok: fixtureExecutionPassed(executed.status, output),
      status: executed.status,
      stdout: executed.stdout || "",
      stderr: executed.stderr || "",
    };
  });
  return {
    ok: results.length > 0 && results.every((result) => result.ok),
    validationReports,
    cases,
    results,
  };
}

async function main() {
  const [command = "validate", ...args] = process.argv.slice(2);
  const options = parseOptions(args);
  const projectRoot = path.resolve(options.root || process.cwd());
  if (command === "create") {
    const created = await createAdapterPackage(projectRoot, options);
    console.log(`已创建 ${created.packageRoot}`);
    console.log("请按 PROJECT_INTAKE.md 完成现场调查，再运行 npm run adapter:status 查看当前阶段");
    return;
  }
  if (command === "status") {
    let reports = await validateAdapterRegistry(projectRoot);
    const requested = `${options.adapter || ""}`.trim().toUpperCase();
    if (requested)
      reports = reports.filter(
        (report) =>
          `${report.id || ""}`.toUpperCase() === requested ||
          `${report.packageName || ""}`.toUpperCase() === requested,
      );
    if (!reports.length)
      throw new Error(requested ? `未找到适配器 ${requested}` : "未找到适配器包");
    console.log(formatAdapterReadiness(reports));
    if (!validationSummary(reports).ok) process.exitCode = 1;
    return;
  }
  if (command === "evidence") {
    const regression = await runAdapterEvidenceRegression(projectRoot);
    if (!validationSummary(regression.validationReports).ok) {
      console.log(formatValidationReport(regression.validationReports));
      process.exitCode = 1;
      return;
    }
    console.log("三方 HIS 业务验收证据回归\n");
    for (const result of regression.results) {
      const optional = result.execution === "OPTIONAL_LIVE" ? "（可选现场）" : "";
      const tasks = result.tasks?.length ? ` [${result.tasks.join("/")}]` : "";
      console.log(`${result.ok ? "✓" : "✗"} ${result.rustTest}${tasks}${optional}`);
      if (!result.ok) console.log(result.stderr || result.stdout);
    }
    console.log(
      `\n合计 ${regression.cases.length} 条业务证据、${regression.results.length} 个唯一测试，${regression.ok ? "全部符合执行策略" : "存在失败"}`,
    );
    if (!regression.ok) process.exitCode = 1;
    return;
  }
  if (command === "fixtures") {
    const regression = await runAdapterFixtureRegression(projectRoot);
    if (!validationSummary(regression.validationReports).ok) {
      console.log(formatValidationReport(regression.validationReports));
      process.exitCode = 1;
      return;
    }
    console.log("三方 HIS 项目变体回归\n");
    for (const result of regression.results) {
      console.log(`${result.ok ? "✓" : "✗"} ${result.rustTest}`);
      if (!result.ok) console.log(result.stderr || result.stdout);
    }
    console.log(
      `\n合计 ${regression.cases.length} 个夹具、${regression.results.length} 个可执行测试，${regression.ok ? "全部通过" : "存在失败"}`,
    );
    if (!regression.ok) process.exitCode = 1;
    return;
  }
  if (command === "examples") {
    const regression = await runAdapterExampleRegression(projectRoot);
    if (!validationSummary(regression.validationReports).ok) {
      console.log(formatValidationReport(regression.validationReports));
      process.exitCode = 1;
      return;
    }
    console.log("三方 HIS 教学适配器验收\n");
    for (const report of regression.validationReports)
      console.log(`✓ ${report.id}（${report.packageName}）`);
    for (const result of regression.results) {
      console.log(`${result.ok ? "✓" : "✗"} ${result.rustTest}`);
      if (!result.ok) console.log(result.stderr || result.stdout);
    }
    console.log(
      `\n合计 ${regression.validationReports.length} 个教学包、${regression.cases.length} 个夹具，${regression.ok ? "全部通过" : "存在失败"}`,
    );
    if (!regression.ok) process.exitCode = 1;
    return;
  }
  if (command === "support") {
    const file = `${options.file || ""}`.trim();
    if (!file)
      throw new Error("缺少 --file，请选择现场导出的适配器支持包 JSON");
    const content = await readFile(path.resolve(projectRoot, file), "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      throw new Error("适配器支持包超过 5 MB，请确认是否选择了正确文件");
    let analysis = analyzeSourceAdapterSupportPackage(content);
    if (analysis.ok) {
      const requestedTasks = normalizedMigrationTasks(options.task);
      const acceptanceFile = options.acceptance
        ? path.resolve(projectRoot, options.acceptance)
        : path.join(
            projectRoot,
            "src-tauri",
            "adapters",
            analysis.summary.adapterId.toLowerCase(),
            "acceptance.json",
          );
      const acceptance = await readJson(acceptanceFile, true);
      analysis = analyzeSourceAdapterSupportPackage(content, acceptance, {
        tasks: requestedTasks,
      });
      if (!acceptance)
        analysis.warnings.push(
          `未找到 ${acceptanceFile}，仅输出结构变化，未进行业务影响分级`,
        );
    }
    console.log(formatSourceAdapterSupportAnalysis(analysis));
    if (!analysis.ok) process.exitCode = 1;
    return;
  }
  if (command === "contract-draft") {
    const file = `${options.file || ""}`.trim();
    if (!file)
      throw new Error("缺少 --file，请选择现场导出的适配器支持包 JSON");
    const supportFile = path.resolve(projectRoot, file);
    const content = await readFile(supportFile, "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      throw new Error("适配器支持包超过 5 MB，请确认是否选择了正确文件");
    const initial = analyzeSourceAdapterSupportPackage(content);
    if (!initial.ok)
      throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
    const acceptanceFile = options.acceptance
      ? path.resolve(projectRoot, options.acceptance)
      : path.join(
          projectRoot,
          "src-tauri",
          "adapters",
          initial.summary.adapterId.toLowerCase(),
          "acceptance.json",
        );
    const acceptance = await readJson(acceptanceFile, true);
    if (!acceptance)
      throw new Error(
        `未找到 ${acceptanceFile}，请使用 --acceptance 指定要比较的 acceptance.json`,
      );
    const draft = buildSourceStructureContractDraft(content, acceptance, {
      tasks: normalizedMigrationTasks(options.task),
    });
    const outputFile = path.resolve(
      projectRoot,
      options.output ||
        `${draft.adapterId.toLowerCase()}-${draft.migrationTask.toLowerCase()}-contract-draft.json`,
    );
    await writeFile(outputFile, `${JSON.stringify(draft, null, 2)}\n`);
    console.log(formatSourceStructureContractDraftSummary(draft, outputFile));
    return;
  }
  if (command === "fixture-draft") {
    const file = `${options.file || ""}`.trim();
    if (!file)
      throw new Error("缺少 --file，请选择现场导出的适配器支持包 JSON");
    const supportFile = path.resolve(projectRoot, file);
    const content = await readFile(supportFile, "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      throw new Error("适配器支持包超过 5 MB，请确认是否选择了正确文件");
    const initial = analyzeSourceAdapterSupportPackage(content);
    if (!initial.ok)
      throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
    const acceptanceFile = options.acceptance
      ? path.resolve(projectRoot, options.acceptance)
      : path.join(
          projectRoot,
          "src-tauri",
          "adapters",
          initial.summary.adapterId.toLowerCase(),
          "acceptance.json",
        );
    const acceptance = await readJson(acceptanceFile, true);
    if (!acceptance)
      throw new Error(
        `未找到 ${acceptanceFile}，请使用 --acceptance 指定要比较的 acceptance.json`,
      );
    const draft = buildAdapterFixtureDraft(content, acceptance, {
      tasks: normalizedMigrationTasks(options.task),
    });
    const outputFile = path.resolve(
      projectRoot,
      options.output || `${draft.adapterId.toLowerCase()}-${draft.id}.draft.json`,
    );
    await writeFile(outputFile, `${JSON.stringify(draft, null, 2)}\n`);
    console.log(formatAdapterFixtureDraftSummary(draft, outputFile));
    return;
  }
  if (command === "handoff") {
    const file = `${options.file || ""}`.trim();
    if (!file)
      throw new Error("缺少 --file，请选择现场导出的适配器支持包 JSON");
    const supportFile = path.resolve(projectRoot, file);
    const content = await readFile(supportFile, "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      throw new Error("适配器支持包超过 5 MB，请确认是否选择了正确文件");
    const initial = analyzeSourceAdapterSupportPackage(content);
    if (!initial.ok)
      throw new Error(`支持包校验失败：${initial.errors.join("；")}`);
    const acceptanceFile = options.acceptance
      ? path.resolve(projectRoot, options.acceptance)
      : path.join(
          projectRoot,
          "src-tauri",
          "adapters",
          initial.summary.adapterId.toLowerCase(),
          "acceptance.json",
        );
    const acceptance = await readJson(acceptanceFile, true);
    if (!acceptance)
      throw new Error(
        `未找到 ${acceptanceFile}，请使用 --acceptance 指定要比较的 acceptance.json`,
      );
    const bundle = buildAdapterMaintenanceHandoff(content, acceptance, {
      tasks: normalizedMigrationTasks(options.task),
    });
    const structureSuffix = bundle.fixtureDraft.id.split("-").at(-1);
    const outputDir = path.resolve(
      projectRoot,
      options.output ||
        path.join(
          "output",
          "adapter-handoffs",
          `${bundle.manifest.adapterId.toLowerCase()}-${bundle.manifest.migrationTask.toLowerCase()}-${structureSuffix}`,
        ),
    );
    await mkdir(path.dirname(outputDir), { recursive: true });
    try {
      await mkdir(outputDir, { recursive: false });
    } catch (error) {
      if (error.code === "EEXIST")
        throw new Error(
          `输出目录已存在：${outputDir}；为避免覆盖既有审核记录，请使用 --output 指定新目录`,
        );
      throw error;
    }
    const contractText = `${JSON.stringify(bundle.contractDraft, null, 2)}\n`;
    const fixtureText = `${JSON.stringify(bundle.fixtureDraft, null, 2)}\n`;
    const planText = bundle.planMarkdown;
    bundle.manifest.artifacts = {
      "contract-draft.json": {
        role: "REVIEW_ONLY_CONTRACT_DRAFT",
        sha256: artifactChecksum(contractText),
      },
      "fixture.draft.json": {
        role: "REVIEW_ONLY_FIXTURE_DRAFT",
        sha256: artifactChecksum(fixtureText),
      },
      "IMPLEMENTATION_PLAN.md": {
        role: "MAINTAINER_IMPLEMENTATION_PLAN",
        sha256: artifactChecksum(planText),
      },
    };
    bundle.manifest.checksum = maintenanceHandoffChecksum(bundle.manifest);
    await writeFile(path.join(outputDir, "contract-draft.json"), contractText);
    await writeFile(path.join(outputDir, "fixture.draft.json"), fixtureText);
    await writeFile(path.join(outputDir, "IMPLEMENTATION_PLAN.md"), planText);
    await writeFile(
      path.join(outputDir, "handoff.json"),
      `${JSON.stringify(bundle.manifest, null, 2)}\n`,
    );
    console.log(formatAdapterMaintenanceHandoffSummary(bundle, outputDir));
    return;
  }
  if (command === "handoff-verify") {
    const directory = `${options.dir || ""}`.trim();
    if (!directory)
      throw new Error("缺少 --dir，请选择 adapter:handoff 生成的交接目录");
    const handoffDir = path.resolve(projectRoot, directory);
    const analysis = await verifyAdapterMaintenanceHandoffDirectory(handoffDir);
    console.log(formatAdapterMaintenanceHandoffVerification(analysis, handoffDir));
    if (!analysis.ok) process.exitCode = 1;
    return;
  }
  if (command === "source-object") {
    const file = `${options.file || ""}`.trim();
    if (!file)
      throw new Error("缺少 --file，请选择现场导出的脱敏结构诊断 JSON");
    const content = await readFile(path.resolve(projectRoot, file), "utf8");
    if (Buffer.byteLength(content, "utf8") > 5_000_000)
      throw new Error("结构诊断超过 5 MB，请确认是否选择了正确文件");
    const analysis = analyzeSourceObjectDiagnostic(content);
    console.log(formatSourceObjectDiagnosticAnalysis(analysis));
    if (!analysis.ok) process.exitCode = 1;
    return;
  }
  if (command !== "validate")
    throw new Error(
      "仅支持 create、status、validate、evidence、fixtures、examples、support、contract-draft、fixture-draft、handoff、handoff-verify 或 source-object 命令",
    );
  const reports = await validateAdapterRegistry(projectRoot);
  console.log(formatValidationReport(reports));
  if (!validationSummary(reports).ok) process.exitCode = 1;
}

if (fileURLToPath(import.meta.url) === path.resolve(process.argv[1] || "")) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
