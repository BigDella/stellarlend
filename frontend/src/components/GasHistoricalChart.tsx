/**
 * Gas Historical Chart Component
 * 
 * Displays historical gas costs over time with min/max ranges
 */

import React, { useState, useEffect } from 'react';
import { LineChart, Calendar, TrendingUp } from 'lucide-react';

interface HistoricalDataPoint {
  timestamp: string;
  averageCost: string;
  minCost: string;
  maxCost: string;
  sampleCount: number;
}

interface HistoricalChartData {
  operation: string;
  dataPoints: HistoricalDataPoint[];
  period: string;
}

interface GasHistoricalChartProps {
  operation: 'deposit' | 'withdraw' | 'borrow' | 'repay' | 'liquidation' | 'flash_loan';
  period?: '24h' | '7d' | '30d';
  apiBaseUrl?: string;
}

export const GasHistoricalChart: React.FC<GasHistoricalChartProps> = ({
  operation,
  period = '7d',
  apiBaseUrl = '/api',
}) => {
  const [data, setData] = useState<HistoricalChartData | null>(null);
  const [selectedPeriod, setSelectedPeriod] = useState(period);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [hoveredPoint, setHoveredPoint] = useState<number | null>(null);

  useEffect(() => {
    fetchChartData();
  }, [operation, selectedPeriod]);

  const fetchChartData = async () => {
    setLoading(true);
    try {
      const response = await fetch(
        `${apiBaseUrl}/gas/chart/${operation}?period=${selectedPeriod}`
      );
      if (!response.ok) throw new Error('Failed to fetch chart data');
      
      const result: HistoricalChartData = await response.json();
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

  const formatDate = (timestamp: string, period: string): string => {
    const date = new Date(timestamp);
    if (period === '24h') {
      return date.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' });
    } else if (period === '7d') {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    } else {
      return date.toLocaleDateString('en-US', { month: 'short', day: 'numeric' });
    }
  };

  const getOperationLabel = (op: string): string => {
    const labels: Record<string, string> = {
      deposit: 'Deposit',
      withdraw: 'Withdraw',
      borrow: 'Borrow',
      repay: 'Repay',
      liquidation: 'Liquidation',
      flash_loan: 'Flash Loan',
    };
    return labels[op] || op;
  };

  if (loading) {
    return (
      <div className="bg-white rounded-lg shadow p-6">
        <div className="animate-pulse">
          <div className="h-6 bg-gray-200 rounded w-1/3 mb-4"></div>
          <div className="h-64 bg-gray-200 rounded"></div>
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

  if (!data || data.dataPoints.length === 0) return null;

  // Calculate chart dimensions and scaling
  const allCosts = data.dataPoints.flatMap(d => [
    Number(d.minCost),
    Number(d.averageCost),
    Number(d.maxCost),
  ]);
  const minY = Math.min(...allCosts);
  const maxY = Math.max(...allCosts);
  const rangeY = maxY - minY;
  const paddingY = rangeY * 0.1;

  const chartHeight = 300;
  const chartWidth = 800;
  const padding = { top: 20, right: 20, bottom: 40, left: 60 };

  const scaleY = (value: number) => {
    const scaled = ((value - minY + paddingY) / (rangeY + 2 * paddingY)) * (chartHeight - padding.top - padding.bottom);
    return chartHeight - padding.bottom - scaled;
  };

  const scaleX = (index: number) => {
    return padding.left + (index / (data.dataPoints.length - 1)) * (chartWidth - padding.left - padding.right);
  };

  // Generate path for average line
  const avgPath = data.dataPoints
    .map((point, i) => {
      const x = scaleX(i);
      const y = scaleY(Number(point.averageCost));
      return `${i === 0 ? 'M' : 'L'} ${x},${y}`;
    })
    .join(' ');

  // Generate area for min-max range
  const areaPath = [
    ...data.dataPoints.map((point, i) => {
      const x = scaleX(i);
      const y = scaleY(Number(point.maxCost));
      return `${i === 0 ? 'M' : 'L'} ${x},${y}`;
    }),
    ...data.dataPoints.reverse().map((point, i) => {
      const x = scaleX(data.dataPoints.length - 1 - i);
      const y = scaleY(Number(point.minCost));
      return `L ${x},${y}`;
    }),
  ].join(' ') + ' Z';
  data.dataPoints.reverse(); // Restore order

  return (
    <div className="bg-white rounded-lg shadow-lg p-6">
      {/* Header */}
      <div className="flex items-center justify-between mb-6">
        <div className="flex items-center gap-3">
          <LineChart className="w-6 h-6 text-blue-600" />
          <h3 className="text-xl font-bold text-gray-900">
            {getOperationLabel(operation)} - Historical Gas Costs
          </h3>
        </div>
        
        {/* Period Selector */}
        <div className="flex gap-2">
          {(['24h', '7d', '30d'] as const).map((p) => (
            <button
              key={p}
              onClick={() => setSelectedPeriod(p)}
              className={`px-3 py-1 text-sm rounded-lg font-medium transition-colors ${
                selectedPeriod === p
                  ? 'bg-blue-600 text-white'
                  : 'bg-gray-100 text-gray-600 hover:bg-gray-200'
              }`}
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      {/* Chart */}
      <div className="relative" style={{ height: chartHeight }}>
        <svg
          width={chartWidth}
          height={chartHeight}
          className="w-full"
          viewBox={`0 0 ${chartWidth} ${chartHeight}`}
          preserveAspectRatio="xMidYMid meet"
        >
          {/* Y-axis grid lines */}
          {[0, 0.25, 0.5, 0.75, 1].map((ratio) => {
            const y = padding.top + ratio * (chartHeight - padding.top - padding.bottom);
            const value = maxY + paddingY - ratio * (rangeY + 2 * paddingY);
            return (
              <g key={ratio}>
                <line
                  x1={padding.left}
                  y1={y}
                  x2={chartWidth - padding.right}
                  y2={y}
                  stroke="#e5e7eb"
                  strokeWidth="1"
                />
                <text
                  x={padding.left - 10}
                  y={y + 4}
                  textAnchor="end"
                  className="text-xs fill-gray-500"
                >
                  {formatStroops(value.toString())}
                </text>
              </g>
            );
          })}

          {/* X-axis labels */}
          {data.dataPoints.map((point, i) => {
            if (i % Math.ceil(data.dataPoints.length / 6) !== 0) return null;
            return (
              <text
                key={i}
                x={scaleX(i)}
                y={chartHeight - padding.bottom + 20}
                textAnchor="middle"
                className="text-xs fill-gray-500"
              >
                {formatDate(point.timestamp, selectedPeriod)}
              </text>
            );
          })}

          {/* Min-Max area */}
          <path
            d={areaPath}
            fill="url(#areaGradient)"
            opacity="0.3"
          />

          {/* Average line */}
          <path
            d={avgPath}
            fill="none"
            stroke="#3b82f6"
            strokeWidth="3"
            strokeLinecap="round"
            strokeLinejoin="round"
          />

          {/* Data points */}
          {data.dataPoints.map((point, i) => (
            <circle
              key={i}
              cx={scaleX(i)}
              cy={scaleY(Number(point.averageCost))}
              r={hoveredPoint === i ? 6 : 4}
              fill="#3b82f6"
              stroke="white"
              strokeWidth="2"
              className="cursor-pointer transition-all"
              onMouseEnter={() => setHoveredPoint(i)}
              onMouseLeave={() => setHoveredPoint(null)}
            />
          ))}

          {/* Gradient definition */}
          <defs>
            <linearGradient id="areaGradient" x1="0" y1="0" x2="0" y2="1">
              <stop offset="0%" stopColor="#3b82f6" stopOpacity="0.5" />
              <stop offset="100%" stopColor="#3b82f6" stopOpacity="0.1" />
            </linearGradient>
          </defs>
        </svg>

        {/* Tooltip */}
        {hoveredPoint !== null && data.dataPoints[hoveredPoint] && (
          <div
            className="absolute bg-gray-900 text-white text-xs rounded-lg p-3 pointer-events-none shadow-lg"
            style={{
              left: scaleX(hoveredPoint),
              top: scaleY(Number(data.dataPoints[hoveredPoint].averageCost)) - 80,
              transform: 'translateX(-50%)',
            }}
          >
            <div className="font-semibold mb-2">
              {formatDate(data.dataPoints[hoveredPoint].timestamp, selectedPeriod)}
            </div>
            <div className="space-y-1">
              <div className="flex justify-between gap-4">
                <span className="text-gray-400">Average:</span>
                <span className="font-mono">
                  {formatStroops(data.dataPoints[hoveredPoint].averageCost)} XLM
                </span>
              </div>
              <div className="flex justify-between gap-4">
                <span className="text-gray-400">Min:</span>
                <span className="font-mono text-green-400">
                  {formatStroops(data.dataPoints[hoveredPoint].minCost)} XLM
                </span>
              </div>
              <div className="flex justify-between gap-4">
                <span className="text-gray-400">Max:</span>
                <span className="font-mono text-red-400">
                  {formatStroops(data.dataPoints[hoveredPoint].maxCost)} XLM
                </span>
              </div>
              <div className="flex justify-between gap-4 pt-1 border-t border-gray-700">
                <span className="text-gray-400">Samples:</span>
                <span>{data.dataPoints[hoveredPoint].sampleCount}</span>
              </div>
            </div>
          </div>
        )}
      </div>

      {/* Legend */}
      <div className="mt-6 flex items-center justify-center gap-6 text-sm">
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 rounded-full bg-blue-600"></div>
          <span className="text-gray-600">Average Cost</span>
        </div>
        <div className="flex items-center gap-2">
          <div className="w-3 h-3 bg-blue-200"></div>
          <span className="text-gray-600">Min-Max Range</span>
        </div>
      </div>
    </div>
  );
};

export default GasHistoricalChart;
