/**
 * Risk Engine Types
 * Covers issues #450 – #453: correlation matrix, dynamic liquidation thresholds,
 * concentration risk monitoring, and risk-adjusted collateral ratios.
 */

// ─── #450 Correlation Matrix ─────────────────────────────────────────────────

export type CorrelationWindow = 30 | 60 | 90;

export interface PricePoint {
  timestamp: number; // Unix ms
  price: number;
}

export interface AssetPairCorrelation {
  assetA: string;
  assetB: string;
  windowDays: CorrelationWindow;
  pearson: number;   // -1 to 1
  spearman: number;  // -1 to 1
  sampleCount: number;
  computedAt: string; // ISO-8601
  isHighlyCorrelated: boolean; // |pearson| > threshold (default 0.8)
}

export interface CorrelationMatrix {
  assets: string[];
  windowDays: CorrelationWindow;
  matrix: Record<string, Record<string, number>>; // [assetA][assetB] = pearson
  spearmanMatrix: Record<string, Record<string, number>>;
  computedAt: string;
  highCorrelationPairs: Array<{ assetA: string; assetB: string; pearson: number }>;
}

export interface CorrelationHistoryPoint {
  computedAt: string;
  pearson: number;
  spearman: number;
}

export interface PositionCorrelationRisk {
  userAddress: string;
  collateralAssets: string[];
  averageCorrelation: number;
  maxPairCorrelation: number;
  healthFactorAdjustment: number; // multiplier, e.g. 0.95 means 5% penalty
  warnings: string[];
}

export interface CorrelationAlertConfig {
  threshold: number; // default 0.8
  windowDays: CorrelationWindow;
}

// ─── #451 Dynamic Liquidation Thresholds ────────────────────────────────────

export type VolatilityWindow = 5 | 20;

export interface AssetVolatility {
  asset: string;
  windowDays: VolatilityWindow;
  realizedVol: number;  // annualised decimal, e.g. 0.45 = 45%
  computedAt: string;
}

export interface LtvAdjustment {
  asset: string;
  baseLtv: number;          // basis points, e.g. 8000 = 80%
  volatilityPremium: number; // basis points to subtract
  adjustedLtv: number;       // clamped to [5000, 9000]
  lockedUntil: string;       // 24h timelock ISO-8601
  isGovernanceOverride: boolean;
  computedAt: string;
}

export interface VolatilityAdjustedLtvResponse {
  asset: string;
  currentLtv: number;
  ltv5d: LtvAdjustment;
  ltv20d: LtvAdjustment;
  recommendedLtv: number; // conservative: min of the two
}

export interface LtvAdjustmentHistory {
  asset: string;
  history: Array<{
    adjustedLtv: number;
    volatilityPremium: number;
    computedAt: string;
  }>;
}

// ─── #452 Concentration Risk ─────────────────────────────────────────────────

export interface ConcentrationMetrics {
  asset: string;
  hhi: number;               // 0-10000 (Herfindahl-Hirschman Index × 10000)
  top5Pct: number;           // percentage 0-100
  top10Pct: number;
  totalPositions: number;
  tvl: number;
  largestPositionPct: number;
  snapshotAt: string;
}

export interface ConcentrationAlert {
  id: string;
  asset: string;
  address: string;
  positionPct: number;
  thresholdPct: number;
  enforcement: 'soft' | 'hard';
  alertedAt: string;
  resolvedAt?: string;
}

export interface ConcentrationConfig {
  maxSinglePositionPct: number; // default 10
  softCapMultiplier: number;    // fee multiplier for positions > soft cap
  hardCapEnabled: boolean;
}

export interface ConcentrationHistoryPoint {
  snapshotAt: string;
  hhi: number;
  top5Pct: number;
  top10Pct: number;
  tvl: number;
}

export interface ConcentrationDashboard {
  assets: ConcentrationMetrics[];
  globalHhi: number;
  totalAlerts: number;
  recentAlerts: ConcentrationAlert[];
  history: ConcentrationHistoryPoint[];
}

// ─── #453 Risk-Adjusted Collateral Ratio ────────────────────────────────────

export interface CollateralRatioFactors {
  baseRatio: number;         // basis points, e.g. 15000 = 150%
  volatilityFactor: number;  // additive basis points
  liquidityFactor: number;
  correlationFactor: number;
  finalRatio: number;        // sum, clamped
  computedAt: string;
}

export interface CollateralRatioResponse {
  asset: string;
  factors: CollateralRatioFactors;
  recommendation: string;
}

export interface RatioFactorWeights {
  volatilityWeight: number;   // 0-1
  liquidityWeight: number;
  correlationWeight: number;
}

export interface BacktestRequest {
  asset: string;
  startDate: string;  // ISO-8601
  endDate: string;
  proposedRatio: number; // basis points
}

export interface BacktestResult {
  asset: string;
  proposedRatio: number;
  periodStart: string;
  periodEnd: string;
  liquidationEvents: number;
  badDebtEvents: number;
  wouldHavePrevented: number;
  minSafeRatio: number;    // minimum ratio that would have prevented all bad debt
  recommendation: string;
}

export interface CollateralRatioHistory {
  asset: string;
  history: Array<{
    computedAt: string;
    finalRatio: number;
    volatilityFactor: number;
    liquidityFactor: number;
    correlationFactor: number;
  }>;
}
