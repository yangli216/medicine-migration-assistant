import { dictionaryItemValue } from "./dictionary.js";
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
  const counts = new Map();
  rows.forEach((row) => {
    const sourceValue = sourceValueKey(row[sourceField]);
    counts.set(sourceValue, (counts.get(sourceValue) || 0) + 1);
  });

  return [...counts.entries()]
    .map(([sourceValue, count]) => {
      const sourceIsBlank = sourceValue === EMPTY_VALUE_MAPPING_SOURCE;
      const sourceItem = sourceDictionaryItem(
        columnMetadata[sourceField],
        sourceIsBlank ? null : sourceValue,
      );
      const directTarget = sourceIsBlank
        ? null
        : findDictionaryItem(sourceValue, dictionary.items);
      const semanticTarget = sourceItem?.text
        ? findDictionaryItem(sourceItem.text, dictionary.items)
        : null;
      const suggestedTarget = semanticTarget || directTarget;
      const ignored =
        Object.prototype.hasOwnProperty.call(configured, sourceValue) &&
        configured[sourceValue] === null;
      const fieldRule = rules[field.key] || {};
      const converted = applyFieldRule(sourceIsBlank ? null : sourceValue, {
        ...fieldRule,
        transform: fieldRule.transform || defaultTransformForField(field),
        valueMappings: configured,
      });
      const convertedTarget = dictionary.items.find(
        (item) => dictionaryItemValue(item) === `${converted ?? ""}`,
      );
      const appliedTarget = convertedTarget
        ? dictionaryItemValue(convertedTarget)
        : "";
      return {
        sourceValue,
        sourceIsBlank,
        sourceText: sourceItem?.text || "",
        sourceProperties: sourceDictionaryPropertySummary(sourceItem),
        count,
        appliedTarget,
        suggestedTarget,
        ignored,
        matched: Boolean(appliedTarget) && !ignored,
      };
    })
    .sort(
      (a, b) =>
        Number(a.matched || a.ignored) - Number(b.matched || b.ignored),
    );
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
    const dictionaryPending = Math.max(
      0,
      dictionaryTotal - dictionaryHandled,
    );

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
