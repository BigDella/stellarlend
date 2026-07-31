import { useState, useCallback } from 'react';
import type { PoolData } from '../types';

/**
 * usePoolData — manage pool data state with filtering and sorting.
 */
export function usePoolData(initialPools: PoolData[] = []) {
  const [pools, setPools] = useState<PoolData[]>(initialPools);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getActivesPools = useCallback(() => pools.filter(p => p.isActive), [pools]);

  const getPoolById = useCallback((id: string) => pools.find(p => p.id === id) ?? null, [pools]);

  const sortByApy = useCallback((type: 'supply' | 'borrow' = 'supply') =>
    [...pools].sort((a, b) =>
      type === 'supply' ? b.supplyApy - a.supplyApy : b.borrowApy - a.borrowApy
    ), [pools]);

  return { pools, isLoading, error, setPools, getActivesPools, getPoolById, sortByApy, setIsLoading, setError };
}
