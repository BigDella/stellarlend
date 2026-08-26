import React, { useState } from 'react';

interface DiffEntry {
  field: string;
  currentValue: number;
  proposedValue: number;
}

interface DryRun {
  shareId: string;
  proposalId: string;
  wouldSucceed: boolean;
  tvlDelta: number;
  apyDeltaBps: number;
  riskScoreDelta: number;
  gasUnitsEstimate: number;
  diffs: DiffEntry[];
  currentState: Record<string, number | boolean>;
  proposedState: Record<string, number | boolean>;
  shareUrl?: string;
}

export const GovernanceProposalSimulator: React.FC = () => {
  const [proposalId, setProposalId] = useState('1');
  const [kind, setKind] = useState('emergency_pause');
  const [result, setResult] = useState<DryRun | null>(null);
  const [shareId, setShareId] = useState('');
  const [error, setError] = useState<string | null>(null);

  const run = async () => {
    setError(null);
    const res = await fetch('/api/governance/simulate', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        proposalId,
        kind,
        proposed: kind === 'emergency_pause' ? { emergencyPause: true } : undefined,
      }),
    });
    const body = await res.json();
    if (!body.success) {
      setError(body.error || 'Simulation failed');
      return;
    }
    setResult({ ...body.data, shareUrl: body.shareUrl });
  };

  const loadShared = async () => {
    const res = await fetch(`/api/governance/simulate/share/${shareId}`);
    const body = await res.json();
    if (!body.success) {
      setError(body.error || 'Not found');
      return;
    }
    setResult(body.data);
  };

  return (
    <div style={{ padding: 24, maxWidth: 900, margin: '0 auto', fontFamily: 'sans-serif' }}>
      <h2>Governance proposal simulation</h2>
      <p>Dry-run execution with state diff, TVL/APY/risk impact, and gas estimate.</p>
      <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
        <input value={proposalId} onChange={(e) => setProposalId(e.target.value)} placeholder="Proposal ID" />
        <select value={kind} onChange={(e) => setKind(e.target.value)}>
          <option value="emergency_pause">Emergency pause</option>
          <option value="min_collateral_ratio">Min collateral ratio</option>
          <option value="risk_params">Risk params</option>
          <option value="interest_rate">Interest rate</option>
        </select>
        <button onClick={run}>Simulate</button>
      </div>
      <div style={{ display: 'flex', gap: 8, marginBottom: 16 }}>
        <input value={shareId} onChange={(e) => setShareId(e.target.value)} placeholder="Share ID" />
        <button onClick={loadShared}>Open shared result</button>
      </div>
      {error && <p style={{ color: '#c00' }}>{error}</p>}
      {result && (
        <div>
          <p>
            Would succeed: <strong>{result.wouldSucceed ? 'yes' : 'no'}</strong> · Gas:{' '}
            {result.gasUnitsEstimate.toLocaleString()} · Share: {result.shareUrl || result.shareId}
          </p>
          <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr 1fr', gap: 12 }}>
            <div>TVL Δ {result.tvlDelta.toFixed(0)}</div>
            <div>APY Δ {result.apyDeltaBps} bps</div>
            <div>Risk Δ {result.riskScoreDelta.toFixed(1)}</div>
          </div>
          <h3>State diff</h3>
          <table style={{ width: '100%', borderCollapse: 'collapse' }}>
            <thead>
              <tr>
                <th>Field</th>
                <th>Current</th>
                <th>Proposed</th>
              </tr>
            </thead>
            <tbody>
              {result.diffs.map((d) => (
                <tr key={d.field}>
                  <td>{d.field}</td>
                  <td>{d.currentValue}</td>
                  <td>{d.proposedValue}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};
