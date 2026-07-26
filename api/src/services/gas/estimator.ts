/**
 * Gas Cost Estimation Service
 * 
 * Provides comprehensive gas cost estimation for lending operations:
 * - Pre-transaction cost estimation with breakdowns
 * - Historical gas data aggregation and analysis
 * - Optimization suggestions (timing, batching, method)
 * - Cost comparison across operations
 * - Accuracy tracking (estimated vs actual)
 * - Configurable cost alerts
 */

import {
  GasOperation,
  GasCostEstimate,
  GasCostBreakdown,
  GasOptimizationSuggestion,
  HistoricalGasData,
  GasEstimateRequest,
  GasComparisonResponse,
  GasComparisonItem,
  GasCostAlert,
  GasAlertConfig,
  GasAccuracyMetric,
  GasAccuracyReport,
  HistoricalGasChartData,
  GasTimingRecommendation,
  BatchGasEstimate,
} from '../../types/gas';
import { StellarService } from '../stellar.service';
import { redisCacheService } from '../redisCache.service';
import { gasUsageAnalyticsService } from '../analytics/gasUsageAnalytics.service';
import logger from '../../utils/logger';
import { LendingOperation } from '../../types';

// Gas cost constants (in stroops, 1 XLM = 10,000,000 stroops)
const BASE_FEE = '100'; // Stellar base fee
const STORAGE_WRITE_COST = '10000'; // Cost per storage write
const CROSS_CONTRACT_CALL_COST = '5000'; // Cost per cross-contract call

// Operation complexity mappings (based on benchmarks)
const OPERATION_COMPLEXITY = {
  deposit: { storageWrites: 2, crossContractCalls: 1 },
  withdraw: { storageWrites: 2, crossContractCalls: 1 },
  borrow: { storageWrites: 3, crossContractCalls: 2 },
  repay: { storageWrites: 3, crossContractCalls: 2 },
  liquidation: { storageWrites: 4, crossContractCalls: 3 },
  flash_loan: { storageWrites: 2, crossContractCalls: 2 },
} as const;

// Baseline gas costs from benchmarks (in CPU instructions)
const BASELINE_CPU_COSTS = {
  deposit: 354765,
  withdraw: 144093,
  borrow: 244830,
  repay: 430316,
  liquidation: 394438,
  flash_loan: 70030,
} as const;

const BASELINE_MEM_COSTS = {
  deposit: 58682,
  withdraw: 20464,
  borrow: 32789,
  repay: 69458,
  liquidation: 48701,
  flash_loan: 13086,
} as const;

export class GasEstimatorService {
  private stellarService: StellarService;
  private alerts: Map<string, GasAlertConfig>;
  private accuracyMetrics: GasAccuracyMetric[];

  constructor() {
    this.stellarService = new StellarService();
    this.alerts = new Map();
    this.accuracyMetrics = [];
    this.loadAlertsFromCache();
  }

  /**
   * Main estimation function - provides comprehensive gas cost estimate
   */
  async estimateGasCost(request: GasEstimateRequest): Promise<GasCostEstimate> {
    try {
      const { operation, userAddress, assetAddress, amount, includeOptimizations, includeHistorical } = request;

      // Get real-time simulation from Stellar
      const simulationResult = await this.getSimulationData(
        operation,
        userAddress,
        assetAddress,
        amount
      );

      // Build cost breakdown
      const breakdown = this.calculateCostBreakdown(operation, simulationResult);

      // Generate optimization suggestions
      const optimizations = includeOptimizations 
        ? await this.generateOptimizations(operation, breakdown, amount)
        : [];

      // Get historical data if requested
      const historical = includeHistorical
        ? await this.getHistoricalData(operation, '30d')
        : undefined;

      // Calculate confidence based on historical data variance
      const confidence = this.calculateConfidence(historical);

      // Estimate USD cost if XLM price is available
      const estimatedUsdCost = await this.estimateUsdCost(breakdown.totalCost);

      const estimate: GasCostEstimate = {
        operation,
        breakdown,
        optimizations,
        historical,
        estimatedUsdCost,
        timestamp: new Date().toISOString(),
        confidence,
      };

      // Cache the estimate
      await this.cacheEstimate(operation, estimate);

      // Check for alerts
      await this.checkAlerts(operation, breakdown.totalCost, userAddress);

      return estimate;
    } catch (error) {
      logger.error('Gas estimation failed:', error);
      throw error;
    }
  }

  /**
   * Get simulation data from Stellar network
   */
  private async getSimulationData(
    operation: GasOperation,
    userAddress: string,
    assetAddress: string | undefined,
    amount: string
  ): Promise<{ cpuInstructions: string; memoryBytes: string; minResourceFee: string }> {
    try {
      // Map gas operation to lending operation
      const lendingOp = this.mapToLendingOperation(operation);
      
      // Use stellar service to estimate gas
      return await this.stellarService.estimateGas(
        lendingOp,
        userAddress,
        assetAddress,
        amount
      );
    } catch (error) {
      // Fallback to baseline if simulation fails
      logger.warn('Simulation failed, using baseline:', error);
      return {
        cpuInstructions: BASELINE_CPU_COSTS[operation].toString(),
        memoryBytes: BASELINE_MEM_COSTS[operation].toString(),
        minResourceFee: this.calculateBaselineFee(operation),
      };
    }
  }

  /**
   * Calculate detailed cost breakdown
   */
  private calculateCostBreakdown(
    operation: GasOperation,
    simulation: { cpuInstructions: string; memoryBytes: string; minResourceFee: string }
  ): GasCostBreakdown {
    const complexity = OPERATION_COMPLEXITY[operation];
    
    const baseCost = BASE_FEE;
    const storageCost = (BigInt(STORAGE_WRITE_COST) * BigInt(complexity.storageWrites)).toString();
    const crossContractCost = (BigInt(CROSS_CONTRACT_CALL_COST) * BigInt(complexity.crossContractCalls)).toString();
    
    const totalCost = (
      BigInt(baseCost) + 
      BigInt(storageCost) + 
      BigInt(crossContractCost) + 
      BigInt(simulation.minResourceFee)
    ).toString();

    return {
      baseCost,
      storageCost,
      crossContractCost,
      totalCost,
      cpuInstructions: simulation.cpuInstructions,
      memoryBytes: simulation.memoryBytes,
      minResourceFee: simulation.minResourceFee,
    };
  }

  /**
   * Generate optimization suggestions
   */
  private async generateOptimizations(
    operation: GasOperation,
    breakdown: GasCostBreakdown,
    amount: string
  ): Promise<GasOptimizationSuggestion[]> {
    const suggestions: GasOptimizationSuggestion[] = [];

    // Timing optimization - suggest low gas period
    const timingOptimization = await this.getTimingOptimization(operation);
    if (timingOptimization) {
      suggestions.push({
        type: 'timing',
        title: 'Execute During Low Gas Period',
        description: `Gas costs are typically ${timingOptimization.savingsPercent}% lower during off-peak hours (00:00-06:00 UTC)`,
        potentialSavings: timingOptimization.savings,
        priority: 'medium',
      });
    }

    // Batching optimization
    const batchAmount = BigInt(amount);
    if (batchAmount > BigInt(1000000)) {
      const batchSavings = (BigInt(breakdown.totalCost) * BigInt(15) / BigInt(100)).toString();
      suggestions.push({
        type: 'batching',
        title: 'Consider Batching Multiple Operations',
        description: 'Batching multiple operations in a single transaction can reduce per-operation costs by up to 15%',
        potentialSavings: batchSavings,
        priority: 'high',
      });
    }

    // Method optimization - alternative execution paths
    if (operation === 'repay') {
      suggestions.push({
        type: 'method',
        title: 'Use Partial Repayment for Large Amounts',
        description: 'For large repayments, splitting into smaller transactions may optimize gas usage',
        potentialSavings: (BigInt(breakdown.totalCost) * BigInt(10) / BigInt(100)).toString(),
        priority: 'low',
      });
    }

    if (operation === 'liquidation') {
      suggestions.push({
        type: 'method',
        title: 'Use Flash Loan for Liquidation',
        description: 'Flash loan liquidations can be more gas-efficient for large positions',
        potentialSavings: (BigInt(breakdown.totalCost) * BigInt(20) / BigInt(100)).toString(),
        priority: 'high',
      });
    }

    return suggestions;
  }

  /**
   * Get historical gas data for an operation
   */
  async getHistoricalData(operation: GasOperation, period: string): Promise<HistoricalGasData> {
    const cacheKey = redisCacheService.buildKey('gas', `historical:${operation}:${period}`);
    const cached = await redisCacheService.get<HistoricalGasData>(cacheKey);
    
    if (cached) {
      return cached;
    }

    // Simulate historical data (in production, aggregate from actual transactions)
    const baseline = BigInt(this.calculateBaselineFee(operation));
    const variance = baseline / BigInt(10); // 10% variance

    const historicalData: HistoricalGasData = {
      operation,
      averageCost: baseline.toString(),
      minCost: (baseline - variance).toString(),
      maxCost: (baseline + variance).toString(),
      stdDeviation: (variance / BigInt(2)).toString(),
      sampleCount: this.getSampleCount(period),
      period,
    };

    await redisCacheService.set(cacheKey, historicalData, 3600); // Cache 1 hour
    return historicalData;
  }

  /**
   * Compare gas costs across all operations
   */
  async compareOperations(): Promise<GasComparisonResponse> {
    const operations: GasOperation[] = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
    const costs: GasComparisonItem[] = [];

    for (const operation of operations) {
      const historical = await this.getHistoricalData(operation, '7d');
      costs.push({
        operation,
        averageCost: historical.averageCost,
        relativeCost: 0, // Will be calculated below
        rank: 0, // Will be calculated below
      });
    }

    // Sort by cost (cheapest first)
    costs.sort((a, b) => {
      const aCost = BigInt(a.averageCost);
      const bCost = BigInt(b.averageCost);
      return aCost < bCost ? -1 : aCost > bCost ? 1 : 0;
    });

    // Calculate relative costs and ranks
    const cheapest = costs[0];
    if (!cheapest) {
      return { operations: [], referenceOperation: 'deposit', timestamp: new Date().toISOString() };
    }
    const referenceCost = BigInt(cheapest.averageCost);
    costs.forEach((item, index) => {
      item.rank = index + 1;
      item.relativeCost = Number(BigInt(item.averageCost) * BigInt(100) / referenceCost) / 100;
    });

    return {
      operations: costs,
      referenceOperation: cheapest.operation,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Get historical gas chart data
   */
  async getHistoricalChart(operation: GasOperation, period: string): Promise<HistoricalGasChartData> {
    const cacheKey = redisCacheService.buildKey('gas', `chart:${operation}:${period}`);
    const cached = await redisCacheService.get<HistoricalGasChartData>(cacheKey);
    
    if (cached) {
      return cached;
    }

    // Generate chart data points (in production, aggregate from actual transactions)
    const dataPoints = this.generateChartDataPoints(operation, period);

    const chartData: HistoricalGasChartData = {
      operation,
      dataPoints,
      period,
    };

    await redisCacheService.set(cacheKey, chartData, 1800); // Cache 30 minutes
    return chartData;
  }

  /**
   * Configure gas cost alert
   */
  async configureAlert(config: GasAlertConfig): Promise<void> {
    const alertId = `${config.userAddress || 'global'}:${config.operation}`;
    this.alerts.set(alertId, config);
    
    // Persist to cache
    const cacheKey = redisCacheService.buildKey('gas', `alerts:${alertId}`);
    await redisCacheService.set(cacheKey, config, 86400 * 30); // 30 days
    
    logger.info('Gas alert configured:', config);
  }

  /**
   * Get all alerts for a user
   */
  async getAlerts(userAddress?: string): Promise<GasAlertConfig[]> {
    const alerts: GasAlertConfig[] = [];
    
    for (const [_, config] of this.alerts) {
      if (!userAddress || config.userAddress === userAddress) {
        alerts.push(config);
      }
    }
    
    return alerts;
  }

  /**
   * Record actual gas cost for accuracy tracking
   */
  async recordActualCost(
    operation: GasOperation,
    estimatedCost: string,
    actualCost: string,
    txHash: string
  ): Promise<void> {
    const estimated = BigInt(estimatedCost);
    const actual = BigInt(actualCost);
    const difference = actual > estimated ? actual - estimated : estimated - actual;
    const errorPercent = Number(difference * BigInt(10000) / actual) / 100;

    const metric: GasAccuracyMetric = {
      operation,
      estimated: estimatedCost,
      actual: actualCost,
      difference: difference.toString(),
      errorPercent,
      txHash,
      timestamp: new Date().toISOString(),
    };

    this.accuracyMetrics.push(metric);

    // Keep only last 1000 metrics
    if (this.accuracyMetrics.length > 1000) {
      this.accuracyMetrics.shift();
    }

    // Cache metric
    const cacheKey = redisCacheService.buildKey('gas', `accuracy:${txHash}`);
    await redisCacheService.set(cacheKey, metric, 86400 * 7); // 7 days

    // Feed the observed cost into gas usage analytics (issue #483) so
    // per-function stats/anomaly detection reflect real transaction data.
    gasUsageAnalyticsService.recordSample({
      functionName: operation,
      gasUsed: Number(actual),
      timestamp: Date.now(),
      txHash,
    });

    logger.info('Gas accuracy recorded:', { operation, errorPercent: `${errorPercent}%` });
  }

  /**
   * Generate accuracy report
   */
  async getAccuracyReport(period: string = '7d'): Promise<GasAccuracyReport> {
    const cutoffDate = this.getPeriodCutoff(period);
    const relevantMetrics = this.accuracyMetrics.filter(
      m => new Date(m.timestamp) >= cutoffDate
    );

    if (relevantMetrics.length === 0) {
      throw new Error('Insufficient data for accuracy report');
    }

    // Calculate overall metrics
    const totalError = relevantMetrics.reduce(
      (sum, m) => sum + BigInt(m.difference),
      BigInt(0)
    );
    const meanAbsoluteError = (totalError / BigInt(relevantMetrics.length)).toString();
    
    const totalPercentError = relevantMetrics.reduce((sum, m) => sum + m.errorPercent, 0);
    const meanPercentageError = totalPercentError / relevantMetrics.length;
    
    const within10Percent = relevantMetrics.filter(m => m.errorPercent <= 10).length;
    const accuracyWithin10Percent = (within10Percent / relevantMetrics.length) * 100;

    // Calculate per-operation metrics
    const byOperation: Record<string, any> = {};
    const operations: GasOperation[] = ['deposit', 'withdraw', 'borrow', 'repay', 'liquidation', 'flash_loan'];
    
    for (const operation of operations) {
      const opMetrics = relevantMetrics.filter(m => m.operation === operation);
      if (opMetrics.length > 0) {
        const opTotalError = opMetrics.reduce((sum, m) => sum + BigInt(m.difference), BigInt(0));
        const opTotalPercent = opMetrics.reduce((sum, m) => sum + m.errorPercent, 0);
        
        byOperation[operation] = {
          meanAbsoluteError: (opTotalError / BigInt(opMetrics.length)).toString(),
          meanPercentageError: opTotalPercent / opMetrics.length,
          sampleCount: opMetrics.length,
        };
      }
    }

    return {
      overall: {
        meanAbsoluteError,
        meanPercentageError,
        accuracyWithin10Percent,
      },
      byOperation: byOperation as any,
      period,
      timestamp: new Date().toISOString(),
    };
  }

  /**
   * Estimate batch gas cost
   */
  async estimateBatchCost(operations: GasEstimateRequest[]): Promise<BatchGasEstimate> {
    if (operations.length === 0) {
      throw new Error('At least one operation required');
    }

    // Estimate individual costs
    const estimates = await Promise.all(
      operations.map(op => this.estimateGasCost(op))
    );

    const individualTotalCost = estimates.reduce(
      (sum, est) => sum + BigInt(est.breakdown.totalCost),
      BigInt(0)
    );

    // Batching saves on base fees and some cross-contract calls
    const batchOverhead = BigInt(BASE_FEE); // Single base fee
    const sharedCostReduction = individualTotalCost * BigInt(12) / BigInt(100); // 12% savings
    const batchedTotalCost = individualTotalCost + batchOverhead - sharedCostReduction;
    const batchSavings = individualTotalCost - batchedTotalCost;
    const batchSavingsPercent = Number(batchSavings * BigInt(10000) / individualTotalCost) / 100;

    return {
      operations,
      individualTotalCost: individualTotalCost.toString(),
      batchedTotalCost: batchedTotalCost.toString(),
      batchSavings: batchSavings.toString(),
      batchSavingsPercent,
      recommendation: batchSavingsPercent > 10 
        ? 'Highly recommended - significant savings'
        : batchSavingsPercent > 5
        ? 'Recommended - moderate savings'
        : 'Optional - minimal savings',
    };
  }

  /**
   * Get timing recommendation
   */
  async getTimingRecommendation(operation: GasOperation): Promise<GasTimingRecommendation> {
    const currentHour = new Date().getUTCHours();
    const isOffPeak = currentHour >= 0 && currentHour < 6;
    
    const historical = await this.getHistoricalData(operation, '7d');
    const currentCost = BigInt(historical.averageCost);
    
    if (isOffPeak) {
      return {
        recommendedTime: 'now',
        expectedCost: currentCost.toString(),
        currentCost: currentCost.toString(),
        savings: '0',
        reason: 'Currently in optimal execution window (off-peak hours)',
      };
    }

    // Recommend next off-peak window
    const hoursUntilOffPeak = 24 - currentHour;
    const recommendedTime = new Date();
    recommendedTime.setUTCHours(recommendedTime.getUTCHours() + hoursUntilOffPeak);
    
    const offPeakCost = currentCost * BigInt(85) / BigInt(100); // 15% reduction
    const savings = currentCost - offPeakCost;

    return {
      recommendedTime: recommendedTime.toISOString(),
      expectedCost: offPeakCost.toString(),
      currentCost: currentCost.toString(),
      savings: savings.toString(),
      reason: `Off-peak hours (00:00-06:00 UTC) typically offer 15% lower gas costs`,
    };
  }

  // ─── Private Helper Methods ────────────────────────────────────────

  private mapToLendingOperation(gasOp: GasOperation): LendingOperation {
    const mapping: Record<GasOperation, LendingOperation> = {
      deposit: 'deposit',
      withdraw: 'withdraw',
      borrow: 'borrow',
      repay: 'repay',
      liquidation: 'withdraw', // Use withdraw as proxy
      flash_loan: 'borrow', // Use borrow as proxy
    };
    return mapping[gasOp];
  }

  private calculateBaselineFee(operation: GasOperation): string {
    const complexity = OPERATION_COMPLEXITY[operation];
    const baseFee = BigInt(BASE_FEE);
    const storageFee = BigInt(STORAGE_WRITE_COST) * BigInt(complexity.storageWrites);
    const crossContractFee = BigInt(CROSS_CONTRACT_CALL_COST) * BigInt(complexity.crossContractCalls);
    const cpuFee = BigInt(BASELINE_CPU_COSTS[operation]) / BigInt(100); // Rough conversion
    
    return (baseFee + storageFee + crossContractFee + cpuFee).toString();
  }

  private calculateConfidence(historical?: HistoricalGasData): 'high' | 'medium' | 'low' {
    if (!historical) return 'medium';
    
    const variance = BigInt(historical.maxCost) - BigInt(historical.minCost);
    const average = BigInt(historical.averageCost);
    const variancePercent = Number(variance * BigInt(100) / average);
    
    if (variancePercent < 10) return 'high';
    if (variancePercent < 25) return 'medium';
    return 'low';
  }

  private async estimateUsdCost(stroopsCost: string): Promise<string | undefined> {
    try {
      // Convert stroops to XLM (1 XLM = 10,000,000 stroops)
      const xlmCost = Number(BigInt(stroopsCost)) / 10000000;
      
      // In production, fetch real XLM price
      // For now, use approximate price of $0.10 per XLM
      const xlmPrice = 0.10;
      const usdCost = xlmCost * xlmPrice;
      
      return usdCost.toFixed(6);
    } catch {
      return undefined;
    }
  }

  private async getTimingOptimization(operation: GasOperation): Promise<{ savings: string; savingsPercent: number } | null> {
    const currentHour = new Date().getUTCHours();
    const isOffPeak = currentHour >= 0 && currentHour < 6;
    
    if (isOffPeak) {
      return null; // Already in optimal window
    }

    const historical = await this.getHistoricalData(operation, '7d');
    const currentCost = BigInt(historical.averageCost);
    const offPeakCost = currentCost * BigInt(85) / BigInt(100); // 15% reduction
    const savings = currentCost - offPeakCost;

    return {
      savings: savings.toString(),
      savingsPercent: 15,
    };
  }

  private getSampleCount(period: string): number {
    const counts: Record<string, number> = {
      '24h': 1440, // Minutes in a day
      '7d': 10080, // Minutes in a week
      '30d': 43200, // Minutes in 30 days
    };
    return counts[period] || 1000;
  }

  private generateChartDataPoints(operation: GasOperation, period: string): HistoricalGasChartData['dataPoints'] {
    const points: HistoricalGasChartData['dataPoints'] = [];
    const baseline = BigInt(this.calculateBaselineFee(operation));
    const now = Date.now();
    
    // Determine interval and count based on period
    const config = this.getChartConfig(period);
    
    for (let i = 0; i < config.pointCount; i++) {
      const timestamp = new Date(now - (config.pointCount - i) * config.intervalMs);
      
      // Add some realistic variance
      const variance = (Math.random() - 0.5) * 0.2; // ±10%
      const avgCost = baseline + (baseline * BigInt(Math.floor(variance * 100)) / BigInt(100));
      const minCost = avgCost * BigInt(95) / BigInt(100);
      const maxCost = avgCost * BigInt(105) / BigInt(100);
      
      points.push({
        timestamp: timestamp.toISOString(),
        averageCost: avgCost.toString(),
        minCost: minCost.toString(),
        maxCost: maxCost.toString(),
        sampleCount: Math.floor(Math.random() * 100) + 10,
      });
    }
    
    return points;
  }

  private getChartConfig(period: string): { pointCount: number; intervalMs: number } {
    const configs: Record<string, { pointCount: number; intervalMs: number }> = {
      '24h': { pointCount: 24, intervalMs: 3600000 }, // Hourly
      '7d': { pointCount: 28, intervalMs: 6 * 3600000 }, // 6-hourly
      '30d': { pointCount: 30, intervalMs: 24 * 3600000 }, // Daily
    };
    return configs[period] ?? { pointCount: 28, intervalMs: 6 * 3600000 };
  }

  private getPeriodCutoff(period: string): Date {
    const now = new Date();
    const periods: Record<string, number> = {
      '24h': 24 * 60 * 60 * 1000,
      '7d': 7 * 24 * 60 * 60 * 1000,
      '30d': 30 * 24 * 60 * 60 * 1000,
    };
    const ms = periods[period] ?? 7 * 24 * 60 * 60 * 1000;
    return new Date(now.getTime() - ms);
  }

  private async cacheEstimate(operation: GasOperation, estimate: GasCostEstimate): Promise<void> {
    const cacheKey = redisCacheService.buildKey('gas', `estimate:${operation}`);
    await redisCacheService.set(cacheKey, estimate, 300); // Cache 5 minutes
  }

  private async checkAlerts(operation: GasOperation, actualCost: string, userAddress?: string): Promise<void> {
    const alertsToCheck = Array.from(this.alerts.values()).filter(
      alert => alert.enabled && alert.operation === operation &&
      (!alert.userAddress || alert.userAddress === userAddress)
    );

    const cost = BigInt(actualCost);

    for (const alert of alertsToCheck) {
      const threshold = BigInt(alert.threshold);
      if (cost > threshold) {
        const overage = cost - threshold;
        const overagePercent = Number(overage * BigInt(10000) / threshold) / 100;

        const costAlert: GasCostAlert = {
          id: `alert_${Date.now()}`,
          operation,
          threshold: alert.threshold,
          actualCost,
          overagePercent,
          timestamp: new Date().toISOString(),
          userAddress: alert.userAddress,
        };

        // In production, emit to WebSocket or notification service
        logger.warn('Gas cost alert triggered:', costAlert);
        
        // Cache alert
        const cacheKey = redisCacheService.buildKey('gas', `alert:${costAlert.id}`);
        await redisCacheService.set(cacheKey, costAlert, 86400); // 24 hours
      }
    }
  }

  private async loadAlertsFromCache(): Promise<void> {
    // In production, load from persistent storage
    logger.info('Gas alerts initialized');
  }
}

export const gasEstimatorService = new GasEstimatorService();
