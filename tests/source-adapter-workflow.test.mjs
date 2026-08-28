import assert from "node:assert/strict";
import test from "node:test";

import {
  adapterProvidesSourceKey,
  inventoryLocationKindLabel,
  inventoryWorkflowFor,
  medicineWorkflowFor,
  sourceKeyDisplayLabel,
  usesMappingPreset,
} from "../src/sourceAdapterWorkflow.js";

test("guided adapters keep user-selected source keys by default", () => {
  const adapter = { id: "GENERIC_DATABASE" };
  assert.equal(adapterProvidesSourceKey(adapter), false);
  assert.equal(
    sourceKeyDisplayLabel(adapter, ["DRUG_ID", "FACTORY_ID"]),
    "DRUG_ID＋FACTORY_ID",
  );
  assert.equal(medicineWorkflowFor(adapter).batchSourceType, "GENERIC");
});

test("inventory adapters expose normalized source kinds while safety policies stay fixed", () => {
  const workflow = inventoryWorkflowFor({
    inventoryWorkflow: {
      normalizedLocationKinds: [" warehouse ", "pharmacy", "WAREHOUSE"],
      sourceProductKeyLabel: " 药品序号:厂家序号 ",
      writeMode: "first_stocktake",
    },
  });
  assert.deepEqual(workflow.normalizedLocationKinds, ["WAREHOUSE", "PHARMACY"]);
  assert.equal(workflow.sourceProductKeyLabel, "药品序号:厂家序号");
  assert.equal(workflow.writeMode, "FIRST_STOCKTAKE");
  assert.equal(workflow.sourceStockKeyMode, "ADAPTER_SCOPED_V1");
  assert.equal(workflow.trialPolicy, "PER_TARGET_STORAGE");
  assert.equal(inventoryLocationKindLabel("WAREHOUSE"), "药库");
  assert.equal(inventoryLocationKindLabel("PHARMACY"), "药房");
});

test("automatic adapters declare their stable source identity and write semantics", () => {
  const adapter = {
    id: "VENDOR_HIS",
    medicineWorkflow: {
      sourceKeyMode: "adapter_provided",
      sourceKeyField: " SOURCE_KEY ",
      sourceKeyLabel: "药品序号:厂家序号",
      mappingPreset: "vendor_preset",
      batchSourceType: "vendor_batch",
      factoryPolicy: "adapter_managed",
      legacyProfileKind: "none",
    },
  };
  const workflow = medicineWorkflowFor(adapter);
  assert.equal(adapterProvidesSourceKey(adapter), true);
  assert.equal(workflow.sourceKeyField, "SOURCE_KEY");
  assert.equal(
    sourceKeyDisplayLabel(adapter, ["SOURCE_KEY"]),
    "药品序号:厂家序号",
  );
  assert.equal(usesMappingPreset(adapter, "VENDOR_PRESET"), true);
  assert.equal(workflow.batchSourceType, "VENDOR_BATCH");
  assert.equal(workflow.factoryPolicy, "ADAPTER_MANAGED");
});
