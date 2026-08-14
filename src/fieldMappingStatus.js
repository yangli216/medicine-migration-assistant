import {
  dictionaryItemValue,
  findDictionarySemanticMatch,
  rankDictionarySemanticMatches,
  rankDictionaryTargetsBySimilarity,
} from "./dictionary.js";
import { defaultTransformForField } from "./migrationFields.js";
import {
  applyFieldRule,
  EMPTY_VALUE_MAPPING_SOURCE,
  parseValueMappings,
} from "./transforms.js";

function sourceValueKey(value) {
  const normalized = `${value ?? ""}`.trim();
  return normalized || EMPTY_VALUE_MAPPING_SOURCE;
}

const sourceValueBucketsCache = new WeakMap();

function sourceValueBuckets(rows, sourceField) {
  let bucketsByField = sourceValueBucketsCache.get(rows);
  if (!bucketsByField) {
    bucketsByField = new Map();
    sourceValueBucketsCache.set(rows, bucketsByField);
  }
  if (bucketsByField.has(sourceField)) return bucketsByField.get(sourceField);
  const buckets = new Map();
  rows.forEach((row, rowIndex) => {
    const sourceValue = sourceValueKey(row[sourceField]);
    if (!buckets.has(sourceValue)) {
      buckets.set(sourceValue, { count: 0, medicines: [] });
    }
    const bucket = buckets.get(sourceValue);
    bucket.count += 1;
    bucket.medicines.push({ row, rowIndex });
  });
  bucketsByField.set(sourceField, buckets);
  return buckets;
}

export function dictionaryRowsForField({
  columnMetadata,
  dictionary,
  field,
  mapping,
  rows,
  rules,
  sourceDictionaryItem,
  sourceDictionaryPropertySummary,
  findDictionaryItem,
}) {
  const sourceField = mapping[field.key];
  if (!dictionary || !sourceField) return [];
  const configured = parseValueMappings(
    rules[field.key]?.valueMappingsText,
  ).mappings;
  const buckets = sourceValueBuckets(rows, sourceField);

  return [...buckets.entries()].map(([sourceValue, bucket]) => {
    const sourceIsBlank = sourceValue === EMPTY_VALUE_MAPPING_SOURCE;
    const sourceItem = sourceDictionaryItem(
      columnMetadata[sourceField],
      sourceIsBlank ? null : sourceValue,
    );
    const sourceMeaning = sourceItem?.text || sourceItem?.na || "";
    const semanticMatch = sourceMeaning
      ? findDictionarySemanticMatch(sourceMeaning, dictionary.items)
      : null;
    const semanticCandidates = sourceMeaning
      ? rankDictionarySemanticMatches(sourceMeaning, dictionary.items)
      : [];
    const targetSimilarityRanking = sourceMeaning
      ? rankDictionaryTargetsBySimilarity(sourceMeaning, dictionary.items)
      : dictionary.items.map((item, sourceOrder) => ({
          item,
          score: 0,
          reason: "",
          sourceOrder,
        }));
    const semanticTarget = semanticMatch?.item || null;
    const suggestedTarget = semanticTarget;
    const explicitlyConfigured = Object.prototype.hasOwnProperty.call(
      configured,
      sourceValue,
    );
    const ignored = explicitlyConfigured && configured[sourceValue] === null;
    const fieldRule = rules[field.key] || {};
    const converted = applyFieldRule(sourceIsBlank ? null : sourceValue, {
      ...fieldRule,
      transform: fieldRule.transform || defaultTransformForField(field),
      valueMappings: configured,
    });
    const convertedTarget = dictionary.items.find(
      (item) => dictionaryItemValue(item) === `${converted ?? ""}`,
    );
    const sameCodeAndMeaning =
      !sourceIsBlank &&
      semanticTarget &&
      dictionaryItemValue(semanticTarget) === sourceValue;
    const appliedTarget =
      convertedTarget && (explicitlyConfigured || sameCodeAndMeaning)
        ? dictionaryItemValue(convertedTarget)
        : "";
    return {
      sourceValue,
      sourceIsBlank,
      sourceText: sourceMeaning,
      sourceProperties: sourceDictionaryPropertySummary(sourceItem),
      count: bucket.count,
      medicines: bucket.medicines,
      appliedTarget,
      suggestedTarget,
      suggestionConfidence: semanticMatch?.score || 0,
      suggestionReason: semanticMatch?.reason || "",
      suggestedCandidates: semanticCandidates.map((candidate) => ({
        target: candidate.item,
        confidence: candidate.score,
        reason: candidate.reason,
      })),
      targetSimilarityRanking,
      ignored,
      matched: Boolean(appliedTarget) && !ignored,
    };
  });
}

export function buildFieldMappingStatuses({
  columnMetadata,
  dictionariesById,
  fields,
  findDictionaryItem,
  mapping,
  rows,
  rules,
  sourceDictionaryItem,
  sourceDictionaryPropertySummary,
}) {
  return fields.map((field, index) => {
    const sourceField = mapping[field.key] || "";
    const hasDefault = Boolean(rules[field.key]?.defaultValue);
    const configured = Boolean(sourceField || hasDefault);
    const dictionary = field.dictionaryId
      ? dictionariesById[field.dictionaryId]
      : null;
    const dictionaryRows = dictionaryRowsForField({
      columnMetadata,
      dictionary,
      field,
      findDictionaryItem,
      mapping,
      rows,
      rules,
      sourceDictionaryItem,
      sourceDictionaryPropertySummary,
    });
    const dictionaryHandled = dictionaryRows.filter(
      (item) => item.matched || item.ignored,
    ).length;
    const dictionaryTotal = dictionaryRows.length;
    const dictionaryPending = Math.max(0, dictionaryTotal - dictionaryHandled);

    let state = "ready";
    let label = "已完成";
    if (!configured) {
      state = field.required ? "pending" : "optional";
      label = field.required ? "待匹配" : "未配置";
    } else if (dictionaryPending > 0) {
      state = "dictionary";
      label = `字典待确认 ${dictionaryPending}`;
    }

    return {
      field,
      index,
      sourceField,
      hasDefault,
      configured,
      dictionary,
      dictionaryRows,
      dictionaryHandled,
      dictionaryTotal,
      dictionaryPending,
      state,
      label,
    };
  });
}
