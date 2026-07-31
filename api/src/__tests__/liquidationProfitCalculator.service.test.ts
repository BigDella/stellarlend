import { calculateLiquidationProfitability } from '../services/liquidationProfitCalculator.service';
import { ValidationError } from '../utils/errors';

describe('liquidationProfitCalculator.service', () => {
  const basePosition = {
    collateralAsset: 'XLM',
    collateralValueUsd: 100_000,
    debtAsset: 'USDC',
    debtValueUsd: 80_000,
    liquidationDiscountBps: 500,
    protocolFeeBps: 1000,
  };

  it('computes profit, ROI, and gas cost for a full single-collateral liquidation', () => {
    const result = calculateLiquidationProfitability({
      positions: [basePosition],
      gasPriceStroops: 500_000,
    });

    expect(result.positions).toHaveLength(1);
    expect(result.totalRepaidDebtUsd).toBe(80_000);
    // 80000 * 1.05 = 84000 seized, bonus = 4000, fee = 400 -> gross = 3600
    expect(result.positions[0]!.collateralSeizedUsd).toBeCloseTo(84_000);
    expect(result.positions[0]!.liquidationBonusUsd).toBeCloseTo(4_000);
    expect(result.grossProfitUsd).toBeCloseTo(3_600);
    expect(result.gasCostStroops).toBe(500_000);
    expect(result.netProfitUsd).toBeCloseTo(result.grossProfitUsd - result.gasCostUsd);
    expect(result.isProfitable).toBe(true);
    expect(result.roiPercent).toBeGreaterThan(0);
  });

  it('supports partial liquidation via repayPercentBps', () => {
    const full = calculateLiquidationProfitability({
      positions: [basePosition],
      gasPriceStroops: 100,
    });
    const partial = calculateLiquidationProfitability({
      positions: [basePosition],
      gasPriceStroops: 100,
      repayPercentBps: 5000,
    });

    expect(partial.totalRepaidDebtUsd).toBeCloseTo(full.totalRepaidDebtUsd / 2);
    expect(partial.grossProfitUsd).toBeCloseTo(full.grossProfitUsd / 2, 1);
  });

  it('caps seized collateral at the available collateral value', () => {
    const result = calculateLiquidationProfitability({
      positions: [
        {
          ...basePosition,
          collateralValueUsd: 81_000,
          liquidationDiscountBps: 5000,
        },
      ],
      gasPriceStroops: 0,
    });

    expect(result.positions[0]!.collateralSeizedUsd).toBe(81_000);
  });

  it('aggregates a multi-collateral basket liquidation', () => {
    const result = calculateLiquidationProfitability({
      positions: [basePosition, { ...basePosition, collateralAsset: 'BTC', debtAsset: 'USDT' }],
      gasPriceStroops: 100_000,
    });

    expect(result.positions).toHaveLength(2);
    expect(result.gasCostStroops).toBe(200_000);
    expect(result.totalRepaidDebtUsd).toBeCloseTo(160_000);
  });

  it('applies price impact for large liquidations relative to pool liquidity', () => {
    const shallow = calculateLiquidationProfitability({
      positions: [{ ...basePosition, poolLiquidityUsd: 50_000 }],
      gasPriceStroops: 0,
    });
    const deep = calculateLiquidationProfitability({
      positions: [{ ...basePosition, poolLiquidityUsd: 50_000_000 }],
      gasPriceStroops: 0,
    });

    expect(shallow.positions[0]!.priceImpactUsd).toBeGreaterThan(0);
    expect(shallow.grossProfitUsd).toBeLessThan(deep.grossProfitUsd);
  });

  it('flags negative-profit (unliquidatable) scenarios', () => {
    const result = calculateLiquidationProfitability({
      positions: [{ ...basePosition, liquidationDiscountBps: 10, protocolFeeBps: 9000 }],
      gasPriceStroops: 1_000_000_000,
    });

    expect(result.netProfitUsd).toBeLessThan(0);
    expect(result.isProfitable).toBe(false);
  });

  it('flags zero-profit liquidations as not meeting a positive desired margin', () => {
    const result = calculateLiquidationProfitability({
      positions: [{ ...basePosition, liquidationDiscountBps: 0, protocolFeeBps: 0 }],
      gasPriceStroops: 0,
    });

    expect(result.netProfitUsd).toBe(0);
    expect(result.isProfitable).toBe(false);
  });

  it('respects an explicit desiredProfitMarginBps threshold', () => {
    const result = calculateLiquidationProfitability({
      positions: [basePosition],
      gasPriceStroops: 500_000,
      desiredProfitMarginBps: 100_000,
    });

    expect(result.meetsDesiredMargin).toBe(false);
  });

  it('rejects an empty positions array', () => {
    expect(() =>
      calculateLiquidationProfitability({ positions: [], gasPriceStroops: 0 })
    ).toThrow(ValidationError);
  });

  it('rejects out-of-range basis-point fields', () => {
    expect(() =>
      calculateLiquidationProfitability({
        positions: [{ ...basePosition, liquidationDiscountBps: 20_000 }],
        gasPriceStroops: 0,
      })
    ).toThrow(ValidationError);
  });
});
