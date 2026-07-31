import { useState, useCallback } from 'react';
import type { Position } from '../types';

interface UsePositionOptions {
  onError?: (error: Error) => void;
}

/**
 * usePosition — manage a single lending position with optimistic updates.
 */
export function usePosition(initialPosition?: Position, options: UsePositionOptions = {}) {
  const [position, setPosition] = useState<Position | null>(initialPosition ?? null);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const update = useCallback((updates: Partial<Position>) => {
    setPosition(prev => prev ? { ...prev, ...updates } : null);
  }, []);

  const reset = useCallback(() => {
    setPosition(initialPosition ?? null);
    setError(null);
  }, [initialPosition]);

  return { position, isLoading, error, update, reset, setIsLoading, setError };
}
