# StellarLend Data Lake

Implements [issue #456](https://github.com/Smartdevs17/stellarlend/issues/456).

## Features

- Raw transaction/event data stored in **Parquet** under Hive partitions `date=` / `event_type=`
- Daily ETL pipeline (`etl/` + `npm run etl:daily`)
- Additive **schema evolution** helpers for contract upgrades
- Athena/Glue/Presto-compatible catalog (`src/catalog`, Terraform Glue table)
- Raw retention **90 days**; aggregates under `agg/` kept indefinitely
- IAM reader/writer roles via Terraform

## Layout

```
services/data-lake/
  etl/                 # issue-scoped ETL exports
  schemas/             # Parquet / Glue schemas
  infra/terraform/     # S3 + Glue + IAM
  docs/querying.md     # data scientist query patterns
  src/
```

## Local run

```bash
cd services/data-lake
cp .env.example .env
npm install
npm test
npm run etl:daily -- 2026-07-25
```
