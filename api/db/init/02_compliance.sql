CREATE TABLE IF NOT EXISTS sanctions_list (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address       TEXT NOT NULL,
    source        TEXT NOT NULL,
    reason        TEXT NOT NULL,
    sanctioned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ,
    active        BOOLEAN NOT NULL DEFAULT TRUE,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sanctions_address ON sanctions_list (address);
CREATE INDEX IF NOT EXISTS idx_sanctions_active ON sanctions_list (active);

CREATE TABLE IF NOT EXISTS kyc_verifications (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address       TEXT NOT NULL,
    verified      BOOLEAN NOT NULL DEFAULT FALSE,
    tier          INTEGER NOT NULL DEFAULT 0,
    verified_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ NOT NULL,
    jurisdiction  TEXT NOT NULL,
    kyc_provider  TEXT NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_kyc_address ON kyc_verifications (address);
CREATE INDEX IF NOT EXISTS idx_kyc_jurisdiction ON kyc_verifications (jurisdiction);

CREATE TABLE IF NOT EXISTS compliance_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type    TEXT NOT NULL,
    address       TEXT NOT NULL,
    amount        NUMERIC,
    asset_address TEXT,
    details       TEXT,
    timestamp     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_compliance_events_type ON compliance_events (event_type);
CREATE INDEX IF NOT EXISTS idx_compliance_events_address ON compliance_events (address);
CREATE INDEX IF NOT EXISTS idx_compliance_events_timestamp ON compliance_events (timestamp);

CREATE TABLE IF NOT EXISTS suspicious_activity_reports (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sar_id        BIGINT NOT NULL,
    address       TEXT NOT NULL,
    reason        TEXT NOT NULL,
    amount        NUMERIC NOT NULL,
    asset_address TEXT NOT NULL,
    filed_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    filed_by      TEXT NOT NULL,
    status        TEXT NOT NULL DEFAULT 'filed',
    notes         TEXT,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_sar_address ON suspicious_activity_reports (address);
CREATE INDEX IF NOT EXISTS idx_sar_status ON suspicious_activity_reports (status);
CREATE UNIQUE INDEX IF NOT EXISTS idx_sar_unique_id ON suspicious_activity_reports (sar_id);

CREATE TABLE IF NOT EXISTS transaction_volume (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    address         TEXT NOT NULL,
    daily_volume    NUMERIC NOT NULL DEFAULT 0,
    weekly_volume   NUMERIC NOT NULL DEFAULT 0,
    last_tx_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(address)
);

CREATE INDEX IF NOT EXISTS idx_tx_volume_address ON transaction_volume (address);

CREATE TABLE IF NOT EXISTS restricted_jurisdictions (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    jurisdiction  TEXT NOT NULL UNIQUE,
    added_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    added_by      TEXT NOT NULL
);
