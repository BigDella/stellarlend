-- Real-time Collateral Ratio Monitoring Schema
-- Extends the risk engine schema with real-time monitoring capabilities

-- ─── Real-time Collateral Ratio Snapshots ────────────────────────────────────────

CREATE TABLE IF NOT EXISTS collateral_ratio_snapshots (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset               TEXT        NOT NULL,
    current_ratio       NUMERIC     NOT NULL,   -- basis points
    required_ratio      NUMERIC     NOT NULL,   -- basis points
    health_factor       NUMERIC     NOT NULL,
    risk_level          TEXT        NOT NULL CHECK (risk_level IN ('safe', 'warning', 'danger', 'critical')),
    collateral_value    NUMERIC     NOT NULL,
    debt_value          NUMERIC     NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_collateral_snapshots_asset       ON collateral_ratio_snapshots (asset);
CREATE INDEX IF NOT EXISTS idx_collateral_snapshots_recorded_at ON collateral_ratio_snapshots (recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_collateral_snapshots_risk_level   ON collateral_ratio_snapshots (risk_level);

-- ─── Position Risk Data ───────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS position_risk_data (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    address             TEXT        NOT NULL,
    asset               TEXT        NOT NULL,
    collateral_amount   NUMERIC     NOT NULL,
    debt_amount         NUMERIC     NOT NULL,
    collateral_value    NUMERIC     NOT NULL,
    debt_value          NUMERIC     NOT NULL,
    current_ratio       NUMERIC     NOT NULL,
    required_ratio      NUMERIC     NOT NULL,
    health_factor       NUMERIC     NOT NULL,
    risk_level          TEXT        NOT NULL CHECK (risk_level IN ('safe', 'warning', 'danger', 'critical')),
    liquidation_price   NUMERIC     NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_position_risk_address       ON position_risk_data (address);
CREATE INDEX IF NOT EXISTS idx_position_risk_asset         ON position_risk_data (asset);
CREATE INDEX IF NOT EXISTS idx_position_risk_risk_level    ON position_risk_data (risk_level);
CREATE INDEX IF NOT EXISTS idx_position_risk_recorded_at   ON position_risk_data (recorded_at DESC);
CREATE INDEX IF NOT EXISTS idx_position_risk_address_asset ON position_risk_data (address, asset);

-- ─── Risk Alerts ──────────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS risk_alerts (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    type                TEXT        NOT NULL CHECK (type IN ('ratio_breach', 'health_factor_low', 'liquidation_imminent')),
    severity            TEXT        NOT NULL CHECK (severity IN ('low', 'medium', 'high', 'critical')),
    address             TEXT        NOT NULL,
    asset               TEXT        NOT NULL,
    message             TEXT        NOT NULL,
    current_value       NUMERIC     NOT NULL,
    threshold_value     NUMERIC     NOT NULL,
    acknowledged        BOOLEAN     NOT NULL DEFAULT FALSE,
    acknowledged_at     TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_risk_alerts_severity       ON risk_alerts (severity);
CREATE INDEX IF NOT EXISTS idx_risk_alerts_type           ON risk_alerts (type);
CREATE INDEX IF NOT EXISTS idx_risk_alerts_asset          ON risk_alerts (asset);
CREATE INDEX IF NOT EXISTS idx_risk_alerts_acknowledged   ON risk_alerts (acknowledged);
CREATE INDEX IF NOT EXISTS idx_risk_alerts_created_at     ON risk_alerts (created_at DESC);

-- ─── Historical Risk Trends ──────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS historical_risk_trends (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset               TEXT        NOT NULL,
    avg_health_factor   NUMERIC     NOT NULL,
    min_health_factor   NUMERIC     NOT NULL,
    max_health_factor   NUMERIC     NOT NULL,
    position_count      INTEGER     NOT NULL,
    danger_count        INTEGER     NOT NULL,
    critical_count      INTEGER     NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_historical_trends_asset       ON historical_risk_trends (asset);
CREATE INDEX IF NOT EXISTS idx_historical_trends_recorded_at ON historical_risk_trends (recorded_at DESC);

-- ─── Asset Risk Metrics ──────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS asset_risk_metrics (
    id                      UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset                   TEXT        NOT NULL,
    total_collateral_value  NUMERIC     NOT NULL,
    total_debt_value        NUMERIC     NOT NULL,
    avg_health_factor       NUMERIC     NOT NULL,
    min_health_factor       NUMERIC     NOT NULL,
    max_health_factor       NUMERIC     NOT NULL,
    position_count          INTEGER     NOT NULL,
    safe_count              INTEGER     NOT NULL,
    warning_count           INTEGER     NOT NULL,
    danger_count            INTEGER     NOT NULL,
    critical_count          INTEGER     NOT NULL,
    recorded_at             TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_asset_metrics_asset       ON asset_risk_metrics (asset);
CREATE INDEX IF NOT EXISTS idx_asset_metrics_recorded_at ON asset_risk_metrics (recorded_at DESC);

-- ─── Risk-adjusted Lending Limits ───────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS risk_adjusted_lending_limits (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    asset               TEXT        NOT NULL,
    base_limit          NUMERIC     NOT NULL,
    risk_adjusted_limit NUMERIC     NOT NULL,
    adjustment_factor   NUMERIC     NOT NULL,
    reason              TEXT        NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_lending_limits_asset       ON risk_adjusted_lending_limits (asset);
CREATE INDEX IF NOT EXISTS idx_lending_limits_recorded_at ON risk_adjusted_lending_limits (recorded_at DESC);

-- ─── Risk Threshold Configuration ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS risk_threshold_config (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    safe_threshold      NUMERIC     NOT NULL DEFAULT 2.0,
    warning_threshold   NUMERIC     NOT NULL DEFAULT 1.5,
    danger_threshold    NUMERIC     NOT NULL DEFAULT 1.1,
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Insert default configuration
INSERT INTO risk_threshold_config (safe_threshold, warning_threshold, danger_threshold)
VALUES (2.0, 1.5, 1.1)
ON CONFLICT DO NOTHING;

-- ─── Cleanup Jobs (Optional: for data retention) ───────────────────────────────

-- Function to delete old collateral ratio snapshots (older than 7 days)
CREATE OR REPLACE FUNCTION cleanup_old_collateral_snapshots()
RETURNS void AS $$
BEGIN
    DELETE FROM collateral_ratio_snapshots
    WHERE recorded_at < NOW() - INTERVAL '7 days';
END;
$$ LANGUAGE plpgsql;

-- Function to delete old position risk data (older than 30 days)
CREATE OR REPLACE FUNCTION cleanup_old_position_risk_data()
RETURNS void AS $$
BEGIN
    DELETE FROM position_risk_data
    WHERE recorded_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Function to delete old historical risk trends (older than 90 days)
CREATE OR REPLACE FUNCTION cleanup_old_historical_trends()
RETURNS void AS $$
BEGIN
    DELETE FROM historical_risk_trends
    WHERE recorded_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;

-- Function to delete acknowledged alerts older than 30 days
CREATE OR REPLACE FUNCTION cleanup_old_acknowledged_alerts()
RETURNS void AS $$
BEGIN
    DELETE FROM risk_alerts
    WHERE acknowledged = true
      AND acknowledged_at < NOW() - INTERVAL '30 days';
END;
$$ LANGUAGE plpgsql;

-- Function to delete old asset risk metrics (older than 90 days)
CREATE OR REPLACE FUNCTION cleanup_old_asset_metrics()
RETURNS void AS $$
BEGIN
    DELETE FROM asset_risk_metrics
    WHERE recorded_at < NOW() - INTERVAL '90 days';
END;
$$ LANGUAGE plpgsql;
