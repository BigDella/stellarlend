-- Issue #611: historical pool performance events and benchmarks

CREATE TABLE IF NOT EXISTS pool_performance_events (
    id            UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address  TEXT         NOT NULL,
    event_type    TEXT         NOT NULL,
    payload       JSONB        NOT NULL DEFAULT '{}'::jsonb,
    recorded_at   TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_perf_events_pool
    ON pool_performance_events (pool_address, recorded_at DESC);

CREATE TABLE IF NOT EXISTS pool_performance_benchmarks (
    id              UUID         PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address    TEXT         NOT NULL,
    period          TEXT         NOT NULL,
    protocol        TEXT         NOT NULL,
    pool_supply_apy NUMERIC      NOT NULL,
    bench_supply_apy NUMERIC     NOT NULL,
    delta_apy       NUMERIC      NOT NULL,
    computed_at     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_perf_bench_pool
    ON pool_performance_benchmarks (pool_address, period);
