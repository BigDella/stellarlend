export interface LiquidationOpportunity {
  positionAddress: string;
  collateralAsset: string;
  debtAsset: string;
  collateralValue: number;
  debtValue: number;
  healthFactor: number;
  estimatedProfitStroops: number;
  gasCostStroops: number;
  netProfitStroops: number;
  profitPercent: number;
  collateralRatio: number;
  riskScore: number;
  updatedAt: number;
}

export interface GasEstimate {
  operation: string;
  baseFeeStroops: number;
  totalCostStroops: number;
  estimatedUsd: number;
}

export class OpportunityExplorerService {
  private opportunities: LiquidationOpportunity[] = [];

  constructor() {
    this.seedMockOpportunities();
  }

  private seedMockOpportunities(): void {
    this.opportunities = [
      {
        positionAddress: 'GABCDE12345',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 50000,
        debtValue: 35000,
        healthFactor: 1.35,
        estimatedProfitStroops: 12500000,
        gasCostStroops: 500000,
        netProfitStroops: 12000000,
        profitPercent: 2400,
        collateralRatio: 1.43,
        riskScore: 35,
        updatedAt: Date.now(),
      },
      {
        positionAddress: 'GFGHIJ67890',
        collateralAsset: 'USDC',
        debtAsset: 'XLM',
        collateralValue: 120000,
        debtValue: 102000,
        healthFactor: 1.08,
        estimatedProfitStroops: 48000000,
        gasCostStroops: 550000,
        netProfitStroops: 47450000,
        profitPercent: 8627,
        collateralRatio: 1.18,
        riskScore: 78,
        updatedAt: Date.now(),
      },
      {
        positionAddress: 'GKLMNO11121',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 25000,
        debtValue: 24000,
        healthFactor: 1.02,
        estimatedProfitStroops: 7500000,
        gasCostStroops: 480000,
        netProfitStroops: 7020000,
        profitPercent: 1463,
        collateralRatio: 1.04,
        riskScore: 92,
        updatedAt: Date.now(),
      },
      {
        positionAddress: 'GPQRST31415',
        collateralAsset: 'USDC',
        debtAsset: 'USDT',
        collateralValue: 85000,
        debtValue: 72000,
        healthFactor: 1.15,
        estimatedProfitStroops: 20000000,
        gasCostStroops: 520000,
        netProfitStroops: 19480000,
        profitPercent: 3746,
        collateralRatio: 1.18,
        riskScore: 65,
        updatedAt: Date.now(),
      },
      {
        positionAddress: 'GUVWXY92626',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralValue: 95000,
        debtValue: 80000,
        healthFactor: 1.12,
        estimatedProfitStroops: 38000000,
        gasCostStroops: 510000,
        netProfitStroops: 37490000,
        profitPercent: 7351,
        collateralRatio: 1.19,
        riskScore: 72,
        updatedAt: Date.now(),
      },
      {
        positionAddress: 'GHIJKL78901',
        collateralAsset: 'USDC',
        debtAsset: 'USDT',
        collateralValue: 200000,
        debtValue: 185000,
        healthFactor: 1.05,
        estimatedProfitStroops: 85000000,
        gasCostStroops: 600000,
        netProfitStroops: 84400000,
        profitPercent: 14067,
        collateralRatio: 1.08,
        riskScore: 88,
        updatedAt: Date.now(),
      },
    ];
  }

  public getOpportunities(filters?: {
    minProfit?: number;
    maxHf?: number;
    minHf?: number;
    asset?: string;
    minCollateral?: number;
    sortBy?: string;
    sortDir?: 'asc' | 'desc';
  }): LiquidationOpportunity[] {
    let result = [...this.opportunities];

    if (filters?.minProfit !== undefined) {
      result = result.filter((o) => o.netProfitStroops >= filters.minProfit!);
    }
    if (filters?.maxHf !== undefined) {
      result = result.filter((o) => o.healthFactor <= filters.maxHf!);
    }
    if (filters?.minHf !== undefined) {
      result = result.filter((o) => o.healthFactor >= filters.minHf!);
    }
    if (filters?.asset) {
      const asset = filters.asset.toUpperCase();
      result = result.filter(
        (o) => o.collateralAsset === asset || o.debtAsset === asset
      );
    }
    if (filters?.minCollateral !== undefined) {
      result = result.filter((o) => o.collateralValue >= filters.minCollateral!);
    }

    const sortField = filters?.sortBy || 'healthFactor';
    const sortDir = filters?.sortDir || 'asc';

    result.sort((a, b) => {
      const aVal = (a as any)[sortField] ?? 0;
      const bVal = (b as any)[sortField] ?? 0;
      return sortDir === 'desc' ? bVal - aVal : aVal - bVal;
    });

    return result;
  }

  public getHistoricalLiquidations(): Array<{
    positionAddress: string;
    profitStroops: number;
    timestamp: number;
  }> {
    return [
      {
        positionAddress: 'GOLD_LIQ_001',
        profitStroops: 15000000,
        timestamp: Date.now() - 86400000 * 2,
      },
      {
        positionAddress: 'GOLD_LIQ_002',
        profitStroops: 32000000,
        timestamp: Date.now() - 86400000 * 5,
      },
      {
        positionAddress: 'GOLD_LIQ_003',
        profitStroops: 8500000,
        timestamp: Date.now() - 86400000 * 7,
      },
      {
        positionAddress: 'GOLD_LIQ_004',
        profitStroops: 42000000,
        timestamp: Date.now() - 86400000 * 10,
      },
    ];
  }

  public getGasEstimate(): GasEstimate {
    return {
      operation: 'liquidation',
      baseFeeStroops: 100,
      totalCostStroops: 525000,
      estimatedUsd: 0.00525,
    };
  }
}

export const opportunityExplorerService = new OpportunityExplorerService();
