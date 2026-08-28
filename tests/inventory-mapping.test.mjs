import assert from "node:assert/strict";
import test from "node:test";
import {
  completeInventoryOrganizationIds,
  inventoryLocationNeedsSourceResolution,
} from "../src/inventoryMapping.js";

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
    requiresSourceLocationResolution: true,
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

test("uses the adapter contract instead of vendor key prefixes for source resolution", () => {
  assert.equal(
    inventoryLocationNeedsSourceResolution({
      sourceKind: "WAREHOUSE",
      sourceLocationKey: "VENDOR:ORG-1:MED-10",
      mappingStatus: "SOURCE_LOCATION_MISSING",
      requiresSourceLocationResolution: true,
    }),
    true,
  );
  assert.equal(
    inventoryLocationNeedsSourceResolution({
      sourceKind: "WAREHOUSE",
      sourceLocationKey: "YK:1001",
      mappingStatus: "PENDING_TARGET_MAPPING",
      requiresSourceLocationResolution: false,
    }),
    false,
  );
});

test("allows one organization batch to include only selected storages", () => {
  assert.deepEqual(
    completeInventoryOrganizationIds(
      locations,
      { "org-a": "target-org-a" },
      { "warehouse-a": "target-storage-1" },
      {},
      ["warehouse-a"],
    ),
    ["org-a"],
  );
  assert.deepEqual(
    completeInventoryOrganizationIds(
      locations,
      { "org-a": "target-org-a" },
      { "warehouse-a": "target-storage-1" },
      {},
      [],
    ),
    [],
  );
});
