-- StellarLend event archiver star schema (TimescaleDB / PostgreSQL)
-- Designed for analytical queries with <1s latency on 6 months of data.

CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

-- Dimension: event types (Deposit, Withdraw, Borrow, Repay, Liquidate)
CREATE TABLE IF NOT EXISTS dim_event_type (
    event_type_id   SMALLINT PRIMARY KEY,
    event_name      TEXT NOT NULL UNIQUE,
    description     TEXT
);

INSERT INTO dim_event_type (event_type_id, event_name, description) VALUES
    (1, 'deposit_event', 'Collateral deposit'),
    (2, 'withdrawal_event', 'Collateral withdrawal'),
    (3, 'borrow_event', 'Asset borrow'),
    (4, 'repay_event', 'Debt repayment'),
    (5, 'liquidation_event', 'Position liquidation')
ON CONFLICT (event_type_id) DO NOTHING;

-- Dimension: contracts
CREATE TABLE IF NOT EXISTS dim_contract (
    contract_sk     SERIAL PRIMARY KEY,
    contract_id     TEXT NOT NULL UNIQUE,
    network         TEXT NOT NULL DEFAULT 'testnet',
    first_seen_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Dimension: calendar (optional helper for BI tools)
CREATE TABLE IF NOT EXISTS dim_date (
    date_key        DATE PRIMARY KEY,
    year            SMALLINT NOT NULL,
    month           SMALLINT NOT NULL,
    day             SMALLINT NOT NULL,
    day_of_week     SMALLINT NOT NULL,
    is_weekend      BOOLEAN NOT NULL
);

-- Fact: raw protocol events
CREATE TABLE IF NOT EXISTS fact_events (
    event_id            BIGSERIAL,
    ledger              BIGINT NOT NULL,
    tx_hash             TEXT NOT NULL,
    event_index         INTEGER NOT NULL DEFAULT 0,
    contract_sk         INTEGER NOT NULL REFERENCES dim_contract(contract_sk),
    event_type_id       SMALLINT NOT NULL REFERENCES dim_event_type(event_type_id),
    event_name          TEXT NOT NULL,
    block_timestamp     TIMESTAMPTZ NOT NULL,
    topics              JSONB NOT NULL DEFAULT '[]'::jsonb,
    payload             JSONB NOT NULL DEFAULT '{}'::jsonb,
    user_address        TEXT,
    asset_address       TEXT,
    amount              NUMERIC(78, 0),
    ingested_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (event_id, block_timestamp)
);

SELECT create_hypertable('fact_events', 'block_timestamp', if_not_exists => TRUE);

CREATE UNIQUE INDEX IF NOT EXISTS uq_fact_events_ledger_tx_idx
    ON fact_events (ledger, tx_hash, event_index, block_timestamp);

CREATE INDEX IF NOT EXISTS idx_fact_events_type_time
    ON fact_events (event_type_id, block_timestamp DESC);

CREATE INDEX IF NOT EXISTS idx_fact_events_user_time
    ON fact_events (user_address, block_timestamp DESC)
    WHERE user_address IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_fact_events_payload_gin
    ON fact_events USING GIN (payload);

-- Sync cursor / integrity bookkeeping
CREATE TABLE IF NOT EXISTS archive_sync_state (
    id                  SMALLINT PRIMARY KEY DEFAULT 1 CHECK (id = 1),
    last_ledger         BIGINT NOT NULL DEFAULT 0,
    last_synced_at      TIMESTAMPTZ,
    events_archived     BIGINT NOT NULL DEFAULT 0,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO archive_sync_state (id, last_ledger)
VALUES (1, 0)
ON CONFLICT (id) DO NOTHING;

CREATE TABLE IF NOT EXISTS ledger_event_counts (
    ledger              BIGINT PRIMARY KEY,
    expected_count      INTEGER,
    archived_count      INTEGER NOT NULL,
    verified_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    integrity_ok        BOOLEAN NOT NULL DEFAULT TRUE
);

-- Aggregated rollup used after detail retention window
CREATE TABLE IF NOT EXISTS fact_events_daily (
    day                 DATE NOT NULL,
    event_type_id       SMALLINT NOT NULL REFERENCES dim_event_type(event_type_id),
    contract_sk         INTEGER NOT NULL REFERENCES dim_contract(contract_sk),
    event_count         BIGINT NOT NULL DEFAULT 0,
    total_amount        NUMERIC(78, 0) NOT NULL DEFAULT 0,
    UNIQUE (day, event_type_id, contract_sk)
);

CREATE INDEX IF NOT EXISTS idx_fact_events_daily_day
    ON fact_events_daily (day DESC);
