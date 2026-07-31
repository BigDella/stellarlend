import React from 'react';
import type { PoolData, LendingTheme } from '../../types';
import { LIGHT_COLORS } from '../../utils/theme';

interface PoolCardProps {
  pool: PoolData;
  theme?: LendingTheme;
  onSupply?: (pool: PoolData) => void;
  onBorrow?: (pool: PoolData) => void;
  isLoading?: boolean;
  className?: string;
}

/**
 * PoolCard — displays a lending pool with utilization, APY, and actions.
 */
export function PoolCard({ pool, theme, onSupply, onBorrow, isLoading = false, className = '' }: PoolCardProps) {
  const colors = theme?.colors ?? LIGHT_COLORS;

  if (isLoading) {
    return (
      <div className={`pool-card ${className}`} style={{ background: colors.surface, border: `1px solid ${colors.border}`, borderRadius: 12, padding: 20 }} role="status">
        <div style={{ height: 16, background: colors.border, borderRadius: 4, marginBottom: 8, width: '50%' }} />
        <div style={{ height: 12, background: colors.border, borderRadius: 4, width: '70%' }} />
      </div>
    );
  }

  if (!pool.isActive) {
    return (
      <div className={`pool-card pool-card--inactive ${className}`} style={{ background: colors.surface, border: `1px solid ${colors.border}`, borderRadius: 12, padding: 20, opacity: 0.5 }} data-testid="pool-card-inactive">
        <h3 style={{ color: colors.text, margin: 0 }}>{pool.asset}</h3>
        <span style={{ color: colors.textMuted, fontSize: 13 }}>Pool inactive</span>
      </div>
    );
  }

  return (
    <div className={`pool-card ${className}`} style={{ background: colors.surface, border: `1px solid ${colors.border}`, borderRadius: 12, padding: 20 }} data-testid="pool-card">
      <div style={{ display: 'flex', justifyContent: 'space-between', marginBottom: 16 }}>
        <div>
          <h3 style={{ color: colors.text, margin: 0 }}>{pool.asset}</h3>
          <span style={{ color: colors.textMuted, fontSize: 13 }}>{pool.symbol}</span>
        </div>
        <div style={{ textAlign: 'right' }}>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Utilization</div>
          <div style={{ color: colors.primary, fontWeight: 600 }}>{(pool.utilization * 100).toFixed(1)}%</div>
        </div>
      </div>

      <div style={{ height: 6, background: colors.border, borderRadius: 3, marginBottom: 16 }}>
        <div style={{ height: '100%', width: `${pool.utilization * 100}%`, background: colors.primary, borderRadius: 3 }} />
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 16 }}>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Supply APY</div>
          <div style={{ color: colors.success, fontWeight: 600 }}>{pool.supplyApy.toFixed(2)}%</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Borrow APY</div>
          <div style={{ color: colors.danger, fontWeight: 600 }}>{pool.borrowApy.toFixed(2)}%</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Total Supply</div>
          <div style={{ color: colors.text, fontSize: 13 }}>${pool.totalSupply.toLocaleString()}</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 11 }}>Total Borrow</div>
          <div style={{ color: colors.text, fontSize: 13 }}>${pool.totalBorrow.toLocaleString()}</div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 8 }}>
        {onSupply && (
          <button onClick={() => onSupply(pool)} style={{ flex: 1, padding: '8px 12px', background: colors.primary, color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>Supply</button>
        )}
        {onBorrow && (
          <button onClick={() => onBorrow(pool)} style={{ flex: 1, padding: '8px 12px', background: colors.surface, color: colors.text, border: `1px solid ${colors.border}`, borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>Borrow</button>
        )}
      </div>
    </div>
  );
}
