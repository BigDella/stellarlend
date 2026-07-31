-- StellarLend Reputation System Schema
-- Issue #606: Add lending pool deployer and user reputation system

-- ─── Deployer Reputation Profiles ─────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS deployer_reputations (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    deployer_address    TEXT        NOT NULL UNIQUE,
    total_pools_created INTEGER     NOT NULL DEFAULT 0,
    total_tvl           NUMERIC     NOT NULL DEFAULT 0,
    active_pools        INTEGER     NOT NULL DEFAULT 0,
    defaults_count      INTEGER     NOT NULL DEFAULT 0,
    abandoned_pools     INTEGER     NOT NULL DEFAULT 0,
    avg_pool_uptime     NUMERIC     NOT NULL DEFAULT 10000,
    reputation_score    INTEGER     NOT NULL DEFAULT 0,
    reputation_tier     TEXT        NOT NULL DEFAULT 'New',
    last_activity_at    TIMESTAMPTZ,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_deployer_reputations_score  ON deployer_reputations (reputation_score DESC);
CREATE INDEX IF NOT EXISTS idx_deployer_reputations_tier   ON deployer_reputations (reputation_tier);

-- ─── Pool Performance Scores ─────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pool_performance_scores (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address        TEXT        NOT NULL UNIQUE,
    uptime_bps          INTEGER     NOT NULL DEFAULT 10000,
    liquidation_events  INTEGER     NOT NULL DEFAULT 0,
    user_satisfaction   NUMERIC     NOT NULL DEFAULT 0,
    avg_apy             NUMERIC,
    total_borrowers     INTEGER     NOT NULL DEFAULT 0,
    performance_score   INTEGER     NOT NULL DEFAULT 0,
    computed_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_pool_perf_scores_pool  ON pool_performance_scores (pool_address);
CREATE INDEX IF NOT EXISTS idx_pool_perf_scores_score ON pool_performance_scores (performance_score DESC);

-- ─── User Reviews ────────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS pool_reviews (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    pool_address        TEXT        NOT NULL,
    reviewer_address    TEXT        NOT NULL,
    rating              INTEGER     NOT NULL CHECK (rating >= 1 AND rating <= 5),
    review_text         TEXT,
    has_deposited       BOOLEAN     NOT NULL DEFAULT FALSE,
    deposit_amount      NUMERIC     NOT NULL DEFAULT 0,
    verified            BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (pool_address, reviewer_address)
);

CREATE INDEX IF NOT EXISTS idx_pool_reviews_pool     ON pool_reviews (pool_address);
CREATE INDEX IF NOT EXISTS idx_pool_reviews_rating   ON pool_reviews (rating DESC);
CREATE INDEX IF NOT EXISTS idx_pool_reviews_verified ON pool_reviews (verified);

-- ─── Reputation Events (decay tracking) ──────────────────────────────────────

CREATE TABLE IF NOT EXISTS reputation_events (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_address      TEXT        NOT NULL,
    event_type          TEXT        NOT NULL,
    score_delta         INTEGER     NOT NULL DEFAULT 0,
    new_score           INTEGER     NOT NULL,
    metadata            JSONB,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_rep_events_entity    ON reputation_events (entity_address);
CREATE INDEX IF NOT EXISTS idx_rep_events_type      ON reputation_events (event_type);
CREATE INDEX IF NOT EXISTS idx_rep_events_recorded  ON reputation_events (recorded_at DESC);

-- ─── Bad Actor Flags ─────────────────────────────────────────────────────────

CREATE TABLE IF NOT EXISTS bad_actor_flags (
    id                  UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_address      TEXT        NOT NULL,
    flag_reason         TEXT        NOT NULL,
    flagged_by          TEXT        NOT NULL DEFAULT 'system',
    resolved            BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at          TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at         TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_bad_actor_flags_entity   ON bad_actor_flags (entity_address);
CREATE INDEX IF NOT EXISTS idx_bad_actor_flags_resolved ON bad_actor_flags (resolved);