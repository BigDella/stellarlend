import { test } from "node:test";
import assert from "node:assert/strict";
import {
  appendHistory,
  applyMigration,
  dryRun,
  estimateCost,
  migrateWithRollback,
  verifyMigration,
} from "./migrator.ts";
import { type Migration, type State } from "./types.ts";

function sampleState(): State {
  return [
    { address: "GA1", collateral: "100", debt: "10" },
    { address: "GA2", collateral: "200", debt: "0" },
  ];
}

test("addField sets a default only when absent", () => {
  const m: Migration = {
    version: 2,
    description: "add tier",
    operations: [{ op: "addField", field: "tier", default: "bronze" }],
  };
  const out = applyMigration(sampleState(), m);
  assert.equal(out[0].tier, "bronze");
  assert.equal(out[1].tier, "bronze");
});

test("removeField, renameField, transformField apply correctly", () => {
  const m: Migration = {
    version: 2,
    description: "reshape",
    operations: [
      { op: "renameField", from: "collateral", to: "collateral_amount" },
      { op: "transformField", field: "debt", using: "toNumber" },
      { op: "removeField", field: "address" },
    ],
  };
  const out = applyMigration(sampleState(), m);
  assert.equal(out[0].collateral_amount, "100");
  assert.equal("collateral" in out[0], false);
  assert.equal(out[0].debt, 10); // transformed to number
  assert.equal("address" in out[0], false);
});

test("applyMigration does not mutate the input state", () => {
  const state = sampleState();
  applyMigration(state, {
    version: 2,
    description: "x",
    operations: [{ op: "removeField", field: "debt" }],
  });
  assert.equal("debt" in state[0], true); // original untouched
});

test("dryRun previews changes and cost without committing", () => {
  const state = sampleState();
  const report = dryRun(state, {
    version: 2,
    description: "add+rename",
    operations: [
      { op: "addField", field: "tier", default: "bronze" },
      { op: "renameField", from: "debt", to: "debt_amount" },
    ],
  });
  assert.deepEqual(report.fieldsAdded, ["tier"]);
  assert.deepEqual(report.fieldsRenamed, [{ from: "debt", to: "debt_amount" }]);
  assert.equal(report.rowsBefore, 2);
  assert.ok(report.estimatedCostUnits > 0);
  // input state must remain unchanged after a dry-run
  assert.equal("debt" in state[0], true);
});

test("estimateCost scales with rows and operations", () => {
  const m: Migration = {
    version: 2,
    description: "two ops",
    operations: [
      { op: "addField", field: "a", default: 1 }, // 2 units/row
      { op: "transformField", field: "a", using: "toString" }, // 3 units/row
    ],
  };
  assert.equal(estimateCost(sampleState(), m), (2 + 3) * 2);
});

test("verifyMigration enforces required/forbidden fields and row count", () => {
  const before = sampleState();
  const after = applyMigration(before, {
    version: 2,
    description: "rm debt",
    operations: [{ op: "removeField", field: "debt" }],
  });
  const ok = verifyMigration(before, after, {
    forbidFields: ["debt"],
    requireFields: ["address"],
    preserveRowCount: true,
  });
  assert.ok(ok.ok, JSON.stringify(ok.errors));

  const bad = verifyMigration(before, after, { requireFields: ["debt"] });
  assert.equal(bad.ok, false);
});

test("migrateWithRollback commits a valid migration", () => {
  const result = migrateWithRollback(
    sampleState(),
    { version: 2, description: "ok", operations: [{ op: "addField", field: "tier", default: "x" }] },
    { expect: { requireFields: ["tier"], preserveRowCount: true } },
  );
  assert.equal(result.status, "applied");
  assert.equal(result.state[0].tier, "x");
  assert.equal(result.record.status, "applied");
});

test("migrateWithRollback restores checkpoint on verification failure", () => {
  const state = sampleState();
  const result = migrateWithRollback(
    state,
    { version: 2, description: "bad", operations: [{ op: "removeField", field: "address" }] },
    { expect: { requireFields: ["address"] } }, // will fail
  );
  assert.equal(result.status, "rolled-back");
  assert.ok(result.errors.length > 0);
  // rolled back to original
  assert.equal("address" in result.state[0], true);
});

test("migrateWithRollback restores checkpoint when an operation throws", () => {
  const result = migrateWithRollback(sampleState(), {
    version: 2,
    description: "bad transform",
    operations: [{ op: "transformField", field: "debt", using: "nonexistent" }],
  });
  assert.equal(result.status, "rolled-back");
  assert.equal("debt" in result.state[0], true);
});

test("appendHistory enforces monotonic applied versions", () => {
  let history = appendHistory([], {
    version: 2,
    description: "v2",
    appliedAt: "t",
    rows: 2,
    checksum: "c",
    status: "applied",
  });
  assert.equal(history.length, 1);
  assert.throws(() =>
    appendHistory(history, {
      version: 2,
      description: "v2-again",
      appliedAt: "t",
      rows: 2,
      checksum: "c",
      status: "applied",
    }),
  );
  // a rolled-back record at the same version is allowed (it didn't advance schema)
  history = appendHistory(history, {
    version: 2,
    description: "v2-rollback",
    appliedAt: "t",
    rows: 2,
    checksum: "c",
    status: "rolled-back",
  });
  assert.equal(history.length, 2);
});
