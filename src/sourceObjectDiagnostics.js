function compactText(value, maximumLength = 500) {
  return `${value || ""}`.trim().slice(0, maximumLength);
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

async function sha256(value) {
  if (!globalThis.crypto?.subtle)
    throw new Error("当前运行环境不支持安全校验值，请升级桌面程序后重试");
  const digest = await globalThis.crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(value),
  );
  return [...new Uint8Array(digest)]
    .map((item) => item.toString(16).padStart(2, "0"))
    .join("");
}

async function diagnosticChecksum(value) {
  return sha256(
    JSON.stringify(canonicalJson({ ...value, checksum: "" })),
  );
}

function observedValueKind(value) {
  if (value === null || value === undefined || `${value}`.trim() === "")
    return "empty";
  if (typeof value === "boolean") return "boolean";
  if (typeof value === "number")
    return Number.isInteger(value) ? "integer" : "decimal";
  if (Array.isArray(value)) return "array";
  if (typeof value === "object") return "object";
  const text = `${value}`.trim();
  if (/^-?\d+$/.test(text)) return "integer-text";
  if (/^-?\d+(?:\.\d+)?$/.test(text)) return "decimal-text";
  if (/^\d{4}[-/]\d{1,2}[-/]\d{1,2}(?:[ T].*)?$/.test(text))
    return "date-text";
  return "text";
}

function safeColumnMetadata(column, metadata = {}, rows = []) {
  const observedKinds = [];
  let nonEmptySampleCount = 0;
  let maximumTextLength = 0;
  rows.forEach((row) => {
    const value = row?.[column];
    const kind = observedValueKind(value);
    if (!observedKinds.includes(kind)) observedKinds.push(kind);
    if (kind !== "empty") {
      nonEmptySampleCount += 1;
      maximumTextLength = Math.max(maximumTextLength, `${value}`.length);
    }
  });
  return {
    name: compactText(column, 256),
    comment: compactText(metadata.comment),
    sourceTable: compactText(metadata.sourceTable, 256),
    sourceColumn: compactText(metadata.sourceColumn, 256),
    mappingEligible: metadata.mappingEligible !== false,
    sampleProfile: {
      nonEmptyCount: nonEmptySampleCount,
      maximumTextLength,
      observedKinds,
    },
  };
}

function countProbeSummary(count) {
  if (!count)
    return {
      status: "PENDING",
      message: "行数探测尚未完成；正式读取仍会执行单批上限校验",
    };
  if (count.error)
    return {
      status: "UNAVAILABLE",
      message: "短时行数探测未完成；正式读取仍会执行单批上限校验",
    };
  return {
    status: count.isExact === false ? "LOWER_BOUND" : "EXACT",
    rowCount: Number(count.rowCount || 0),
    probeLimit: Number(count.probeLimit || 0),
    elapsedMs: Number(count.elapsedMs || 0),
  };
}

export async function buildSourceObjectDiagnostic({
  databaseFamily,
  schema,
  objectName,
  preview,
  metadataMessage,
  rowCount,
  suitability,
  generatedAt = new Date().toISOString(),
} = {}) {
  if (!objectName || !preview?.columns?.length)
    throw new Error("请先预览来源表或视图，再导出结构诊断");
  const metadataByName = Object.fromEntries(
    (preview.columnMetadata || []).map((item) => [item.name, item]),
  );
  const rows = Array.isArray(preview.rows) ? preview.rows : [];
  const diagnostic = {
    formatVersion: 1,
    packageType: "GENERIC_SOURCE_OBJECT_DIAGNOSTIC",
    generatedAt,
    source: {
      databaseFamily: compactText(databaseFamily, 64),
      schema: compactText(schema, 256),
      objectName: compactText(objectName, 256),
    },
    preview: {
      columnCount: preview.columns.length,
      sampledRowCount: rows.length,
      truncated: Boolean(preview.truncated),
      elapsedMs: Number(preview.elapsedMs || 0),
      metadataMessage: compactText(metadataMessage),
      rowCountProbe: countProbeSummary(rowCount),
    },
    medicineFieldAssessment: suitability
      ? {
          status: compactText(suitability.tone, 32),
          title: compactText(suitability.title),
          description: compactText(suitability.description),
          foundCount: Number(suitability.foundCount || 0),
          clues: (suitability.clues || []).map((clue) => ({
            key: compactText(clue.key, 64),
            label: compactText(clue.label, 128),
            column: compactText(clue.column, 256),
            hasNonEmptySample: Boolean(clue.hasSample),
          })),
        }
      : null,
    columns: preview.columns.map((column) =>
      safeColumnMetadata(column, metadataByName[column], rows),
    ),
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
  diagnostic.checksum = await diagnosticChecksum(diagnostic);
  return {
    fileName: "generic-source-object-diagnostic.json",
    content: `${JSON.stringify(diagnostic, null, 2)}\n`,
    diagnostic,
  };
}
