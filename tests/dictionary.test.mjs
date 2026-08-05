import test from "node:test";
import assert from "node:assert/strict";
import {
  buildDictionaryValueMappings,
  findDictionaryItem,
  mergeValueMappingText,
  recommendCostMergeMappings,
} from "../src/dictionary.js";

const items = [
  { key: "1", cd: "1", text: "西药", na: "西药", py: "xy" },
  { key: "2", cd: "2", text: "中药", na: "中药", py: "zy" },
];

test("matches target dictionary by code name pinyin and common combined label", () => {
  assert.equal(findDictionaryItem("西药", items)?.key, "1");
  assert.equal(findDictionaryItem("XY", items)?.key, "1");
  assert.equal(findDictionaryItem("1-西药", items)?.key, "1");
  assert.equal(findDictionaryItem("西药（1）", items)?.key, "1");
  assert.equal(
    findDictionaryItem("xy", [...items, { key: "9", text: "其它", py: "xy" }]),
    null,
  );
});

test("recommends medicine cost merge by target medicine type", () => {
  const costs = [
    { key: "cost-west", text: "西药费", active: true },
    { key: "cost-herb", text: "草药费", active: true },
    { key: "cost-material", text: "卫生材料费", active: true },
  ];
  assert.deepEqual(
    recommendCostMergeMappings(
      [
        { key: "1", text: "西药" },
        { key: "3", text: "草药" },
        { key: "5", text: "医用耗材" },
        { key: "6", text: "固定资产" },
      ],
      costs,
    ),
    { "1": "cost-west", "3": "cost-herb", "5": "cost-material" },
  );
});

test("builds only safe exact dictionary mappings and keeps manual rules", () => {
  assert.deepEqual(buildDictionaryValueMappings(["西药", "2", "不确定"], items), {
    西药: "1",
  });
  assert.equal(
    mergeValueMappingText("旧西药 = 1", { 西药: "1", 旧西药: "2" }),
    "旧西药 = 1\n西药 = 1",
  );
});
