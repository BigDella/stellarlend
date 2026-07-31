-- StellarLend Auto-Compounding Vault Schema
-- Issue #607: Build lending pool yield optimizer with auto-compounding rewards

-- ─── Vault Config ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS vault_configs (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address       TEXT        NOT NULL UNIQUE,
    underlying_asset    TEXT        NOT NULL,
    reward_asset        TEXT        NOT NULL,
    performance_fee_bps INTEGER     NOT NULL DEFAULT 500,
    management_fee_bps  INTEGER     NOT NULL DEFAULT 100,
    harvest_interval_secs BIGINT    NOT NULL DEFAULT 86400,
    slippage_tolerance_bps INTEGER NOT NULL DEFAULT 100,
    deposit_paused      BOOLEAN     NOT NULL DEFAULT FALSE,
    withdraw_paused     BOOLEAN     NOT NULL DEFAULT FALSE,
    active              BOOLEAN     NOT NULL DEFAULT TRUE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ─── Vault Snapshots ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS vault_snapshots (
    id                      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address           TEXT        NOT NULL,
    total_assets            NUMERIC     NOT NULL,
    total_shares            NUMERIC     NOT NULL,
    share_price             NUMERIC     NOT NULL,
    last_harvested_at       TIMESTAMPTZ,
    accrued_management_fees NUMERIC     NOT NULL DEFAULT 0,
    accrued_performance_fees NUMERIC    NOT NULL DEFAULT 0,
    snapshot_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vault_snapshots_vault      ON vault_snapshots (vault_address);
CREATE INDEX IF NOT EXISTS idx_vault_snapshots_time       ON vault_snapshots (snapshot_at DESC);

-- ─── User Vault Positions ────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS vault_positions (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address       TEXT        NOT NULL,
    user_address        TEXT        NOT NULL,
    deposited_amount    NUMERIC     NOT NULL DEFAULT 0,
    shares_owned        NUMERIC     NOT NULL DEFAULT 0,
    last_deposit_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_withdraw_at    TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (vault_address, user_address)
);

CREATE INDEX IF NOT EXISTS idx_vault_positions_vault   ON vault_positions (vault_address);
CREATE INDEX IF NOT EXISTS idx_vault_positions_user    ON vault_positions (user_address);

-- ─── Harvest Events ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS vault_harvest_events (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    vault_address       TEXT        NOT NULL,
    rewards_claimed     NUMERIC     NOT NULL,
    rewards_reinvested  NUMERIC     NOT NULL,
    performance_fee     NUMERIC     NOT NULL DEFAULT 0,
    management_fee      NUMERIC     NOT NULL DEFAULT 0,
    total_assets_after  NUMERIC     NOT NULL,
    tx_hash             TEXT,
    harvested_at        TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_vault_harvest_vault ON vault_harvest_events (vault_address);
CREATE INDEX IF NOT EXISTS idx_vault_harvest_time  ON vault_harvest_events (harvested_at DESC);