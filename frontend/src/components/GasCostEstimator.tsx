/**
 * Gas Cost Estimator Component
 * 
 * Displays comprehensive gas cost estimation with:
 * - Cost breakdown (base, storage, cross-contract)
 * - Historical comparison
 * - Optimization suggestions
 * - USD cost estimate
 */

import React, { useState, useEffect } from 'react';
import { AlertCircle, TrendingUp, Clock, Zap, DollarSign } from 'lucide-react';

interface GasCostBreakdown {
  baseCost: string;
  storageCost: string;
  crossContractCost: string;
  totalCost: string;
  cpuInstructions: string;
  memoryBytes: string;
  minResourceFee: string;
}

interface GasOptimizationSuggestion {
  type: 'timing' | 'batching' | 'method';
  title: string;
  description: string;
  potentialSavings: string;
  priority: 'high' | 'medium' | 'low';
}

interface HistoricalGasData {
  operation: string;
  averageCost: string;
  minCost: string;
  maxCost: string;
  stdDeviation: string;
  sampleCount: number;
  period: string;
}

interface GasCostEstimate {
  operation: string;
  breakdown: GasCostBreakdown;
  optimizations: GasOptimizationSuggestion[];
  historical?: HistoricalGasData;
  estimatedUsdCost?: string;
  timestamp: string;
  confidence: 'high' | 'medium' | 'low';
}

interface GasCostEstimatorProps {
  operation: 'deposit' | 'withdraw' | 'borrow' | 'repay' | 'liquidation' | 'flash_loan';
  userAddress: string;
  assetAddress?: string;
  amount: string;
  apiBaseUrl?: string;
  onEstimateComplete?: (estimate: GasCostEstimate) => void;
}

export const GasCostEstimator: React.FC<GasCostEstimatorProps> = ({
  operation,
  userAddress,
  assetAddress,
  amount,
  apiBaseUrl = '/api',
  onEstimateComplete,
}) => {
  const [estimate, setEstimate] = useState<GasCostEstimate | null>(null);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    if (operation && userAddress && amount) {
      fetchEstimate();
    }
  }, [operation, userAddress, assetAddress, amount]);

  const fetchEstimate = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(`${apiBaseUrl}/gas/estimate`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({
          operation,
          userAddress,
          assetAddress,
          amount,
          includeOptimizations: true,
          includeHistorical: true,
        }),
      });

      if (!response.ok) {
        throw new Error('Failed to estimate gas cost');
      }

      const data: GasCostEstimate = await response.json();
      setEstimate(data);
      onEstimateComplete?.(data);
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

  const getPriorityColor = (priority: string): string => {
    switch (priority) {
      case 'high': return 'text-red-600 bg-red-50';
      case 'medium': return 'text-yellow-600 bg-yellow-50';
      case 'low': return 'text-blue-600 bg-blue-50';
      default: return 'text-gray-600 bg-gray-50';
    }
  };

  const getConfidenceColor = (confidence: string): string => {
    switch (confidence) {
      case 'high': return 'text-green-600';
      case 'medium': return 'text-yellow-600';
      case 'low': return 'text-red-600';
      default: return 'text-gray-600';
    }
  };

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse">
          <div className="h-4 bg-gray-200 rounded w-1/4 mb-4"></div>
          <div className="h-8 bg-gray-200 rounded w-1/2 mb-4"></div>
          <div className="h-4 bg-gray-200 rounded w-full mb-2"></div>
          <div className="h-4 bg-gray-200 rounded w-3/4"></div>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4 flex items-start gap-3">
        <AlertCircle className="w-5 h-5 text-red-600 flex-shrink-0 mt-0.5" />
        <div>
          <h3 className="font-semibold text-red-900">Estimation Failed</h3>
          <p className="text-sm text-red-700 mt-1">{error}</p>
        </div>
      </div>
    );
  }

  if (!estimate) {
    return null;
  }

  return (
    <div className="space-y-6">
      {/* Main Cost Display */}
      <div className="bg-gradient-to-r from-blue-500 to-purple-600 rounded-lg shadow-lg p-6 text-white">
        <div className="flex items-center justify-between mb-4">
          <h3 className="text-lg font-semibold opacity-90">Estimated Gas Cost</h3>
          <span className={`text-xs px-2 py-1 rounded-full ${getConfidenceColor(estimate.confidence)} bg-white bg-opacity-20`}>
            {estimate.confidence} confidence
          </span>
        </div>
        
        <div className="flex items-baseline gap-2 mb-2">
          <span className="text-4xl font-bold">{formatStroops(estimate.breakdown.totalCost)}</span>
          <span className="text-xl opacity-80">XLM</span>
        </div>
        
        {estimate.estimatedUsdCost && (
          <div className="flex items-center gap-2 text-sm opacity-90">
            <DollarSign className="w-4 h-4" />
            <span>≈ ${estimate.estimatedUsdCost} USD</span>
          </div>
        )}
      </div>

      {/* Cost Breakdown */}
      <div className="bg-white rounded-lg shadow p-6">
        <h4 className="font-semibold text-gray-900 mb-4">Cost Breakdown</h4>
        
        <div className="space-y-3">
          <div className="flex justify-between items-center py-2 border-b border-gray-100">
            <span className="text-sm text-gray-600">Base Transaction Fee</span>
            <span className="font-medium">{formatStroops(estimate.breakdown.baseCost)} XLM</span>
          </div>
          
          <div className="flex justify-between items-center py-2 border-b border-gray-100">
            <span className="text-sm text-gray-600">Storage Operations</span>
            <span className="font-medium">{formatStroops(estimate.breakdown.storageCost)} XLM</span>
          </div>
          
          <div className="flex justify-between items-center py-2 border-b border-gray-100">
            <span className="text-sm text-gray-600">Cross-Contract Calls</span>
            <span className="font-medium">{formatStroops(estimate.breakdown.crossContractCost)} XLM</span>
          </div>
          
          <div className="flex justify-between items-center py-2 border-b border-gray-100">
            <span className="text-sm text-gray-600">Resource Fee</span>
            <span className="font-medium">{formatStroops(estimate.breakdown.minResourceFee)} XLM</span>
          </div>
          
          <div className="flex justify-between items-center pt-3">
            <span className="font-semibold text-gray-900">Total Cost</span>
            <span className="font-bold text-lg text-blue-600">
              {formatStroops(estimate.breakdown.totalCost)} XLM
            </span>
          </div>
        </div>

        {/* Technical Details */}
        <div className="mt-4 pt-4 border-t border-gray-200">
          <details className="text-sm">
            <summary className="cursor-pointer text-gray-600 hover:text-gray-900 font-medium">
              Technical Details
            </summary>
            <div className="mt-3 space-y-2 text-gray-600">
              <div className="flex justify-between">
                <span>CPU Instructions:</span>
                <span className="font-mono">{Number(estimate.breakdown.cpuInstructions).toLocaleString()}</span>
              </div>
              <div className="flex justify-between">
                <span>Memory Bytes:</span>
                <span className="font-mono">{Number(estimate.breakdown.memoryBytes).toLocaleString()}</span>
              </div>
            </div>
          </details>
        </div>
      </div>

      {/* Historical Comparison */}
      {estimate.historical && (
        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center gap-2 mb-4">
            <TrendingUp className="w-5 h-5 text-gray-600" />
            <h4 className="font-semibold text-gray-900">Historical Comparison ({estimate.historical.period})</h4>
          </div>
          
          <div className="grid grid-cols-3 gap-4">
            <div className="text-center">
              <div className="text-xs text-gray-500 mb-1">Average</div>
              <div className="font-semibold text-gray-900">{formatStroops(estimate.historical.averageCost)} XLM</div>
            </div>
            <div className="text-center">
              <div className="text-xs text-gray-500 mb-1">Min</div>
              <div className="font-semibold text-green-600">{formatStroops(estimate.historical.minCost)} XLM</div>
            </div>
            <div className="text-center">
              <div className="text-xs text-gray-500 mb-1">Max</div>
              <div className="font-semibold text-red-600">{formatStroops(estimate.historical.maxCost)} XLM</div>
            </div>
          </div>
          
          <div className="mt-4 text-xs text-gray-500 text-center">
            Based on {estimate.historical.sampleCount.toLocaleString()} transactions
          </div>
        </div>
      )}

      {/* Optimization Suggestions */}
      {estimate.optimizations.length > 0 && (
        <div className="bg-white rounded-lg shadow p-6">
          <div className="flex items-center gap-2 mb-4">
            <Zap className="w-5 h-5 text-yellow-500" />
            <h4 className="font-semibold text-gray-900">Optimization Suggestions</h4>
          </div>
          
          <div className="space-y-3">
            {estimate.optimizations.map((opt, index) => (
              <div
                key={index}
                className={`p-4 rounded-lg border ${getPriorityColor(opt.priority)} border-current border-opacity-20`}
              >
                <div className="flex items-start justify-between mb-2">
                  <h5 className="font-semibold">{opt.title}</h5>
                  <span className="text-xs px-2 py-1 rounded-full bg-white bg-opacity-50">
                    {opt.priority}
                  </span>
                </div>
                <p className="text-sm mb-2 opacity-80">{opt.description}</p>
                <div className="flex items-center gap-2 text-sm font-medium">
                  <span>Potential savings:</span>
                  <span className="font-bold">{formatStroops(opt.potentialSavings)} XLM</span>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Timestamp */}
      <div className="flex items-center justify-center gap-2 text-xs text-gray-500">
        <Clock className="w-4 h-4" />
        <span>Estimated at {new Date(estimate.timestamp).toLocaleString()}</span>
      </div>
    </div>
  );
};

export default GasCostEstimator;
