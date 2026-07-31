import { useState, useCallback } from 'react';
import type { RatePoint } from '../types';

/**
 * useRates — manage historical rate data for a pool.
 */
export function useRates(initialData: RatePoint[] = []) {
  const [data, setData] = useState<RatePoint[]>(initialData);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const getLatest = useCallback((): RatePoint | null =>
    data.length > 0 ? data[data.length - 1] : null, [data]);

  const getAverageApy = useCallback((type: 'supply' | 'borrow' = 'supply') => {
    if (data.length === 0) return 0;
    const sum = data.reduce((s, d) => s + (type === 'supply' ? d.supplyApy : d.borrowApy), 0);
    return sum / data.length;
  }, [data]);

  return { data, isLoading, error, setData, getLatest, getAverageApy, setIsLoading, setError };
}
