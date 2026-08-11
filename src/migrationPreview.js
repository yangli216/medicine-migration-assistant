import { dictionaryItemValue, findDictionaryItem } from "./dictionary";

export const medicinePreviewColumns = [
  "DRUG_NAME",
  "SPEC",
  "FORM_CODE",
  "PRE_UNIT",
  "DOSE",
  "DOSE_UNIT",
  "FACTORY_NAME",
  "PRODUCT_NAME",
];

export const medicinePreviewLabels = {
  DRUG_NAME: "药品名称",
  SPEC: "制剂规格",
  FORM_CODE: "剂型",
  PRE_UNIT: "制剂单位",
  DOSE: "制剂剂量",
  DOSE_UNIT: "剂量单位",
  FACTORY_NAME: "生产厂家",
  PRODUCT_NAME: "商品名",
};

export function parseCsv(text) {
  const rows = [];
  let row = [],
    cell = "",
    quoted = false;
  for (let i = 0; i < text.length; i += 1) {
    const char = text[i],
      next = text[i + 1];
    if (char === '"' && quoted && next === '"') {
      cell += '"';
      i += 1;
    } else if (char === '"') quoted = !quoted;
    else if (char === "," && !quoted) {
      row.push(cell);
      cell = "";
    } else if ((char === "\n" || char === "\r") && !quoted) {
      if (char === "\r" && next === "\n") i += 1;
      row.push(cell);
      if (row.some((item) => item !== "")) rows.push(row);
      row = [];
      cell = "";
    } else cell += char;
  }
  row.push(cell);
  if (row.some((item) => item !== "")) rows.push(row);
  const [headers = [], ...data] = rows;
  return data.map((values) =>
    Object.fromEntries(
      headers.map((header, index) => [header.trim(), values[index] ?? ""]),
    ),
  );
}

export function firstPresentValue(row, keys) {
  for (const key of keys) {
    const value = row?.[key];
    if (value !== null && value !== undefined && `${value}`.trim() !== "") {
      return `${value}`;
    }
  }
  return "";
}

export function sourceDictionaryItem(metadata, value) {
  const normalized = `${value ?? ""}`.trim();
  if (!normalized) return null;
  return (
    metadata?.sourceDictionary?.items?.find(
      (item) => `${item.key ?? ""}`.trim() === normalized,
    ) || null
  );
}

export function sourceValueLabel(value, metadata) {
  if (value === null || value === undefined || `${value}`.trim() === "") {
    return "空值";
  }
  const item = sourceDictionaryItem(metadata, value);
  return item?.text ? `${item.text}（${value}）` : `${value}`;
}

const sourceDictionaryPropertyLabels = {
  MRCS: "每日次数",
  ZXSJ: "执行时间",
  ZXZQ: "执行周期",
  RZXZQ: "日执行周期",
  PYDM: "拼音码",
  FYXH: "费用序号",
  SYFW: "适用范围",
};

export function sourceDictionaryPropertySummary(item) {
  return Object.entries(item?.properties || {})
    .filter(([, value]) => `${value ?? ""}`.trim())
    .map(
      ([key, value]) =>
        `${sourceDictionaryPropertyLabels[key] || key}：${value}`,
    )
    .join(" · ");
}

export function sourceFieldDisplayName(column, metadata = {}) {
  const comment = `${metadata.comment || ""}`.trim();
  const physicalColumn = `${metadata.sourceColumn || ""}`.trim();
  return comment || physicalColumn || column;
}

export function sourceFieldPhysicalOrigin(metadata = {}) {
  return [metadata.sourceTable, metadata.sourceColumn]
    .map((part) => `${part || ""}`.trim())
    .filter(Boolean)
    .join(".");
}

export function medicineSampleContext(row, metadata = {}) {
  const formCode = firstPresentValue(row, [
    "FORM_CODE",
    "YPSX",
    "DOSE_FORM",
    "sdDose",
  ]);
  return [
    ["药品名称", firstPresentValue(row, ["DRUG_NAME", "YPMC", "GENERIC_NAME", "naMed"])],
    ["制剂规格", firstPresentValue(row, ["SPEC", "YPGG", "DRUG_SPEC", "spec"])],
    ["剂型", formCode ? sourceValueLabel(formCode, metadata.FORM_CODE) : ""],
    ["制剂单位", firstPresentValue(row, ["PRE_UNIT", "YPDW", "MIN_UNIT", "unitPre"])],
    ["生产厂家", firstPresentValue(row, ["FACTORY_NAME", "CDMC", "MANUFACTURER", "naFac"])],
    ["商品名", firstPresentValue(row, ["PRODUCT_NAME", "YBSPMC", "BRAND_NAME", "naMedPro"])],
  ].filter(([, value]) => value);
}

export function medicineSampleLabel(row, index) {
  const name = firstPresentValue(row, ["DRUG_NAME", "YPMC", "GENERIC_NAME", "naMed"]);
  const spec = firstPresentValue(row, ["SPEC", "YPGG", "DRUG_SPEC", "spec"]);
  const factory = firstPresentValue(row, ["FACTORY_NAME", "CDMC", "MANUFACTURER", "naFac"]);
  const sourceKey = firstPresentValue(row, ["SOURCE_KEY", "_sourceKey"]);
  return [
    `第 ${index + 1} 行`,
    name || "未命名药品",
    spec,
    factory,
    sourceKey ? `来源键 ${sourceKey}` : "",
  ]
    .filter(Boolean)
    .join(" · ");
}

export function randomRowIndex(length, current = -1) {
  if (length <= 1) return 0;
  const offset = 1 + Math.floor(Math.random() * (length - 1));
  return (Math.max(current, 0) + offset) % length;
}

export function inventoryFinancialTotals(rows) {
  return rows.reduce(
    (totals, row) => {
      const amount = Number(row.normalizedData?.amount || 0);
      const pricePur = Number(row.normalizedData?.pricePur || 0);
      const priceSale = Number(row.normalizedData?.priceSale || 0);
      const purchaseTotal = Number(row.normalizedData?.purchaseTotal);
      const retailTotal = Number(row.normalizedData?.retailTotal);
      if (Number.isFinite(purchaseTotal)) {
        totals.purchase += purchaseTotal;
      } else if (Number.isFinite(amount) && Number.isFinite(pricePur)) {
        totals.purchase += amount * pricePur;
      }
      if (Number.isFinite(retailTotal)) {
        totals.retail += retailTotal;
      } else if (Number.isFinite(amount) && Number.isFinite(priceSale)) {
        totals.retail += amount * priceSale;
      }
      return totals;
    },
    { purchase: 0, retail: 0 },
  );
}

export function formatInventoryMoney(value) {
  return new Intl.NumberFormat("zh-CN", {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  }).format(Number.isFinite(value) ? value : 0);
}
