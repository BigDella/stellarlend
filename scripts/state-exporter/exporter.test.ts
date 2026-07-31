import { test } from "node:test";
import assert from "node:assert/strict";
import {
  buildExport,
  checksum,
  diff,
  rowCount,
  toCSV,
  toJSON,
  verifyIntegrity,
} from "./exporter.ts";
import { type ContractState } from "./types.ts";

function sampleState(): ContractState {
  return {
    positions: [
      { address: "GA...1", collateral: "1000", debt: "400", healthFactor: "2.5" },
      { address: "GA...2", collateral: "500", debt: "0", healthFactor: "max" },
    ],
    pools: [{ asset: "USDC", supplyCap: "1000000", interestRateBps: 350, collateralFactorBps: 8000 }],
    governance: [
      { proposalId: 1, status: "executed", votesFor: "900", votesAgainst: "100", executed: true },
    ],
  };
}

test("rowCount totals each section", () => {
  const rc = rowCount(sampleState());
  assert.equal(rc.positions, 2);
  assert.equal(rc.pools, 1);
  assert.equal(rc.governance, 1);
  assert.equal(rc.total, 4);
});

test("toJSON round-trips to an equal state", () => {
  const state = sampleState();
  assert.deepEqual(JSON.parse(toJSON(state)), state);
});

test("checksum is deterministic and content-sensitive", () => {
  const a = toJSON(sampleState());
  assert.equal(checksum(a), checksum(a));
  assert.notEqual(checksum(a), checksum(a + " "));
});

test("buildExport (json) is integrity-verifiable", () => {
  const { payload, manifest } = buildExport(sampleState(), { format: "json" });
  assert.equal(manifest.rowCount.total, 4);
  assert.equal(manifest.incremental, false);
  const result = verifyIntegrity(payload, manifest);
  assert.ok(result.ok, JSON.stringify(result));
});

test("buildExport (csv) row count and integrity", () => {
  const { payload, manifest } = buildExport(sampleState(), { format: "csv" });
  assert.equal(manifest.format, "csv");
  assert.equal(manifest.rowCount.total, 4);
  const result = verifyIntegrity(payload, manifest);
  assert.ok(result.ok, JSON.stringify(result));
});

test("buildExport with gzip round-trips and verifies", () => {
  const { payload, manifest } = buildExport(sampleState(), { format: "json", compressed: true });
  assert.equal(manifest.compressed, true);
  assert.ok(Buffer.isBuffer(payload));
  assert.ok(verifyIntegrity(payload, manifest).ok);
});

test("incremental diff captures only new/changed rows (CDC)", () => {
  const prev = sampleState();
  const next = sampleState();
  // Change one position, add a pool, leave governance untouched.
  next.positions[0].debt = "450";
  next.pools.push({ asset: "XLM", supplyCap: "5000000", interestRateBps: 200, collateralFactorBps: 7500 });

  const delta = diff(prev, next);
  assert.equal(delta.positions.length, 1);
  assert.equal(delta.positions[0].address, "GA...1");
  assert.equal(delta.pools.length, 1);
  assert.equal(delta.pools[0].asset, "XLM");
  assert.equal(delta.governance.length, 0);
});

test("buildExport incremental sets manifest + delta row counts", () => {
  const prev = sampleState();
  const next = sampleState();
  next.positions[0].debt = "450";

  const { manifest } = buildExport(next, { format: "json", previous: prev });
  assert.equal(manifest.incremental, true);
  assert.equal(manifest.rowCount.total, 1);
});

test("integrity check fails on tampered payload", () => {
  const { payload, manifest } = buildExport(sampleState(), { format: "json" });
  const tampered = String(payload).replace("1000", "9999");
  assert.equal(verifyIntegrity(tampered, manifest).ok, false);
});

test("toCSV emits a section marker per section", () => {
  const csv = toCSV(sampleState());
  assert.ok(csv.includes("# section: positions"));
  assert.ok(csv.includes("# section: pools"));
  assert.ok(csv.includes("# section: governance"));
});
