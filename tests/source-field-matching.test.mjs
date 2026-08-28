import assert from "node:assert/strict";
import test from "node:test";
import { targetFields } from "../src/migrationFields.js";
import {
  sourceFieldMatchScore,
  sourceObjectSuitability,
} from "../src/sourceFieldMatching.js";

test("matches physical columns and database comments without unsafe short partials", () => {
  const medicineName = targetFields.find((field) => field.key === "naMed");
  const specification = targetFields.find((field) => field.key === "spec");
  assert.equal(sourceFieldMatchScore("DRUG_NAME", medicineName), 96);
  assert.equal(
    sourceFieldMatchScore("COLUMN_01", medicineName, { comment: "药品名称" }),
    96,
  );
  assert.equal(sourceFieldMatchScore("INSPECTION_TYPE", specification), 0);
});

test("reports a medicine-like source object using explainable field clues", () => {
  const summary = sourceObjectSuitability({
    columns: [
      "DRUG_NAME",
      "SPEC",
      "FORM_CODE",
      "PRE_UNIT",
      "FACTORY_NAME",
      "PRODUCT_NAME",
    ],
    rows: [
      {
        DRUG_NAME: "阿莫西林胶囊",
        SPEC: "0.25g*24粒",
        FORM_CODE: "CAP",
        PRE_UNIT: "粒",
        FACTORY_NAME: "示例药厂",
        PRODUCT_NAME: "",
      },
    ],
    targetFields,
  });
  assert.equal(summary.tone, "ready");
  assert.equal(summary.foundCount, 6);
  assert.equal(
    summary.clues.find((clue) => clue.key === "naMedPro").hasSample,
    false,
  );
});

test("warns clearly when no readable medicine name is present", () => {
  const summary = sourceObjectSuitability({
    columns: ["SPEC", "FORM_CODE", "PRE_UNIT"],
    rows: [{ SPEC: "20mg", FORM_CODE: "TAB", PRE_UNIT: "片" }],
    targetFields,
  });
  assert.equal(summary.tone, "danger");
  assert.match(summary.title, /药品名称/);
});
