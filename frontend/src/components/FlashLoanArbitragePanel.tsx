import React, { useState } from 'react';

interface SimResult {
  profitable: boolean;
  estimatedProfit: number;
  flashFee: number;
  collateralSeized: number;
  gasUnitsEstimate: number;
  status?: string;
  reason?: string;
}

export const FlashLoanArbitragePanel: React.FC = () => {
  const [debtAmount, setDebtAmount] = useState(1_000_000);
  const [feeBps, setFeeBps] = useState(9);
  const [incentiveBps, setIncentiveBps] = useState(1000);
  const [result, setResult] = useState<SimResult | null>(null);

  const simulate = async () => {
    const res = await fetch('/api/flash-loan/simulate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ debtAmount, feeBps, incentiveBps }),
    });
    const body = await res.json();
    if (body.success) setResult(body.data);
  };

  const execute = async () => {
    const res = await fetch('/api/flash-loan/execute', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        liquidator: 'GDEMO',
        borrower: 'GUNDER',
        debtAsset: 'USDC',
        collateralAsset: 'XLM',
        debtAmount,
      }),
    });
    const body = await res.json();
    setResult(body.data);
  };

  return (
    <div style={{ padding: 24, maxWidth: 720, margin: '0 auto', fontFamily: 'sans-serif' }}>
      <h2>Flash loan liquidation arb</h2>
      <p>Pre-execution profit simulation with rollback when the combo is unprofitable.</p>
      <label>
        Debt amount{' '}
        <input type="number" value={debtAmount} onChange={(e) => setDebtAmount(Number(e.target.value))} />
      </label>
      <label>
        Fee bps <input type="number" value={feeBps} onChange={(e) => setFeeBps(Number(e.target.value))} />
      </label>
      <label>
        Incentive bps{' '}
        <input type="number" value={incentiveBps} onChange={(e) => setIncentiveBps(Number(e.target.value))} />
      </label>
      <div style={{ display: 'flex', gap: 8, marginTop: 12 }}>
        <button onClick={simulate}>Simulate</button>
        <button onClick={execute}>Execute if profitable</button>
      </div>
      {result && (
        <pre style={{ background: '#f6f8fa', padding: 12, marginTop: 16 }}>
          {JSON.stringify(result, null, 2)}
        </pre>
      )}
    </div>
  );
};
