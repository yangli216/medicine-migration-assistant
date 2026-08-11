import assert from "node:assert/strict";
import test from "node:test";
import { completeInventoryOrganizationIds } from "../src/inventoryMapping.js";

const locations = [
  {
    organizationId: "org-a",
    sourceLocationKey: "warehouse-a",
    mappingStatus: "READY",
  },
  {
    organizationId: "org-a",
    sourceLocationKey: "pharmacy-a",
    mappingStatus: "READY",
  },
  {
    organizationId: "org-b",
    sourceLocationKey: "warehouse-b",
    mappingStatus: "SOURCE_LOCATION_AMBIGUOUS",
  },
];

test("returns only organizations whose institution and every location are mapped", () => {
  assert.deepEqual(
    completeInventoryOrganizationIds(
      locations,
      { "org-a": "target-a", "org-b": "target-b" },
      {
        "warehouse-a": "target-warehouse-a",
        "pharmacy-a": "target-pharmacy-a",
      },
      {},
    ),
    ["org-a"],
  );
});

test("requires an explicit legacy warehouse for ambiguous stock ledgers", () => {
  assert.deepEqual(
    completeInventoryOrganizationIds(
      locations,
      { "org-a": "target-a", "org-b": "target-b" },
      {
        "warehouse-a": "target-warehouse-a",
        "pharmacy-a": "target-pharmacy-a",
        "warehouse-b": "target-warehouse-b",
      },
      { "warehouse-b": "legacy-warehouse-b" },
    ),
    ["org-a", "org-b"],
  );
});
