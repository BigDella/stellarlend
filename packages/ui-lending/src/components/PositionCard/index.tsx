import React from 'react';
import type { Position, LendingTheme } from '../../types';
import { LIGHT_COLORS } from '../../utils/theme';

interface PositionCardProps {
  position: Position;
  theme?: LendingTheme;
  onSupply?: (position: Position) => void;
  onBorrow?: (position: Position) => void;
  onRepay?: (position: Position) => void;
  onWithdraw?: (position: Position) => void;
  isLoading?: boolean;
  className?: string;
}

/**
 * PositionCard — displays a single lending position with supply/borrow details.
 * Supports dark/light theming and loading/empty states.
 */
export function PositionCard({
  position,
  theme,
  onSupply,
  onBorrow,
  onRepay,
  onWithdraw,
  isLoading = false,
  className = '',
}: PositionCardProps) {
  const colors = theme?.colors ?? LIGHT_COLORS;

  if (isLoading) {
    return (
      <div
        className={`position-card position-card--loading ${className}`}
        style={{ background: colors.surface, border: `1px solid ${colors.border}`, borderRadius: 12, padding: 20 }}
        role="status"
        aria-label="Loading position"
      >
        <div style={{ height: 16, background: colors.border, borderRadius: 4, marginBottom: 8, width: '60%' }} />
        <div style={{ height: 12, background: colors.border, borderRadius: 4, width: '40%' }} />
      </div>
    );
  }

  const netValue = (position.supplied - position.borrowed) * position.price;

  return (
    <div
      className={`position-card ${className}`}
      style={{ background: colors.surface, border: `1px solid ${colors.border}`, borderRadius: 12, padding: 20 }}
      data-testid="position-card"
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 16 }}>
        <div>
          <h3 style={{ color: colors.text, margin: 0, fontSize: 18, fontWeight: 600 }}>{position.asset}</h3>
          <span style={{ color: colors.textMuted, fontSize: 13 }}>{position.symbol}</span>
        </div>
        <span style={{ color: netValue >= 0 ? colors.success : colors.danger, fontWeight: 600 }}>
          ${netValue.toFixed(2)}
        </span>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: 12, marginBottom: 16 }}>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 12, marginBottom: 4 }}>Supplied</div>
          <div style={{ color: colors.text, fontWeight: 500 }}>{position.supplied.toFixed(4)}</div>
          <div style={{ color: colors.success, fontSize: 12 }}>{position.supplyApy.toFixed(2)}% APY</div>
        </div>
        <div>
          <div style={{ color: colors.textMuted, fontSize: 12, marginBottom: 4 }}>Borrowed</div>
          <div style={{ color: colors.text, fontWeight: 500 }}>{position.borrowed.toFixed(4)}</div>
          <div style={{ color: colors.danger, fontSize: 12 }}>{position.borrowApy.toFixed(2)}% APY</div>
        </div>
      </div>

      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
        {onSupply && (
          <button onClick={() => onSupply(position)} style={{ flex: 1, padding: '8px 12px', background: colors.primary, color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>
            Supply
          </button>
        )}
        {onBorrow && (
          <button onClick={() => onBorrow(position)} style={{ flex: 1, padding: '8px 12px', background: colors.secondary, color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>
            Borrow
          </button>
        )}
        {onRepay && position.borrowed > 0 && (
          <button onClick={() => onRepay(position)} style={{ flex: 1, padding: '8px 12px', background: colors.warning, color: '#fff', border: 'none', borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>
            Repay
          </button>
        )}
        {onWithdraw && position.supplied > 0 && (
          <button onClick={() => onWithdraw(position)} style={{ flex: 1, padding: '8px 12px', background: colors.surface, color: colors.text, border: `1px solid ${colors.border}`, borderRadius: 8, cursor: 'pointer', fontSize: 13 }}>
            Withdraw
          </button>
        )}
      </div>
    </div>
  );
}
