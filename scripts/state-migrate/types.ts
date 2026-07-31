/**
 * Types for the contract storage state-migration tool (#497).
 *
 * State is modeled as a table of records (each a field map), so schema
 * migrations — add / rename / remove / transform field — are expressed
 * declaratively and applied deterministically with dry-run and rollback.
 */

export type Row = Record<string, unknown>;
export type State = Row[];

export type MigrationOperation =
  | { op: "addField"; field: string; default: unknown }
  | { op: "removeField"; field: string }
  | { op: "renameField"; from: string; to: string }
  | { op: "transformField"; field: string; using: string };

export interface Migration {
  /** Monotonic schema version this migration upgrades TO. */
  version: number;
  description: string;
  operations: MigrationOperation[];
}

/** Named, pure field transforms referenced by `transformField.using`. */
export type TransformRegistry = Record<string, (value: unknown, row: Row) => unknown>;

export interface MigrationExpectation {
  /** Every row must have these fields. */
  requireFields?: string[];
  /** No row may have these fields. */
  forbidFields?: string[];
  /** Row count must be unchanged after migration. */
  preserveRowCount?: boolean;
}

export interface MigrationRecord {
  version: number;
  description: string;
  appliedAt: string;
  rows: number;
  checksum: string;
  status: "applied" | "rolled-back";
}

/** Heuristic gas/cost units per operation type (per row). On-chain estimation
 * via simulation is a documented follow-up. */
export const OP_COST_UNITS: Record<MigrationOperation["op"], number> = {
  addField: 2,
  removeField: 1,
  renameField: 2,
  transformField: 3,
};
