const MISSING_LEDGER_TEXT = "缺少完整的 id_med/id_med_pro 基础迁移台账";

export function normalizeMedicineMatchText(value) {
  return `${value ?? ""}`
    .normalize("NFKC")
    .toLocaleLowerCase("zh-CN")
    .replace(/[×✕＊*]/g, "x")
    .replace(/[μµ]/g, "u")
    .replace(/[（【［]/g, "(")
    .replace(/[）】］]/g, ")")
    .replace(/[\s·•,，。.;；:：/\\_\-()]/g, "");
}

function bigrams(value) {
  const normalized = normalizeMedicineMatchText(value);
  if (!normalized) return [];
  if (normalized.length === 1) return [normalized];
  return Array.from({ length: normalized.length - 1 }, (_, index) =>
    normalized.slice(index, index + 2),
  );
}

export function medicineTextSimilarity(left, right) {
  const normalizedLeft = normalizeMedicineMatchText(left);
  const normalizedRight = normalizeMedicineMatchText(right);
  if (!normalizedLeft || !normalizedRight) return 0;
  if (normalizedLeft === normalizedRight) return 1;
  const leftPairs = bigrams(normalizedLeft);
  const rightPairs = bigrams(normalizedRight);
  const remaining = new Map();
  rightPairs.forEach((pair) => remaining.set(pair, (remaining.get(pair) || 0) + 1));
  let overlap = 0;
  leftPairs.forEach((pair) => {
    if ((remaining.get(pair) || 0) > 0) {
      overlap += 1;
      remaining.set(pair, remaining.get(pair) - 1);
    }
  });
  const dice = (2 * overlap) / (leftPairs.length + rightPairs.length);
  const containment =
    normalizedLeft.includes(normalizedRight) || normalizedRight.includes(normalizedLeft)
      ? Math.min(normalizedLeft.length, normalizedRight.length) /
        Math.max(normalizedLeft.length, normalizedRight.length)
      : 0;
  return Math.max(dice, containment * 0.92);
}

export function inventoryUnmatchedMedicines(rows = []) {
  const unmatched = new Map();
  rows.forEach((row) => {
    if (
      !`${row.errorMessage || ""}`.includes(MISSING_LEDGER_TEXT) ||
      (row.idMed && row.idMedPro)
    )
      return;
    const sourceProductKey = `${row.rawData?.sourceProductKey || ""}`.trim();
    const targetOrganizationId = `${row.normalizedData?.idOrg || ""}`.trim();
    if (!sourceProductKey || !targetOrganizationId) return;
    const key = `${sourceProductKey}::${targetOrganizationId}`;
    if (!unmatched.has(key)) {
      unmatched.set(key, {
        key,
        sourceProductKey,
        targetOrganizationId,
        drugName: row.rawData?.drugName || "",
        specification: row.rawData?.specification || "",
        factoryName: row.rawData?.factoryName || "",
        productName: row.rawData?.productName || "",
        rowCount: 0,
      });
    }
    unmatched.get(key).rowCount += 1;
  });
  return [...unmatched.values()];
}

function scoreCandidate(source, target) {
  const name = Math.max(
    medicineTextSimilarity(source.drugName, target.drugName),
    medicineTextSimilarity(source.drugName, target.productName),
    medicineTextSimilarity(source.productName, target.productName),
  );
  const specification = Math.max(
    medicineTextSimilarity(source.specification, target.specification),
    medicineTextSimilarity(source.specification, target.saleSpecification),
  );
  const factory = medicineTextSimilarity(source.factoryName, target.factoryName);
  const score = name * 0.5 + specification * 0.3 + factory * 0.2;
  const nameExact =
    Boolean(normalizeMedicineMatchText(source.drugName)) && name === 1;
  const specificationExact =
    Boolean(normalizeMedicineMatchText(source.specification)) && specification === 1;
  const factoryExact =
    Boolean(normalizeMedicineMatchText(source.factoryName)) && factory === 1;
  const exact = nameExact && specificationExact && factoryExact;
  const reasons = [
    nameExact ? "名称一致" : `名称 ${Math.round(name * 100)}%`,
    specificationExact ? "规格一致" : `规格 ${Math.round(specification * 100)}%`,
    factoryExact ? "厂家一致" : `厂家 ${Math.round(factory * 100)}%`,
  ];
  return {
    target,
    score,
    scorePercent: Math.round(score * 100),
    exact,
    reason: reasons.join(" · "),
  };
}

export function recommendInventoryMedicineMatches(
  sources = [],
  targetMedicines = [],
  limit = 8,
) {
  return sources.map((source) => {
    const eligible = targetMedicines.filter(
      (target) =>
        !target.private ||
        `${target.organizationId || ""}` === source.targetOrganizationId,
    );
    const candidates = eligible
      .map((target) => scoreCandidate(source, target))
      .sort(
        (left, right) =>
          Number(right.exact) - Number(left.exact) ||
          right.score - left.score ||
          `${left.target.drugName}`.localeCompare(`${right.target.drugName}`, "zh-CN"),
      )
      .slice(0, limit);
    const exactCandidates = candidates.filter((candidate) => candidate.exact);
    return {
      ...source,
      candidates,
      autoCandidate:
        exactCandidates.length === 1 ? exactCandidates[0].target : null,
      confidence:
        exactCandidates.length === 1
          ? "EXACT"
          : candidates[0]?.score >= 0.78
            ? "HIGH"
            : candidates[0]?.score >= 0.55
              ? "MEDIUM"
              : "LOW",
    };
  });
}

