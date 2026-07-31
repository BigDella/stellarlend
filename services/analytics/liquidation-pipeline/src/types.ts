export interface LiquidationEvent {
  ledger: number;
  txHash: string;
  timestamp: Date;
  liquidator: string;
  borrower: string;
  debtAsset: string | null;
  collateralAsset: string | null;
  debtLiquidated: number;
  collateralSeized: number;
  incentiveAmount: number;
  /** Optional market context */
  debtAssetPrice?: number;
  collateralAssetPrice?: number;
  gasCost?: number;
}

export interface LiquidationMetrics {
  txHash: string;
  timestamp: Date;
  discount: number;
  profit: number;
  gasCost: number;
  netProfit: number;
  hourOfDay: number;
  dayOfWeek: number;
  collateralAsset: string;
  debtAsset: string;
  debtLiquidated: number;
  collateralSeized: number;
}

export interface ProfitabilityDistribution {
  count: number;
  meanProfit: number;
  medianProfit: number;
  p25: number;
  p75: number;
  p95: number;
  profitableShare: number;
}

export interface TimeClusterBucket {
  key: string;
  count: number;
}

export interface CollateralFrequency {
  asset: string;
  count: number;
  share: number;
}

export interface Anomaly {
  txHash: string;
  reason: string;
  score: number;
  metrics: LiquidationMetrics;
}

export interface LiquidationReport {
  period: 'daily' | 'weekly' | 'monthly';
  from: string;
  to: string;
  generatedAt: string;
  totalLiquidations: number;
  profitability: ProfitabilityDistribution;
  hourOfDay: TimeClusterBucket[];
  dayOfWeek: TimeClusterBucket[];
  collateralFrequency: CollateralFrequency[];
  anomalies: Anomaly[];
}
