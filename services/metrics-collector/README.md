# Protocol Metrics Time-Series Collector

Implements [issue #455](https://github.com/Smartdevs17/stellarlend/issues/455).

## Capabilities

- Samples every minute: TVL, totalBorrows, utilization rate, liquidations, deposits, active users
- Per-asset samples: supply, borrow, available liquidity, price, volatility, APY
- TimescaleDB hypertables + 1h continuous aggregates
- Retention: raw **30 days**, 1h aggregates retained via CAgg policy (**1 year**)
- Automated gap detection + backfill
- API: `GET /api/metrics/timeseries?metric=&from=&to=&interval=`

## Schema

```bash
psql "$DATABASE_URL" -f schema/001_hypertables.sql
```

## Pipelines

- `pipelines/collect-protocol-metrics.ts` — CronJob / worker entrypoints
- `src/collector/` — sampling, gap detection, orchestration

## API

Mounted in the main API:

```http
GET /api/metrics/timeseries?metric=tvl&from=2026-01-01T00:00:00Z&to=2026-01-02T00:00:00Z&interval=1h
```

Supported metrics: `tvl`, `totalBorrows`, `utilizationRate`, `liquidations`, `totalDeposits`, `activeUsers`  
Supported intervals: `1m`, `5m`, `1h`, `1d`

## Run

```bash
cd services/metrics-collector
cp .env.example .env
npm install
npm test
npm run dev
```
