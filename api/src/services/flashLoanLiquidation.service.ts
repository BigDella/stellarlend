import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';

export interface FlashLoanLegInput {
  asset: string;
  amount: number;
}

export interface FlashLoanLiquidationSim {
  debtAmount: number;
  flashFee: number;
  collateralSeized: number;
  incentiveAmount: number;
  estimatedProfit: number;
  profitable: boolean;
  gasUnitsEstimate: number;
  rollbackOnUnprofitable: true;
}

export interface ComboExecutionResult extends FlashLoanLiquidationSim {
  status: 'simulated' | 'submitted' | 'rejected';
  reason?: string;
}

const BPS = 10_000;
const DEFAULT_FEE_BPS = 9;
const DEFAULT_INCENTIVE_BPS = 1_000;
const COMBO_GAS_UNITS = 80_000;
const MULTI_LEG_GAS = 12_000;
const MAX_LEGS = 5;

export function simulateFlashLoanLiquidation(input: {
  debtAmount: number;
  debtPrice?: number;
  collateralPrice?: number;
  feeBps?: number;
  incentiveBps?: number;
}): FlashLoanLiquidationSim {
  const debtAmount = input.debtAmount;
  const feeBps = input.feeBps ?? DEFAULT_FEE_BPS;
  const incentiveBps = input.incentiveBps ?? DEFAULT_INCENTIVE_BPS;
  const debtPrice = input.debtPrice ?? 1;
  const collateralPrice = input.collateralPrice ?? 1;

  const flashFee = (debtAmount * feeBps) / BPS;
  const incentiveAmount = (debtAmount * incentiveBps) / BPS;
  const debtValue = debtAmount * debtPrice;
  const seizedValue = debtValue * (1 + incentiveBps / BPS);
  const collateralSeized = collateralPrice > 0 ? seizedValue / collateralPrice : 0;
  const estimatedProfit = seizedValue - debtValue - flashFee * debtPrice;

  return {
    debtAmount,
    flashFee,
    collateralSeized,
    incentiveAmount,
    estimatedProfit,
    profitable: estimatedProfit > 0,
    gasUnitsEstimate: COMBO_GAS_UNITS,
    rollbackOnUnprofitable: true,
  };
}

export function simulateMultiAssetFlashLoan(legs: FlashLoanLegInput[], feeBps = DEFAULT_FEE_BPS) {
  if (legs.length === 0) {
    return { ok: false as const, error: 'empty_legs', totalFee: 0, gasUnitsEstimate: 0 };
  }
  if (legs.length > MAX_LEGS) {
    return { ok: false as const, error: 'too_many_legs', totalFee: 0, gasUnitsEstimate: 0 };
  }
  const totalFee = legs.reduce((sum, leg) => sum + (leg.amount * feeBps) / BPS, 0);
  return {
    ok: true as const,
    error: null,
    totalFee,
    gasUnitsEstimate: COMBO_GAS_UNITS + legs.length * MULTI_LEG_GAS,
    legs,
  };
}

export async function prepareComboExecution(input: {
  liquidator: string;
  borrower: string;
  debtAsset: string;
  collateralAsset: string;
  debtAmount: number;
  debtPrice?: number;
  collateralPrice?: number;
}): Promise<ComboExecutionResult> {
  const sim = simulateFlashLoanLiquidation(input);
  if (!sim.profitable) {
    return { ...sim, status: 'rejected', reason: 'unprofitable' };
  }
  const stellar = new StellarService();
  try {
    await stellar.getPoolStateAt(input.debtAsset, Math.floor(Date.now() / 1000));
  } catch {
    // Simulation still stands even if live pool lookup is unavailable.
  }
  await redisCacheService.set(
    redisCacheService.buildKey('pool', `flash-liq:${input.borrower}`),
    sim,
    30
  );
  return { ...sim, status: 'simulated' };
}
