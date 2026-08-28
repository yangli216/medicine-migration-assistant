function normalizedKeyValue(value) {
  if (value === null || value === undefined) return { empty: true, value: "" };
  if (typeof value === "object")
    return { empty: false, unsupported: true, value: "" };
  if (typeof value === "number" && !Number.isFinite(value))
    return { empty: false, unsupported: true, value: "" };
  const text = `${value}`.trim();
  return { empty: !text, unsupported: false, value: text };
}

export const MAX_SOURCE_KEY_FIELDS = 3;

export function normalizeSourceKeyFields(fields = [], legacySourceKey = "") {
  const candidates = Array.isArray(fields) ? fields : [];
  const normalized = [];
  [...candidates, ...(candidates.length ? [] : [legacySourceKey])].forEach(
    (field) => {
      const value = `${field || ""}`.trim();
      if (value && !normalized.includes(value)) normalized.push(value);
    },
  );
  return normalized.slice(0, MAX_SOURCE_KEY_FIELDS);
}

export function encodeSourceKey(row, fields = []) {
  const normalizedFields = normalizeSourceKeyFields(fields);
  if (!normalizedFields.length) return "";
  const values = normalizedFields.map((field) =>
    normalizedKeyValue(row?.[field]),
  );
  if (values.some((value) => value.empty || value.unsupported)) return "";
  const scalars = values.map((value) => value.value);
  return scalars.length === 1 ? scalars[0] : JSON.stringify(scalars);
}

export function assessSourceKeyFields({ rows = [], fields = [] } = {}) {
  const normalizedFields = normalizeSourceKeyFields(fields);
  const distinct = new Set();
  let emptyCount = 0;
  let unsupportedCount = 0;
  let duplicateCount = 0;
  rows.forEach((row) => {
    const values = normalizedFields.map((field) =>
      normalizedKeyValue(row?.[field]),
    );
    if (!normalizedFields.length || values.some((value) => value.empty)) {
      emptyCount += 1;
      return;
    }
    if (values.some((value) => value.unsupported)) {
      unsupportedCount += 1;
      return;
    }
    const encoded =
      values.length === 1
        ? values[0].value
        : JSON.stringify(values.map((value) => value.value));
    if (distinct.has(encoded)) duplicateCount += 1;
    else distinct.add(encoded);
  });
  return {
    fields: normalizedFields,
    label: normalizedFields.join("＋"),
    totalCount: rows.length,
    distinctCount: distinct.size,
    emptyCount,
    unsupportedCount,
    duplicateCount,
    completeUnique:
      normalizedFields.length > 0 &&
      rows.length > 0 &&
      emptyCount === 0 &&
      unsupportedCount === 0 &&
      duplicateCount === 0 &&
      distinct.size === rows.length,
  };
}

function identifierHint(column, metadata = {}) {
  const name = `${column || ""}`.toUpperCase();
  const normalized = name.replace(/[^A-Z0-9一-龥]+/g, "_");
  const comment = `${metadata.comment || ""}`.trim();
  const evidence = `${normalized} ${comment}`;
  let score = 0;
  const reasons = [];
  if (
    /(^|_)(SOURCE_KEY|PRIMARY_KEY|ID|KEY|CODE|PK|UUID|GUID)(_|$)/.test(
      normalized,
    )
  ) {
    score += 90;
    reasons.push("字段名包含主键/编码线索");
  } else if (/(YPXH|YPLSH|XH|BM|BH|DM|ID|KEY|CODE)$/.test(normalized)) {
    score += 68;
    reasons.push("字段名带有常见编号后缀");
  }
  if (/(主键|唯一|编码|编号|序号|流水号)/.test(comment)) {
    score += 55;
    reasons.push("字段注释包含稳定编号线索");
  }
  if (
    /(NAME|MC|SPEC|GG|PRICE|JG|AMOUNT|JE|DATE|TIME|RQ|SJ|UNIT|DW|FACTORY|CJ|FLAG|BZ|STATUS|ZT)/.test(
      evidence,
    ) ||
    /(名称|规格|价格|金额|日期|时间|单位|厂家|状态|标志)/.test(comment)
  ) {
    score -= 70;
    reasons.push("名称或注释更像可变业务属性");
  }
  return { score, reason: reasons.join("；") || "没有明确的稳定键命名线索" };
}

export function assessSourceKeyColumns({
  rows = [],
  columns = [],
  columnMetadata = {},
} = {}) {
  const metadataByName = Array.isArray(columnMetadata)
    ? Object.fromEntries(columnMetadata.map((item) => [item.name, item]))
    : columnMetadata || {};
  const assessments = columns.map((column, originalIndex) => {
    const distinct = new Set();
    let emptyCount = 0;
    let duplicateCount = 0;
    let unsupportedCount = 0;
    rows.forEach((row) => {
      const normalized = normalizedKeyValue(row?.[column]);
      if (normalized.empty) {
        emptyCount += 1;
        return;
      }
      if (normalized.unsupported) {
        unsupportedCount += 1;
        return;
      }
      if (distinct.has(normalized.value)) duplicateCount += 1;
      else distinct.add(normalized.value);
    });
    const completeUnique =
      rows.length > 0 &&
      emptyCount === 0 &&
      duplicateCount === 0 &&
      unsupportedCount === 0 &&
      distinct.size === rows.length;
    const hint = identifierHint(column, metadataByName[column]);
    return {
      column,
      originalIndex,
      totalCount: rows.length,
      distinctCount: distinct.size,
      emptyCount,
      duplicateCount,
      unsupportedCount,
      completeUnique,
      hintScore: hint.score,
      hintReason: hint.reason,
      comment: `${metadataByName[column]?.comment || ""}`.trim(),
    };
  });
  const ranked = [...assessments].sort(
    (left, right) =>
      Number(right.completeUnique) - Number(left.completeUnique) ||
      right.hintScore - left.hintScore ||
      left.originalIndex - right.originalIndex,
  );
  const recommended = ranked.find(
    (assessment) => assessment.completeUnique && assessment.hintScore > 0,
  );
  return {
    assessments: ranked,
    recommendedColumn: recommended?.column || "",
    qualifiedCount: ranked.filter((assessment) => assessment.completeUnique)
      .length,
    totalRows: rows.length,
  };
}

function candidateCombinations(
  values,
  size,
  start = 0,
  selected = [],
  result = [],
) {
  if (selected.length === size) {
    result.push([...selected]);
    return result;
  }
  for (let index = start; index < values.length; index += 1) {
    selected.push(values[index]);
    candidateCombinations(values, size, index + 1, selected, result);
    selected.pop();
  }
  return result;
}

export function recommendCompositeSourceKey({ rows = [], review } = {}) {
  if (review?.recommendedColumn) return [review.recommendedColumn];
  const candidates = (review?.assessments || [])
    .filter(
      (assessment) =>
        assessment.emptyCount === 0 &&
        assessment.unsupportedCount === 0 &&
        assessment.hintScore > 0,
    )
    .sort(
      (left, right) =>
        right.hintScore - left.hintScore ||
        right.distinctCount - left.distinctCount ||
        left.originalIndex - right.originalIndex,
    )
    .slice(0, 12)
    .map((assessment) => assessment.column);
  for (const size of [2, 3]) {
    const matches = candidateCombinations(candidates, size)
      .map((fields) => ({
        fields,
        result: assessSourceKeyFields({ rows, fields }),
      }))
      .filter((candidate) => candidate.result.completeUnique);
    if (matches.length) return matches[0].fields;
  }
  return [];
}

export function chooseInitialSourceKeyFields({
  rows = [],
  review,
  preferredFields = [],
  legacySourceKey = "",
} = {}) {
  const preferred = normalizeSourceKeyFields(preferredFields, legacySourceKey);
  if (assessSourceKeyFields({ rows, fields: preferred }).completeUnique)
    return preferred;
  return recommendCompositeSourceKey({ rows, review });
}

export function chooseInitialSourceKey(review, preferred = "") {
  const preferredAssessment = review?.assessments?.find(
    (assessment) => assessment.column === preferred,
  );
  if (preferredAssessment?.completeUnique) return preferred;
  return review?.recommendedColumn || "";
}

export function selectedSourceKeyAssessment(review, sourceKey) {
  return (
    review?.assessments?.find(
      (assessment) => assessment.column === sourceKey,
    ) || null
  );
}

export function sourceKeyOptions(review) {
  return (review?.assessments || []).map((assessment) => {
    const invalidReasons = [
      assessment.emptyCount ? `${assessment.emptyCount} 行为空` : "",
      assessment.duplicateCount ? `${assessment.duplicateCount} 行重复` : "",
      assessment.unsupportedCount
        ? `${assessment.unsupportedCount} 行为数组或对象`
        : "",
    ].filter(Boolean);
    const recommended = assessment.column === review.recommendedColumn;
    return {
      value: assessment.column,
      label: assessment.column,
      group: recommended
        ? "系统推荐（仍需确认业务稳定性）"
        : assessment.completeUnique
          ? "当前批完整且唯一"
          : "不可作为唯一标识",
      description: assessment.completeUnique
        ? `${assessment.totalCount} 行均非空且唯一；${assessment.hintReason}${assessment.comment ? `；注释：${assessment.comment}` : ""}`
        : invalidReasons.join("；") || "当前批无法证明完整唯一",
      keywords: `${assessment.column} ${assessment.comment} ${assessment.hintReason}`,
      disabled: !assessment.completeUnique,
      data: assessment,
    };
  });
}

export function sourceKeyComponentOptions(
  review,
  selectedFields = [],
  current = "",
) {
  const selected = new Set(
    normalizeSourceKeyFields(selectedFields).filter(
      (field) => field !== current,
    ),
  );
  return (review?.assessments || [])
    .map((assessment) => {
      const unavailable =
        assessment.emptyCount > 0 || assessment.unsupportedCount > 0;
      const reasons = [
        assessment.emptyCount ? `${assessment.emptyCount} 行为空` : "",
        assessment.unsupportedCount
          ? `${assessment.unsupportedCount} 行为数组或对象`
          : "",
        selected.has(assessment.column) ? "已用于其他组合位置" : "",
      ].filter(Boolean);
      return {
        value: assessment.column,
        label: assessment.column,
        group:
          assessment.hintScore > 0
            ? "稳定编号候选"
            : unavailable
              ? "不可用于来源键"
              : "其他完整字段",
        description:
          reasons.join("；") ||
          `${assessment.distinctCount} 个不同值；${assessment.hintReason}`,
        keywords: `${assessment.column} ${assessment.comment} ${assessment.hintReason}`,
        disabled: unavailable || selected.has(assessment.column),
        data: assessment,
      };
    })
    .sort((left, right) => {
      const groupRank = (option) =>
        option.group === "稳定编号候选"
          ? 0
          : option.group === "其他完整字段"
            ? 1
            : 2;
      return (
        groupRank(left) - groupRank(right) ||
        Number(left.disabled) - Number(right.disabled) ||
        right.data.hintScore - left.data.hintScore ||
        right.data.distinctCount - left.data.distinctCount ||
        left.data.originalIndex - right.data.originalIndex
      );
    });
}
