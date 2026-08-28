import assert from "node:assert/strict";
import test from "node:test";
import {
  assessSourceKeyFields,
  assessSourceKeyColumns,
  chooseInitialSourceKeyFields,
  chooseInitialSourceKey,
  encodeSourceKey,
  recommendCompositeSourceKey,
  selectedSourceKeyAssessment,
  sourceKeyComponentOptions,
  sourceKeyOptions,
} from "../src/sourceKeyAssessment.js";

test("source key recommendation uses full-batch completeness and uniqueness before name clues", () => {
  const review = assessSourceKeyColumns({
    columns: ["DRUG_NAME", "DRUG_CODE", "LEGACY_ID", "EMPTY_ID"],
    columnMetadata: {
      DRUG_CODE: { comment: "药品编码" },
      LEGACY_ID: { comment: "内部主键" },
    },
    rows: [
      { DRUG_NAME: "当归", DRUG_CODE: "001", LEGACY_ID: 10, EMPTY_ID: "" },
      { DRUG_NAME: "黄芪", DRUG_CODE: "002", LEGACY_ID: 11, EMPTY_ID: "x" },
      { DRUG_NAME: "丹参", DRUG_CODE: "003", LEGACY_ID: 12, EMPTY_ID: "y" },
    ],
  });
  assert.equal(review.recommendedColumn, "DRUG_CODE");
  assert.equal(review.qualifiedCount, 3);
  assert.equal(chooseInitialSourceKey(review, "EMPTY_ID"), "DRUG_CODE");
  assert.equal(chooseInitialSourceKey(review, "LEGACY_ID"), "LEGACY_ID");
  assert.equal(
    selectedSourceKeyAssessment(review, "DRUG_CODE").completeUnique,
    true,
  );
});

test("trim-equivalent duplicates, blanks, and structured values are never selectable", () => {
  const review = assessSourceKeyColumns({
    columns: ["CODE", "OPTIONAL_ID", "OBJECT_ID"],
    rows: [
      { CODE: "A", OPTIONAL_ID: "1", OBJECT_ID: { id: 1 } },
      { CODE: " A ", OPTIONAL_ID: null, OBJECT_ID: { id: 2 } },
    ],
  });
  const code = selectedSourceKeyAssessment(review, "CODE");
  assert.equal(code.duplicateCount, 1);
  assert.equal(
    selectedSourceKeyAssessment(review, "OPTIONAL_ID").emptyCount,
    1,
  );
  assert.equal(
    selectedSourceKeyAssessment(review, "OBJECT_ID").unsupportedCount,
    2,
  );
  assert.equal(review.qualifiedCount, 0);
  assert.equal(review.recommendedColumn, "");
  assert.equal(chooseInitialSourceKey(review, "CODE"), "");
  assert.ok(sourceKeyOptions(review).every((option) => option.disabled));
});

test("a unique medicine name stays selectable but is not silently recommended as a stable key", () => {
  const review = assessSourceKeyColumns({
    columns: ["DRUG_NAME"],
    rows: [{ DRUG_NAME: "当归" }, { DRUG_NAME: "黄芪" }],
  });
  assert.equal(review.qualifiedCount, 1);
  assert.equal(review.recommendedColumn, "");
  assert.equal(sourceKeyOptions(review)[0].disabled, false);
});

test("a composite source key is recommended when identifier columns are only unique together", () => {
  const rows = [
    { DRUG_ID: "1001", FACTORY_ID: "10", DRUG_NAME: "当归" },
    { DRUG_ID: "1001", FACTORY_ID: "11", DRUG_NAME: "当归" },
    { DRUG_ID: "1002", FACTORY_ID: "10", DRUG_NAME: "黄芪" },
  ];
  const review = assessSourceKeyColumns({
    columns: ["DRUG_ID", "FACTORY_ID", "DRUG_NAME"],
    rows,
  });
  assert.deepEqual(recommendCompositeSourceKey({ rows, review }), [
    "DRUG_ID",
    "FACTORY_ID",
  ]);
  assert.deepEqual(chooseInitialSourceKeyFields({ rows, review }), [
    "DRUG_ID",
    "FACTORY_ID",
  ]);
  assert.equal(
    assessSourceKeyFields({ rows, fields: ["DRUG_ID", "FACTORY_ID"] })
      .completeUnique,
    true,
  );
});

test("composite source keys use collision-free JSON tuples and reject incomplete parts", () => {
  assert.equal(
    encodeSourceKey({ A: " x ", B: "y|z" }, ["A", "B"]),
    '["x","y|z"]',
  );
  assert.notEqual(
    encodeSourceKey({ A: "a|b", B: "c" }, ["A", "B"]),
    encodeSourceKey({ A: "a", B: "b|c" }, ["A", "B"]),
  );
  assert.equal(encodeSourceKey({ A: "x", B: "" }, ["A", "B"]), "");
});

test("legacy single-field templates restore while composite components need only be complete", () => {
  const rows = [
    { DRUG_ID: "1", FACTORY_ID: "10" },
    { DRUG_ID: "1", FACTORY_ID: "11" },
    { DRUG_ID: "2", FACTORY_ID: "10" },
    { DRUG_ID: "2", FACTORY_ID: "11" },
  ];
  const review = assessSourceKeyColumns({
    columns: ["DRUG_ID", "FACTORY_ID"],
    rows,
  });
  assert.deepEqual(
    chooseInitialSourceKeyFields({
      rows,
      review,
      legacySourceKey: "DRUG_ID",
    }),
    ["DRUG_ID", "FACTORY_ID"],
  );
  const options = sourceKeyComponentOptions(review, ["DRUG_ID"], "DRUG_ID");
  assert.equal(
    options.find((option) => option.value === "DRUG_ID").disabled,
    false,
  );
  assert.equal(
    options.find((option) => option.value === "FACTORY_ID").disabled,
    false,
  );
});
