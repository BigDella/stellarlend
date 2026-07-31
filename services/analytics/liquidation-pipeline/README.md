# Historical Liquidation Analysis Pipeline

Implements [issue #457](https://github.com/Smartdevs17/stellarlend/issues/457).

## Capabilities

- Extract `liquidation_event` rows from contract logs / archiver payloads
- Per-event metrics: discount, profit, gas cost, block time (hour / weekday)
- Profitability distribution (mean / median / percentiles)
- Time-of-day and day-of-week clustering
- Collateral-type frequency analysis
- Automated daily / weekly / monthly report generation under `reports/`
- Dashboard chart payload helper (`toDashboardCharts`)
- Anomaly detection for unusual discount/profit patterns

## Layout

```
services/analytics/liquidation-pipeline/
  reports/           # generated JSON + Markdown reports
  src/extract/
  src/metrics/
  src/anomaly/
  src/reports/
  tests/
```

## Run

```bash
cd services/analytics/liquidation-pipeline
npm install
npm test
npm run report -- daily ./sample-events.json
```
