export interface AssetPositionInput {
  asset: string;
  collateral: number;
  debt: number;
  price: number;
  collateralFactorBps?: number;
  liquidationThresholdBps?: number;
  volatilityBps?: number;
}

export interface UnifiedHealthResult {
  totalCollateralValue: number;
  weightedCollateralValue: number;
  totalDebtValue: number;
  weightedDebtValue: number;
  healthFactor: number;
  isLiquidatable: boolean;
  borrowCapacity: number;
  correlationPenaltyBps: number;
  dynamicCollateralFactors: Record<string, number>;
}

export interface PairThreshold {
  debtAsset: string;
  collateralAsset: string;
  thresholdBps: number;
}

export interface ArbitrageOpportunity {
  borrowAsset: string;
  supplyAsset: string;
  spreadBps: number;
}

const BPS = 10_000;
const MIN_CF = 2_000;

const correlation = new Map<string, number>();
const volatility = new Map<string, number>();

function pairKey(a: string, b: string): string {
  return a < b ? `${a}|${b}` : `${b}|${a}`;
}

export function resetCrossAssetRisk(): void {
  correlation.clear();
  volatility.clear();
}

export function setCorrelation(a: string, b: string, bps: number): void {
  if (bps < -BPS || bps > BPS) {
    throw new Error('correlation must be between -10000 and 10000 bps');
  }
  correlation.set(pairKey(a, b), bps);
}

export function getCorrelation(a: string, b: string): number {
  return correlation.get(pairKey(a, b)) ?? 0;
}

export function setVolatility(asset: string, bps: number): void {
  if (bps < 0 || bps > BPS) {
    throw new Error('volatility must be between 0 and 10000 bps');
  }
  volatility.set(asset, bps);
}

export function dynamicCollateralFactor(baseCf: number, asset: string): number {
  const vol = volatility.get(asset) ?? 0;
  const adjusted = Math.floor((baseCf * (BPS - Math.floor(vol / 4))) / BPS);
  return Math.max(MIN_CF, adjusted);
}

export function pairLiquidationThreshold(
  debtLt: number,
  collLt: number,
  debtAsset: string,
  collAsset: string
): number {
  const base = Math.max(debtLt, collLt);
  const boost = Math.floor((Math.abs(getCorrelation(debtAsset, collAsset)) * 500) / BPS);
  return Math.min(base + boost, 9_500);
}

export function computeUnifiedHealth(positions: AssetPositionInput[]): UnifiedHealthResult {
  let totalCollateralValue = 0;
  let weightedCollateralValue = 0;
  let totalDebtValue = 0;
  const dynamicCollateralFactors: Record<string, number> = {};

  for (const p of positions) {
    const collValue = p.collateral * p.price;
    const debtValue = p.debt * p.price;
    const baseCf = p.collateralFactorBps ?? 7_500;
    const lt = p.liquidationThresholdBps ?? 8_000;
    const dynCf = dynamicCollateralFactor(baseCf, p.asset);
    dynamicCollateralFactors[p.asset] = dynCf;
    totalCollateralValue += collValue;
    weightedCollateralValue += (collValue * lt) / BPS;
    totalDebtValue += debtValue;
    if (p.volatilityBps !== undefined) {
      setVolatility(p.asset, p.volatilityBps);
    }
  }

  let corrSum = 0;
  let corrCount = 0;
  for (let i = 0; i < positions.length; i++) {
    for (let j = i + 1; j < positions.length; j++) {
      corrSum += Math.abs(getCorrelation(positions[i]!.asset, positions[j]!.asset));
      corrCount += 1;
    }
  }
  const correlationPenaltyBps = corrCount > 0 ? Math.floor(corrSum / corrCount / 10) : 0;
  weightedCollateralValue = (weightedCollateralValue * (BPS - correlationPenaltyBps)) / BPS;

  const healthFactor = totalDebtValue > 0 ? (weightedCollateralValue * BPS) / totalDebtValue : Number.POSITIVE_INFINITY;
  const isLiquidatable = totalDebtValue > 0 && healthFactor < BPS;
  const borrowCapacity = Math.max(weightedCollateralValue - totalDebtValue, 0);

  return {
    totalCollateralValue,
    weightedCollateralValue,
    totalDebtValue,
    weightedDebtValue: totalDebtValue,
    healthFactor,
    isLiquidatable,
    borrowCapacity,
    correlationPenaltyBps,
    dynamicCollateralFactors,
  };
}

export function detectArbitrage(
  pools: Array<{ asset: string; utilizationBps: number }>
): ArbitrageOpportunity[] {
  const opps: ArbitrageOpportunity[] = [];
  for (const borrow of pools) {
    for (const supply of pools) {
      if (borrow.asset === supply.asset) continue;
      if (supply.utilizationBps > borrow.utilizationBps + 500) {
        opps.push({
          borrowAsset: borrow.asset,
          supplyAsset: supply.asset,
          spreadBps: supply.utilizationBps - borrow.utilizationBps,
        });
      }
    }
  }
  return opps;
}

export function portfolioRiskScore(healthFactor: number): number {
  if (!Number.isFinite(healthFactor) || healthFactor >= 15_000) return 0;
  if (healthFactor <= 5_000) return 10_000;
  return Math.min(10_000, Math.floor(((15_000 - healthFactor) * 10_000) / 10_000));
}
