import assert from "node:assert/strict";
import test from "node:test";
import {
  sourceObjectFromSafeQuery,
  sourceObjectMedicineClue,
  sourceObjectOptions,
} from "../src/sourceDatabaseObjects.js";

test("restores only one backend-generated table or view query", () => {
  assert.equal(sourceObjectFromSafeQuery("SELECT * FROM `T_DRUG_INFO`"), "T_DRUG_INFO");
  assert.equal(sourceObjectFromSafeQuery("select * from `drug``master`"), "drug`master");
  assert.equal(sourceObjectFromSafeQuery('SELECT * FROM "V_DRUG_CATALOG"'), "V_DRUG_CATALOG");
  assert.equal(
    sourceObjectFromSafeQuery('SELECT * FROM "PHIS"."V_DRUG_CATALOG"'),
    "V_DRUG_CATALOG",
  );
  assert.equal(sourceObjectFromSafeQuery('SELECT * FROM "DRUG""MASTER"'), 'DRUG"MASTER');
  assert.equal(
    sourceObjectFromSafeQuery("SELECT d.id, f.name FROM drug d JOIN factory f ON f.id=d.fac"),
    "",
  );
});

test("ranks explainable medicine-like objects ahead in a very large schema", () => {
  const objects = [
    ...Array.from({ length: 260 }, (_, index) => `SYS_TABLE_${String(index).padStart(3, "0")}`),
    "MEDICAL_RECORD",
    "T_DRUG_PRICE",
    "V_DRUG_CATALOG",
    "YK_TYPK",
  ];
  const options = sourceObjectOptions(objects);
  assert.deepEqual(
    options.slice(0, 3).map((option) => option.value),
    ["YK_TYPK", "V_DRUG_CATALOG", "T_DRUG_PRICE"],
  );
  assert.equal(options.find((option) => option.value === "MEDICAL_RECORD").data.medicineClueScore, 0);
  assert.match(options[0].description, /仅|预览|确认/);
  assert.match(options.find((option) => option.value === "T_DRUG_PRICE").description, /业务明细线索/);
});

test("medicine object clues stay name-only and never claim a semantic match", () => {
  assert.ok(sourceObjectMedicineClue("DRUG_MASTER").score > sourceObjectMedicineClue("DRUG_LOG").score);
  assert.equal(sourceObjectMedicineClue("PATIENT_MASTER").score, 0);
  assert.equal(sourceObjectMedicineClue("MEDICAL_RECORD").score, 0);
});
