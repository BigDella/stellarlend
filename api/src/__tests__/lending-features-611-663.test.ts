import {
  simulateFlashLoanLiquidation,
  simulateMultiAssetFlashLoan,
} from '../services/flashLoanLiquidation.service';
import { simulateProposal, getSharedSimulation, resetSimulationCache } from '../services/governanceSimulation.service';
import {
  computeUnifiedHealth,
  detectArbitrage,
  pairLiquidationThreshold,
  resetCrossAssetRisk,
  setCorrelation,
  setVolatility,
  dynamicCollateralFactor,
} from '../services/crossAssetRisk.service';

describe('flash loan liquidation combo (#661)', () => {
  it('flags profitable combos when incentive exceeds flash fee', () => {
    const sim = simulateFlashLoanLiquidation({ debtAmount: 1_000_000, feeBps: 9, incentiveBps: 1000 });
    expect(sim.profitable).toBe(true);
    expect(sim.estimatedProfit).toBeGreaterThan(0);
    expect(sim.rollbackOnUnprofitable).toBe(true);
  });

  it('rejects unprofitable combos', () => {
    const sim = simulateFlashLoanLiquidation({ debtAmount: 1_000_000, feeBps: 5000, incentiveBps: 500 });
    expect(sim.profitable).toBe(false);
  });

  it('supports multi-asset legs and rejects empty/too-many', () => {
    expect(simulateMultiAssetFlashLoan([]).ok).toBe(false);
    const ok = simulateMultiAssetFlashLoan([
      { asset: 'USDC', amount: 1000 },
      { asset: 'XLM', amount: 2000 },
    ]);
    expect(ok.ok).toBe(true);
    expect(ok.gasUnitsEstimate).toBeGreaterThan(80_000);
  });
});

describe('governance proposal dry-run (#662)', () => {
  beforeEach(() => resetSimulationCache());

  it('returns state diffs, impact metrics, gas, and a shareable id', () => {
    const result = simulateProposal({
      proposalId: '42',
      kind: 'emergency_pause',
      proposed: { emergencyPause: true },
    });
    expect(result.gasUnitsEstimate).toBe(24_000);
    expect(result.diffs.some((d) => d.field === 'emergencyPause')).toBe(true);
    expect(result.tvlDelta).toBeLessThan(0);
    expect(getSharedSimulation(result.shareId)?.proposalId).toBe('42');
  });

  it('caches repeated views of the same proposal', () => {
    const a = simulateProposal({ proposalId: '7', kind: 'risk_params', proposed: { closeFactor: 4000 } });
    const b = simulateProposal({ proposalId: '7', kind: 'risk_params', proposed: { closeFactor: 2000 } });
    expect(b.shareId).toBe(a.shareId);
    expect(b.diffs).toEqual(a.diffs);
  });
});

describe('cross-asset unified health factor (#663)', () => {
  beforeEach(() => resetCrossAssetRisk());

  it('applies correlation haircut and dynamic CF from volatility', () => {
    setCorrelation('XLM', 'USDC', 9000);
    setVolatility('XLM', 4000);
    const health = computeUnifiedHealth([
      { asset: 'XLM', collateral: 10_000, debt: 0, price: 1, collateralFactorBps: 7500, liquidationThresholdBps: 8000 },
      { asset: 'USDC', collateral: 5_000, debt: 4_000, price: 1, collateralFactorBps: 8000, liquidationThresholdBps: 8500 },
    ]);
    expect(health.correlationPenaltyBps).toBeGreaterThan(0);
    expect(health.healthFactor).toBeGreaterThan(0);
    expect(dynamicCollateralFactor(7500, 'XLM')).toBe(6750);
  });

  it('tightens pair liquidation thresholds when assets are correlated', () => {
    setCorrelation('ETH', 'BTC', 10_000);
    expect(pairLiquidationThreshold(8000, 8000, 'ETH', 'BTC')).toBeGreaterThan(8000);
  });

  it('detects cross-asset utilization arbitrage', () => {
    const opps = detectArbitrage([
      { asset: 'XLM', utilizationBps: 3000 },
      { asset: 'USDC', utilizationBps: 8000 },
    ]);
    expect(opps.some((o) => o.borrowAsset === 'XLM' && o.supplyAsset === 'USDC')).toBe(true);
  });
});
