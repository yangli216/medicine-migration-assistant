import assert from "node:assert/strict";
import test from "node:test";
import {
  databaseSourceIdentity,
  legacySourceMappingPromotionRequest,
  normalizeSourceQueryIdentity,
  sourceAdapterId,
  sourceMappingProfileForSelection,
  sourceMappingProfileScope,
  sourceSelectionIdentity,
} from "../src/sourceMappingProfiles.js";

test("database source mapping scope is stable and tenant isolated", () => {
  const profile = {
    kind: "Oracle",
    host: "10.0.0.8",
    port: 1521,
    database: "phis",
    serviceName: "ORCL",
    schema: "HIS",
    username: "reader",
  };
  assert.equal(
    databaseSourceIdentity(profile),
    "oracle:10.0.0.8:1521/orcl:his",
  );
  assert.deepEqual(
    sourceMappingProfileScope({
      adapterId: sourceAdapterId(),
      profile,
      targetTenantId: "Tenant-A",
    }),
    {
      adapterId: "GENERIC_DATABASE",
      sourceIdentity: "database:oracle:10.0.0.8:1521/orcl:his",
      targetTenantId: "tenant-a",
    },
  );
});

test("file and PHIS adapters receive independent template identities", () => {
  assert.equal(sourceAdapterId({ sourceMode: "file" }), "GENERIC_FILE");
  assert.equal(sourceAdapterId({ phis27: true }), "PHIS27");
  assert.deepEqual(
    sourceMappingProfileScope({
      adapterId: "GENERIC_FILE",
      fileName: "Medicine.CSV",
      sourceMode: "file",
      targetTenantId: "tenant-a",
    }),
    {
      adapterId: "GENERIC_FILE",
      sourceIdentity: "file:medicine.csv",
      targetTenantId: "tenant-a",
    },
  );
});

test("database mappings are isolated by table view or exact read query", () => {
  const profile = {
    kind: "mysql",
    host: "127.0.0.1",
    port: 3306,
    database: "legacy",
  };
  const tableScope = sourceMappingProfileScope({
    adapterId: "GENERIC_DATABASE",
    profile,
    sourceObject: "V_DRUG_CATALOG",
    targetTenantId: "tenant-a",
  });
  const anotherTableScope = sourceMappingProfileScope({
    adapterId: "GENERIC_DATABASE",
    profile,
    sourceObject: "T_DRUG_INFO",
    targetTenantId: "tenant-a",
  });
  const queryScope = sourceMappingProfileScope({
    adapterId: "GENERIC_DATABASE",
    profile,
    sourceQuery: " SELECT * FROM V_DRUG_CATALOG; ",
    targetTenantId: "tenant-a",
  });
  assert.match(tableScope.sourceIdentity, /^database-selection:.*\|object:/);
  assert.notEqual(tableScope.sourceIdentity, anotherTableScope.sourceIdentity);
  assert.match(queryScope.sourceIdentity, /^database-selection:.*\|query:/);
  assert.notEqual(tableScope.sourceIdentity, queryScope.sourceIdentity);
  assert.equal(
    normalizeSourceQueryIdentity(" SELECT * FROM V_DRUG_CATALOG;;  "),
    "SELECT * FROM V_DRUG_CATALOG",
  );
  assert.equal(
    sourceSelectionIdentity({ sourceObject: "V_DRUG_CATALOG" }),
    sourceSelectionIdentity({ sourceObject: "V_DRUG_CATALOG" }),
  );
});

test("a legacy database-level mapping is restored only for the exact same query", () => {
  const legacyProfile = {
    sourceQuery: "SELECT * FROM V_DRUG_CATALOG;",
    mapping: { naMed: "DRUG_NAME" },
  };
  assert.equal(
    sourceMappingProfileForSelection({
      legacyProfile,
      resolvedQuery: "SELECT * FROM V_DRUG_CATALOG",
    }),
    legacyProfile,
  );
  assert.equal(
    sourceMappingProfileForSelection({
      legacyProfile,
      resolvedQuery: "SELECT * FROM T_DRUG_INFO",
    }),
    null,
  );
  const scopedProfile = { sourceQuery: "", mapping: { naMed: "YPMC" } };
  assert.equal(
    sourceMappingProfileForSelection({
      scopedProfile,
      legacyProfile,
      resolvedQuery: "SELECT * FROM T_DRUG_INFO",
    }),
    scopedProfile,
  );
});

test("ODBC connection-string identities exclude credentials and survive password changes", () => {
  const first = databaseSourceIdentity({
    kind: "oracle",
    connectionString:
      "Driver={Oracle 19 ODBC driver};Dbq=10.0.0.8:1521/ORCL;Uid=reader;Pwd=secret-one;",
  });
  const passwordChanged = databaseSourceIdentity({
    kind: "oracle",
    connectionString:
      "PWD=secret-two;UID=another-reader;DBQ=10.0.0.8:1521/ORCL;DRIVER={Oracle 19 ODBC driver}",
  });
  const anotherDatabase = databaseSourceIdentity({
    kind: "oracle",
    connectionString:
      "Driver={Oracle 19 ODBC driver};Dbq=10.0.0.9:1521/ORCL;Uid=reader;Pwd=secret-one;",
  });
  assert.equal(first, passwordChanged);
  assert.notEqual(first, anotherDatabase);
  assert.doesNotMatch(first, /secret|reader|10\.0\.0\.8/);
});

test("an exact legacy mapping can be promoted to the current object scope", () => {
  const scopedScope = {
    adapterId: "GENERIC_DATABASE",
    sourceIdentity: "database-selection:abc|object:def:v_drug",
    targetTenantId: "tenant-a",
  };
  const legacyProfile = {
    adapterVersion: 1,
    sourceKey: "DRUG_CODE",
    sourceQuery: "SELECT * FROM `V_DRUG`",
    mapping: { naMed: "DRUG_NAME" },
    rules: { naMed: { transform: "trim" } },
    dictionaryOverrides: {},
  };
  const request = legacySourceMappingPromotionRequest({
    scopedScope,
    legacyProfile,
    resolvedQuery: "SELECT * FROM `V_DRUG`;",
  });
  assert.equal(request.scope, scopedScope);
  assert.equal(request.sourceQuery, "SELECT * FROM `V_DRUG`");
  assert.deepEqual(request.mapping, legacyProfile.mapping);
  assert.equal(
    legacySourceMappingPromotionRequest({
      scopedScope,
      legacyProfile,
      resolvedQuery: "SELECT * FROM `T_DRUG`",
    }),
    null,
  );
  assert.equal(
    legacySourceMappingPromotionRequest({
      scopedScope,
      scopedProfile: { mapping: {} },
      legacyProfile,
      resolvedQuery: "SELECT * FROM `V_DRUG`",
    }),
    null,
  );
});
