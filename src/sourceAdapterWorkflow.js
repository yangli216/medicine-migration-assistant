const DEFAULT_MEDICINE_WORKFLOW = Object.freeze({
  sourceKeyMode: "USER_SELECTED",
  sourceKeyField: "",
  sourceKeyLabel: "",
  mappingPreset: "GUIDED",
  batchSourceType: "GENERIC",
  factoryPolicy: "OPTIONAL_CREATE",
  legacyProfileKind: "NONE",
});

const DEFAULT_INVENTORY_WORKFLOW = Object.freeze({
  normalizedLocationKinds: [],
  sourceProductKeyLabel: "来源药品商品键",
  sourceStockKeyMode: "ADAPTER_SCOPED_V1",
  writeMode: "FIRST_STOCKTAKE",
  organizationMapping: "REQUIRED",
  medicineLedger: "REQUIRED",
  trialPolicy: "PER_TARGET_STORAGE",
  undoPolicy: "VERIFIED_BATCH_ONLY",
});

export function medicineWorkflowFor(adapter) {
  const configured = adapter?.medicineWorkflow || {};
  const workflow = {
    ...DEFAULT_MEDICINE_WORKFLOW,
    ...configured,
  };
  workflow.sourceKeyMode = `${workflow.sourceKeyMode || ""}`.toUpperCase();
  workflow.sourceKeyField = `${workflow.sourceKeyField || ""}`.trim();
  workflow.sourceKeyLabel = `${workflow.sourceKeyLabel || ""}`.trim();
  workflow.mappingPreset = `${workflow.mappingPreset || ""}`.toUpperCase();
  workflow.batchSourceType = `${workflow.batchSourceType || ""}`.toUpperCase();
  workflow.factoryPolicy = `${workflow.factoryPolicy || ""}`.toUpperCase();
  workflow.legacyProfileKind =
    `${workflow.legacyProfileKind || ""}`.toUpperCase();
  return workflow;
}

export function adapterProvidesSourceKey(adapter) {
  const workflow = medicineWorkflowFor(adapter);
  return (
    workflow.sourceKeyMode === "ADAPTER_PROVIDED" &&
    Boolean(workflow.sourceKeyField)
  );
}

export function sourceKeyDisplayLabel(adapter, selectedFields = []) {
  const workflow = medicineWorkflowFor(adapter);
  if (adapterProvidesSourceKey(adapter)) {
    return workflow.sourceKeyLabel || workflow.sourceKeyField;
  }
  return selectedFields.filter(Boolean).join("＋");
}

export function usesMappingPreset(adapter, preset) {
  return (
    medicineWorkflowFor(adapter).mappingPreset === `${preset}`.toUpperCase()
  );
}

export function inventoryWorkflowFor(adapter) {
  const configured = adapter?.inventoryWorkflow || {};
  const workflow = {
    ...DEFAULT_INVENTORY_WORKFLOW,
    ...configured,
  };
  workflow.normalizedLocationKinds = [
    ...new Set(
      (workflow.normalizedLocationKinds || [])
        .map((kind) => `${kind || ""}`.trim().toUpperCase())
        .filter(Boolean),
    ),
  ];
  workflow.sourceProductKeyLabel =
    `${workflow.sourceProductKeyLabel || ""}`.trim();
  for (const key of [
    "writeMode",
    "sourceStockKeyMode",
    "organizationMapping",
    "medicineLedger",
    "trialPolicy",
    "undoPolicy",
  ]) {
    workflow[key] = `${workflow[key] || ""}`.trim().toUpperCase();
  }
  return workflow;
}

export function inventoryLocationKindLabel(kind) {
  return `${kind || ""}`.toUpperCase() === "WAREHOUSE" ? "药库" : "药房";
}
