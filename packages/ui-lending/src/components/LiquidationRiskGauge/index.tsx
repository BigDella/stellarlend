import React from 'react';
import type { LiquidationRisk, LendingTheme } from '../../types';
import { LIGHT_COLORS, getRiskColor } from '../../utils/theme';

interface LiquidationRiskGaugeProps {
  risk: LiquidationRisk;
  theme?: LendingTheme;
  isLoading?: boolean;
  className?: string;
}

/**
 * LiquidationRiskGauge — displays liquidation risk with price buffer visualization.
 */
export function LiquidationRiskGauge({ risk, theme, isLoading = false, className = '' }: LiquidationRiskGaugeProps) {
  const colors = theme?.colors ?? LIGHT_COLORS;
  const riskColor = getRiskColor(risk.riskLevel, colors);
  const bufferPct = Math.max(0, Math.min(100, risk.safetyBuffer * 100));

  if (isLoading) {
    return (
      <div className={`liquidation-gauge ${className}`} style={{ padding: 16, background: colors.surface, borderRadius: 12, border: `1px solid ${colors.border}` }}>
        <div style={{ height: 12, background: colors.border, borderRadius: 6 }} />
      </div>
    );
  }

  return (
    <div className={`liquidation-gauge ${className}`} style={{ padding: 16, background: colors.surface, borderRadius: 12, border: `1px solid ${colors.border}` }} data-testid="liquidation-gauge">
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 12 }}>
        <span style={{ color: colors.text, fontWeight: 600 }}>Liquidation Risk</span>
        <span style={{ color: riskColor, fontWeight: 700, textTransform: 'uppercase', fontSize: 13 }}>{risk.riskLevel}</span>
      </div>

      <div style={{ height: 10, background: colors.border, borderRadius: 5, overflow: 'hidden', marginBottom: 16 }}>
        <div style={{ height: '100%', width: `${bufferPct}%`, background: riskColor, borderRadius: 5, transition: 'width 0.3s ease' }} />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Current Price</div>
          <div style={{ color: colors.text, fontWeight: 500 }}>${risk.currentPrice.toFixed(2)}</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Liquidation Price</div>
          <div style={{ color: colors.danger, fontWeight: 500 }}>${risk.liquidationPrice.toFixed(2)}</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Safety Buffer</div>
          <div style={{ color: riskColor, fontWeight: 500 }}>{(risk.safetyBuffer * 100).toFixed(1)}%</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Health Factor</div>
          <div style={{ color: riskColor, fontWeight: 500 }}>{risk.healthFactor.toFixed(2)}</div>
        </div>
      </div>
    </div>
  );
}
