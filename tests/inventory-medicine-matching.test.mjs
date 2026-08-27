import test from "node:test";
import assert from "node:assert/strict";
import {
  inventoryUnmatchedMedicines,
  medicineTextSimilarity,
  recommendInventoryMedicineMatches,
} from "../src/inventoryMedicineMatching.js";

const missing = "药品 101:4003 缺少完整的 id_med/id_med_pro 基础迁移台账";

test("extracts one unmatched medicine per source product and target organization", () => {
  const rows = [1, 2].map((rowNo) => ({
    rowNo,
    errorMessage: missing,
    idMed: "",
    idMedPro: "",
    rawData: {
      sourceProductKey: "101:4003",
      drugName: "多巴胺注射液",
      specification: "2ml:20mg",
      factoryName: "上海禾丰制药有限公司",
    },
    normalizedData: { idOrg: "org-1" },
  }));
  const result = inventoryUnmatchedMedicines(rows);
  assert.equal(result.length, 1);
  assert.equal(result[0].key, "101:4003::org-1");
  assert.equal(result[0].sourceProductKey, "101:4003");
  assert.equal(result[0].rowCount, 2);
});

test("unique exact name specification and factory match is preselectable", () => {
  const source = {
    key: "s::org-1",
    sourceProductKey: "s",
    targetOrganizationId: "org-1",
    drugName: "多巴胺注射液",
    specification: "2ml：20mg",
    factoryName: "上海禾丰制药有限公司",
  };
  const exact = {
    idMed: "m1",
    idMedPro: "p1",
    drugName: "多巴胺注射液",
    specification: "2ml:20mg",
    factoryName: "上海禾丰制药有限公司",
    private: false,
  };
  const result = recommendInventoryMedicineMatches([source], [exact])[0];
  assert.equal(result.autoCandidate.idMedPro, "p1");
  assert.equal(result.candidates[0].scorePercent, 100);
});

test("ambiguous exact matches require manual choice and private products stay in their organization", () => {
  const source = {
    key: "s::org-1",
    sourceProductKey: "s",
    targetOrganizationId: "org-1",
    drugName: "阿莫西林胶囊",
    specification: "0.25g*24粒/盒",
    factoryName: "甲药业",
  };
  const base = {
    drugName: source.drugName,
    specification: source.specification,
    factoryName: source.factoryName,
  };
  const result = recommendInventoryMedicineMatches(
    [source],
    [
      { ...base, idMedPro: "p1", idMed: "m1", private: false },
      { ...base, idMedPro: "p2", idMed: "m2", private: true, organizationId: "org-1" },
      { ...base, idMedPro: "p3", idMed: "m3", private: true, organizationId: "org-2" },
    ],
  )[0];
  assert.equal(result.autoCandidate, null);
  assert.deepEqual(result.candidates.map((item) => item.target.idMedPro), ["p1", "p2"]);
});

test("similarity ranks a close medicine ahead without auto selecting it", () => {
  const result = recommendInventoryMedicineMatches(
    [{
      key: "s::org-1",
      sourceProductKey: "s",
      targetOrganizationId: "org-1",
      drugName: "阿莫西林胶囊",
      specification: "0.25g*24粒",
      factoryName: "华北制药股份有限公司",
    }],
    [
      { idMed: "m1", idMedPro: "close", drugName: "阿莫西林胶囊", specification: "0.25g*24粒/盒", factoryName: "华北制药股份有限公司", private: false },
      { idMed: "m2", idMedPro: "far", drugName: "维生素C片", specification: "100mg*100片", factoryName: "其他厂家", private: false },
    ],
  )[0];
  assert.equal(result.candidates[0].target.idMedPro, "close");
  assert.equal(result.autoCandidate, null);
  assert.ok(medicineTextSimilarity("0.25g*24粒", "0.25g*24粒/盒") > 0.7);
});
