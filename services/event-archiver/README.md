# On-chain Event Archiver

Implements [issue #454](https://github.com/Smartdevs17/stellarlend/issues/454): archive StellarLend Soroban contract events into a queryable historical warehouse.

## What it captures

| Topic | Warehouse event type |
| --- | --- |
| `deposit_event` | Deposit |
| `withdrawal_event` | Withdraw |
| `borrow_event` | Borrow |
| `repay_event` | Repay |
| `liquidation_event` | Liquidate |

Each row stores:

- raw payload (`topics`, `payload` JSON)
- `ledger`, `tx_hash`, `block_timestamp`
- denormalized `user_address`, `asset_address`, `amount` for analytical filters

## Schema

Star schema under `schema/`:

- `dim_event_type`, `dim_contract`, `dim_date`
- `fact_events` Timescale hypertable (detail retention: **2 years**)
- `fact_events_daily` aggregates retained indefinitely after detail purge
- `ledger_event_counts` for per-ledger integrity verification
- `archive_sync_state` cursor for incremental sync

Apply:

```bash
psql "$DATABASE_URL" -f schema/001_star_schema.sql
psql "$DATABASE_URL" -f schema/002_retention.sql
```

## Sync modes

1. **Backfill** from genesis / `START_LEDGER`:
   ```bash
   ARCHIVER_BACKFILL=true npm start
   ```
2. **Incremental** poll every `POLL_INTERVAL_MS` (default 60s / per-block window via RPC `getEvents`)
3. **Integrity**: archived counts recorded per ledger in `ledger_event_counts`
4. **Retention**: daily job calls `archive_apply_retention(730)` — detail → daily aggregates

## Query latency

Hypertables + indexes on `(event_type_id, block_timestamp)` and GIN on `payload` target **&lt; 1s** for standard 6-month analytical queries. Hourly continuous aggregates further reduce scan cost.

## Configuration

See `.env.example`.

```bash
cd services/event-archiver
cp .env.example .env
npm install
npm test
npm run dev
```

## Layout

```
services/event-archiver/
  schema/           # SQL star schema + retention
  src/archive/      # normalize, integrity, archiver worker
  src/rpc/          # Soroban getEvents client
  src/db/           # Postgres + in-memory repositories
  tests/
```
