-- StellarLend Auction, Debt Token, and Performance Tracking Schema
-- Issue #575: Dutch Auction Liquidation System
-- Issue #576: Debt Tokenization with Secondary Market
-- Issue #577: Pool Performance Tracking with Historical Returns
-- Issue #578: Pool Migration Tool

-- ─── #575 Dutch Auction Events ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS auction_events (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id      BIGINT      NOT NULL,
    event_type      TEXT        NOT NULL,  -- 'created', 'bid_placed', 'settled', 'expired'
    pool_address    TEXT        NOT NULL,
    collateral_asset TEXT       NOT NULL,
    debt_asset      TEXT        NOT NULL,
    collateral_amount NUMERIC   NOT NULL,
    debt_amount     NUMERIC     NOT NULL,
    oracle_price    NUMERIC     NOT NULL,
    start_price     NUMERIC     NOT NULL,
    final_price     NUMERIC,
    bidder          TEXT,
    debt_repaid     NUMERIC,
    collateral_received NUMERIC,
    premium_bps     INTEGER,
    duration_secs   BIGINT,
    time_to_fill_secs BIGINT,
    status          TEXT        NOT NULL DEFAULT 'active',
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settled_at      TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_auction_events_auction_id  ON auction_events (auction_id);
CREATE INDEX IF NOT EXISTS idx_auction_events_pool        ON auction_events (pool_address);
CREATE INDEX IF NOT EXISTS idx_auction_events_status      ON auction_events (status);
CREATE INDEX IF NOT EXISTS idx_auction_events_created_at  ON auction_events (created_at DESC);
CREATE INDEX IF NOT EXISTS idx_auction_events_bidder      ON auction_events (bidder);

-- ─── #575 Auction Settlements ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS auction_settlements (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    auction_id          BIGINT      NOT NULL UNIQUE,
    bidder              TEXT        NOT NULL,
    debt_repaid         NUMERIC     NOT NULL,
    collateral_received NUMERIC     NOT NULL,
    premium_bps         INTEGER     NOT NULL,
    gas_cost_stroops    BIGINT,
    net_profit_stroops  BIGINT,
    settled_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_auction_settlements_bidder    ON auction_settlements (bidder);
CREATE INDEX IF NOT EXISTS idx_auction_settlements_settled   ON auction_settlements (settled_at DESC);

-- ─── #576 Debt Token Positions ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS debt_token_positions (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    token_address       TEXT        NOT NULL,
    owner               TEXT        NOT NULL,
    principal           NUMERIC     NOT NULL,
    minted_tokens       NUMERIC     NOT NULL,
    interest_index_at_deposit NUMERIC NOT NULL,
    deposit_timestamp   TIMESTAMPTZ NOT NULL,
    last_interest_update TIMESTAMPTZ NOT NULL,
    accrued_interest    NUMERIC     NOT NULL DEFAULT 0,
    is_locked           BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_debt_positions_owner       ON debt_token_positions (owner);
CREATE INDEX IF NOT EXISTS idx_debt_positions_token       ON debt_token_positions (token_address);
CREATE INDEX IF NOT EXISTS idx_debt_positions_owner_token ON debt_token_positions (owner, token_address);

-- ─── #576 Debt Token Transfers ────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS debt_token_transfers (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    token_address   TEXT        NOT NULL,
    from_address    TEXT        NOT NULL,
    to_address      TEXT        NOT NULL,
    amount          NUMERIC     NOT NULL,
    interest_accrued NUMERIC    NOT NULL DEFAULT 0,
    tx_hash         TEXT,
    recorded_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_debt_transfers_from   ON debt_token_transfers (from_address);
CREATE INDEX IF NOT EXISTS idx_debt_transfers_to     ON debt_token_transfers (to_address);
CREATE INDEX IF NOT EXISTS idx_debt_transfers_time   ON debt_token_transfers (recorded_at DESC);

-- ─── #577 Pool Performance Snapshots ──────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pool_snapshots (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address    TEXT        NOT NULL,
    tvl             NUMERIC     NOT NULL,
    utilization     NUMERIC     NOT NULL,
    borrow_apy      NUMERIC     NOT NULL,
    supply_apy      NUMERIC     NOT NULL,
    bad_debt        NUMERIC     NOT NULL DEFAULT 0,
    total_deposits  NUMERIC     NOT NULL,
    total_borrows   NUMERIC     NOT NULL,
    snapshot_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_snapshots_pool      ON pool_snapshots (pool_address);
CREATE INDEX IF NOT EXISTS idx_pool_snapshots_pool_time ON pool_snapshots (pool_address, snapshot_at DESC);
CREATE INDEX IF NOT EXISTS idx_pool_snapshots_time      ON pool_snapshots (snapshot_at DESC);

-- ─── #577 Pool Performance Aggregates ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pool_performance (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address        TEXT        NOT NULL,
    period              TEXT        NOT NULL,  -- '7d', '30d', '90d', '1y'
    avg_supply_apy      NUMERIC     NOT NULL,
    avg_borrow_apy      NUMERIC     NOT NULL,
    avg_utilization     NUMERIC     NOT NULL,
    volatility          NUMERIC     NOT NULL,
    cumulative_return   NUMERIC     NOT NULL,
    max_drawdown        NUMERIC     NOT NULL,
    sharpe_ratio        NUMERIC     NOT NULL,
    computed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (pool_address, period)
);

CREATE INDEX IF NOT EXISTS idx_pool_performance_pool   ON pool_performance (pool_address);
CREATE INDEX IF NOT EXISTS idx_pool_performance_period ON pool_performance (period);

-- ─── #578 Migration Events ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS migration_events (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    migration_id        BIGINT      NOT NULL,
    user                TEXT        NOT NULL,
    source_pool         TEXT        NOT NULL,
    destination_pool    TEXT        NOT NULL,
    asset               TEXT        NOT NULL,
    amount              NUMERIC     NOT NULL,
    percentage          INTEGER,
    interest_at_migration NUMERIC   NOT NULL DEFAULT 0,
    is_partial          BOOLEAN     NOT NULL DEFAULT FALSE,
    status              TEXT        NOT NULL DEFAULT 'pending',
    estimated_gas       BIGINT,
    actual_gas          BIGINT,
    estimated_slippage_bps INTEGER,
    actual_slippage_bps INTEGER,
    rollback_reason     TEXT,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at        TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_migration_events_user       ON migration_events (user);
CREATE INDEX IF NOT EXISTS idx_migration_events_source     ON migration_events (source_pool);
CREATE INDEX IF NOT EXISTS idx_migration_events_dest       ON migration_events (destination_pool);
CREATE INDEX IF NOT EXISTS idx_migration_events_status     ON migration_events (status);
CREATE INDEX IF NOT EXISTS idx_migration_events_created_at ON migration_events (created_at DESC);
