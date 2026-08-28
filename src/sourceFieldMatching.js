function normalizeSourceFieldName(value) {
  return `${value || ""}`
    .toUpperCase()
    .replace(/[^A-Z0-9\u4e00-\u9fa5]/g, "");
}

export function sourceFieldAliasMatchScore(value, field) {
  const normalized = normalizeSourceFieldName(value);
  if (!normalized) return 0;
  const exact = field.aliases.find(
    (alias) => normalizeSourceFieldName(alias) === normalized,
  );
  if (exact) return 96;
  const partial = field.aliases.some((alias) => {
    const normalizedAlias = normalizeSourceFieldName(alias);
    const lengthSimilarity =
      Math.min(normalized.length, normalizedAlias.length) /
      Math.max(normalized.length, normalizedAlias.length);
    // SPEC/DOSE/TYPE 等短字段含义过宽，只允许精确命中，避免误配到监管字段。
    return (
      Math.min(normalized.length, normalizedAlias.length) >= 6 &&
      lengthSimilarity >= 0.8 &&
      (normalized.includes(normalizedAlias) ||
        normalizedAlias.includes(normalized))
    );
  });
  if (partial) return 82;
  const key = normalizeSourceFieldName(field.key);
  const keySimilarity =
    Math.min(normalized.length, key.length) /
    Math.max(normalized.length, key.length);
  if (
    Math.min(normalized.length, key.length) >= 6 &&
    keySimilarity >= 0.8 &&
    (normalized.includes(key) || key.includes(normalized))
  )
    return 72;
  return 0;
}

export function sourceFieldMatchScore(column, field, metadata = {}) {
  return Math.max(
    sourceFieldAliasMatchScore(column, field),
    sourceFieldAliasMatchScore(metadata.sourceColumn || "", field),
    sourceFieldAliasMatchScore(metadata.comment || "", field),
  );
}

const medicineClues = [
  ["naMed", "药品名称"],
  ["spec", "规格"],
  ["sdDose", "剂型"],
  ["unitPre", "最小单位"],
  ["naFac", "生产厂家"],
  ["naMedPro", "商品名"],
];

export function sourceObjectSuitability({
  columns = [],
  columnMetadata = [],
  rows = [],
  targetFields = [],
} = {}) {
  const metadataByName = Array.isArray(columnMetadata)
    ? Object.fromEntries(columnMetadata.map((item) => [item.name, item]))
    : columnMetadata || {};
  const targetByKey = Object.fromEntries(
    targetFields.map((field) => [field.key, field]),
  );
  const clues = medicineClues.map(([key, label]) => {
    const field = targetByKey[key];
    const best = field
      ? columns
          .map((column) => ({
            column,
            score: sourceFieldMatchScore(
              column,
              field,
              metadataByName[column],
            ),
          }))
          .sort((left, right) => right.score - left.score)[0]
      : null;
    const column = best?.score >= 55 ? best.column : "";
    const hasSample = column
      ? rows.some(
          (row) =>
            row?.[column] !== null &&
            row?.[column] !== undefined &&
            `${row[column]}`.trim() !== "",
        )
      : false;
    return { key, label, column, hasSample };
  });
  const foundCount = clues.filter((clue) => clue.column).length;
  const hasMedicineName = clues.some(
    (clue) => clue.key === "naMed" && clue.column && clue.hasSample,
  );
  if (!hasMedicineName) {
    return {
      tone: "danger",
      title: "尚未识别药品名称字段",
      description: "建议换一个表/视图，或让数据库人员先整理药品业务视图。",
      clues,
      foundCount,
    };
  }
  if (foundCount >= 4) {
    return {
      tone: "ready",
      title: `已识别 ${foundCount} 项常用药品字段`,
      description: "可以继续读取，后续仍需逐字段核对和数据校验。",
      clues,
      foundCount,
    };
  }
  return {
    tone: "warning",
    title: `仅识别到 ${foundCount} 项常用药品字段`,
    description: "仍可继续，但可能需要较多手工映射或改用已整理的多表视图。",
    clues,
    foundCount,
  };
}
