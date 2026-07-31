export interface PositionInput {
  collateral: { asset: string; amount: number; price: number }[];
  borrow: { asset: string; amount: number; price: number }[];
  poolId?: string;
}

export interface SimulationScenario {
  name: string;
  description: string;
  priceChanges: { asset: string; changePercent: number }[];
  rateChanges?: { asset: string; changeBps: number }[];
}

export interface SimulationResult {
  scenario: SimulationScenario;
  currentHealthFactor: number;
  afterScenarioHealthFactor: number;
  currentLtv: number;
  afterScenarioLtv: number;
  isLiquidatable: boolean;
  liquidationPrice: Record<string, number>;
  safetyMargin: number;
  collateralValueChange: number;
  debtValueChange: number;
  recommendation: string;
}

export interface CorrelationInput {
  assets: string[];
  prices: Record<string, number[]>;
}

export interface CorrelationMatrix {
  assets: string[];
  matrix: number[][];
  diversificationScore: number;
}

export interface Recommendation {
  type: 'add_collateral' | 'repay_debt' | 'swap_collateral' | 'reduce_exposure';
  priority: number;
  description: string;
  expectedHealthImprovement: number;
  action: {
    asset?: string;
    amount?: number;
    targetHealthFactor?: number;
  };
}

class RiskSimulationService {
  private readonly LIQUIDATION_THRESHOLD = 1.0;
  private readonly BPS_DIVISOR = 10000;

  simulatePosition(
    position: PositionInput,
    scenario: SimulationScenario
  ): SimulationResult {
    const totalCollateralValue = position.collateral.reduce(
      (sum, c) => sum + c.amount * c.price,
      0
    );
    const totalDebtValue = position.borrow.reduce(
      (sum, b) => sum + b.amount * b.price,
      0
    );

    const currentHealthFactor = totalDebtValue > 0
      ? totalCollateralValue / totalDebtValue
      : Infinity;

    const currentLtv = totalCollateralValue > 0
      ? totalDebtValue / totalCollateralValue
      : 0;

    const priceChanges = new Map(
      scenario.priceChanges.map((pc) => [pc.asset, pc.changePercent])
    );

    let afterCollateralValue = 0;
    for (const c of position.collateral) {
      const change = priceChanges.get(c.asset) ?? 0;
      afterCollateralValue += c.amount * c.price * (1 + change / 100);
    }

    let afterDebtValue = 0;
    for (const b of position.borrow) {
      const change = priceChanges.get(b.asset) ?? 0;
      afterDebtValue += b.amount * b.price * (1 + change / 100);
    }

    const afterHealthFactor = afterDebtValue > 0
      ? afterCollateralValue / afterDebtValue
      : Infinity;

    const afterLtv = afterCollateralValue > 0
      ? afterDebtValue / afterCollateralValue
      : 0;

    const isLiquidatable = afterHealthFactor <= this.LIQUIDATION_THRESHOLD;

    const liquidationPrice: Record<string, number> = {};
    for (const c of position.collateral) {
      const totalBorrowValue = position.borrow.reduce(
        (sum, b) => sum + b.amount * b.price,
        0
      );
      liquidationPrice[c.asset] = totalBorrowValue / (c.amount * 0.8);
    }

    const safetyMargin = (afterHealthFactor - this.LIQUIDATION_THRESHOLD) * 100;

    let recommendation = 'Position is safe under this scenario.';
    if (isLiquidatable) {
      const shortfall = afterDebtValue - afterCollateralValue * 0.8;
      recommendation = `Liquidation risk detected. Add ${shortfall.toFixed(2)} in collateral or repay debt to restore health.`;
    } else if (afterHealthFactor < 1.5) {
      recommendation = 'Position approaching liquidation threshold. Consider adding collateral or repaying debt.';
    }

    return {
      scenario,
      currentHealthFactor,
      afterScenarioHealthFactor: afterHealthFactor,
      currentLtv,
      afterScenarioLtv: afterLtv,
      isLiquidatable,
      liquidationPrice,
      safetyMargin,
      collateralValueChange: afterCollateralValue - totalCollateralValue,
      debtValueChange: afterDebtValue - totalDebtValue,
      recommendation,
    };
  }

  simulateMultipleScenarios(
    position: PositionInput,
    scenarios: SimulationScenario[]
  ): SimulationResult[] {
    return scenarios.map((scenario) =>
      this.simulatePosition(position, scenario)
    );
  }

  getPredefinedScenarios(): SimulationScenario[] {
    return [
      {
        name: '5% Collateral Drop',
        description: 'Simulates a 5% drop in collateral asset prices',
        priceChanges: [{ asset: 'XLM', changePercent: -5 }],
      },
      {
        name: '10% Market Correction',
        description: 'Simulates a 10% market correction across all assets',
        priceChanges: [
          { asset: 'XLM', changePercent: -10 },
          { asset: 'BTC', changePercent: -10 },
          { asset: 'ETH', changePercent: -10 },
        ],
      },
      {
        name: '25% Flash Crash',
        description: 'Simulates a rapid 25% crash in collateral assets',
        priceChanges: [{ asset: 'XLM', changePercent: -25 }],
      },
      {
        name: '50% Black Swan',
        description: 'Simulates an extreme 50% drop in collateral value',
        priceChanges: [{ asset: 'XLM', changePercent: -50 }],
      },
      {
        name: 'Interest Rate Shock',
        description: 'Simulates a 500bps increase in borrow rates',
        priceChanges: [],
        rateChanges: [{ asset: 'XLM', changeBps: 500 }],
      },
      {
        name: 'Combined Stress',
        description: '20% price drop with 300bps rate increase',
        priceChanges: [
          { asset: 'XLM', changePercent: -20 },
          { asset: 'BTC', changePercent: -20 },
        ],
        rateChanges: [{ asset: 'XLM', changeBps: 300 }],
      },
    ];
  }

  calculateCorrelationMatrix(input: CorrelationInput): CorrelationMatrix {
    const { assets, prices } = input;
    const n = assets.length;
    const matrix: number[][] = Array.from({ length: n }, () =>
      Array(n).fill(0)
    );

    for (let i = 0; i < n; i++) {
      for (let j = 0; j < n; j++) {
        if (i === j) {
          matrix[i][j] = 1.0;
        } else {
          const p1 = prices[assets[i]] ?? [];
          const p2 = prices[assets[j]] ?? [];
          matrix[i][j] = this.pearsonCorrelation(p1, p2);
        }
      }
    }

    let avgCorrelation = 0;
    let count = 0;
    for (let i = 0; i < n; i++) {
      for (let j = i + 1; j < n; j++) {
        avgCorrelation += matrix[i][j];
        count++;
      }
    }
    avgCorrelation = count > 0 ? avgCorrelation / count : 0;
    const diversificationScore = Math.max(0, (1 - avgCorrelation) * 100);

    return { assets, matrix, diversificationScore };
  }

  private pearsonCorrelation(xArray: number[], yArray: number[]): number {
    const n = Math.min(xArray.length, yArray.length);
    if (n < 2) return 0;

    const meanX = xArray.slice(0, n).reduce((a, b) => a + b, 0) / n;
    const meanY = yArray.slice(0, n).reduce((a, b) => a + b, 0) / n;

    let num = 0;
    let denX = 0;
    let denY = 0;

    for (let i = 0; i < n; i++) {
      const dx = xArray[i] - meanX;
      const dy = yArray[i] - meanY;
      num += dx * dy;
      denX += dx * dx;
      denY += dy * dy;
    }

    const den = Math.sqrt(denX * denY);
    if (den === 0) return 0;
    const r = num / den;
    return Math.max(-1, Math.min(1, r));
  }

  generateRecommendations(
    position: PositionInput,
    targetHealthFactor: number = 2.0
  ): Recommendation[] {
    const totalCollateralValue = position.collateral.reduce(
      (sum, c) => sum + c.amount * c.price,
      0
    );
    const totalDebtValue = position.borrow.reduce(
      (sum, b) => sum + b.amount * b.price,
      0
    );
    const currentHealth = totalDebtValue > 0
      ? totalCollateralValue / totalDebtValue
      : Infinity;

    const recommendations: Recommendation[] = [];

    if (currentHealth < targetHealthFactor) {
      const collateralNeeded = (targetHealthFactor * totalDebtValue - totalCollateralValue);
      recommendations.push({
        type: 'add_collateral',
        priority: 1,
        description: `Add ${collateralNeeded.toFixed(2)} in additional collateral to reach target health factor of ${targetHealthFactor.toFixed(1)}`,
        expectedHealthImprovement: collateralNeeded / totalDebtValue,
        action: { amount: collateralNeeded, targetHealthFactor },
      });

      const debtToRepay = totalDebtValue - (totalCollateralValue / targetHealthFactor);
      if (debtToRepay > 0) {
        recommendations.push({
          type: 'repay_debt',
          priority: 2,
          description: `Repay ${debtToRepay.toFixed(2)} of debt to reach target health factor of ${targetHealthFactor.toFixed(1)}`,
          expectedHealthImprovement: (totalCollateralValue / (totalDebtValue - debtToRepay)) - currentHealth,
          action: { amount: debtToRepay, targetHealthFactor },
        });
      }
    }

    if (position.collateral.length > 1) {
      recommendations.push({
        type: 'reduce_exposure',
        priority: 3,
        description: 'Consider diversifying collateral across multiple uncorrelated assets',
        expectedHealthImprovement: 0.1,
        action: { targetHealthFactor },
      });
    }

    return recommendations;
  }
}

export const riskSimulationService = new RiskSimulationService();
