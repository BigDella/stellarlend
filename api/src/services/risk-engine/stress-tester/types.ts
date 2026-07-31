export interface StressScenario {
  id: string;
  name: string;
  description: string;
  category: 'historical' | 'custom' | 'hypothetical';
  version: string;
  created: string;
  author?: string;
  priceChanges: AssetPriceChange[];
  correlationShifts: CorrelationShift[];
  volatilityMultipliers: AssetVolatilityMultiplier[];
  cascadingLiquidation?: boolean;
  durationSteps?: number;
  tags: string[];
}

export interface AssetPriceChange {
  asset: string;
  changePercent: number;
  step?: number;
}

export interface CorrelationShift {
  asset1: string;
  asset2: string;
  newCorrelation: number;
}

export interface AssetVolatilityMultiplier {
  asset: string;
  multiplier: number;
}

export interface PositionSnapshot {
  user: string;
  pool: string;
  collateral: { asset: string; amount: number; price: number }[];
  borrow: { asset: string; amount: number; price: number }[];
  healthFactor: number;
}

export interface StressTestInput {
  scenario: StressScenario;
  positions: PositionSnapshot[];
  totalCollateralValue: number;
  totalDebtValue: number;
  protocolLiquidity: number;
  riskThresholds?: RiskThresholds;
  parallel?: boolean;
}

export interface RiskThresholds {
  healthFactorMin: number;
  maxBadDebtRatio: number;
  maxLiquidationCascade: number;
  solvencyRatioMin: number;
}

export interface StressTestResult {
  scenarioId: string;
  scenarioName: string;
  executedAt: number;
  durationMs: number;
  summary: StressSummary;
  affectedPositions: AffectedPosition[];
  waterfallAnalysis: WaterfallStep[];
  cascadingLiquidationImpact?: CascadingLiquidationResult;
  recommendations: string[];
  passed: boolean;
}

export interface StressSummary {
  totalShortfall: number;
  badDebtTotal: number;
  protocolInsolvent: boolean;
  solvencyRatio: number;
  totalPositionsAffected: number;
  totalLiquidationsTriggered: number;
  worstCaseRecovery: number;
  riskLevel: 'LOW' | 'MEDIUM' | 'HIGH' | 'CRITICAL';
}

export interface AffectedPosition {
  user: string;
  previousHealthFactor: number;
  afterScenarioHealthFactor: number;
  liquidatedCollateral: number;
  remainingDebt: number;
  shortfall: number;
  isInsolvent: boolean;
}

export interface WaterfallStep {
  step: number;
  priceLevel: number;
  liquidationsTriggered: number;
  cumulativeShortfall: number;
  remainingLiquidity: number;
  protocolSolvent: boolean;
}

export interface CascadingLiquidationResult {
  totalCascadeRounds: number;
  totalLiquidatedValue: number;
  totalShortfall: number;
  affectedUsers: string[];
  maxCascadeDepth: number;
}

export interface StressTestReport {
  id: string;
  timestamp: number;
  scenario: StressScenario;
  result: StressTestResult;
  ciMetadata?: CiMetadata;
}

export interface CiMetadata {
  commitSha: string;
  branch: string;
  parameterChanges?: string[];
  triggeredBy: string;
}
