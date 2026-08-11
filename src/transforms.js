const TRUE_VALUES = new Set([
  "1",
  "true",
  "yes",
  "是",
  "y",
  "on",
  "启用",
  "有",
  "需要",
  "需",
  "有效",
  "正常",
  "rx",
  "处方药",
  "处方药品",
]);
const FALSE_VALUES = new Set([
  "0",
  "2",
  "false",
  "no",
  "否",
  "n",
  "off",
  "停用",
  "无",
  "不需要",
  "无需",
  "无效",
  "otc",
  "非处方药",
  "非处方药品",
]);

export const EMPTY_VALUE_MAPPING_SOURCE = "<空值>";
export const IGNORE_VALUE_MAPPING_TARGET = "<忽略>";

export function parseValueMappings(text = "") {
  const mappings = {};
  const invalidLines = [];
  `${text}`.split(/\r?\n/u).forEach((rawLine, index) => {
    const line = rawLine.trim();
    if (!line || line.startsWith("#")) return;
    const arrowIndex = line.indexOf("=>");
    const equalIndex = line.indexOf("=");
    const tabIndex = line.indexOf("\t");
    const separator = arrowIndex >= 0
      ? { index: arrowIndex, width: 2 }
      : equalIndex >= 0
        ? { index: equalIndex, width: 1 }
        : tabIndex >= 0
          ? { index: tabIndex, width: 1 }
          : null;
    if (!separator) {
      invalidLines.push(index + 1);
      return;
    }
    const source = line.slice(0, separator.index).trim();
    const target = line.slice(separator.index + separator.width).trim();
    if (!source) {
      invalidLines.push(index + 1);
      return;
    }
    mappings[source] = target === IGNORE_VALUE_MAPPING_TARGET ? null : target;
  });
  return { mappings, invalidLines };
}

export function applyFieldRule(value, rule = {}) {
  let next = value;
  let usedEmptyMapping = false;
  const mappings = rule.valueMappings || {};
  if (isBlank(next)) {
    if (
      Object.prototype.hasOwnProperty.call(
        mappings,
        EMPTY_VALUE_MAPPING_SOURCE,
      )
    ) {
      next = mappings[EMPTY_VALUE_MAPPING_SOURCE];
      usedEmptyMapping = true;
    } else if (`${rule.defaultValue ?? ""}`.trim()) {
      next = rule.defaultValue;
    }
  }
  if (isBlank(next)) return null;

  const text = valueText(next);
  if (!usedEmptyMapping && Object.prototype.hasOwnProperty.call(mappings, text)) {
    next = mappings[text];
  } else if (rule.valueMappingCaseInsensitive) {
    const normalized = text.toLocaleLowerCase();
    const matchedKey = Object.keys(mappings).find(
      (key) => key.toLocaleLowerCase() === normalized,
    );
    if (matchedKey !== undefined) next = mappings[matchedKey];
  }

  return transformValue(next, rule.transform || "TRIM");
}

export function transformValue(value, operation = "TRIM") {
  if (isBlank(value)) return null;
  const text = valueText(value);
  switch (`${operation}`.trim().toUpperCase()) {
    case "UPPER":
      return text.toLocaleUpperCase();
    case "LOWER":
      return text.toLocaleLowerCase();
    case "COLLAPSE_WHITESPACE":
      return text.replace(/\s+/gu, " ");
    case "REMOVE_WHITESPACE":
      return text.replace(/\s+/gu, "");
    case "INTEGER": {
      const number = Number(normalizedNumber(text));
      return Number.isSafeInteger(number) ? number : text;
    }
    case "DECIMAL": {
      const number = Number(normalizedNumber(text));
      return Number.isFinite(number) ? number : text;
    }
    case "BOOLEAN_01": {
      const normalized = text.toLocaleLowerCase();
      if (
        FALSE_VALUES.has(normalized) ||
        normalized.includes("非处方") ||
        normalized.includes("otc")
      )
        return "0";
      if (
        TRUE_VALUES.has(normalized) ||
        (normalized.includes("处方") && !normalized.includes("非处方"))
      )
        return "1";
      return text;
    }
    case "DATE_YYYY_MM_DD":
      return normalizeDate(text) || text;
    default:
      return text;
  }
}

function valueText(value) {
  if (value === null || value === undefined) return "";
  if (typeof value === "boolean") return value ? "1" : "0";
  return `${value}`.trim();
}

function isBlank(value) {
  return value === null || value === undefined || `${value}`.trim() === "";
}

function normalizedNumber(text) {
  return text.replace(/[,，\s]/gu, "");
}

function normalizeDate(text) {
  const datePart = text.split(/[ T]/u)[0];
  let matched = datePart.match(/^(\d{4})[-/.](\d{1,2})[-/.](\d{1,2})$/u);
  if (!matched) matched = datePart.match(/^(\d{4})(\d{2})(\d{2})$/u);
  if (!matched) return null;
  const [, year, month, day] = matched;
  const monthNumber = Number(month);
  const dayNumber = Number(day);
  const checked = new Date(Date.UTC(Number(year), monthNumber - 1, dayNumber));
  if (
    checked.getUTCFullYear() !== Number(year) ||
    checked.getUTCMonth() + 1 !== monthNumber ||
    checked.getUTCDate() !== dayNumber
  ) return null;
  return `${year}-${`${monthNumber}`.padStart(2, "0")}-${`${dayNumber}`.padStart(2, "0")}`;
}
