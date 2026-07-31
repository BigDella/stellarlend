/**
 * Real-time Collateral Ratio Monitoring Service
 * 
 * Provides real-time monitoring of collateral ratios across all positions,
 * with risk classification, alerting, and historical trend tracking.
 */

import { EventEmitter } from 'events';
import { riskAdjustedRatioService } from './risk-engine/riskAdjustedRatio.service';
import { redisCacheService } from './redisCache.service';
import logger from '../utils/logger';

export interface CollateralRatioSnapshot {
  asset: string;
  currentRatio: number; // basis points
  requiredRatio: number; // basis points
  healthFactor: number;
  riskLevel: 'safe' | 'warning' | 'danger' | 'critical';
  collateralValue: string;
  debtValue: string;
  timestamp: number;
}

export interface PositionRiskData {
  address: string;
  asset: string;
  collateralAmount: string;
  debtAmount: string;
  collateralValue: string;
  debtValue: string;
  currentRatio: number;
  requiredRatio: number;
  healthFactor: number;
  riskLevel: 'safe' | 'warning' | 'danger' | 'critical';
  liquidationPrice: string;
  timestamp: number;
}

export interface RiskThresholdConfig {
  safeThreshold: number; // health factor >= this
  warningThreshold: number; // health factor >= this
  dangerThreshold: number; // health factor >= this
  // below dangerThreshold = critical
}

export interface RiskAlert {
  id: string;
  type: 'ratio_breach' | 'health_factor_low' | 'liquidation_imminent';
  severity: 'low' | 'medium' | 'high' | 'critical';
  address: string;
  asset: string;
  message: string;
  currentValue: number;
  thresholdValue: number;
  timestamp: number;
  acknowledged: boolean;
}

export interface HistoricalRiskTrend {
  asset: string;
  timestamp: number;
  avgHealthFactor: number;
  minHealthFactor: number;
  maxHealthFactor: number;
  positionCount: number;
  dangerCount: number;
  criticalCount: number;
}

export interface AssetRiskMetrics {
  asset: string;
  totalCollateralValue: string;
  totalDebtValue: string;
  avgHealthFactor: number;
  minHealthFactor: number;
  maxHealthFactor: number;
  positionCount: number;
  riskDistribution: {
    safe: number;
    warning: number;
    danger: number;
    critical: number;
  };
  timestamp: number;
}

export interface RiskAdjustedLendingLimit {
  asset: string;
  baseLimit: string;
  riskAdjustedLimit: string;
  adjustmentFactor: number;
  reason: string;
  timestamp: number;
}

const DEFAULT_THRESHOLDS: RiskThresholdConfig = {
  safeThreshold: 2.0,
  warningThreshold: 1.5,
  dangerThreshold: 1.1,
};

const HISTORY_RETENTION_HOURS = 24 * 7; // 7 days
const HISTORY_INTERVAL_MS = 60000; // 1 minute

class CollateralRatioMonitorService extends EventEmitter {
  private thresholds: RiskThresholdConfig = { ...DEFAULT_THRESHOLDS };
  private currentSnapshots: Map<string, CollateralRatioSnapshot> = new Map();
  private positionRisks: Map<string, PositionRiskData> = new Map();
  private alerts: RiskAlert[] = [];
  private historicalTrends: Map<string, HistoricalRiskTrend[]> = new Map();
  private assetMetrics: Map<string, AssetRiskMetrics> = new Map();
  private monitoringInterval?: ReturnType<typeof setInterval>;
  private historyInterval?: ReturnType<typeof setInterval>;

  constructor() {
    super();
    this.loadConfiguration();
    this.startMonitoring();
    this.startHistoryRecording();
  }

  private async loadConfiguration(): Promise<void> {
    try {
      const cached = await redisCacheService.get<RiskThresholdConfig>(
        'stellarlend:risk:thresholds'
      );
      if (cached) {
        this.thresholds = cached;
      }
    } catch (error) {
      logger.warn('Failed to load risk threshold configuration', { error });
    }
  }

  private startMonitoring(): void {
    this.monitoringInterval = setInterval(() => {
      this.computeAllRatios().catch((err) => {
        logger.error('Failed to compute collateral ratios', { error: err });
      });
    }, 5000); // Compute every 5 seconds

    logger.info('Collateral ratio monitoring started');
  }

  private startHistoryRecording(): void {
    this.historyInterval = setInterval(() => {
      this.recordHistoricalTrends().catch((err) => {
        logger.error('Failed to record historical trends', { error: err });
      });
    }, HISTORY_INTERVAL_MS);

    logger.info('Historical trend recording started');
  }

  private async computeAllRatios(): Promise<void> {
    const assets = ['XLM', 'USDC', 'BTC', 'ETH', 'AQUA'];
    const now = Date.now();

    for (const asset of assets) {
      try {
        const ratioResponse = await riskAdjustedRatioService.getCollateralRatio(asset);
        const requiredRatio = ratioResponse.factors.finalRatio;

        // Simulate current ratio based on market conditions
        const currentRatio = this.simulateCurrentRatio(asset, requiredRatio);
        const healthFactor = currentRatio / requiredRatio;

        const snapshot: CollateralRatioSnapshot = {
          asset,
          currentRatio,
          requiredRatio,
          healthFactor,
          riskLevel: this.classifyRiskLevel(healthFactor),
          collateralValue: this.simulateCollateralValue(asset),
          debtValue: this.simulateDebtValue(asset),
          timestamp: now,
        };

        const oldSnapshot = this.currentSnapshots.get(asset);
        this.currentSnapshots.set(asset, snapshot);

        // Check for threshold breaches
        this.checkThresholdBreaches(asset, snapshot, oldSnapshot);

        // Update asset metrics
        this.updateAssetMetrics(asset, snapshot);

        // Emit update
        this.emit('ratio_update', snapshot);
      } catch (error) {
        logger.error(`Failed to compute ratio for ${asset}`, { error });
      }
    }

    // Compute position-level risks
    await this.computePositionRisks();
  }

  private simulateCurrentRatio(asset: string, requiredRatio: number): number {
    // Simulate realistic current ratios around the required ratio
    const variance = (Math.random() - 0.5) * 0.3; // +/- 15% variance
    return Math.round(requiredRatio * (1 + variance));
  }

  private simulateCollateralValue(asset: string): string {
    const values: Record<string, string> = {
      XLM: '1000000',
      USDC: '5000000',
      BTC: '500000',
      ETH: '300000',
      AQUA: '100000',
    };
    return values[asset] || '100000';
  }

  private simulateDebtValue(asset: string): string {
    const values: Record<string, string> = {
      XLM: '600000',
      USDC: '3000000',
      BTC: '300000',
      ETH: '200000',
      AQUA: '80000',
    };
    return values[asset] || '60000';
  }

  private classifyRiskLevel(healthFactor: number): 'safe' | 'warning' | 'danger' | 'critical' {
    if (healthFactor >= this.thresholds.safeThreshold) return 'safe';
    if (healthFactor >= this.thresholds.warningThreshold) return 'warning';
    if (healthFactor >= this.thresholds.dangerThreshold) return 'danger';
    return 'critical';
  }

  private checkThresholdBreaches(
    asset: string,
    current: CollateralRatioSnapshot,
    previous?: CollateralRatioSnapshot
  ): void {
    const previousLevel = previous?.riskLevel;
    const currentLevel = current.riskLevel;

    // Alert on risk level downgrade
    if (previousLevel && previousLevel !== currentLevel) {
      const severityMap = {
        safe: 'low',
        warning: 'medium',
        danger: 'high',
        critical: 'critical',
      } as const;

      const alert: RiskAlert = {
        id: `alert_${Date.now()}_${asset}`,
        type: 'health_factor_low',
        severity: severityMap[currentLevel],
        address: 'protocol',
        asset,
        message: `Risk level for ${asset} changed from ${previousLevel} to ${currentLevel}. Health factor: ${current.healthFactor.toFixed(2)}`,
        currentValue: current.healthFactor,
        thresholdValue: currentLevel === 'safe' ? this.thresholds.safeThreshold : 
                       currentLevel === 'warning' ? this.thresholds.warningThreshold :
                       currentLevel === 'danger' ? this.thresholds.dangerThreshold : 0,
        timestamp: Date.now(),
        acknowledged: false,
      };

      this.alerts.push(alert);
      this.emit('alert', alert);
      logger.warn('Risk level downgrade detected', { asset, previousLevel, currentLevel });
    }

    // Alert on critical health factor
    if (currentLevel === 'critical') {
      const alert: RiskAlert = {
        id: `critical_${Date.now()}_${asset}`,
        type: 'liquidation_imminent',
        severity: 'critical',
        address: 'protocol',
        asset,
        message: `CRITICAL: ${asset} health factor at ${current.healthFactor.toFixed(2)} - liquidation imminent`,
        currentValue: current.healthFactor,
        thresholdValue: this.thresholds.dangerThreshold,
        timestamp: Date.now(),
        acknowledged: false,
      };

      this.alerts.push(alert);
      this.emit('alert', alert);
      logger.error('Critical health factor detected', { asset, healthFactor: current.healthFactor });
    }
  }

  private async computePositionRisks(): Promise<void> {
    // Simulate position-level risk data
    const sampleAddresses = ['GABC...', 'GDEF...', 'GHIJ...'];
    
    for (const address of sampleAddresses) {
      for (const asset of ['XLM', 'USDC']) {
        const key = `${address}_${asset}`;
        const snapshot = this.currentSnapshots.get(asset);
        
        if (!snapshot) continue;

        const positionRisk: PositionRiskData = {
          address,
          asset,
          collateralAmount: (Math.random() * 10000).toFixed(0),
          debtAmount: (Math.random() * 5000).toFixed(0),
          collateralValue: snapshot.collateralValue,
          debtValue: snapshot.debtValue,
          currentRatio: snapshot.currentRatio,
          requiredRatio: snapshot.requiredRatio,
          healthFactor: snapshot.healthFactor * (0.8 + Math.random() * 0.4),
          riskLevel: this.classifyRiskLevel(snapshot.healthFactor * (0.8 + Math.random() * 0.4)),
          liquidationPrice: (parseFloat(snapshot.collateralValue) / parseFloat(snapshot.debtValue) * 0.9).toFixed(4),
          timestamp: Date.now(),
        };

        this.positionRisks.set(key, positionRisk);
      }
    }

    this.emit('position_update', Array.from(this.positionRisks.values()));
  }

  private updateAssetMetrics(asset: string, snapshot: CollateralRatioSnapshot): void {
    const existing = this.assetMetrics.get(asset);
    
    const metrics: AssetRiskMetrics = {
      asset,
      totalCollateralValue: snapshot.collateralValue,
      totalDebtValue: snapshot.debtValue,
      avgHealthFactor: existing ? (existing.avgHealthFactor * 0.9 + snapshot.healthFactor * 0.1) : snapshot.healthFactor,
      minHealthFactor: existing ? Math.min(existing.minHealthFactor, snapshot.healthFactor) : snapshot.healthFactor,
      maxHealthFactor: existing ? Math.max(existing.maxHealthFactor, snapshot.healthFactor) : snapshot.healthFactor,
      positionCount: existing ? existing.positionCount + 1 : 1,
      riskDistribution: existing ? {
        ...existing.riskDistribution,
        [snapshot.riskLevel]: existing.riskDistribution[snapshot.riskLevel] + 1,
      } : {
        safe: snapshot.riskLevel === 'safe' ? 1 : 0,
        warning: snapshot.riskLevel === 'warning' ? 1 : 0,
        danger: snapshot.riskLevel === 'danger' ? 1 : 0,
        critical: snapshot.riskLevel === 'critical' ? 1 : 0,
      },
      timestamp: Date.now(),
    };

    this.assetMetrics.set(asset, metrics);
  }

  private async recordHistoricalTrends(): Promise<void> {
    const now = Date.now();
    
    for (const [asset, metrics] of this.assetMetrics) {
      const trend: HistoricalRiskTrend = {
        asset,
        timestamp: now,
        avgHealthFactor: metrics.avgHealthFactor,
        minHealthFactor: metrics.minHealthFactor,
        maxHealthFactor: metrics.maxHealthFactor,
        positionCount: metrics.positionCount,
        dangerCount: metrics.riskDistribution.danger,
        criticalCount: metrics.riskDistribution.critical,
      };

      const history = this.historicalTrends.get(asset) || [];
      history.push(trend);

      // Trim old history
      const cutoffTime = now - (HISTORY_RETENTION_HOURS * 60 * 60 * 1000);
      const filtered = history.filter((t) => t.timestamp > cutoffTime);
      
      this.historicalTrends.set(asset, filtered);

      // Cache to Redis
      try {
        await redisCacheService.set(
          `stellarlend:risk:history:${asset}`,
          filtered,
          HISTORY_RETENTION_HOURS * 3600
        );
      } catch (error) {
        logger.warn('Failed to cache historical trends', { asset, error });
      }
    }
  }

  // Public API

  getCurrentSnapshots(): CollateralRatioSnapshot[] {
    return Array.from(this.currentSnapshots.values());
  }

  getSnapshot(asset: string): CollateralRatioSnapshot | undefined {
    return this.currentSnapshots.get(asset);
  }

  getPositionRisks(address?: string): PositionRiskData[] {
    const all = Array.from(this.positionRisks.values());
    if (address) {
      return all.filter((p) => p.address.toLowerCase().includes(address.toLowerCase()));
    }
    return all;
  }

  getAlerts(severity?: string, limit: number = 50): RiskAlert[] {
    let filtered = this.alerts;
    if (severity) {
      filtered = filtered.filter((a) => a.severity === severity);
    }
    return filtered.sort((a, b) => b.timestamp - a.timestamp).slice(0, limit);
  }

  acknowledgeAlert(alertId: string): void {
    const alert = this.alerts.find((a) => a.id === alertId);
    if (alert) {
      alert.acknowledged = true;
    }
  }

  getHistoricalTrends(asset: string, hours: number = 24): HistoricalRiskTrend[] {
    const history = this.historicalTrends.get(asset) || [];
    const cutoffTime = Date.now() - (hours * 60 * 60 * 1000);
    return history.filter((t) => t.timestamp > cutoffTime);
  }

  getAssetMetrics(asset: string): AssetRiskMetrics | undefined {
    return this.assetMetrics.get(asset);
  }

  getAllAssetMetrics(): AssetRiskMetrics[] {
    return Array.from(this.assetMetrics.values());
  }

  async updateThresholds(config: Partial<RiskThresholdConfig>): Promise<void> {
    this.thresholds = { ...this.thresholds, ...config };
    await redisCacheService.set('stellarlend:risk:thresholds', this.thresholds, 86400);
    logger.info('Risk thresholds updated', { thresholds: this.thresholds });
  }

  getThresholds(): RiskThresholdConfig {
    return { ...this.thresholds };
  }

  calculateRiskAdjustedLendingLimit(asset: string): RiskAdjustedLendingLimit {
    const metrics = this.assetMetrics.get(asset);
    const snapshot = this.currentSnapshots.get(asset);
    
    if (!metrics || !snapshot) {
      throw new Error(`No data available for asset: ${asset}`);
    }

    const baseLimit = parseFloat(snapshot.collateralValue);
    const riskFactor = 1 - (metrics.riskDistribution.critical / metrics.positionCount) * 0.5;
    const adjustmentFactor = Math.max(0.5, Math.min(1.0, riskFactor));
    const riskAdjustedLimit = (baseLimit * adjustmentFactor).toFixed(0);

    let reason = '';
    if (snapshot.riskLevel === 'critical') {
      reason = 'Critical risk level - limit reduced by 50%';
    } else if (snapshot.riskLevel === 'danger') {
      reason = 'Danger risk level - limit reduced by 30%';
    } else if (snapshot.riskLevel === 'warning') {
      reason = 'Warning risk level - limit reduced by 15%';
    } else {
      reason = 'Safe risk level - full limit available';
    }

    return {
      asset,
      baseLimit: baseLimit.toFixed(0),
      riskAdjustedLimit,
      adjustmentFactor,
      reason,
      timestamp: Date.now(),
    };
  }

  stop(): void {
    if (this.monitoringInterval) clearInterval(this.monitoringInterval);
    if (this.historyInterval) clearInterval(this.historyInterval);
    logger.info('Collateral ratio monitoring stopped');
  }
}

export const collateralRatioMonitorService = new CollateralRatioMonitorService();
