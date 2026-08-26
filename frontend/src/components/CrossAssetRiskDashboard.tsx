import React, { useState } from 'react';

interface HealthResult {
  healthFactor: number;
  isLiquidatable: boolean;
  borrowCapacity: number;
  correlationPenaltyBps: number;
  portfolioRiskScore: number;
  dynamicCollateralFactors: Record<string, number>;
}

export const CrossAssetRiskDashboard: React.FC = () => {
  const [health, setHealth] = useState<HealthResult | null>(null);
  const [arbs, setArbs] = useState<Array<{ borrowAsset: string; supplyAsset: string; spreadBps: number }>>([]);

  const load = async () => {
    const [healthRes, arbRes] = await Promise.all([
      fetch('/api/v1/lending/cross-asset/health', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          positions: [
            { asset: 'XLM', collateral: 10000, debt: 0, price: 0.12, collateralFactorBps: 7500 },
            { asset: 'USDC', collateral: 5000, debt: 2000, price: 1, collateralFactorBps: 8000 },
          ],
        }),
      }),
      fetch('/api/v1/lending/cross-asset/arbitrage', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          pools: [
            { asset: 'XLM', utilizationBps: 4000 },
            { asset: 'USDC', utilizationBps: 7200 },
          ],
        }),
      }),
    ]);
    const healthBody = await healthRes.json();
    const arbBody = await arbRes.json();
    if (healthBody.success) setHealth(healthBody.data);
    if (arbBody.success) setArbs(arbBody.data);
  };

  return (
    <div style={{ padding: 24, maxWidth: 900, margin: '0 auto', fontFamily: 'sans-serif' }}>
      <h2>Cross-asset portfolio risk</h2>
      <p>Unified health factor, correlation haircut, dynamic collateral factors, and arb signals.</p>
      <button onClick={load}>Compute sample portfolio</button>
      {health && (
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(3, 1fr)', gap: 12, marginTop: 16 }}>
          <div>Health factor {(health.healthFactor / 10000).toFixed(2)}x</div>
          <div>Liquidatable {health.isLiquidatable ? 'yes' : 'no'}</div>
          <div>Risk score {health.portfolioRiskScore}</div>
          <div>Borrow capacity {health.borrowCapacity.toFixed(0)}</div>
          <div>Corr. penalty {health.correlationPenaltyBps} bps</div>
        </div>
      )}
      {arbs.length > 0 && (
        <div style={{ marginTop: 16 }}>
          <h3>Cross-asset arbitrage</h3>
          <ul>
            {arbs.map((a) => (
              <li key={`${a.borrowAsset}-${a.supplyAsset}`}>
                Borrow {a.borrowAsset} / supply {a.supplyAsset} · {a.spreadBps} bps
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  );
};
