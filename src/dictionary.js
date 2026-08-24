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

export function findDictionaryItemByMeaning(value, items = []) {
  return findDictionarySemanticMatch(value, items)?.item || null;
}

const SAFE_MEDICAL_SEMANTIC_GROUPS = [
  ["胶囊", "胶囊剂"],
  ["片", "片剂"],
  ["颗粒", "颗粒剂"],
  ["滴丸", "滴丸剂"],
  ["软膏", "软膏剂"],
  ["乳膏", "乳膏剂"],
  ["凝胶", "凝胶剂"],
  ["喷雾", "喷雾剂"],
  ["气雾", "气雾剂"],
  ["贴剂", "贴膏", "贴膏剂"],
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
  ["每日一次", "每天一次", "一日一次", "1日1次", "qd"],
  ["每日两次", "每天两次", "一日两次", "1日2次", "bid"],
  ["每日三次", "每天三次", "一日三次", "1日3次", "tid"],
  ["每日四次", "每天四次", "一日四次", "1日4次", "qid"],
  ["隔日一次", "隔天一次", "qod"],
  ["必要时", "需要时", "按需", "prn"],
  ["睡前", "临睡前", "hs"],
  ["饭前", "餐前", "ac"],
  ["饭后", "餐后", "pc"],
  ["上午", "早晨", "早上", "am"],
  ["下午", "晚上", "pm"],
  ["常温", "室温", "常温保存", "室温保存"],
  ["冷藏", "冷藏保存"],
  ["冷冻", "冷冻保存"],
  ["甲", "甲类"],
  ["乙", "乙类"],
  ["丙", "丙类"],
  ["处方药", "处方药品", "rx"],
  ["非处方药", "非处方药品", "otc"],
  ["病区发药", "住院发药"],
  ["门诊发药", "门诊药房发药"],
];

function compactMeaning(value) {
  return normalized(value).replace(
    /[\s,，.。·、/\\_\-—:：;；()（）\[\]【】]/gu,
    "",
  );
}

const COMPOUND_MEANING_SEPARATOR = /[,，、/\\|;；]+/gu;

function meaningSegments(value) {
  const source = normalized(value);
  if (!source) return [];
  const segments = new Set();
  const add = (segment) => {
    const compact = compactMeaning(segment);
    if (compact) segments.add(compact);
  };
  add(source);
  source.split(COMPOUND_MEANING_SEPARATOR).forEach((part) => {
    add(part);
    add(part.replace(/[（(][^()（）]+[）)]/gu, ""));
    for (const match of part.matchAll(/[（(]([^()（）]+)[）)]/gu)) {
      match[1].split(COMPOUND_MEANING_SEPARATOR).forEach(add);
    }
  });
  return [...segments];
}

function levenshteinDistance(left, right) {
  const previous = Array.from(
    { length: right.length + 1 },
    (_, index) => index,
  );
  for (let leftIndex = 1; leftIndex <= left.length; leftIndex += 1) {
    const current = [leftIndex];
    for (let rightIndex = 1; rightIndex <= right.length; rightIndex += 1) {
      current[rightIndex] = Math.min(
        current[rightIndex - 1] + 1,
        previous[rightIndex] + 1,
        previous[rightIndex - 1] +
          (left[leftIndex - 1] === right[rightIndex - 1] ? 0 : 1),
      );
    }
    previous.splice(0, previous.length, ...current);
  }
  return previous[right.length];
}

function bigrams(value) {
  if (value.length < 2) return [];
  return Array.from({ length: value.length - 1 }, (_, index) =>
    value.slice(index, index + 2),
  );
}

function fuzzyMeaningSimilarity(left, right) {
  if (!left || !right || left === right) return left === right ? 1 : 0;
  const shorter = left.length <= right.length ? left : right;
  const longer = left.length > right.length ? left : right;
  if (shorter.length < 2) return 0;
  const containment = longer.includes(shorter)
    ? 0.78 + 0.12 * (shorter.length / longer.length)
    : 0;
  const leftBigrams = bigrams(left);
  const rightBigrams = bigrams(right);
  const rightPool = [...rightBigrams];
  let intersection = 0;
  leftBigrams.forEach((part) => {
    const index = rightPool.indexOf(part);
    if (index >= 0) {
      intersection += 1;
      rightPool.splice(index, 1);
    }
  });
  const dice =
    leftBigrams.length + rightBigrams.length
      ? (2 * intersection) / (leftBigrams.length + rightBigrams.length)
      : 0;
  if (!containment && intersection === 0) return 0;
  const editSimilarity =
    1 - levenshteinDistance(left, right) / Math.max(left.length, right.length);
  return Math.max(containment, dice, editSimilarity);
}

function meaningWithoutCodeQualifier(value) {
  return compactMeaning(
    `${value ?? ""}`.replace(/[（(][a-z\d\s/_-]+[）)]/giu, ""),
  );
}

function semanticGroupKey(value) {
  const source = compactMeaning(value);
  if (!source) return "";
  const index = SAFE_MEDICAL_SEMANTIC_GROUPS.findIndex((group) =>
    group.some((alias) => compactMeaning(alias) === source),
  );
  return index >= 0 ? `medical-${index}` : "";
}

const dictionarySemanticRankingCache = new WeakMap();

function cachedDictionarySemanticRanking(value, items) {
  if (!Array.isArray(items) || !items.length) return [];
  const source = normalized(value);
  if (!source) {
    return items.map((item, sourceOrder) => ({
      item,
      score: 0,
      reason: "",
      sourceOrder,
    }));
  }
  let rankingsBySource = dictionarySemanticRankingCache.get(items);
  if (!rankingsBySource) {
    rankingsBySource = new Map();
    dictionarySemanticRankingCache.set(items, rankingsBySource);
  }
  if (rankingsBySource.has(source)) return rankingsBySource.get(source);

  const sourceCompact = compactMeaning(value);
  const sourceWithoutQualifier = meaningWithoutCodeQualifier(value);
  const sourceGroup = semanticGroupKey(value);
  const sourceSegments = meaningSegments(value);
  const ranking = items
    .map((item, sourceOrder) => {
      const meanings = [...new Set([item.text, item.na].filter(Boolean))];
      let best = { item, score: 0, reason: "", sourceOrder };
      meanings.forEach((meaning) => {
        let score = 0;
        let reason = "";
        if (normalized(meaning) === source) {
          score = 100;
          reason = "中文含义一致";
        } else if (compactMeaning(meaning) === sourceCompact) {
          score = 98;
          reason = "忽略空格和标点后一致";
        } else if (
          sourceWithoutQualifier &&
          meaningWithoutCodeQualifier(meaning) === sourceWithoutQualifier
        ) {
          score = 96;
          reason = "忽略编码或缩写注释后一致";
        } else if (sourceGroup && semanticGroupKey(meaning) === sourceGroup) {
          score = 94;
          reason = "常用医学同义表达";
        } else {
          const targetSegments = meaningSegments(meaning);
          const compoundExact = sourceSegments.some((sourceSegment) =>
            targetSegments.some(
              (targetSegment) =>
                sourceSegment === targetSegment &&
                (sourceSegments.length > 1 || targetSegments.length > 1),
            ),
          );
          if (compoundExact) {
            score = 97;
            reason = "目标复合含义包含来源名称";
          } else {
            let bestSimilarity = 0;
            sourceSegments.forEach((sourceSegment) => {
              targetSegments.forEach((targetSegment) => {
                bestSimilarity = Math.max(
                  bestSimilarity,
                  fuzzyMeaningSimilarity(sourceSegment, targetSegment),
                );
              });
            });
            if (bestSimilarity > 0) {
              score = Math.min(89, Math.round(bestSimilarity * 100));
              reason =
                bestSimilarity >= 0.78
                  ? "名称存在包含或近似关系，需人工确认"
                  : bestSimilarity >= 0.65
                    ? "名称相似，需人工确认"
                    : "弱相似，仅用于候选排序";
            }
          }
        }
        if (score > best.score) best = { item, score, reason, sourceOrder };
      });
      return best;
    })
    .sort(
      (left, right) =>
        right.score - left.score || left.sourceOrder - right.sourceOrder,
    );
  rankingsBySource.set(source, ranking);
  return ranking;
}

export function findDictionarySemanticMatch(value, items = []) {
  const candidates = rankDictionarySemanticMatches(value, items);
  if (!candidates.length) return null;
  const best = candidates[0];
  const second = candidates[1];
  const hasSafeMargin = !second || best.score - second.score >= 8;
  return best.score >= 94 && hasSafeMargin ? best : null;
}

export function rankDictionarySemanticMatches(value, items = []) {
  return cachedDictionarySemanticRanking(value, items).filter(
    (candidate) => candidate.score >= 65,
  );
}

export function rankDictionaryTargetsBySimilarity(value, items = []) {
  return cachedDictionarySemanticRanking(value, items);
}

function booleanMeaning(value) {
  const text = normalized(value);
  if (!text) return null;
  if (
    [
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
    ].includes(text) ||
    text.includes("非处方") ||
    text.includes("otc")
  )
    return false;
  if (
    [
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
    ].includes(text) ||
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
    if (!source || Object.prototype.hasOwnProperty.call(mappings, source))
      return;
    const sourceItem = findDictionaryItem(source, sourceItems);
    const sourceMeaning = sourceItem?.text || sourceItem?.na || source;
    const item =
      findDictionarySemanticMatch(sourceMeaning, items)?.item ||
      findBooleanDictionaryItem(booleanMeaning(sourceMeaning), items);
    const target = item ? dictionaryItemValue(item) : "";
    // Keep same-code mappings as explicit semantic confirmations. A shared key alone
    // is not evidence that two independently maintained dictionaries mean the same thing.
    if (target) mappings[source] = target;
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
  const source = `${sourceValue ?? ""}`.trim() || EMPTY_VALUE_MAPPING_SOURCE;
  const lines = `${text}`
    .split(/\r?\n/u)
    .map((line) => line.trim())
    .filter(Boolean)
    .filter((line) => {
      const [lineSource] = line.split(/=>|=|\t/u, 1);
      return normalized(lineSource) !== normalized(source);
    });
  const target = `${targetValue}`.trim();
  if (target) lines.push(`${source} = ${target}`);
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
    group.some(
      (alias) => normalized(alias).replace(/[\s()（）]/gu, "") === text,
    ),
  );
  return groupIndex >= 0 ? `usage-${groupIndex}` : text;
}

function buildUsageValueMappings(values = [], items = [], sourceItems = []) {
  const mappings = buildDictionaryValueMappings(values, items, sourceItems);
  values.forEach((rawValue) => {
    const source = `${rawValue ?? ""}`.trim();
    if (!source || Object.prototype.hasOwnProperty.call(mappings, source))
      return;
    const sourceItem = findDictionaryItem(source, sourceItems);
    if (!sourceItem) return;
    const meanings = [
      sourceItem.text,
      sourceItem.na,
      STANDARD_USAGE_TEXT_BY_CODE[
        `${sourceItem.properties?.BZYF ?? ""}`.trim()
      ],
    ].filter(Boolean);
    const semanticKeys = new Set(
      meanings.map(usageSemanticKey).filter(Boolean),
    );
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
        : buildDictionaryValueMappings(
            sourceValues,
            dictionary.items,
            sourceItems,
          );
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
        (cost) =>
          cost.active !== false && `${cost.text || ""}`.trim() === costName,
      );
      return articleKey && matches.length === 1
        ? [[articleKey, `${matches[0].key || ""}`.trim()]]
        : [];
    }),
  );
}

export function restoreCostMergeMappings(
  articleItems = [],
  costItems = [],
  savedMappings = null,
) {
  const recommendations = recommendCostMergeMappings(articleItems, costItems);
  const availableCostIds = new Set(
    costItems
      .filter((cost) => cost.active !== false)
      .map((cost) => `${cost.key || ""}`.trim())
      .filter(Boolean),
  );
  const mappings = {};
  let restoredCount = 0;
  let staleCount = 0;
  articleItems.forEach((article) => {
    const articleKey = dictionaryItemValue(article);
    if (!articleKey) return;
    if (
      savedMappings &&
      Object.prototype.hasOwnProperty.call(savedMappings, articleKey)
    ) {
      const savedCostId = `${savedMappings[articleKey] ?? ""}`.trim();
      restoredCount += 1;
      if (!savedCostId || availableCostIds.has(savedCostId)) {
        mappings[articleKey] = savedCostId;
      } else {
        mappings[articleKey] = "";
        staleCount += 1;
      }
      return;
    }
    if (Object.prototype.hasOwnProperty.call(recommendations, articleKey)) {
      mappings[articleKey] = recommendations[articleKey];
    }
  });
  return { mappings, restoredCount, staleCount };
}
