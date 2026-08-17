import assert from "node:assert/strict";
import test from "node:test";
import {
  applySourceDictionaryOverrides,
  normalizeSourceDictionaryItems,
  phis27DictionaryScopeKey,
  updateScopedDictionaryOverride,
} from "../src/sourceDictionaryOverrides.js";

test("二系列来源字典覆盖按数据库项目隔离", () => {
  assert.equal(
    phis27DictionaryScopeKey(
      {
        kind: "Oracle",
        host: "10.19.40.42",
        port: 1521,
        serviceName: "phis",
      },
      "phis27",
    ),
    "oracle:10.19.40.42:1521/phis:PHIS27",
  );
});

test("来源字典覆盖会去空、去重并保留扩展属性", () => {
  assert.deepEqual(
    normalizeSourceDictionaryItems([
      { key: " 3 ", text: " 喹诺酮类 ", properties: { py: "KNT" } },
      { key: "3", text: "重复" },
      { key: "", text: "无效" },
    ]),
    [{ key: "3", text: "喹诺酮类", properties: { py: "KNT" } }],
  );
});

test("应用覆盖时保留预设项用于恢复默认", () => {
  const metadata = applySourceDictionaryOverrides(
    [
      {
        name: "ALLERGY_CODE",
        sourceDictionary: {
          id: "phis.dictionary.gmywlb",
          items: [{ key: "3", text: "喹诺酮" }],
        },
      },
    ],
    {
      "phis.dictionary.gmywlb": [{ key: "3", text: "喹诺酮类" }],
    },
  );
  assert.equal(metadata[0].sourceDictionary.customized, true);
  assert.equal(metadata[0].sourceDictionary.items[0].text, "喹诺酮类");
  assert.equal(metadata[0].sourceDictionary.presetItems[0].text, "喹诺酮");

  const saved = updateScopedDictionaryOverride(
    {},
    "project-a",
    "phis.dictionary.gmywlb",
    metadata[0].sourceDictionary.items,
  );
  assert.equal(saved["project-a"]["phis.dictionary.gmywlb"][0].key, "3");
  assert.deepEqual(
    updateScopedDictionaryOverride(
      saved,
      "project-a",
      "phis.dictionary.gmywlb",
      null,
    ),
    {},
  );
});
