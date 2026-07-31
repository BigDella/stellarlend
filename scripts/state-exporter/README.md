# State Exporter (#499)

Exports structured contract state — user positions, pool configs, governance —
to verifiable, structured files.

## Usage

No install needed on Node ≥ 22 (uses `--experimental-strip-types`):

```bash
# Full snapshot export (JSON)
node --experimental-strip-types scripts/state-exporter/index.ts \
  --in state.json --format json --out export.json

# CSV, gzip-compressed
node --experimental-strip-types scripts/state-exporter/index.ts \
  --in state.json --format csv --out export.csv --gzip

# Incremental (change-data-capture) export vs a previous snapshot
node --experimental-strip-types scripts/state-exporter/index.ts \
  --in state.json --since prev.json --out delta.json

# Verify an export against its manifest (checksum + row count)
node --experimental-strip-types scripts/state-exporter/index.ts \
  --verify export.json
```

Every export writes a sidecar `*.manifest.json` with `generatedAt`, format,
`rowCount` (per section + total), and a SHA-256 `checksum` of the payload.

## Tests

```bash
node --experimental-strip-types --test scripts/state-exporter/exporter.test.ts
```

## Covered acceptance criteria

- Export user positions (collateral, debt, health factor), pool configs, and
  governance proposals/votes/executed actions.
- Formats: JSON and CSV.
- Incremental export (CDC): only rows changed/added since a previous snapshot.
- Full snapshot export on demand.
- Export verification: SHA-256 checksum + row count integrity check.
- Compression: gzip.

## Follow-ups (intentionally out of this PR's core)

- Live Soroban RPC `StateSource` (read via contract view functions + events).
  The exporter is written against a `StateSource` interface; the default file
  source keeps the tool runnable/testable offline.
- Additional formats (Parquet, SQL) and sinks (S3/GCS).
- Scheduled/automated exports (daily/weekly worker) under `services/state-archiver/`.
