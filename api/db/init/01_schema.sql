-- StellarLend – initial schema
-- Applied automatically by postgres on first container start.

CREATE TABLE IF NOT EXISTS api_keys (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    prefix        VARCHAR(8)   NOT NULL,
    hash          TEXT         NOT NULL,
    name          VARCHAR(255),
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    last_used_at  TIMESTAMPTZ,
    revoked_at    TIMESTAMPTZ,
    created_by    TEXT
);

CREATE INDEX IF NOT EXISTS idx_api_keys_prefix ON api_keys (prefix);

CREATE TABLE IF NOT EXISTS audit_logs (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sequence      BIGINT       NOT NULL,
    action        TEXT         NOT NULL,
    actor         TEXT         NOT NULL,
    status        TEXT         NOT NULL,
    tx_hash       TEXT,
    ledger        BIGINT,
    amount        TEXT,
    asset_address TEXT,
    ip            TEXT,
    prev_hash     TEXT         NOT NULL,
    hash          TEXT         NOT NULL,
    timestamp     TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_actor  ON audit_logs (actor);
CREATE INDEX IF NOT EXISTS idx_audit_logs_action ON audit_logs (action);

-- ============================================================================
-- Referral Program Tables (Issue #579)
-- ============================================================================

CREATE TABLE IF NOT EXISTS referral_codes (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address  TEXT         NOT NULL UNIQUE,
    code          VARCHAR(8)   NOT NULL UNIQUE,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_referral_codes_code ON referral_codes (code);

CREATE TABLE IF NOT EXISTS referral_rewards (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_address TEXT      NOT NULL,
    referee_address TEXT       NOT NULL,
    fee_amount    NUMERIC      NOT NULL DEFAULT 0,
    reward_amount NUMERIC      NOT NULL DEFAULT 0,
    claimed       BOOLEAN      NOT NULL DEFAULT FALSE,
    created_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    claimed_at    TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_referral_rewards_referrer ON referral_rewards (referrer_address);

CREATE TABLE IF NOT EXISTS referral_tiers (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_address  TEXT         NOT NULL UNIQUE,
    tier_level    INTEGER      NOT NULL DEFAULT 0,
    total_referrals INTEGER    NOT NULL DEFAULT 0,
    total_earned  NUMERIC      NOT NULL DEFAULT 0,
    updated_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

-- ============================================================================
-- SNS Integration Tables (Issue #580)
-- ============================================================================

CREATE TABLE IF NOT EXISTS sns_names (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name          TEXT         NOT NULL UNIQUE,
    address       TEXT         NOT NULL,
    owner         TEXT         NOT NULL,
    registered_at TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ  NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_sns_names_owner ON sns_names (owner);
CREATE INDEX IF NOT EXISTS idx_sns_names_address ON sns_names (address);

-- ============================================================================
-- Emergency Event Tables (Issue #582)
-- ============================================================================

CREATE TABLE IF NOT EXISTS emergency_events (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_type    TEXT         NOT NULL,
    trigger_reason TEXT,
    started_at    TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    window_opens_at TIMESTAMPTZ,
    window_closes_at TIMESTAMPTZ,
    resolved_at   TIMESTAMPTZ,
    metadata      JSONB
);

CREATE TABLE IF NOT EXISTS emergency_withdrawals (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id      UUID         NOT NULL REFERENCES emergency_events(id),
    user_address  TEXT         NOT NULL,
    asset_address TEXT,
    amount        NUMERIC      NOT NULL,
    loss_share_bps INTEGER     NOT NULL DEFAULT 0,
    amount_after_loss NUMERIC  NOT NULL,
    withdrawn_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_emergency_withdrawals_user ON emergency_withdrawals (user_address);
CREATE INDEX IF NOT EXISTS idx_emergency_withdrawals_event ON emergency_withdrawals (event_id);
