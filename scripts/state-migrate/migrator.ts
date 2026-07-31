/**
 * Core, pure migration engine for contract storage schema upgrades (#497).
 *
 * Applies declarative migrations to a table of records with dry-run, checkpoint
 * rollback, post-migration verification, history tracking, and a heuristic cost
 * estimate. Atomic on-chain execution / real gas metering are documented
 * follow-ups; this engine is the safety-critical, fully-tested core.
 */

import { createHash } from "node:crypto";
import {
  type Migration,
  type MigrationExpectation,
  type MigrationOperation,
  type MigrationRecord,
  type Row,
  type State,
  type TransformRegistry,
  OP_COST_UNITS,
} from "./types.ts";

/** Built-in field transforms. Callers may extend via a custom registry. */
export const DEFAULT_TRANSFORMS: TransformRegistry = {
  toString: (v) => String(v ?? ""),
  toNumber: (v) => Number(v ?? 0),
  identity: (v) => v,
};

function clone(state: State): State {
  return state.map((row) => ({ ...row }));
}

export function stateChecksum(state: State): string {
  return createHash("sha256").update(JSON.stringify(state)).digest("hex");
}

function applyOperation(
  state: State,
  op: MigrationOperation,
  transforms: TransformRegistry,
): State {
  return state.map((row) => {
    const next: Row = { ...row };
    switch (op.op) {
      case "addField":
        if (!(op.field in next)) next[op.field] = op.default;
        break;
      case "removeField":
        delete next[op.field];
        break;
      case "renameField":
        if (op.from in next) {
          next[op.to] = next[op.from];
          delete next[op.from];
        }
        break;
      case "transformField": {
        const fn = transforms[op.using];
        if (!fn) throw new Error(`unknown transform "${op.using}"`);
        if (op.field in next) next[op.field] = fn(next[op.field], next);
        break;
      }
    }
    return next;
  });
}

/** Apply a migration to a copy of `state` (input is never mutated). */
export function applyMigration(
  state: State,
  migration: Migration,
  transforms: TransformRegistry = DEFAULT_TRANSFORMS,
): State {
  let result = clone(state);
  for (const op of migration.operations) {
    result = applyOperation(result, op, transforms);
  }
  return result;
}

export interface DryRunReport {
  version: number;
  rowsBefore: number;
  rowsAfter: number;
  fieldsAdded: string[];
  fieldsRemoved: string[];
  fieldsRenamed: Array<{ from: string; to: string }>;
  fieldsTransformed: string[];
  estimatedCostUnits: number;
  /** Resulting state — NOT committed by the caller in dry-run mode. */
  preview: State;
}

/** Simulate a migration without committing: returns a change summary + preview. */
export function dryRun(
  state: State,
  migration: Migration,
  transforms: TransformRegistry = DEFAULT_TRANSFORMS,
): DryRunReport {
  const preview = applyMigration(state, migration, transforms);
  const report: DryRunReport = {
    version: migration.version,
    rowsBefore: state.length,
    rowsAfter: preview.length,
    fieldsAdded: [],
    fieldsRemoved: [],
    fieldsRenamed: [],
    fieldsTransformed: [],
    estimatedCostUnits: estimateCost(state, migration),
    preview,
  };
  for (const op of migration.operations) {
    if (op.op === "addField") report.fieldsAdded.push(op.field);
    else if (op.op === "removeField") report.fieldsRemoved.push(op.field);
    else if (op.op === "renameField") report.fieldsRenamed.push({ from: op.from, to: op.to });
    else report.fieldsTransformed.push(op.field);
  }
  return report;
}

/** Heuristic cost estimate: sum of per-op unit cost × row count. */
export function estimateCost(state: State, migration: Migration): number {
  return migration.operations.reduce(
    (sum, op) => sum + OP_COST_UNITS[op.op] * state.length,
    0,
  );
}

/** Post-migration integrity checks. */
export function verifyMigration(
  before: State,
  after: State,
  expect: MigrationExpectation,
): { ok: boolean; errors: string[] } {
  const errors: string[] = [];
  if (expect.preserveRowCount && before.length !== after.length) {
    errors.push(`row count changed: ${before.length} -> ${after.length}`);
  }
  for (const field of expect.requireFields ?? []) {
    if (!after.every((r) => field in r)) errors.push(`missing required field "${field}"`);
  }
  for (const field of expect.forbidFields ?? []) {
    if (after.some((r) => field in r)) errors.push(`forbidden field "${field}" still present`);
  }
  return { ok: errors.length === 0, errors };
}

export interface MigrateResult {
  status: "applied" | "rolled-back";
  state: State;
  errors: string[];
  record: MigrationRecord;
}

/**
 * Apply a migration atomically: snapshot a checkpoint first, run the migration
 * and verification, and restore the checkpoint (rollback) if anything fails —
 * so storage can never be left in a partially-migrated state.
 */
export function migrateWithRollback(
  state: State,
  migration: Migration,
  options: {
    transforms?: TransformRegistry;
    expect?: MigrationExpectation;
  } = {},
): MigrateResult {
  const transforms = options.transforms ?? DEFAULT_TRANSFORMS;
  const checkpoint = clone(state);
  let errors: string[] = [];
  let next: State;
  try {
    next = applyMigration(state, migration, transforms);
    if (options.expect) {
      const verdict = verifyMigration(checkpoint, next, options.expect);
      if (!verdict.ok) errors = verdict.errors;
    }
  } catch (err) {
    errors = [err instanceof Error ? err.message : String(err)];
    next = checkpoint;
  }

  const rolledBack = errors.length > 0;
  const finalState = rolledBack ? checkpoint : next;
  return {
    status: rolledBack ? "rolled-back" : "applied",
    state: finalState,
    errors,
    record: {
      version: migration.version,
      description: migration.description,
      appliedAt: new Date().toISOString(),
      rows: finalState.length,
      checksum: stateChecksum(finalState),
      status: rolledBack ? "rolled-back" : "applied",
    },
  };
}

/** Append a migration record to a history log, enforcing monotonic versions. */
export function appendHistory(
  history: MigrationRecord[],
  record: MigrationRecord,
): MigrationRecord[] {
  const lastApplied = [...history].reverse().find((r) => r.status === "applied");
  if (record.status === "applied" && lastApplied && record.version <= lastApplied.version) {
    throw new Error(
      `non-monotonic migration version ${record.version} (last applied ${lastApplied.version})`,
    );
  }
  return [...history, record];
}
