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

export function buildDictionaryValueMappings(values = [], items = []) {
  const mappings = {};
  values.forEach((rawValue) => {
    const source = `${rawValue ?? ""}`.trim();
    if (!source || Object.prototype.hasOwnProperty.call(mappings, source)) return;
    const item = findDictionaryItem(source, items);
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
