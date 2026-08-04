import test from "node:test";
import assert from "node:assert/strict";
import {
  applyFieldRule,
  parseValueMappings,
  transformValue,
} from "../src/transforms.js";

test("parses visual value mapping rules", () => {
  assert.deepEqual(parseValueMappings("西药 = 1\nCAP => 胶囊\n# 注释"), {
    mappings: { 西药: "1", CAP: "胶囊" },
    invalidLines: [],
  });
  assert.deepEqual(parseValueMappings("缺少分隔符").invalidLines, [1]);
});

test("applies dictionary before case and whitespace transforms", () => {
  assert.equal(
    applyFieldRule(" cap ", {
      transform: "UPPER",
      valueMappings: { CAP: "capsule form" },
      valueMappingCaseInsensitive: true,
    }),
    "CAPSULE FORM",
  );
  assert.equal(transformValue("  阿莫西林   胶囊 ", "COLLAPSE_WHITESPACE"), "阿莫西林 胶囊");
});

test("normalizes common numeric boolean and date values safely", () => {
  assert.equal(transformValue("1,024.0", "INTEGER"), 1024);
  assert.equal(transformValue("2026/8/4 12:30:00", "DATE_YYYY_MM_DD"), "2026-08-04");
  assert.equal(transformValue("是", "BOOLEAN_01"), "1");
  assert.equal(transformValue("未知", "BOOLEAN_01"), "未知");
});
