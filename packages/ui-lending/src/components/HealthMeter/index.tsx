import React from 'react';
import type { HealthFactor, LendingTheme } from '../../types';
import { LIGHT_COLORS, getHealthColor } from '../../utils/theme';

interface HealthMeterProps {
  health: HealthFactor;
  theme?: LendingTheme;
  showDetails?: boolean;
  isLoading?: boolean;
  className?: string;
}

/**
 * HealthMeter — visual gauge for account health factor.
 * Green ≥ 2.0, Yellow ≥ 1.5, Red < 1.5, Critical < 1.0.
 */
export function HealthMeter({
  health,
  theme,
  showDetails = true,
  isLoading = false,
  className = '',
}: HealthMeterProps) {
  const colors = theme?.colors ?? LIGHT_COLORS;
  const healthColor = getHealthColor(health.value, colors);
  const percentage = Math.min(100, (health.value / 3) * 100);

  if (isLoading) {
    return (
      <div className={`health-meter ${className}`} style={{ padding: 16, background: colors.surface, borderRadius: 12, border: `1px solid ${colors.border}` }}>
        <div style={{ height: 12, background: colors.border, borderRadius: 6 }} />
      </div>
    );
  }

  return (
    <div className={`health-meter ${className}`} style={{ padding: 16, background: colors.surface, borderRadius: 12, border: `1px solid ${colors.border}` }} data-testid="health-meter">
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 8 }}>
        <span style={{ color: colors.textMuted, fontSize: 13 }}>Health Factor</span>
        <span style={{ color: healthColor, fontWeight: 700, fontSize: 18 }}>
          {health.value === Infinity ? '∞' : health.value.toFixed(2)}
        </span>
      </div>

      <div style={{ height: 8, background: colors.border, borderRadius: 4, overflow: 'hidden', marginBottom: 12 }}>
        <div style={{ height: '100%', width: `${percentage}%`, background: healthColor, borderRadius: 4, transition: 'width 0.3s ease' }} />
      </div>

      <div style={{ display: 'flex', justifyContent: 'center' }}>
        <span style={{ fontSize: 12, color: healthColor, fontWeight: 600, textTransform: 'uppercase' }}>
          {health.status}
        </span>
      </div>

      {showDetails && (
        <div style={{ marginTop: 12, display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 8 }}>
          <div>
            <div style={{ color: colors.textMuted, fontSize: 11 }}>Collateral Value</div>
            <div style={{ color: colors.text, fontSize: 13, fontWeight: 500 }}>${health.collateralValue.toFixed(2)}</div>
          </div>
          <div>
            <div style={{ color: colors.textMuted, fontSize: 11 }}>Borrowed Value</div>
            <div style={{ color: colors.text, fontSize: 13, fontWeight: 500 }}>${health.borrowedValue.toFixed(2)}</div>
          </div>
        </div>
      )}
    </div>
  );
}
