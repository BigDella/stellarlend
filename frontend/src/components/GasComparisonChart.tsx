/**
 * Gas Comparison Chart Component
 * 
 * Displays a comparison of gas costs across all operations
 */

import React, { useState, useEffect } from 'react';
import { BarChart3, TrendingDown, Info } from 'lucide-react';

interface GasComparisonItem {
  operation: string;
  averageCost: string;
  relativeCost: number;
  rank: number;
}

interface GasComparisonResponse {
  operations: GasComparisonItem[];
  referenceOperation: string;
  timestamp: string;
}

interface GasComparisonChartProps {
  apiBaseUrl?: string;
  autoRefresh?: boolean;
  refreshInterval?: number; // in seconds
}

export const GasComparisonChart: React.FC<GasComparisonChartProps> = ({
  apiBaseUrl = '/api',
  autoRefresh = false,
  refreshInterval = 60,
}) => {
  const [data, setData] = useState<GasComparisonResponse | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchComparison();

    if (autoRefresh) {
      const interval = setInterval(fetchComparison, refreshInterval * 1000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh, refreshInterval]);

  const fetchComparison = async () => {
    try {
      const response = await fetch(`${apiBaseUrl}/gas/compare`);
      if (!response.ok) throw new Error('Failed to fetch comparison');
      
      const result: GasComparisonResponse = await response.json();
      setData(result);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  const formatStroops = (stroops: string): string => {
    const xlm = Number(stroops) / 10_000_000;
    return xlm.toFixed(7);
  };

  const getOperationLabel = (operation: string): string => {
    const labels: Record<string, string> = {
      deposit: 'Deposit',
      withdraw: 'Withdraw',
      borrow: 'Borrow',
      repay: 'Repay',
      liquidation: 'Liquidation',
      flash_loan: 'Flash Loan',
    };
    return labels[operation] || operation;
  };

  const getBarColor = (rank: number): string => {
    if (rank === 1) return 'bg-green-500';
    if (rank <= 2) return 'bg-blue-500';
    if (rank <= 4) return 'bg-yellow-500';
    return 'bg-red-500';
  };

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse space-y-4">
          <div className="h-6 bg-gray-200 rounded w-1/3"></div>
          {[...Array(6)].map((_, i) => (
            <div key={i} className="h-12 bg-gray-200 rounded"></div>
          ))}
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <p className="text-red-700">{error}</p>
      </div>
    );
  }

  if (!data) return null;

  const maxCost = Math.max(...data.operations.map(op => Number(op.averageCost)));

  return (
    <div className="bg-white rounded-lg shadow-lg p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <BarChart3 className="w-6 h-6 text-blue-600" />
          <h3 className="text-xl font-bold text-gray-900">Gas Cost Comparison</h3>
        </div>
        <button
          onClick={fetchComparison}
          className="text-sm text-blue-600 hover:text-blue-700 font-medium"
        >
          Refresh
        </button>
      </div>

      {/* Info Banner */}
      <div className="bg-blue-50 border border-blue-200 rounded-lg p-4 mb-6 flex items-start gap-3">
        <Info className="w-5 h-5 text-blue-600 flex-shrink-0 mt-0.5" />
        <div className="text-sm text-blue-900">
          <p className="font-medium mb-1">Cost Comparison</p>
          <p className="text-blue-700">
            Operations ranked by average gas cost. Lower costs = more efficient operations.
          </p>
        </div>
      </div>

      {/* Chart */}
      <div className="space-y-4">
        {data.operations.map((item) => {
          const percentage = (Number(item.averageCost) / maxCost) * 100;
          
          return (
            <div key={item.operation} className="space-y-2">
              <div className="flex items-center justify-between text-sm">
                <div className="flex items-center gap-2">
                  <span className="font-semibold text-gray-900 w-24">
                    {getOperationLabel(item.operation)}
                  </span>
                  <span className="text-xs text-gray-500">
                    #{item.rank}
                  </span>
                </div>
                <div className="flex items-center gap-3">
                  <span className="text-gray-600 font-mono text-xs">
                    {formatStroops(item.averageCost)} XLM
                  </span>
                  <span className="text-gray-500 text-xs w-16 text-right">
                    {item.relativeCost.toFixed(2)}x
                  </span>
                </div>
              </div>
              
              <div className="w-full bg-gray-100 rounded-full h-3 overflow-hidden">
                <div
                  className={`h-full ${getBarColor(item.rank)} transition-all duration-500 rounded-full`}
                  style={{ width: `${percentage}%` }}
                />
              </div>
            </div>
          );
        })}
      </div>

      {/* Legend */}
      <div className="mt-6 pt-6 border-t border-gray-200">
        <div className="flex items-center gap-2 text-xs text-gray-500 mb-2">
          <TrendingDown className="w-4 h-4" />
          <span>Relative cost compared to {getOperationLabel(data.referenceOperation)}</span>
        </div>
        
        <div className="flex flex-wrap gap-4 text-xs">
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 bg-green-500 rounded"></div>
            <span className="text-gray-600">Most efficient</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 bg-blue-500 rounded"></div>
            <span className="text-gray-600">Efficient</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 bg-yellow-500 rounded"></div>
            <span className="text-gray-600">Moderate</span>
          </div>
          <div className="flex items-center gap-2">
            <div className="w-3 h-3 bg-red-500 rounded"></div>
            <span className="text-gray-600">Expensive</span>
          </div>
        </div>
      </div>

      {/* Timestamp */}
      <div className="mt-4 text-xs text-gray-400 text-center">
        Last updated: {new Date(data.timestamp).toLocaleString()}
      </div>
    </div>
  );
};

export default GasComparisonChart;
