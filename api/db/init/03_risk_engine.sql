-- StellarLend Risk Engine Schema
-- Issue #450: Correlation matrix for multi-collateral positions
-- Issue #451: Dynamic liquidation threshold based on market volatility
-- Issue #452: Concentration risk monitoring for large position holders
-- Issue #453: Risk-adjusted collateral ratio calculator

-- ─── #450 Price History (oracle snapshots, minimum 6 months retained) ──────────

CREATE TABLE IF NOT EXISTS asset_price_history (
    id          UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset       TEXT        NOT NULL,
    price       NUMERIC     NOT NULL,
    source      TEXT        NOT NULL DEFAULT 'oracle',
    recorded_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_price_history_asset        ON asset_price_history (asset);
CREATE INDEX IF NOT EXISTS idx_price_history_asset_time   ON asset_price_history (asset, recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_price_history_recorded_at  ON asset_price_history (recorded_at DESC);

-- ─── #450 Correlation Matrix Cache ──────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS asset_correlations (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset_a         TEXT        NOT NULL,
    asset_b         TEXT        NOT NULL,
    window_days     INTEGER     NOT NULL,   -- 30, 60 or 90
    pearson         NUMERIC     NOT NULL,
    spearman        NUMERIC     NOT NULL,
    sample_count    INTEGER     NOT NULL,
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset_a, asset_b, window_days)
);

CREATE INDEX IF NOT EXISTS idx_correlations_pair        ON asset_correlations (asset_a, asset_b);
CREATE INDEX IF NOT EXISTS idx_correlations_window      ON asset_correlations (window_days);
CREATE INDEX IF NOT EXISTS idx_correlations_computed_at ON asset_correlations (computed_at DESC);

-- ─── #451 Volatility Snapshots ──────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS asset_volatility (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset           TEXT        NOT NULL,
    window_days     INTEGER     NOT NULL,   -- 5 or 20
    realized_vol    NUMERIC     NOT NULL,   -- annualised, decimal (e.g. 0.45 = 45%)
    computed_at     TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (asset, window_days)
);

CREATE INDEX IF NOT EXISTS idx_volatility_asset        ON asset_volatility (asset);
CREATE INDEX IF NOT EXISTS idx_volatility_computed_at  ON asset_volatility (computed_at DESC);

-- ─── #451 Dynamic LTV Adjustments (audit trail) ─────────────────────────────────

CREATE TABLE IF NOT EXISTS ltv_adjustments (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset               TEXT        NOT NULL,
    base_ltv            NUMERIC     NOT NULL,
    volatility_premium  NUMERIC     NOT NULL,
    adjusted_ltv        NUMERIC     NOT NULL,
    locked_until        TIMESTAMPTZ NOT NULL,          -- 24h timelock
    governance_override BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ltv_adjustments_asset      ON ltv_adjustments (asset);
CREATE INDEX IF NOT EXISTS idx_ltv_adjustments_created_at ON ltv_adjustments (created_at DESC);

-- ─── #452 Concentration Snapshots ───────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS concentration_snapshots (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset           TEXT        NOT NULL,
    hhi             NUMERIC     NOT NULL,   -- 0-10000 (HHI × 10000)
    top5_pct        NUMERIC     NOT NULL,   -- percentage, 0-100
    top10_pct       NUMERIC     NOT NULL,
    total_positions INTEGER     NOT NULL,
    tvl             NUMERIC     NOT NULL,
    snapshot_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_conc_snapshots_asset       ON concentration_snapshots (asset);
CREATE INDEX IF NOT EXISTS idx_conc_snapshots_snapshot_at ON concentration_snapshots (snapshot_at DESC);

-- ─── #452 Concentration Alerts ──────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS concentration_alerts (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset           TEXT        NOT NULL,
    address         TEXT        NOT NULL,
    position_pct    NUMERIC     NOT NULL,
    threshold_pct   NUMERIC     NOT NULL,
    enforcement     TEXT        NOT NULL DEFAULT 'soft',  -- soft | hard
    alerted_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at     TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_conc_alerts_asset   ON concentration_alerts (asset);
CREATE INDEX IF NOT EXISTS idx_conc_alerts_address ON concentration_alerts (address);

-- ─── #453 Collateral Ratio Calculations ─────────────────────────────────────────

CREATE TABLE IF NOT EXISTS collateral_ratio_history (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset               TEXT        NOT NULL,
    base_ratio          NUMERIC     NOT NULL,
    volatility_factor   NUMERIC     NOT NULL,
    liquidity_factor    NUMERIC     NOT NULL,
    correlation_factor  NUMERIC     NOT NULL,
    final_ratio         NUMERIC     NOT NULL,
    computed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_ratio_history_asset       ON collateral_ratio_history (asset);
CREATE INDEX IF NOT EXISTS idx_ratio_history_computed_at ON collateral_ratio_history (computed_at DESC);
