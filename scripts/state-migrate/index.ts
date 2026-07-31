#!/usr/bin/env node
/**
 * state-migrate CLI (#497) — declarative contract storage schema migrations
 * with dry-run, atomic checkpoint rollback, verification, and history.
 *
 * Run (Node >= 22, no install):
 *   # Dry-run (no writes): preview changes + cost
 *   node --experimental-strip-types scripts/state-migrate/index.ts \
 *     --state state.json --plan migration.json --dry-run
 *
 *   # Apply with verification + history; rolls back automatically on failure
 *   node --experimental-strip-types scripts/state-migrate/index.ts \
 *     --state state.json --plan migration.json --out migrated.json \
 *     --expect expect.json --history history.json
 */

import * as fs from "node:fs";
import {
  appendHistory,
  dryRun,
  migrateWithRollback,
} from "./migrator.ts";
import {
  type Migration,
  type MigrationExpectation,
  type MigrationRecord,
  type State,
} from "./types.ts";

function arg(name: string): string | undefined {
  const i = process.argv.indexOf(`--${name}`);
  return i >= 0 ? process.argv[i + 1] : undefined;
}
function flag(name: string): boolean {
  return process.argv.includes(`--${name}`);
}
function readJSON<T>(path: string): T {
  return JSON.parse(fs.readFileSync(path, "utf8")) as T;
}

function main(): void {
  const statePath = arg("state");
  const planPath = arg("plan");
  if (!statePath || !planPath) {
    console.error("Usage: --state <state.json> --plan <migration.json> [--dry-run] [--out ...] [--expect ...] [--history ...]");
    process.exit(2);
  }

  const state = readJSON<State>(statePath);
  const migration = readJSON<Migration>(planPath);
  const expect = arg("expect") ? readJSON<MigrationExpectation>(arg("expect")!) : undefined;

  if (flag("dry-run")) {
    const report = dryRun(state, migration);
    // Don't print the full preview to keep output readable.
    const { preview, ...summary } = report;
    void preview;
    console.log(JSON.stringify(summary, null, 2));
    return;
  }

  const result = migrateWithRollback(state, migration, { expect });

  const outPath = arg("out") ?? "migrated.json";
  fs.writeFileSync(outPath, JSON.stringify(result.state, null, 2));

  const historyPath = arg("history");
  if (historyPath) {
    const history = fs.existsSync(historyPath)
      ? readJSON<MigrationRecord[]>(historyPath)
      : [];
    fs.writeFileSync(historyPath, JSON.stringify(appendHistory(history, result.record), null, 2));
  }

  console.log(JSON.stringify({ status: result.status, errors: result.errors, record: result.record }, null, 2));
  process.exit(result.status === "applied" ? 0 : 1);
}

main();
