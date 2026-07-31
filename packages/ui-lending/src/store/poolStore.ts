import { useState, useCallback } from 'react';
import type { PoolData } from '../types';

/**
 * poolStore — shared state management for lending pools.
 */
export function usePoolStore() {
  const [pools, setPools] = useState<PoolData[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const upsertPool = useCallback((pool: PoolData) => {
    setPools(prev => [...prev.filter(p => p.id !== pool.id), pool]);
  }, []);

  const getPool = useCallback((id: string) =>
    pools.find(p => p.id === id) ?? null, [pools]);

  const getActivePools = useCallback(() =>
    pools.filter(p => p.isActive), [pools]);

  const getTotalTVL = useCallback(() =>
    pools.reduce((sum, p) => sum + p.totalSupply, 0), [pools]);

  return { pools, isLoading, error, upsertPool, getPool, getActivePools, getTotalTVL, setPools, setIsLoading, setError };
}
