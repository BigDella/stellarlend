# State Migration Tool (#497)

A declarative framework for migrating contract storage between schema versions,
with dry-run, atomic checkpoint rollback, verification, and audit history.

## Migration definition

```json
{
  "version": 2,
  "description": "add tier, rename collateral, normalize debt",
  "operations": [
    { "op": "addField", "field": "tier", "default": "bronze" },
    { "op": "renameField", "from": "collateral", "to": "collateral_amount" },
    { "op": "transformField", "field": "debt", "using": "toNumber" },
    { "op": "removeField", "field": "legacy_flag" }
  ]
}
```

Supported operations: `addField`, `removeField`, `renameField`, `transformField`
(via a named, pure transform registry — `toString`, `toNumber`, `identity`
built in, extensible).

## Usage

```bash
# Dry-run: preview changes + heuristic cost, no writes
node --experimental-strip-types scripts/state-migrate/index.ts \
  --state state.json --plan migration.json --dry-run

# Apply with verification + history (auto-rolls-back on failure)
node --experimental-strip-types scripts/state-migrate/index.ts \
  --state state.json --plan migration.json --out migrated.json \
  --expect expect.json --history history.json
```

`expect.json` (post-migration integrity):
```json
{ "requireFields": ["tier"], "forbidFields": ["legacy_flag"], "preserveRowCount": true }
```

## Tests

```bash
node --experimental-strip-types --test scripts/state-migrate/migrator.test.ts
```

## Covered acceptance criteria

- Declarative migration definitions; operations add/rename/remove/transform field.
- Dry-run simulation that never mutates state.
- Atomic execution with checkpoint + automatic rollback on op error or failed verification.
- Post-migration verification (required/forbidden fields, row-count preservation).
- Versioned, audit-trailed migration history (monotonic-version enforced).
- Heuristic per-operation cost estimate.
- CLI: run, dry-run, view status, append history.

## Follow-ups (out of this PR's core)

- On-chain atomic execution against a live contract and real gas metering via
  transaction simulation (the engine exposes a heuristic cost model today).
- Migration-hub integration (`contracts/migration-tool/`).
