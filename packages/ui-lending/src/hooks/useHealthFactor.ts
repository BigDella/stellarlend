import { useState, useCallback } from 'react';
import type { HealthFactor, Position } from '../types';

function computeHealthStatus(value: number): HealthFactor['status'] {
  if (value >= 2) return 'safe';
  if (value >= 1.5) return 'warning';
  if (value >= 1) return 'danger';
  return 'liquidatable';
}

/**
 * useHealthFactor — compute and track account health factor from positions.
 */
export function useHealthFactor(positions: Position[] = []) {
  const [isLoading, setIsLoading] = useState(false);

  const compute = useCallback((): HealthFactor => {
    if (positions.length === 0) {
      return { value: Infinity, status: 'safe', collateralValue: 0, borrowedValue: 0, liquidationThreshold: 0.8 };
    }

    const collateralValue = positions.reduce(
      (sum, p) => sum + p.supplied * p.price * p.collateralFactor, 0
    );
    const borrowedValue = positions.reduce(
      (sum, p) => sum + p.borrowed * p.price, 0
    );

    const liquidationThreshold = 0.8;
    const value = borrowedValue === 0 ? Infinity : (collateralValue * liquidationThreshold) / borrowedValue;

    return { value, status: computeHealthStatus(value), collateralValue, borrowedValue, liquidationThreshold };
  }, [positions]);

  return { healthFactor: compute(), isLoading };
}
