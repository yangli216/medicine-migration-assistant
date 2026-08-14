import test from "node:test";
import assert from "node:assert/strict";
import {
  buildDictionaryValueMappings,
  buildPhis27PresetRules,
  findDictionaryItem,
  findDictionarySemanticMatch,
  rankDictionarySemanticMatches,
  mergeValueMappingText,
  replaceValueMappingText,
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
    { 1: "cost-west", 3: "cost-herb", 5: "cost-material" },
  );
});

test("builds only safe exact dictionary mappings and keeps manual rules", () => {
  assert.deepEqual(
    buildDictionaryValueMappings(["西药", "2", "不确定"], items),
    {
      西药: "1",
    },
  );
  assert.equal(
    mergeValueMappingText("旧西药 = 1", { 西药: "1", 旧西药: "2" }),
    "旧西药 = 1\n西药 = 1",
  );
});

test("maps a legacy dictionary code through its semantic text", () => {
  assert.deepEqual(
    buildDictionaryValueMappings(
      ["9"],
      [{ key: "VACCINE", text: "疫苗" }],
      [{ key: "9", text: "疫苗" }],
    ),
    { 9: "VACCINE" },
  );
});

test("never treats a shared code as a shared meaning", () => {
  assert.deepEqual(
    buildDictionaryValueMappings(
      ["25"],
      [{ key: "25", text: "滴剂" }],
      [{ key: "25", text: "粉剂" }],
    ),
    {},
  );
  assert.deepEqual(
    buildDictionaryValueMappings(
      ["24"],
      [{ key: "25", text: "滴剂" }],
      [{ key: "24", text: "滴剂" }],
    ),
    { 24: "25" },
  );
});

test("matches safe medical synonyms without relying on shared codes", () => {
  const targetItems = [
    { key: "25", text: "胶囊剂" },
    { key: "402", text: "静脉滴注" },
    { key: "QD", text: "每日一次" },
    { key: "2", text: "病区发药" },
  ];
  assert.deepEqual(
    buildDictionaryValueMappings(["A", "B", "C", "D"], targetItems, [
      { key: "A", text: "胶囊" },
      { key: "B", text: "静滴" },
      { key: "C", text: "每天一次" },
      { key: "D", text: "住院发药" },
    ]),
    { A: "25", B: "402", C: "QD", D: "2" },
  );
  assert.equal(findDictionarySemanticMatch("静滴", targetItems)?.score, 94);
  assert.equal(
    findDictionarySemanticMatch("处方药品（RX）", [
      { key: "1", text: "处方药品" },
    ])?.reason,
    "忽略编码或缩写注释后一致",
  );
});

test("rejects ambiguous semantic suggestions", () => {
  assert.equal(
    findDictionarySemanticMatch("每天一次", [
      { key: "A", text: "每日一次" },
      { key: "B", text: "一日一次" },
    ]),
    null,
  );
});

test("ranks semantic candidates by confidence and preserves dictionary order for ties", () => {
  const ranked = rankDictionarySemanticMatches("胶囊", [
    { key: "A", text: "胶囊剂" },
    { key: "B", text: "胶囊" },
    { key: "C", text: "胶囊剂" },
    { key: "D", text: "注射剂" },
  ]);
  assert.deepEqual(
    ranked.map(({ item, score }) => [item.key, score]),
    [
      ["B", 100],
      ["A", 94],
      ["C", 94],
    ],
  );
});

test("matches one meaning inside a compound target dictionary label", () => {
  const targetItems = [
    { key: "62", text: "气雾剂,水雾剂" },
    { key: "63", text: "吸入粉雾剂" },
  ];
  const ranked = rankDictionarySemanticMatches("气雾剂", targetItems);
  assert.deepEqual(
    ranked.map(({ item, score, reason }) => [item.key, score, reason]),
    [["62", 97, "目标复合含义包含来源名称"]],
  );
  assert.equal(
    findDictionarySemanticMatch("气雾剂", targetItems)?.item.key,
    "62",
  );
});

test("keeps fuzzy and ambiguous meanings as review-only candidates", () => {
  const fuzzy = rankDictionarySemanticMatches("外用散剂", [
    { key: "A", text: "散剂" },
  ]);
  assert.equal(fuzzy[0]?.item.key, "A");
  assert.ok(fuzzy[0]?.score >= 65 && fuzzy[0]?.score <= 89);
  assert.equal(
    findDictionarySemanticMatch("外用散剂", [{ key: "A", text: "散剂" }]),
    null,
  );

  const ambiguous = [
    { key: "A", text: "气雾剂,水雾剂" },
    { key: "B", text: "气雾剂,喷雾剂" },
  ];
  assert.equal(findDictionarySemanticMatch("气雾剂", ambiguous), null);
  assert.deepEqual(
    rankDictionarySemanticMatches("气雾剂", ambiguous).map(
      ({ item }) => item.key,
    ),
    ["A", "B"],
  );
});

test("maps common legacy 1/2 and RX/OTC flags to target 1/0 semantics", () => {
  const sourceItems = [
    { key: "1", text: "处方药品（RX）" },
    { key: "2", text: "非处方药品（OTC）" },
  ];
  const targetItems = [
    { key: "1", text: "处方药" },
    { key: "0", text: "非处方药" },
  ];
  assert.deepEqual(
    buildDictionaryValueMappings(["1", "2"], targetItems, sourceItems),
    { 1: "1", 2: "0" },
  );
});

test("stores an explicit same-code confirmation", () => {
  assert.equal(replaceValueMappingText("", "25", "25"), "25 = 25");
});

test("builds safe PHIS27 preset mappings for fixed and dynamic dictionaries", () => {
  const mapping = {
    fgMedRx: "RX_FLAG",
    sdChrgitmLv: "INSURANCE_LEVEL",
    sdAllergy: "ALLERGY_CODE",
    sdStorage: "STORAGE_CODE",
    sdRound: "ROUND_CODE",
    dftUsage: "USAGE_CODE",
    dftFreq: "FREQ_CODE",
  };
  const targetFields = [
    ["sdChrgitmLv", "phis.medicareLevel"],
    ["fgMedRx", "rbmh.base.med.prescriptiondrugIdentification"],
    ["sdAllergy", "rbmh.base.med.sdAllergy"],
    ["sdStorage", "phis.storageType"],
    ["sdRound", "rbmh.base.med.roundingStrategy"],
    ["dftUsage", "rbmh.base.med.usage"],
    ["dftFreq", "rbmh.base.freq"],
  ].map(([key, dictionaryId]) => ({ key, dictionaryId }));
  const dictionariesById = {
    "rbmh.base.med.prescriptiondrugIdentification": {
      items: [
        { key: "1", text: "处方药" },
        { key: "2", text: "非处方药" },
      ],
    },
    "phis.medicareLevel": {
      items: [
        ["01", "甲类"],
        ["02", "乙类"],
        ["03", "丙类"],
      ].map(([key, text]) => ({ key, text })),
    },
    "rbmh.base.med.sdAllergy": { items: [{ key: "1", text: "青霉素" }] },
    "phis.storageType": { items: [{ key: "1", text: "常温" }] },
    "rbmh.base.med.roundingStrategy": {
      items: [
        ["1", "每次发药数量取整"],
        ["2", "每天发药数量取整"],
        ["3", "不取整"],
      ].map(([key, text]) => ({ key, text })),
    },
    "rbmh.base.med.usage": {
      items: [
        { key: "100", text: "口服" },
        { key: "402", text: "静脉滴注" },
      ],
    },
    "rbmh.base.freq": { items: [{ key: "QD", text: "每日一次" }] },
  };
  const columnMetadata = {
    USAGE_CODE: {
      sourceDictionary: {
        items: [
          { key: "1", text: "口服", properties: { BZYF: "1" } },
          { key: "2", text: "静滴", properties: { BZYF: "405" } },
        ],
      },
    },
    FREQ_CODE: {
      sourceDictionary: { items: [{ key: "qd", text: "每日一次" }] },
    },
  };
  const rules = buildPhis27PresetRules({
    rows: [
      {
        RX_FLAG: "2",
        INSURANCE_LEVEL: "1",
        ALLERGY_CODE: "0",
        STORAGE_CODE: "0",
        ROUND_CODE: "0",
        USAGE_CODE: "1",
        FREQ_CODE: "qd",
      },
      { RX_FLAG: "1", USAGE_CODE: "2" },
      { USAGE_CODE: "9" },
    ],
    mapping,
    columnMetadata,
    targetFields,
    dictionariesById,
    rules: {
      fgMedRx: {
        transform: "BOOLEAN_01",
        valueMappingsText: "2 = 0",
      },
    },
  });
  assert.equal(rules.fgMedRx.transform, "TRIM");
  assert.doesNotMatch(rules.fgMedRx.valueMappingsText, /2 = 0/);
  assert.match(rules.sdChrgitmLv.valueMappingsText, /1 = 01/);
  assert.match(rules.sdAllergy.valueMappingsText, /0 =\s*$/m);
  assert.match(rules.sdStorage.valueMappingsText, /0 =\s*$/m);
  assert.match(rules.sdRound.valueMappingsText, /0 = 1/);
  assert.match(rules.dftUsage.valueMappingsText, /1 = 100/);
  assert.match(rules.dftUsage.valueMappingsText, /2 = 402/);
  assert.match(rules.dftUsage.valueMappingsText, /9 = <忽略>/);
  assert.match(rules.dftFreq.valueMappingsText, /qd = QD/);
});

test("replaces or clears one visual dictionary mapping", () => {
  assert.equal(
    replaceValueMappingText("1 = A\n2 = B", "1", "C"),
    "2 = B\n1 = C",
  );
  assert.equal(
    replaceValueMappingText("1 = A\n2 = B", "1", "1"),
    "2 = B\n1 = 1",
  );
  assert.equal(
    replaceValueMappingText("1 = A", null, "UNKNOWN"),
    "1 = A\n<空值> = UNKNOWN",
  );
  assert.equal(
    replaceValueMappingText("1 = A\n<空值> = UNKNOWN", "", ""),
    "1 = A",
  );
});
