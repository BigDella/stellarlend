-- Retention: full detail for 2 years, then keep daily aggregates only.

-- Continuous aggregate for hourly rollups (optional analytics accelerator)
CREATE MATERIALIZED VIEW IF NOT EXISTS fact_events_hourly
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', block_timestamp) AS bucket,
    event_type_id,
    contract_sk,
    COUNT(*) AS event_count,
    COALESCE(SUM(amount), 0) AS total_amount
FROM fact_events
GROUP BY bucket, event_type_id, contract_sk
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'fact_events_hourly',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '1 hour',
    if_not_exists => TRUE
);

-- Drop raw detail older than DETAIL_RETENTION_DAYS (default 730 = 2 years)
-- Invoke via archiver retention job: SELECT archive_apply_retention(730);
CREATE OR REPLACE FUNCTION archive_apply_retention(retention_days INTEGER DEFAULT 730)
RETURNS BIGINT
LANGUAGE plpgsql
AS $$
DECLARE
    cutoff TIMESTAMPTZ := NOW() - make_interval(days => retention_days);
    moved BIGINT := 0;
BEGIN
    -- Roll remaining detail into daily aggregates before delete
    INSERT INTO fact_events_daily (day, event_type_id, contract_sk, event_count, total_amount)
    SELECT
        DATE(block_timestamp) AS day,
        event_type_id,
        contract_sk,
        COUNT(*) AS event_count,
        COALESCE(SUM(amount), 0) AS total_amount
    FROM fact_events
    WHERE block_timestamp < cutoff
    GROUP BY DATE(block_timestamp), event_type_id, contract_sk
    ON CONFLICT (day, event_type_id, contract_sk) DO UPDATE
    SET
        event_count = fact_events_daily.event_count + EXCLUDED.event_count,
        total_amount = fact_events_daily.total_amount + EXCLUDED.total_amount;

    GET DIAGNOSTICS moved = ROW_COUNT;

    DELETE FROM fact_events WHERE block_timestamp < cutoff;
    RETURN moved;
END;
$$;

-- Timescale compression for older chunks (speeds analytical scans)
ALTER TABLE fact_events SET (
    timescaledb.compress,
    timescaledb.compress_segmentby = 'event_type_id,contract_sk'
);

SELECT add_compression_policy('fact_events', INTERVAL '30 days', if_not_exists => TRUE);
