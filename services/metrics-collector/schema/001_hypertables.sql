-- TimescaleDB schema for protocol metrics time-series (#455)

CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Protocol-level metrics sampled every minute
CREATE TABLE IF NOT EXISTS protocol_metrics (
    time                TIMESTAMPTZ NOT NULL,
    tvl                 DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_borrows       DOUBLE PRECISION NOT NULL DEFAULT 0,
    utilization_rate    DOUBLE PRECISION NOT NULL DEFAULT 0,
    liquidations        DOUBLE PRECISION NOT NULL DEFAULT 0,
    total_deposits      DOUBLE PRECISION NOT NULL DEFAULT 0,
    active_users        DOUBLE PRECISION NOT NULL DEFAULT 0,
    source              TEXT NOT NULL DEFAULT 'collector',
    PRIMARY KEY (time)
);

SELECT create_hypertable('protocol_metrics', 'time', if_not_exists => TRUE);

-- Per-asset metrics
CREATE TABLE IF NOT EXISTS asset_metrics (
    time                TIMESTAMPTZ NOT NULL,
    asset               TEXT NOT NULL,
    supply              DOUBLE PRECISION NOT NULL DEFAULT 0,
    borrow              DOUBLE PRECISION NOT NULL DEFAULT 0,
    available_liquidity DOUBLE PRECISION NOT NULL DEFAULT 0,
    price               DOUBLE PRECISION,
    volatility          DOUBLE PRECISION,
    apy                 DOUBLE PRECISION,
    PRIMARY KEY (time, asset)
);

SELECT create_hypertable('asset_metrics', 'time', if_not_exists => TRUE);

CREATE INDEX IF NOT EXISTS idx_asset_metrics_asset_time
    ON asset_metrics (asset, time DESC);

-- Gap tracking for missed intervals
CREATE TABLE IF NOT EXISTS metrics_gaps (
    gap_id              BIGSERIAL PRIMARY KEY,
    metric_family       TEXT NOT NULL,
    gap_start           TIMESTAMPTZ NOT NULL,
    gap_end             TIMESTAMPTZ NOT NULL,
    detected_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    backfilled          BOOLEAN NOT NULL DEFAULT FALSE,
    backfilled_at       TIMESTAMPTZ
);

CREATE TABLE IF NOT EXISTS metrics_collector_state (
    id                  SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    last_collected_at   TIMESTAMPTZ,
    samples_written     BIGINT NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO metrics_collector_state (id) VALUES (1) ON CONFLICT DO NOTHING;

-- Continuous aggregate: 1h rollup retained for 1 year
CREATE MATERIALIZED VIEW IF NOT EXISTS protocol_metrics_1h
WITH (timescaledb.continuous) AS
SELECT
    time_bucket('1 hour', time) AS bucket,
    AVG(tvl) AS tvl,
    AVG(total_borrows) AS total_borrows,
    AVG(utilization_rate) AS utilization_rate,
    SUM(liquidations) AS liquidations,
    AVG(total_deposits) AS total_deposits,
    AVG(active_users) AS active_users
FROM protocol_metrics
GROUP BY bucket
WITH NO DATA;

SELECT add_continuous_aggregate_policy(
    'protocol_metrics_1h',
    start_offset => INTERVAL '3 days',
    end_offset => INTERVAL '1 hour',
    schedule_interval => INTERVAL '15 minutes',
    if_not_exists => TRUE
);

-- Raw retention 30 days; keep 1h aggregates via CAgg (managed separately)
SELECT add_retention_policy('protocol_metrics', INTERVAL '30 days', if_not_exists => TRUE);
SELECT add_retention_policy('asset_metrics', INTERVAL '30 days', if_not_exists => TRUE);
