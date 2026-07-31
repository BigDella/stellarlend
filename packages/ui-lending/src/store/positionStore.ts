import { useState, useCallback } from 'react';
import type { Position } from '../types';

/**
 * positionStore — shared state management for all user positions.
 * Lightweight store using React hooks — swap for Zustand/Redux if needed.
 */
export function usePositionStore() {
  const [positions, setPositions] = useState<Position[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const addPosition = useCallback((position: Position) => {
    setPositions(prev => [...prev.filter(p => p.id !== position.id), position]);
  }, []);

  const removePosition = useCallback((id: string) => {
    setPositions(prev => prev.filter(p => p.id !== id));
  }, []);

  const updatePosition = useCallback((id: string, updates: Partial<Position>) => {
    setPositions(prev => prev.map(p => p.id === id ? { ...p, ...updates } : p));
  }, []);

  const getTotalSupplied = useCallback(() =>
    positions.reduce((sum, p) => sum + p.supplied * p.price, 0), [positions]);

  const getTotalBorrowed = useCallback(() =>
    positions.reduce((sum, p) => sum + p.borrowed * p.price, 0), [positions]);

  return { positions, isLoading, error, addPosition, removePosition, updatePosition, getTotalSupplied, getTotalBorrowed, setIsLoading, setError };
}
