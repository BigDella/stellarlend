import React from 'react';
import type { RatePoint, LendingTheme } from '../../types';
import { LIGHT_COLORS } from '../../utils/theme';

interface RateChartProps {
  data: RatePoint[];
  theme?: LendingTheme;
  showSupply?: boolean;
  showBorrow?: boolean;
  height?: number;
  isLoading?: boolean;
  isEmpty?: boolean;
  className?: string;
}

/**
 * RateChart — sparkline chart for supply/borrow APY over time.
 * Pure SVG, no external chart library dependency.
 */
export function RateChart({
  data,
  theme,
  showSupply = true,
  showBorrow = true,
  height = 120,
  isLoading = false,
  isEmpty = false,
  className = '',
}: RateChartProps) {
  const colors = theme?.colors ?? LIGHT_COLORS;
  const width = 300;

  if (isLoading) {
    return (
      <div className={`rate-chart ${className}`} style={{ background: colors.surface, borderRadius: 12, padding: 16, border: `1px solid ${colors.border}` }}>
        <div style={{ height, background: colors.border, borderRadius: 8 }} />
      </div>
    );
  }

  if (isEmpty || data.length === 0) {
    return (
      <div className={`rate-chart rate-chart--empty ${className}`} style={{ background: colors.surface, borderRadius: 12, padding: 16, border: `1px solid ${colors.border}`, height, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        <span style={{ color: colors.textMuted, fontSize: 13 }}>No rate data available</span>
      </div>
    );
  }

  const maxApy = Math.max(...data.flatMap(d => [d.supplyApy, d.borrowApy]));
  const minApy = Math.min(...data.flatMap(d => [d.supplyApy, d.borrowApy]));
  const range = maxApy - minApy || 1;

  const toX = (i: number) => (i / (data.length - 1)) * width;
  const toY = (v: number) => height - ((v - minApy) / range) * (height - 20) - 10;

  const supplyPath = data.map((d, i) => `${i === 0 ? 'M' : 'L'} ${toX(i)} ${toY(d.supplyApy)}`).join(' ');
  const borrowPath = data.map((d, i) => `${i === 0 ? 'M' : 'L'} ${toX(i)} ${toY(d.borrowApy)}`).join(' ');

  return (
    <div className={`rate-chart ${className}`} style={{ background: colors.surface, borderRadius: 12, padding: 16, border: `1px solid ${colors.border}` }} data-testid="rate-chart">
      <div style={{ display: 'flex', gap: 16, marginBottom: 8 }}>
        {showSupply && <span style={{ fontSize: 12, color: colors.success }}>● Supply APY</span>}
        {showBorrow && <span style={{ fontSize: 12, color: colors.danger }}>● Borrow APY</span>}
      </div>
      <svg width="100%" viewBox={`0 0 ${width} ${height}`} style={{ overflow: 'visible' }}>
        {showSupply && <path d={supplyPath} fill="none" stroke={colors.success} strokeWidth={2} />}
        {showBorrow && <path d={borrowPath} fill="none" stroke={colors.danger} strokeWidth={2} />}
      </svg>
    </div>
  );
}
