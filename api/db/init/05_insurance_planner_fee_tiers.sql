CREATE TABLE IF NOT EXISTS insurance_providers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), address TEXT UNIQUE NOT NULL, name TEXT NOT NULL,
  kyc_status TEXT NOT NULL CHECK (kyc_status IN ('pending','approved','rejected')),
  collateral NUMERIC NOT NULL CHECK (collateral > 0), available_collateral NUMERIC NOT NULL,
  rating NUMERIC NOT NULL DEFAULT 0, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS insurance_policies (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), provider_id UUID NOT NULL REFERENCES insurance_providers(id),
  coverage_amount NUMERIC NOT NULL, premium_bps INTEGER NOT NULL, duration_days INTEGER NOT NULL,
  terms TEXT NOT NULL, covered_triggers JSONB NOT NULL, exclusions JSONB NOT NULL, active BOOLEAN NOT NULL DEFAULT TRUE
);
CREATE TABLE IF NOT EXISTS insurance_coverages (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), policy_id UUID NOT NULL REFERENCES insurance_policies(id),
  lender TEXT NOT NULL, position_id TEXT NOT NULL, coverage_amount NUMERIC NOT NULL, premium_paid NUMERIC NOT NULL,
  starts_at TIMESTAMPTZ NOT NULL, expires_at TIMESTAMPTZ NOT NULL
);
CREATE TABLE IF NOT EXISTS insurance_claims (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), coverage_id UUID NOT NULL REFERENCES insurance_coverages(id),
  trigger TEXT NOT NULL, evidence TEXT NOT NULL, amount NUMERIC NOT NULL, status TEXT NOT NULL,
  submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(), resolved_at TIMESTAMPTZ
);
CREATE TABLE IF NOT EXISTS budget_plans (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), lender TEXT NOT NULL, capital NUMERIC NOT NULL,
  horizon_days INTEGER NOT NULL, goal_amount NUMERIC, plan JSONB NOT NULL, created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE TABLE IF NOT EXISTS fee_tiers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(), name TEXT UNIQUE NOT NULL, min_deposits NUMERIC NOT NULL,
  min_borrow_volume NUMERIC NOT NULL, min_account_days INTEGER NOT NULL, min_loyal_days INTEGER NOT NULL,
  discount_bps INTEGER NOT NULL CHECK (discount_bps BETWEEN 0 AND 5000), loyalty_bonus_bps INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS user_fee_tiers (
  user_address TEXT PRIMARY KEY, tier_id UUID NOT NULL REFERENCES fee_tiers(id), total_savings NUMERIC NOT NULL DEFAULT 0,
  evaluated_at TIMESTAMPTZ NOT NULL, effective_at TIMESTAMPTZ NOT NULL
);
