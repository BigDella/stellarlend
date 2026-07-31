import { ValidationError } from '../utils/errors';

/** Simulated XLM/USD price used to convert stroop-denominated gas costs to USD. */
const SIMULATED_XLM_PRICE_USD = 0.12;
const STROOPS_PER_XLM = 10_000_000;
const BPS_DENOMINATOR = 10_000;

export interface LiquidationPositionInput {
  collateralAsset: string;
  /** Value of the collateral available to seize, in USD. */
  collateralValueUsd: number;
  debtAsset: string;
  /** Outstanding debt value, in USD. */
  debtValueUsd: number;
  /** Liquidation bonus/discount the liquidator receives, in basis points (500 = 5%). */
  liquidationDiscountBps: number;
  /** Share of the liquidation bonus taken by the protocol, in basis points. */
  protocolFeeBps: number;
  /** Available on-chain/DEX liquidity for the collateral asset, in USD — used for price-impact estimation. */
  poolLiquidityUsd?: number;
}

export interface ProfitabilityRequest {
  /** One entry per collateral asset. A single-collateral liquidation is an array of length 1. */
  positions: LiquidationPositionInput[];
  /** Stellar network fee, in stroops, for a single liquidation operation. */
  gasPriceStroops: number;
  /** Minimum acceptable ROI, in basis points, used to flag whether the opportunity clears the desired margin. */
  desiredProfitMarginBps?: number;
  /** Fraction of the debt being repaid, in basis points. 10000 = full liquidation. Defaults to full. */
  repayPercentBps?: number;
  /** Extra slippage assumption applied to seized collateral, in basis points. */
  slippageBufferBps?: number;
  xlmPriceUsd?: number;
}

export interface PositionProfitBreakdown {
  collateralAsset: string;
  debtAsset: string;
  repaidDebtUsd: number;
  collateralSeizedUsd: number;
  liquidationBonusUsd: number;
  protocolFeeUsd: number;
  priceImpactUsd: number;
  slippageUsd: number;
  positionProfitUsd: number;
}

export interface ProfitabilityResult {
  positions: PositionProfitBreakdown[];
  repayPercentBps: number;
  totalRepaidDebtUsd: number;
  grossProfitUsd: number;
  gasCostStroops: number;
  gasCostUsd: number;
  netProfitUsd: number;
  roiPercent: number;
  isProfitable: boolean;
  meetsDesiredMargin: boolean;
}

function validateRequest(req: ProfitabilityRequest): void {
  if (!Array.isArray(req.positions) || req.positions.length === 0) {
    throw new ValidationError('positions must be a non-empty array');
  }
  if (typeof req.gasPriceStroops !== 'number' || req.gasPriceStroops < 0) {
    throw new ValidationError('gasPriceStroops must be a non-negative number');
  }
  for (const bps of [req.repayPercentBps, req.slippageBufferBps]) {
    if (bps !== undefined && (bps < 0 || bps > BPS_DENOMINATOR)) {
      throw new ValidationError('repayPercentBps and slippageBufferBps must be between 0 and 10000');
    }
  }
  if (req.desiredProfitMarginBps !== undefined && req.desiredProfitMarginBps < 0) {
    throw new ValidationError('desiredProfitMarginBps must be non-negative');
  }

  req.positions.forEach((position, index) => {
    if (!position.collateralAsset || !position.debtAsset) {
      throw new ValidationError(`positions[${index}]: collateralAsset and debtAsset are required`);
    }
    if (position.collateralValueUsd < 0 || position.debtValueUsd < 0) {
      throw new ValidationError(`positions[${index}]: values must be non-negative`);
    }
    if (position.liquidationDiscountBps < 0 || position.liquidationDiscountBps > BPS_DENOMINATOR) {
      throw new ValidationError(`positions[${index}]: liquidationDiscountBps must be between 0 and 10000`);
    }
    if (position.protocolFeeBps < 0 || position.protocolFeeBps > BPS_DENOMINATOR) {
      throw new ValidationError(`positions[${index}]: protocolFeeBps must be between 0 and 10000`);
    }
  });
}

/**
 * Simple concave price-impact model: impact grows with the square root of the
 * seized-value-to-liquidity ratio, so doubling trade size doesn't double impact.
 */
function estimatePriceImpactUsd(seizedValueUsd: number, poolLiquidityUsd?: number): number {
  if (!poolLiquidityUsd || poolLiquidityUsd <= 0) return 0;
  const ratio = seizedValueUsd / poolLiquidityUsd;
  const impactFraction = Math.min(1, Math.sqrt(ratio) * 0.1);
  return seizedValueUsd * impactFraction;
}

function calculatePosition(
  position: LiquidationPositionInput,
  repayPercentBps: number,
  slippageBufferBps: number
): PositionProfitBreakdown {
  const repaidDebtUsd = (position.debtValueUsd * repayPercentBps) / BPS_DENOMINATOR;

  const uncappedSeizedUsd =
    repaidDebtUsd * (1 + position.liquidationDiscountBps / BPS_DENOMINATOR);
  const collateralSeizedUsd = Math.min(uncappedSeizedUsd, position.collateralValueUsd);

  const liquidationBonusUsd = Math.max(0, collateralSeizedUsd - repaidDebtUsd);
  const protocolFeeUsd = (liquidationBonusUsd * position.protocolFeeBps) / BPS_DENOMINATOR;
  const priceImpactUsd = estimatePriceImpactUsd(collateralSeizedUsd, position.poolLiquidityUsd);
  const slippageUsd = (collateralSeizedUsd * slippageBufferBps) / BPS_DENOMINATOR;

  const positionProfitUsd = liquidationBonusUsd - protocolFeeUsd - priceImpactUsd - slippageUsd;

  return {
    collateralAsset: position.collateralAsset,
    debtAsset: position.debtAsset,
    repaidDebtUsd,
    collateralSeizedUsd,
    liquidationBonusUsd,
    protocolFeeUsd,
    priceImpactUsd,
    slippageUsd,
    positionProfitUsd,
  };
}

export function calculateLiquidationProfitability(req: ProfitabilityRequest): ProfitabilityResult {
  validateRequest(req);

  const repayPercentBps = req.repayPercentBps ?? BPS_DENOMINATOR;
  const slippageBufferBps = req.slippageBufferBps ?? 0;
  const xlmPriceUsd = req.xlmPriceUsd ?? SIMULATED_XLM_PRICE_USD;

  const positions = req.positions.map((position) =>
    calculatePosition(position, repayPercentBps, slippageBufferBps)
  );

  const totalRepaidDebtUsd = positions.reduce((sum, p) => sum + p.repaidDebtUsd, 0);
  const grossProfitUsd = positions.reduce((sum, p) => sum + p.positionProfitUsd, 0);

  const gasCostStroops = req.gasPriceStroops * req.positions.length;
  const gasCostUsd = (gasCostStroops / STROOPS_PER_XLM) * xlmPriceUsd;

  const netProfitUsd = grossProfitUsd - gasCostUsd;
  const roiPercent = totalRepaidDebtUsd > 0 ? (netProfitUsd / totalRepaidDebtUsd) * 100 : 0;

  const isProfitable = netProfitUsd > 0;
  const meetsDesiredMargin =
    req.desiredProfitMarginBps !== undefined
      ? roiPercent >= req.desiredProfitMarginBps / 100
      : isProfitable;

  return {
    positions,
    repayPercentBps,
    totalRepaidDebtUsd,
    grossProfitUsd,
    gasCostStroops,
    gasCostUsd,
    netProfitUsd,
    roiPercent,
    isProfitable,
    meetsDesiredMargin,
  };
}
