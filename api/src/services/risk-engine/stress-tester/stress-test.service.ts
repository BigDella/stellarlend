import { StressTestInput, StressTestResult, PositionSnapshot } from '../risk-engine/stress-tester/types';
import { stressTester } from '../risk-engine/stress-tester/engine';
import { scenarioRegistry } from '../risk-engine/stress-tester/scenario-registry';

class StressTestService {
  runStressTest(input: StressTestInput): StressTestResult {
    return stressTester.execute(input);
  }

  runAllPredefined(input: Omit<StressTestInput, 'scenario'>): Map<string, StressTestResult> {
    const results = new Map<string, StressTestResult>();
    const scenarios = scenarioRegistry.getAllScenarios();

    for (const scenario of scenarios) {
      const result = stressTester.execute({ ...input, scenario });
      results.set(scenario.id, result);
    }

    return results;
  }

  runByCategory(input: Omit<StressTestInput, 'scenario'>, category: string): Map<string, StressTestResult> {
    const results = new Map<string, StressTestResult>();
    const scenarios = scenarioRegistry.getScenariosByCategory(category);

    for (const scenario of scenarios) {
      const result = stressTester.execute({ ...input, scenario });
      results.set(scenario.id, result);
    }

    return results;
  }

  runCustomScenario(
    input: Omit<StressTestInput, 'scenario'>,
    customConfig: {
      name: string;
      description: string;
      priceChanges: { asset: string; changePercent: number }[];
      correlationShifts?: { asset1: string; asset2: string; newCorrelation: number }[];
      volatilityMultipliers?: { asset: string; multiplier: number }[];
      tags?: string[];
    }
  ): StressTestResult {
    const scenario = scenarioRegistry.buildCustomScenario(customConfig);
    scenarioRegistry.addCustomScenario(scenario);
    return stressTester.execute({ ...input, scenario });
  }

  getDefaultPositions(): PositionSnapshot[] {
    return [
      {
        user: 'GABC123DEF456',
        pool: 'pool-1',
        collateral: [
          { asset: 'XLM', amount: 50000, price: 0.12 },
          { asset: 'BTC', amount: 1, price: 65000 },
        ],
        borrow: [
          { asset: 'USDC', amount: 50000, price: 1.0 },
        ],
        healthFactor: 1.42,
      },
      {
        user: 'GDEF789ABC012',
        pool: 'pool-1',
        collateral: [
          { asset: 'XLM', amount: 100000, price: 0.12 },
        ],
        borrow: [
          { asset: 'USDC', amount: 8000, price: 1.0 },
        ],
        healthFactor: 1.5,
      },
      {
        user: 'GHIJ345DEF678',
        pool: 'pool-2',
        collateral: [
          { asset: 'ETH', amount: 10, price: 3200 },
          { asset: 'XLM', amount: 20000, price: 0.12 },
        ],
        borrow: [
          { asset: 'USDC', amount: 25000, price: 1.0 },
        ],
        healthFactor: 1.28,
      },
      {
        user: 'GABC999XXX111',
        pool: 'pool-1',
        collateral: [
          { asset: 'XLM', amount: 10000, price: 0.12 },
        ],
        borrow: [
          { asset: 'USDC', amount: 1100, price: 1.0 },
        ],
        healthFactor: 1.09,
      },
      {
        user: 'GWHALE001002',
        pool: 'pool-2',
        collateral: [
          { asset: 'BTC', amount: 5, price: 65000 },
          { asset: 'ETH', amount: 50, price: 3200 },
        ],
        borrow: [
          { asset: 'USDC', amount: 300000, price: 1.0 },
        ],
        healthFactor: 1.75,
      },
    ];
  }

  getDefaultInput(): Omit<StressTestInput, 'scenario'> {
    const positions = this.getDefaultPositions();
    return {
      positions,
      totalCollateralValue: positions.reduce(
        (s, p) => s + p.collateral.reduce((cs, c) => cs + c.amount * c.price, 0),
        0
      ),
      totalDebtValue: positions.reduce(
        (s, p) => s + p.borrow.reduce((bs, b) => bs + b.amount * b.price, 0),
        0
      ),
      protocolLiquidity: 1000000,
    };
  }
}

export const stressTestService = new StressTestService();
