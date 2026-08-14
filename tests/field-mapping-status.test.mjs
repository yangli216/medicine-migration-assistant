import assert from "node:assert/strict";
import test from "node:test";
import { buildFieldMappingStatuses } from "../src/fieldMappingStatus.js";
import { findDictionaryItem } from "../src/dictionary.js";

const dictionary = {
  dicId: "dose",
  items: [
    { key: "2", text: "胶囊剂" },
    { key: "3", text: "注射剂" },
  ],
};
const field = {
  key: "sdDose",
  label: "剂型编码",
  group: "基本信息",
  required: true,
  dictionaryId: "dose",
};

function statuses(valueMappingsText) {
  return buildFieldMappingStatuses({
    columnMetadata: { FORM_CODE: {} },
    dictionariesById: { dose: dictionary },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "CAP" }, { FORM_CODE: "INJ" }],
    rules: { sdDose: { valueMappingsText } },
    sourceDictionaryItem: () => null,
    sourceDictionaryPropertySummary: () => "",
  });
}

test("field status distinguishes a selected source from completed dictionary values", () => {
  const [partial] = statuses("CAP = 2");
  assert.equal(partial.configured, true);
  assert.equal(partial.dictionaryHandled, 1);
  assert.equal(partial.dictionaryPending, 1);
  assert.equal(partial.state, "dictionary");

  const [complete] = statuses("CAP = 2\nINJ = 3");
  assert.equal(complete.dictionaryHandled, 2);
  assert.equal(complete.dictionaryPending, 0);
  assert.equal(complete.state, "ready");
});

test("same code with a different source meaning remains unmatched", () => {
  const [status] = buildFieldMappingStatuses({
    columnMetadata: {
      FORM_CODE: {
        sourceDictionary: { items: [{ key: "2", text: "粉剂" }] },
      },
    },
    dictionariesById: {
      dose: { dicId: "dose", items: [{ key: "2", text: "胶囊剂" }] },
    },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "2" }],
    rules: { sdDose: { valueMappingsText: "" } },
    sourceDictionaryItem: (metadata, value) =>
      metadata.sourceDictionary.items.find((item) => item.key === value),
    sourceDictionaryPropertySummary: () => "",
  });

  assert.equal(status.dictionaryHandled, 0);
  assert.equal(status.dictionaryPending, 1);
  assert.equal(status.dictionaryRows[0].suggestedTarget, null);

  const [confirmed] = buildFieldMappingStatuses({
    columnMetadata: {
      FORM_CODE: {
        sourceDictionary: { items: [{ key: "2", text: "粉剂" }] },
      },
    },
    dictionariesById: {
      dose: { dicId: "dose", items: [{ key: "2", text: "胶囊剂" }] },
    },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "2" }],
    rules: { sdDose: { valueMappingsText: "2 = 2" } },
    sourceDictionaryItem: (metadata, value) =>
      metadata.sourceDictionary.items.find((item) => item.key === value),
    sourceDictionaryPropertySummary: () => "",
  });
  assert.equal(confirmed.dictionaryHandled, 1);
  assert.equal(confirmed.dictionaryPending, 0);
});

test("dictionary rows keep first-source occurrence order after updates", () => {
  const [status] = buildFieldMappingStatuses({
    columnMetadata: { FORM_CODE: {} },
    dictionariesById: { dose: dictionary },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "INJ" }, { FORM_CODE: "CAP" }, { FORM_CODE: "INJ" }],
    rules: { sdDose: { valueMappingsText: "INJ = 3" } },
    sourceDictionaryItem: () => null,
    sourceDictionaryPropertySummary: () => "",
  });

  assert.deepEqual(
    status.dictionaryRows.map((item) => item.sourceValue),
    ["INJ", "CAP"],
  );
  assert.deepEqual(
    status.dictionaryRows[0].medicines.map((item) => item.rowIndex),
    [0, 2],
  );
  assert.equal(status.dictionaryRows[0].medicines[0].row.FORM_CODE, "INJ");
});

test("invalid dispensing method is pending during field mapping", () => {
  const dispensingField = {
    key: "sdDps",
    label: "发药方式编码",
    group: "用药规则",
    required: false,
    dictionaryId: "rbmh.base.med.dispensingMethod",
  };
  const [status] = buildFieldMappingStatuses({
    columnMetadata: {
      DISPENSE_CODE: {
        sourceDictionary: { items: [{ key: "6", text: "住院发药" }] },
      },
    },
    dictionariesById: {
      "rbmh.base.med.dispensingMethod": {
        dicId: "rbmh.base.med.dispensingMethod",
        items: ["1", "2", "3", "9"].map((key) => ({
          key,
          text: `目标发药方式${key}`,
        })),
      },
    },
    fields: [dispensingField],
    findDictionaryItem,
    mapping: { sdDps: "DISPENSE_CODE" },
    rows: [{ DISPENSE_CODE: "6" }, { DISPENSE_CODE: "6" }],
    rules: { sdDps: { valueMappingsText: "" } },
    sourceDictionaryItem: (metadata, value) =>
      metadata.sourceDictionary.items.find((item) => item.key === value),
    sourceDictionaryPropertySummary: () => "",
  });

  assert.equal(status.state, "dictionary");
  assert.equal(status.dictionaryTotal, 1);
  assert.equal(status.dictionaryPending, 1);
  assert.equal(status.dictionaryRows[0].sourceValue, "6");
  assert.equal(status.dictionaryRows[0].count, 2);
});

test("dictionary suggestion exposes safe semantic confidence and reason", () => {
  const [status] = buildFieldMappingStatuses({
    columnMetadata: {
      FORM_CODE: {
        sourceDictionary: { items: [{ key: "CAP", text: "胶囊" }] },
      },
    },
    dictionariesById: {
      dose: { dicId: "dose", items: [{ key: "2", text: "胶囊剂" }] },
    },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "CAP" }],
    rules: { sdDose: { valueMappingsText: "" } },
    sourceDictionaryItem: (metadata, value) =>
      metadata.sourceDictionary.items.find((item) => item.key === value),
    sourceDictionaryPropertySummary: () => "",
  });

  assert.equal(status.dictionaryRows[0].suggestedTarget.key, "2");
  assert.equal(status.dictionaryRows[0].suggestionConfidence, 94);
  assert.equal(status.dictionaryRows[0].suggestionReason, "常用医学同义表达");
  assert.deepEqual(
    status.dictionaryRows[0].suggestedCandidates.map((item) => [
      item.target.key,
      item.confidence,
    ]),
    [["2", 94]],
  );
});

test("compound dictionary meaning is exposed as a high-confidence suggestion", () => {
  const [status] = buildFieldMappingStatuses({
    columnMetadata: {
      FORM_CODE: {
        sourceDictionary: { items: [{ key: "7", text: "气雾剂" }] },
      },
    },
    dictionariesById: {
      dose: { dicId: "dose", items: [{ key: "62", text: "气雾剂,水雾剂" }] },
    },
    fields: [field],
    findDictionaryItem,
    mapping: { sdDose: "FORM_CODE" },
    rows: [{ FORM_CODE: "7" }],
    rules: { sdDose: { valueMappingsText: "" } },
    sourceDictionaryItem: (metadata, value) =>
      metadata.sourceDictionary.items.find((item) => item.key === value),
    sourceDictionaryPropertySummary: () => "",
  });

  assert.equal(status.dictionaryRows[0].suggestedTarget.key, "62");
  assert.equal(status.dictionaryRows[0].suggestionConfidence, 97);
  assert.equal(
    status.dictionaryRows[0].suggestionReason,
    "目标复合含义包含来源名称",
  );
  assert.deepEqual(
    status.dictionaryRows[0].targetSimilarityRanking.map(
      ({ item }) => item.key,
    ),
    ["62"],
  );
});
