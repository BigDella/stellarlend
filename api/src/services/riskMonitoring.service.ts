import { StellarService } from './stellar.service';

export interface PoolHealthMetrics {
  poolId: string;
  utilizationRate: number;
  totalSupplied: string;
  totalBorrowed: string;
  availableLiquidity: string;
  averageLtv: number;
  concentrationRisk: number;
}

export interface LiquidationRiskEntry {
  user: string;
  poolId: string;
  healthFactor: number;
  collateralValue: string;
  debtValue: string;
  liquidationThreshold: number;
  riskLevel: 'Safe' | 'Warning' | 'Danger' | 'Critical';
}

export interface OracleHealthStatus {
  asset: string;
  lastUpdateTimestamp: number;
  price: string;
  stalenessSeconds: number;
  deviationFromTwap: number;
  isHealthy: boolean;
}

export interface ProtocolSafetyScore {
  overallScore: number;
  liquidityScore: number;
  solvencyScore: number;
  oracleHealthScore: number;
  concentrationScore: number;
  timestamp: number;
}

export interface AlertConfig {
  healthFactorThreshold: number;
  utilizationThreshold: number;
  concentrationThreshold: number;
  oracleStalenessThreshold: number;
}

export interface RiskAlert {
  id: string;
  severity: 'low' | 'medium' | 'high' | 'critical';
  type: string;
  message: string;
  timestamp: number;
  acknowledged: boolean;
}

export interface DashboardSnapshot {
  timestamp: number;
  poolHealth: PoolHealthMetrics[];
  oracleHealth: OracleHealthStatus[];
  safetyScore: ProtocolSafetyScore;
  activeAlerts: RiskAlert[];
  liquidationAtRisk: number;
}

class RiskMonitoringService {
  private stellarService: StellarService;
  private alertConfig: AlertConfig = {
    healthFactorThreshold: 12000,
    utilizationThreshold: 9000,
    concentrationThreshold: 3000,
    oracleStalenessThreshold: 300,
  };

  constructor() {
    this.stellarService = new StellarService();
  }

  async getPoolHealthMetrics(poolId: string): Promise<PoolHealthMetrics> {
    const totalSupplied = '1000000';
    const totalBorrowed = '750000';

    const utilizationRate = (parseInt(totalBorrowed) / parseInt(totalSupplied)) * 10000;
    const availableLiquidity = (parseInt(totalSupplied) - parseInt(totalBorrowed)).toString();

    return {
      poolId,
      utilizationRate,
      totalSupplied,
      totalBorrowed,
      availableLiquidity,
      averageLtv: 6500,
      concentrationRisk: 2500,
    };
  }

  async getLiquidationRiskHeatmap(poolId?: string): Promise<Map<string, LiquidationRiskEntry[]>> {
    const heatmap = new Map<string, LiquidationRiskEntry[]>();

    const mockEntry: LiquidationRiskEntry = {
      user: 'GABC...',
      poolId: poolId || 'pool-1',
      healthFactor: 11000,
      collateralValue: '100000',
      debtValue: '80000',
      liquidationThreshold: 8000,
      riskLevel: 'Danger',
    };

    heatmap.set(poolId || 'pool-1', [mockEntry]);
    return heatmap;
  }

  async getOracleHealthStatus(): Promise<OracleHealthStatus[]> {
    return [
      {
        asset: 'XLM',
        lastUpdateTimestamp: Date.now() - 120000,
        price: '0.12',
        stalenessSeconds: 120,
        deviationFromTwap: 150,
        isHealthy: true,
      },
      {
        asset: 'USDC',
        lastUpdateTimestamp: Date.now() - 60000,
        price: '1.00',
        stalenessSeconds: 60,
        deviationFromTwap: 10,
        isHealthy: true,
      },
    ];
  }

  async getProtocolSafetyScore(): Promise<ProtocolSafetyScore> {
    const liquidityScore = 8500;
    const solvencyScore = 9000;
    const oracleHealthScore = 8800;
    const concentrationScore = 7500;

    const overallScore = Math.floor(
      liquidityScore * 0.25 +
        solvencyScore * 0.35 +
        oracleHealthScore * 0.2 +
        concentrationScore * 0.2
    );

    return {
      overallScore,
      liquidityScore,
      solvencyScore,
      oracleHealthScore,
      concentrationScore,
      timestamp: Date.now(),
    };
  }

  async getMetricTrends(metric: string, period: string): Promise<{ timestamp: number; value: number; metric: string }[]> {
    const trends = [];
    const now = Date.now();
    const periodMs = period === '24h' ? 24 * 60 * 60 * 1000 : 7 * 24 * 60 * 60 * 1000;

    for (let i = 0; i < 10; i++) {
      trends.push({
        timestamp: now - (periodMs * i) / 10,
        value: 7000 + Math.floor(Math.random() * 2000),
        metric,
      });
    }

    return trends.reverse();
  }

  /**
   * Generate alerts from current metrics against the configured thresholds.
   * Returns alerts sorted by severity (critical first).
   */
  async getActiveAlerts(severity?: string, limit: number = 50): Promise<RiskAlert[]> {
    const alerts: RiskAlert[] = [];

    const [poolHealth, oracleHealth, heatmap] = await Promise.all([
      this.getPoolHealthMetrics('pool-1'),
      this.getOracleHealthStatus(),
      this.getLiquidationRiskHeatmap(),
    ]);

    // High-utilization alert
    if (poolHealth.utilizationRate >= this.alertConfig.utilizationThreshold) {
      alerts.push({
        id: 'util-pool-1',
        severity: poolHealth.utilizationRate >= 9500 ? 'critical' : 'high',
        type: 'high_utilization',
        message: `Pool pool-1 utilization at ${(poolHealth.utilizationRate / 100).toFixed(1)}% — above ${(this.alertConfig.utilizationThreshold / 100).toFixed(0)}% threshold`,
        timestamp: Date.now(),
        acknowledged: false,
      });
    }

    // Concentration risk alert
    if (poolHealth.concentrationRisk >= this.alertConfig.concentrationThreshold) {
      alerts.push({
        id: 'conc-pool-1',
        severity: 'medium',
        type: 'concentration_risk',
        message: `Pool pool-1 concentration risk at ${(poolHealth.concentrationRisk / 100).toFixed(1)}%`,
        timestamp: Date.now(),
        acknowledged: false,
      });
    }

    // Oracle staleness alerts
    for (const oracle of oracleHealth) {
      if (oracle.stalenessSeconds > this.alertConfig.oracleStalenessThreshold) {
        alerts.push({
          id: `oracle-stale-${oracle.asset}`,
          severity: 'high',
          type: 'oracle_staleness',
          message: `Oracle price for ${oracle.asset} is ${oracle.stalenessSeconds}s stale (threshold: ${this.alertConfig.oracleStalenessThreshold}s)`,
          timestamp: Date.now(),
          acknowledged: false,
        });
      }
      if (!oracle.isHealthy) {
        alerts.push({
          id: `oracle-unhealthy-${oracle.asset}`,
          severity: 'critical',
          type: 'oracle_unhealthy',
          message: `Oracle for ${oracle.asset} is reporting unhealthy status`,
          timestamp: Date.now(),
          acknowledged: false,
        });
      }
    }

    // Liquidation-risk alerts from heatmap
    for (const [pool, entries] of heatmap) {
      for (const entry of entries) {
        if (entry.healthFactor < this.alertConfig.healthFactorThreshold) {
          const sev = entry.riskLevel === 'Critical' ? 'critical' : entry.riskLevel === 'Danger' ? 'high' : 'medium';
          alerts.push({
            id: `liq-${pool}-${entry.user}`,
            severity: sev,
            type: 'liquidation_risk',
            message: `User ${entry.user} in pool ${pool} has health factor ${(entry.healthFactor / 10000).toFixed(2)} — ${entry.riskLevel}`,
            timestamp: Date.now(),
            acknowledged: false,
          });
        }
      }
    }

    // Sort by severity
    const severityOrder = { critical: 0, high: 1, medium: 2, low: 3 };
    alerts.sort((a, b) => severityOrder[a.severity] - severityOrder[b.severity]);

    const filtered = severity ? alerts.filter((a) => a.severity === severity) : alerts;
    return filtered.slice(0, limit);
  }

  async updateAlertConfiguration(config: Partial<AlertConfig>): Promise<void> {
    this.alertConfig = { ...this.alertConfig, ...config };
  }

  async getUserRiskProfile(address: string): Promise<{
    address: string;
    healthFactor: number;
    totalCollateralValue: string;
    totalDebtValue: string;
    ltv: number;
    liquidationPrice: string;
    riskLevel: string;
    positions: Array<{ poolId: string; collateral: string; debt: string; healthFactor: number }>;
  }> {
    return {
      address,
      healthFactor: 13500,
      totalCollateralValue: '150000',
      totalDebtValue: '90000',
      ltv: 6000,
      liquidationPrice: '0.08',
      riskLevel: 'Warning',
      positions: [
        {
          poolId: 'pool-1',
          collateral: '100000',
          debt: '60000',
          healthFactor: 14000,
        },
      ],
    };
  }

  /** Aggregate dashboard: all key metrics in one call. */
  async getDashboard(): Promise<DashboardSnapshot> {
    const [poolHealth, oracleHealth, safetyScore, activeAlerts, heatmap] = await Promise.all([
      this.getPoolHealthMetrics('pool-1'),
      this.getOracleHealthStatus(),
      this.getProtocolSafetyScore(),
      this.getActiveAlerts(),
      this.getLiquidationRiskHeatmap(),
    ]);

    const liquidationAtRisk = Array.from(heatmap.values())
      .flat()
      .filter((e) => e.riskLevel === 'Danger' || e.riskLevel === 'Critical').length;

    return {
      timestamp: Date.now(),
      poolHealth: [poolHealth],
      oracleHealth,
      safetyScore,
      activeAlerts,
      liquidationAtRisk,
    };
  }
}

export const riskMonitoringService = new RiskMonitoringService();
