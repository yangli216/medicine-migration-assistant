import {
  EMPTY_VALUE_MAPPING_SOURCE,
  IGNORE_VALUE_MAPPING_TARGET,
} from "./transforms.js";

function normalized(value) {
  return `${value ?? ""}`
    .normalize("NFKC")
    .trim()
    .replace(/\s+/gu, " ")
    .toLocaleLowerCase();
}

export function dictionaryItemValue(item = {}) {
  return `${item.key || item.cd || ""}`.trim();
}

export function findDictionaryItem(value, items = []) {
  const source = normalized(value);
  if (!source) return null;
  const matches = items.filter((item) => {
    const key = dictionaryItemValue(item);
    const name = `${item.text || item.na || ""}`.trim();
    const aliases = [
      key,
      item.cd,
      item.text,
      item.na,
      item.py,
      item.wb,
      key && name ? `${key}-${name}` : "",
      key && name ? `${key} ${name}` : "",
      key && name ? `${name}(${key})` : "",
      key && name ? `${name}（${key}）` : "",
    ];
    return aliases.some((alias) => alias && normalized(alias) === source);
  });
  return matches.length === 1 ? matches[0] : null;
}

function booleanMeaning(value) {
  const text = normalized(value);
  if (!text) return null;
  if (
    ["0", "2", "false", "no", "否", "n", "off", "停用", "无", "不需要", "无需", "无效", "otc", "非处方药", "非处方药品"].includes(text) ||
    text.includes("非处方") ||
    text.includes("otc")
  )
    return false;
  if (
    ["1", "true", "yes", "是", "y", "on", "启用", "有", "需要", "需", "有效", "正常", "rx", "处方药", "处方药品"].includes(text) ||
    (text.includes("处方") && !text.includes("非处方"))
  )
    return true;
  return null;
}

function findBooleanDictionaryItem(meaning, items = []) {
  if (meaning === null) return null;
  const matches = items.filter((item) => {
    const label = `${item.text || item.na || ""}`.trim();
    const itemMeaning = booleanMeaning(label || dictionaryItemValue(item));
    return itemMeaning === meaning;
  });
  return matches.length === 1 ? matches[0] : null;
}

export function buildDictionaryValueMappings(
  values = [],
  items = [],
  sourceItems = [],
) {
  const mappings = {};
  values.forEach((rawValue) => {
    const source = `${rawValue ?? ""}`.trim();
    if (!source || Object.prototype.hasOwnProperty.call(mappings, source)) return;
    const sourceItem = findDictionaryItem(source, sourceItems);
    const sourceMeaning = sourceItem?.text || sourceItem?.na || source;
    const item =
      findDictionaryItem(sourceMeaning, items) ||
      findBooleanDictionaryItem(booleanMeaning(sourceMeaning), items);
    const target = item ? dictionaryItemValue(item) : "";
    if (target && source !== target) mappings[source] = target;
  });
  return mappings;
}

export function mergeValueMappingText(text = "", additions = {}) {
  const lines = `${text}`
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean);
  const existingSources = new Set(
    lines.map((line) => normalized(line.split(/=>|=|\t/u, 1)[0])),
  );
  Object.entries(additions).forEach(([source, target]) => {
    if (!existingSources.has(normalized(source))) {
      lines.push(`${source} = ${target}`);
      existingSources.add(normalized(source));
    }
  });
  return lines.join("\n");
}

export function replaceValueMappingText(
  text = "",
  sourceValue = "",
  targetValue = "",
) {
  const source =
    `${sourceValue ?? ""}`.trim() || EMPTY_VALUE_MAPPING_SOURCE;
  const lines = `${text}`
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => {
      const [lineSource] = line.split(/=>|=|\t/u, 1);
      return normalized(lineSource) !== normalized(source);
    });
  const target = `${targetValue}`.trim();
  if (target && target !== source) lines.push(`${source} = ${target}`);
  return lines.join("\n");
}

const PHIS27_PRESET_VALUE_MAPPINGS = {
  fgMedRx: {
    sourceField: "RX_FLAG",
    mappings: { 0: "2" },
  },
  sdChrgitmLv: {
    sourceField: "INSURANCE_LEVEL",
    mappings: { 1: "01", 2: "02", 3: "03" },
  },
  sdAllergy: {
    sourceField: "ALLERGY_CODE",
    mappings: { 0: "" },
  },
  sdStorage: {
    sourceField: "STORAGE_CODE",
    mappings: { 0: "" },
  },
  sdRound: {
    sourceField: "ROUND_CODE",
    mappings: { 0: "1", 1: "2", 2: "3" },
  },
};

const STANDARD_USAGE_TEXT_BY_CODE = {
  1: "口服",
  2: "直肠给药",
  3: "舌下给药",
  4: "注射给药",
  401: "皮下注射",
  402: "皮内注射",
  403: "肌肉注射",
  404: "静脉注射",
  405: "静脉滴注",
  5: "吸入给药",
  6: "局部用药",
  605: "阴道用药",
  607: "滴眼",
  608: "滴鼻",
  610: "含化",
};

const USAGE_SEMANTIC_GROUPS = [
  ["口服", "内服", "口服给药"],
  ["直肠给药", "直肠用药", "肛门给药"],
  ["舌下给药", "舌下含服", "舌下"],
  ["静脉注射", "静注"],
  ["静脉滴注", "静滴", "静脉输注", "静脉点滴"],
  ["肌肉注射", "肌内注射", "肌注"],
  ["皮下注射", "皮下注"],
  ["皮内注射", "皮内注"],
  ["吸入给药", "吸入", "雾化吸入"],
  ["局部用药", "局部给药", "外用"],
  ["阴道用药", "阴道给药"],
  ["滴眼", "眼用"],
  ["滴鼻", "鼻用"],
  ["含化", "口含", "含服"],
];

function usageSemanticKey(value) {
  const text = normalized(value).replace(/[\s()（）]/gu, "");
  if (!text || /\d/u.test(text)) return "";
  const groupIndex = USAGE_SEMANTIC_GROUPS.findIndex((group) =>
    group.some((alias) => normalized(alias).replace(/[\s()（）]/gu, "") === text),
  );
  return groupIndex >= 0 ? `usage-${groupIndex}` : text;
}

function buildUsageValueMappings(values = [], items = [], sourceItems = []) {
  const mappings = buildDictionaryValueMappings(values, items, sourceItems);
  values.forEach((rawValue) => {
    const source = `${rawValue ?? ""}`.trim();
    if (!source || Object.prototype.hasOwnProperty.call(mappings, source)) return;
    const sourceItem = findDictionaryItem(source, sourceItems);
    if (!sourceItem) return;
    const meanings = [
      sourceItem.text,
      sourceItem.na,
      STANDARD_USAGE_TEXT_BY_CODE[`${sourceItem.properties?.BZYF ?? ""}`.trim()],
    ].filter(Boolean);
    const semanticKeys = new Set(meanings.map(usageSemanticKey).filter(Boolean));
    const matches = items.filter((item) =>
      semanticKeys.has(usageSemanticKey(item.text || item.na)),
    );
    if (matches.length !== 1) return;
    const target = dictionaryItemValue(matches[0]);
    if (target && target !== source) mappings[source] = target;
  });
  return mappings;
}

export function buildPhis27PresetRules({
  rows = [],
  mapping = {},
  columnMetadata = {},
  targetFields = [],
  dictionariesById = {},
  rules = {},
} = {}) {
  const result = Object.fromEntries(
    Object.entries(rules).map(([key, value]) => [key, { ...value }]),
  );

  Object.entries(PHIS27_PRESET_VALUE_MAPPINGS).forEach(
    ([targetField, preset]) => {
      if (mapping[targetField] !== preset.sourceField) return;
      result[targetField] = {
        ...result[targetField],
        valueMappingsText: mergeValueMappingText(
          result[targetField]?.valueMappingsText,
          preset.mappings,
        ),
      };
    },
  );

  if (mapping.fgMedRx === "RX_FLAG") {
    let valueMappingsText = result.fgMedRx?.valueMappingsText || "";
    valueMappingsText = replaceValueMappingText(valueMappingsText, "0", "2");
    valueMappingsText = replaceValueMappingText(valueMappingsText, "1", "1");
    valueMappingsText = replaceValueMappingText(valueMappingsText, "2", "2");
    result.fgMedRx = {
      ...result.fgMedRx,
      transform: "TRIM",
      valueMappingsText,
    };
  }

  targetFields.forEach((field) => {
    const sourceField = mapping[field.key];
    const dictionary = field.dictionaryId
      ? dictionariesById[field.dictionaryId]
      : null;
    if (!sourceField || !dictionary?.items?.length) return;
    const sourceValues = rows.map((row) => row[sourceField]);
    const sourceItems =
      columnMetadata[sourceField]?.sourceDictionary?.items || [];
    const additions =
      field.key === "dftUsage"
        ? buildUsageValueMappings(sourceValues, dictionary.items, sourceItems)
        : buildDictionaryValueMappings(sourceValues, dictionary.items, sourceItems);
    if (
      field.key === "dftUsage" &&
      sourceValues.some((value) => `${value ?? ""}`.trim() === "9")
    ) {
      additions["9"] = IGNORE_VALUE_MAPPING_TARGET;
    }
    if (!Object.keys(additions).length) return;
    result[field.key] = {
      ...result[field.key],
      valueMappingsText: mergeValueMappingText(
        result[field.key]?.valueMappingsText,
        additions,
      ),
    };
  });

  return result;
}

export function recommendCostMergeMappings(articleItems = [], costItems = []) {
  const preferredName = (articleText) => {
    if (articleText.includes("西药")) return "西药费";
    if (articleText.includes("草药")) return "草药费";
    if (articleText.includes("疫苗")) return "疫苗费";
    if (articleText.includes("耗材") || articleText.includes("材料"))
      return "卫生材料费";
    if (
      articleText.includes("中药") ||
      articleText.includes("民族药") ||
      articleText.includes("院内制剂")
    )
      return "成药费";
    return "";
  };
  return Object.fromEntries(
    articleItems.flatMap((article) => {
      const articleKey = dictionaryItemValue(article);
      const costName = preferredName(`${article.text || article.na || ""}`);
      const matches = costItems.filter(
        (cost) => cost.active !== false && `${cost.text || ""}`.trim() === costName,
      );
      return articleKey && matches.length === 1
        ? [[articleKey, `${matches[0].key || ""}`.trim()]]
        : [];
    }),
  );
}
