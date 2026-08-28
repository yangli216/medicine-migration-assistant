export function sourceDictionaryScopeKey(profile = {}, schema = "") {
  const kind = `${profile.kind || "oracle"}`.trim().toLowerCase();
  const host = `${profile.host || ""}`.trim().toLowerCase();
  const port = `${profile.port || ""}`.trim();
  const service = `${profile.serviceName || profile.database || ""}`
    .trim()
    .toLowerCase();
  return `${kind}:${host}:${port}/${service}:${`${schema}`.trim().toUpperCase()}`;
}

export const phis27DictionaryScopeKey = sourceDictionaryScopeKey;

export function normalizeSourceDictionaryItems(items = []) {
  const seen = new Set();
  return items.flatMap((item) => {
    const key = `${item?.key ?? ""}`.trim();
    const text = `${item?.text ?? ""}`.trim();
    if (!key || !text || seen.has(key)) return [];
    seen.add(key);
    return [
      {
        ...item,
        key,
        text,
        properties:
          item?.properties && typeof item.properties === "object"
            ? item.properties
            : {},
      },
    ];
  });
}

export function applySourceDictionaryOverrides(
  columnMetadata = [],
  scopedOverrides = {},
) {
  return columnMetadata.map((metadata) => {
    const dictionary = metadata.sourceDictionary;
    if (!dictionary) return metadata;
    const presetItems = normalizeSourceDictionaryItems(dictionary.items || []);
    const override = scopedOverrides[dictionary.id];
    if (!Array.isArray(override)) {
      return {
        ...metadata,
        sourceDictionary: {
          ...dictionary,
          items: presetItems,
          presetItems,
          originalLoadStatus: dictionary.loadStatus,
          originalLoadMessage: dictionary.loadMessage,
          customized: false,
        },
      };
    }
    const items = normalizeSourceDictionaryItems(override);
    return {
      ...metadata,
        sourceDictionary: {
          ...dictionary,
          items,
          presetItems,
          originalLoadStatus: dictionary.loadStatus,
          originalLoadMessage: dictionary.loadMessage,
          customized: true,
        loadStatus: "customized",
        loadMessage: `当前项目已人工调整 ${items.length} 个来源字典项`,
      },
    };
  });
}

export function updateScopedDictionaryOverride(
  allOverrides = {},
  scopeKey,
  dictionaryId,
  items,
) {
  const next = { ...allOverrides };
  const scoped = { ...(next[scopeKey] || {}) };
  if (items === null) {
    delete scoped[dictionaryId];
  } else {
    scoped[dictionaryId] = normalizeSourceDictionaryItems(items);
  }
  if (Object.keys(scoped).length) next[scopeKey] = scoped;
  else delete next[scopeKey];
  return next;
}
