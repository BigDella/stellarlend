export interface Position {
  asset: string;
  collateralValue: number;
  debtValue: number;
  volatility: number;
  weight: number;
}

export interface VaRInput {
  positions: Position[];
  confidenceLevel: number;
  timeHorizon: number;
  portfolioValue: number;
}

export interface VaRResult {
  var: number;
  cvar: number;
  confidenceLevel: number;
  timeHorizon: number;
  method: 'parametric' | 'historical' | 'monte-carlo';
  marginalContributions: MarginalVaR[];
  componentVaR: number[];
  timestamp: number;
}

export interface MarginalVaR {
  asset: string;
  marginalVaR: number;
  componentVaR: number;
  percentContribution: number;
}

export interface HistoricalPrice {
  timestamp: number;
  prices: Record<string, number>;
}

export interface CorrelationPair {
  asset1: string;
  asset2: string;
  correlation: number;
}

export interface VaRHistoryEntry {
  date: string;
  var95: number;
  var99: number;
  cvar95: number;
  cvar99: number;
  portfolioValue: number;
  method: string;
}

export interface StressVaRScenario {
  name: string;
  description: string;
  volatilityMultiplier: number;
  correlationShift: number;
  priceDropPercent: number;
}
