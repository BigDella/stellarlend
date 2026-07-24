export interface PositionHealth {
  address: string;
  collateralAsset: string;
  debtAsset: string;
  collateralValue: number;
  debtValue: number;
  healthFactor: number;
  utilizationBps: number;
  liquidationPrice: number;
  estimatedProfit: number;
  gasCostStroops: number;
  netProfit: number;
  riskCategory: 'safe' | 'moderate' | 'danger' | 'critical';
  lastUpdated: number;
}

export interface HealthAlertThreshold {
  address: string;
  dangerHf: number;
  warningHf: number;
  enabled: boolean;
}

export class LiquidationMonitorService {
  private positions: Map<string, PositionHealth> = new Map();
  private thresholds: Map<string, HealthAlertThreshold> = new Map();
  private listeners: Array<(positions: PositionHealth[]) => void> = [];

  constructor() {
    this.seedMockPositions();
  }

  private seedMockPositions(): void {
    const now = Date.now();
    const mockPositions: PositionHealth[] = [
      {
        address: 'GABCDE12345',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 50000,
        debtValue: 35000,
        healthFactor: 1.35,
        utilizationBps: 7400,
        liquidationPrice: 0.09,
        estimatedProfit: 1250,
        gasCostStroops: 500000,
        netProfit: 1200,
        riskCategory: 'moderate',
        lastUpdated: now,
      },
      {
        address: 'GFGHIJ67890',
        collateralAsset: 'USDC',
        debtAsset: 'XLM',
        collateralValue: 120000,
        debtValue: 102000,
        healthFactor: 1.08,
        utilizationBps: 8500,
        liquidationPrice: 0.11,
        estimatedProfit: 4800,
        gasCostStroops: 550000,
        netProfit: 4745,
        riskCategory: 'danger',
        lastUpdated: now,
      },
      {
        address: 'GKLMNO11121',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 25000,
        debtValue: 24000,
        healthFactor: 1.02,
        utilizationBps: 9600,
        liquidationPrice: 0.08,
        estimatedProfit: 750,
        gasCostStroops: 480000,
        netProfit: 702,
        riskCategory: 'critical',
        lastUpdated: now,
      },
      {
        address: 'GPQRST31415',
        collateralAsset: 'USDC',
        debtAsset: 'USDT',
        collateralValue: 80000,
        debtValue: 40000,
        healthFactor: 1.85,
        utilizationBps: 5000,
        liquidationPrice: 0.15,
        estimatedProfit: 2000,
        gasCostStroops: 520000,
        netProfit: 1948,
        riskCategory: 'safe',
        lastUpdated: now,
      },
      {
        address: 'GUVWXY92626',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 95000,
        debtValue: 80000,
        healthFactor: 1.12,
        utilizationBps: 8420,
        liquidationPrice: 0.10,
        estimatedProfit: 3800,
        gasCostStroops: 510000,
        netProfit: 3749,
        riskCategory: 'danger',
        lastUpdated: now,
      },
    ];

    for (const p of mockPositions) {
      this.positions.set(p.address, p);
    }
  }

  public getAllPositions(): PositionHealth[] {
    return Array.from(this.positions.values()).sort(
      (a, b) => a.healthFactor - b.healthFactor
    );
  }

  public getPosition(address: string): PositionHealth | undefined {
    return this.positions.get(address);
  }

  public getPositionsByRisk(category: PositionHealth['riskCategory']): PositionHealth[] {
    return this.getAllPositions().filter((p) => p.riskCategory === category);
  }

  public getLiquidatablePositions(minProfit: number = 0): PositionHealth[] {
    return this.getAllPositions().filter(
      (p) => p.healthFactor < 1.2 && p.netProfit >= minProfit
    );
  }

  public updatePosition(address: string, update: Partial<PositionHealth>): void {
    const existing = this.positions.get(address);
    if (!existing) return;
    const updated: PositionHealth = { ...existing, ...update, lastUpdated: Date.now() };
    this.positions.set(address, updated);
    this.notifyListeners();
  }

  public onUpdate(listener: (positions: PositionHealth[]) => void): void {
    this.listeners.push(listener);
  }

  public setThreshold(threshold: HealthAlertThreshold): void {
    this.thresholds.set(threshold.address, threshold);
  }

  public getThreshold(address: string): HealthAlertThreshold | undefined {
    return this.thresholds.get(address);
  }

  public getAlerts(): PositionHealth[] {
    const alerts: PositionHealth[] = [];
    for (const [address, pos] of this.positions) {
      const threshold = this.thresholds.get(address);
      const effectiveDanger = threshold?.dangerHf ?? 1.1;
      if (pos.healthFactor <= effectiveDanger) {
        alerts.push(pos);
      }
    }
    return alerts.sort((a, b) => a.healthFactor - b.healthFactor);
  }

  private notifyListeners(): void {
    const all = this.getAllPositions();
    for (const listener of this.listeners) {
      try {
        listener(all);
      } catch { /* skip failed listener */ }
    }
  }
}

export const liquidationMonitorService = new LiquidationMonitorService();
