/**
 * Protocol Health Score Types — Issue #484
 */

export interface HealthScoreComponents {
  capitalEfficiency: number;
  liquidity: number;
  badDebt: number;
  concentration: number;
  oracleHealth: number;
  governanceHealth: number;
}

export interface HealthScoreWeights {
  capitalEfficiency: number;
  liquidity: number;
  badDebt: number;
  concentration: number;
  oracleHealth: number;
  governanceHealth: number;
}

export interface ProtocolHealthScore {
  overallScore: number;
  components: HealthScoreComponents;
  weights: HealthScoreWeights;
  timestamp: string;
}

export interface HealthScoreHistoryPoint {
  timestamp: string;
  overallScore: number;
  components: HealthScoreComponents;
}

export interface HealthScoreAlert {
  threshold: number;
  overallScore: number;
  triggeredAt: string;
  message: string;
}
