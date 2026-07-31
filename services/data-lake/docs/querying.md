# Querying the StellarLend data lake

Raw blockchain data lands as Parquet files partitioned by:

```
s3://<bucket>/raw/date=YYYY-MM-DD/event_type=<topic>/part-*.parquet
```

## Athena / Presto

After Terraform applies the Glue catalog:

```sql
MSCK REPAIR TABLE stellarlend_dev_lake.raw_transactions;

SELECT event_type, COUNT(*) AS events
FROM stellarlend_dev_lake.raw_transactions
WHERE date BETWEEN '2026-01-01' AND '2026-01-07'
GROUP BY event_type
ORDER BY events DESC;
```

Filter by partition columns first (`date`, `event_type`) to keep scans cheap.

```sql
SELECT tx_hash, user_address, amount, payload_json
FROM stellarlend_dev_lake.raw_transactions
WHERE date = '2026-01-15'
  AND event_type = 'liquidation_event'
LIMIT 100;
```

## Schema evolution

Schemas live in `services/data-lake/schemas/`. Evolution is **additive only**:

- bump `schema_version` when appending columns
- never remove/rename fields in place — add a new field and deprecate the old one in docs
- Glue table DDL must be updated when new columns are introduced

## Retention

| Tier | Path prefix | Retention |
| --- | --- | --- |
| Raw | `raw/` | 90 days (S3 lifecycle + ETL retention job) |
| Aggregated | `agg/` | Indefinite |

## Access control

- Data scientists: assume `lake-reader` IAM role (S3 Get/List + Glue read)
- ETL workers: assume `lake-writer` IAM role (S3 Put on `raw/` and `agg/`)
- Public access is blocked on the lake bucket
