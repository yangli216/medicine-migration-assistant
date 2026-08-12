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
