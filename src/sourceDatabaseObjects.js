export function sourceObjectFromSafeQuery(query) {
  const normalized = `${query || ""}`.trim();
  const mysql = normalized.match(/^SELECT\s+\*\s+FROM\s+`((?:``|[^`])+)`$/i);
  if (mysql) return mysql[1].replaceAll("``", "`");
  const quoted = normalized.match(
    /^SELECT\s+\*\s+FROM\s+(?:"(?:""|[^"])+"\.)?"((?:""|[^"])+)"$/i,
  );
  return quoted ? quoted[1].replaceAll('""', '"') : "";
}

const medicineNamePatterns = [
  { pattern: /^(?:YK_TYPK|YK_YPCD)$/i, score: 140, reason: "二系列药品主数据常用表名" },
  { pattern: /^(?:YK_YPXX|YF_YPXX)$/i, score: 105, reason: "二系列药库/药房药品关系常用表名" },
  {
    pattern: /(?:^|_)(?:DRUG|MEDICINE|MEDICATION)(?:_|$)/i,
    score: 100,
    reason: "名称包含药品关键词",
  },
  {
    pattern: /(?:^|_)(?:TYPK|YPCD|YPXX|YPML|YPMC)(?:_|$)/i,
    score: 92,
    reason: "名称包含常见药品缩写",
  },
  { pattern: /药品|药物/u, score: 100, reason: "名称包含中文药品关键词" },
  {
    pattern: /(?:^|_)(?:MED|MEDS)(?:_|$)/i,
    score: 72,
    reason: "名称包含医疗物品缩写",
  },
];

const masterPattern = /(?:^|_)(?:MASTER|CATALOG|DICT|INFO|BASE|PRODUCT)(?:_|$)/i;
const transactionalPattern =
  /(?:^|_)(?:ORDER|LOG|BILL|PRICE|STOCK|INVENTORY|RECORD|DETAIL|HISTORY|TXN)(?:_|$)/i;

export function sourceObjectMedicineClue(objectName) {
  const name = `${objectName || ""}`.trim();
  if (!name) return { score: 0, reason: "" };
  const matched = medicineNamePatterns
    .filter((rule) => rule.pattern.test(name))
    .sort((left, right) => right.score - left.score)[0];
  if (!matched) return { score: 0, reason: "" };
  const masterBonus = masterPattern.test(name) ? 18 : 0;
  const transactionalPenalty = transactionalPattern.test(name) ? 34 : 0;
  return {
    score: Math.max(1, matched.score + masterBonus - transactionalPenalty),
    reason: `${matched.reason}${masterBonus ? "，且带有目录/主数据线索" : ""}${transactionalPenalty ? "；同时带有业务明细线索，请重点核对" : ""}`,
  };
}

export function sourceObjectOptions(objects) {
  return [...new Set((objects || []).map((name) => `${name || ""}`).filter(Boolean))]
    .map((name, index) => ({
      name,
      index,
      ...sourceObjectMedicineClue(name),
    }))
    .sort((left, right) => right.score - left.score || left.index - right.index)
    .map(({ name, score, reason }) => ({
      value: name,
      label: name,
      group: score > 0 ? "可能的药品来源（仅按名称判断）" : "全部可见对象",
      description: score > 0 ? `${reason}；请预览字段后确认` : "当前账号可见的来源表或视图",
      keywords: score > 0 ? `${name} 药品 药物 medicine drug` : name,
      data: { medicineClueScore: score, medicineClueReason: reason },
    }));
}
