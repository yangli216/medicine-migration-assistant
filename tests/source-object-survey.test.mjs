import assert from "node:assert/strict";
import test from "node:test";
import { targetFields } from "../src/migrationFields.js";
import {
  failedSourceObjectSurveyResult,
  rankSourceObjectSurveyResults,
  sourceObjectSurveyCandidates,
  sourceObjectSurveyResult,
} from "../src/sourceObjectSurvey.js";

test("bounded source survey inspects only the strongest explicit name candidates", () => {
  const candidates = sourceObjectSurveyCandidates([
    "SYS_USER",
    "T_DRUG_PRICE",
    "V_DRUG_CATALOG",
    "YK_TYPK",
    "YK_YPCD",
    "T_MEDICINE_INFO",
    "MEDICAL_RECORD",
    "DRUG_LOG",
  ], 3);
  assert.equal(candidates.length, 3);
  assert.deepEqual(
    candidates.map((candidate) => candidate.value),
    ["YK_TYPK", "YK_YPCD", "V_DRUG_CATALOG"],
  );
  assert.ok(candidates.every((candidate) => candidate.data.medicineClueScore > 0));
});

test("survey ranking prefers actual readable medicine fields over a stronger table-name clue", () => {
  const options = sourceObjectSurveyCandidates(["YK_TYPK", "T_DRUG_PRICE"], 5);
  const optionByName = Object.fromEntries(options.map((option) => [option.value, option]));
  const master = sourceObjectSurveyResult({
    option: optionByName.YK_TYPK,
    targetFields,
    sourceResult: {
      preview: {
        columns: ["YPMC", "YPGG", "JX", "ZXDW", "CDMC"],
        columnMetadata: [
          { name: "YPMC", comment: "药品名称" },
          { name: "YPGG", comment: "规格" },
          { name: "JX", comment: "剂型" },
          { name: "ZXDW", comment: "最小单位" },
          { name: "CDMC", comment: "生产厂家" },
        ],
        rows: [{ YPMC: "当归", YPGG: "10g", JX: "饮片", ZXDW: "袋", CDMC: "药厂" }],
      },
    },
  });
  const price = sourceObjectSurveyResult({
    option: optionByName.T_DRUG_PRICE,
    targetFields,
    sourceResult: {
      preview: {
        columns: ["DRUG_NAME", "PRICE"],
        columnMetadata: [{ name: "DRUG_NAME", comment: "药品名称" }],
        rows: [{ DRUG_NAME: "当归", PRICE: 12.5 }],
      },
    },
  });
  const failed = failedSourceObjectSurveyResult({
    value: "V_DRUG_CATALOG",
    data: { medicineClueScore: 99 },
  });
  const ranked = rankSourceObjectSurveyResults([price, failed, master]);
  assert.equal(ranked[0].objectName, "YK_TYPK");
  assert.equal(ranked[1].objectName, "T_DRUG_PRICE");
  assert.equal(ranked[2].status, "FAILED");
  assert.equal(ranked[0].hasMedicineName, true);
});
