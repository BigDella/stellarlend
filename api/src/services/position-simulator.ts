import logger from '../utils/logger';

interface Position {
  collateral: number;
  debt: number;
  asset: string;
}

interface HealthFactor {
  current: number;
  after_scenario: number;
  is_liquidatable: boolean;
  liquidation_price: number;
  safety_margin: number;
}

interface ScenarioResult {
  scenario_name: string;
  initial_health: number;
  final_health: number;
  collateral_change: number;
  debt_change: number;
  is_liquidatable: boolean;
  estimated_gas_cost: number;
  timestamp: number;
}

interface SimulationComparison {
  scenario_a: ScenarioResult;
  scenario_b: ScenarioResult;
  health_difference: number;
  recommendation: string;
}

// Simplified price data (would come from oracle in production)
const mockPriceHistory: Map<string, number[]> = new Map();
const simulationResults: Map<string, ScenarioResult[]> = new Map();

export const positionSimulator = {
  /**
   * Simulate position health after a price drop
   */
  simulatePriceDrop(
    position: Position,
    priceDropPercent: number,
  ): HealthFactor {
    const currentHealth = this.calculateHealthFactor(position);

    const collateralAfterDrop = position.collateral * (1 - priceDropPercent / 100);
    const positionAfterDrop: Position = {
      collateral: collateralAfterDrop,
      debt: position.debt,
      asset: position.asset,
    };

    const healthAfterDrop = this.calculateHealthFactor(positionAfterDrop);

    return {
      current: currentHealth,
      after_scenario: healthAfterDrop,
      is_liquidatable: healthAfterDrop < 1.0,
      liquidation_price: this.calculateLiquidationPrice(position),
      safety_margin: healthAfterDrop - 1.0,
    };
  },

  /**
   * Simulate position health after interest rate increase
   */
  simulateRateIncrease(
    position: Position,
    rateIncreasePercent: number,
  ): HealthFactor {
    const currentHealth = this.calculateHealthFactor(position);

    const debtAfterRateIncrease = position.debt * (1 + rateIncreasePercent / 100);
    const positionAfterRate: Position = {
      collateral: position.collateral,
      debt: debtAfterRateIncrease,
      asset: position.asset,
    };

    const healthAfterRate = this.calculateHealthFactor(positionAfterRate);

    return {
      current: currentHealth,
      after_scenario: healthAfterRate,
      is_liquidatable: healthAfterRate < 1.0,
      liquidation_price: this.calculateLiquidationPrice(position),
      safety_margin: healthAfterRate - 1.0,
    };
  },

  /**
   * Simulate what-if: additional deposit
   */
  simulateAdditionalDeposit(
    position: Position,
    depositAmount: number,
  ): HealthFactor {
    const currentHealth = this.calculateHealthFactor(position);

    const positionAfterDeposit: Position = {
      collateral: position.collateral + depositAmount,
      debt: position.debt,
      asset: position.asset,
    };

    const healthAfterDeposit = this.calculateHealthFactor(positionAfterDeposit);

    return {
      current: currentHealth,
      after_scenario: healthAfterDeposit,
      is_liquidatable: false,
      liquidation_price: this.calculateLiquidationPrice(positionAfterDeposit),
      safety_margin: healthAfterDeposit - 1.0,
    };
  },

  /**
   * Simulate what-if: partial repayment
   */
  simulatePartialRepayment(
    position: Position,
    repaymentAmount: number,
  ): HealthFactor {
    const currentHealth = this.calculateHealthFactor(position);

    const positionAfterRepay: Position = {
      collateral: position.collateral,
      debt: Math.max(0, position.debt - repaymentAmount),
      asset: position.asset,
    };

    const healthAfterRepay = this.calculateHealthFactor(positionAfterRepay);

    return {
      current: currentHealth,
      after_scenario: healthAfterRepay,
      is_liquidatable: false,
      liquidation_price: this.calculateLiquidationPrice(positionAfterRepay),
      safety_margin: healthAfterRepay - 1.0,
    };
  },

  /**
   * Compare two scenarios side-by-side
   */
  compareScenarios(
    scenarioA: ScenarioResult,
    scenarioB: ScenarioResult,
  ): SimulationComparison {
    const healthDiff = scenarioB.final_health - scenarioA.final_health;
    let recommendation = '';

    if (healthDiff > 0) {
      recommendation = `Scenario B is safer with ${healthDiff.toFixed(2)} higher health factor`;
    } else if (healthDiff < 0) {
      recommendation = `Scenario A is safer with ${Math.abs(healthDiff).toFixed(2)} higher health factor`;
    } else {
      recommendation = 'Both scenarios have equivalent health factors';
    }

    return {
      scenario_a: scenarioA,
      scenario_b: scenarioB,
      health_difference: healthDiff,
      recommendation,
    };
  },

  /**
   * Replay historical scenario: would this position have been liquidated?
   */
  replayHistoricalScenario(
    position: Position,
    historicalDate: Date,
    pricesOnDate: Map<string, number>,
  ): ScenarioResult {
    // Simplified: assume price data is available
    const priceMultiplier = Array.from(pricesOnDate.values())[0] || 1;
    const collateralAtTime = position.collateral * priceMultiplier;

    const positionAtTime: Position = {
      collateral: collateralAtTime,
      debt: position.debt,
      asset: position.asset,
    };

    const healthAtTime = this.calculateHealthFactor(positionAtTime);

    const result: ScenarioResult = {
      scenario_name: `Historical Replay - ${historicalDate.toDateString()}`,
      initial_health: this.calculateHealthFactor(position),
      final_health: healthAtTime,
      collateral_change: collateralAtTime - position.collateral,
      debt_change: 0,
      is_liquidatable: healthAtTime < 1.0,
      estimated_gas_cost: 45000,
      timestamp: historicalDate.getTime(),
    };

    return result;
  },

  /**
   * Real-time simulation as user types amounts
   */
  simulateRealTimeChange(
    position: Position,
    changeType: 'deposit' | 'withdraw' | 'borrow' | 'repay',
    amount: number,
  ): HealthFactor {
    const updatedPosition = { ...position };

    switch (changeType) {
      case 'deposit':
        updatedPosition.collateral += amount;
        break;
      case 'withdraw':
        updatedPosition.collateral = Math.max(0, updatedPosition.collateral - amount);
        break;
      case 'borrow':
        updatedPosition.debt += amount;
        break;
      case 'repay':
        updatedPosition.debt = Math.max(0, updatedPosition.debt - amount);
        break;
    }

    const currentHealth = this.calculateHealthFactor(position);
    const newHealth = this.calculateHealthFactor(updatedPosition);

    return {
      current: currentHealth,
      after_scenario: newHealth,
      is_liquidatable: newHealth < 1.0,
      liquidation_price: this.calculateLiquidationPrice(updatedPosition),
      safety_margin: newHealth - 1.0,
    };
  },

  /**
   * Export scenario as shareable result
   */
  exportScenarioResult(result: ScenarioResult): string {
    const json = JSON.stringify(result, null, 2);
    logger.info(`Scenario exported: ${result.scenario_name}`);
    return json;
  },

  /**
   * Get accuracy comparison: simulated vs actual
   */
  getSimulationAccuracy(
    simulatedResult: ScenarioResult,
    actualResult: ScenarioResult,
  ): { accuracy_percent: number; deviation: number } {
    const deviation = Math.abs(simulatedResult.final_health - actualResult.final_health);
    const maxHealth = Math.max(simulatedResult.final_health, actualResult.final_health);
    const accuracyPercent = Math.max(0, (1 - deviation / maxHealth) * 100);

    return {
      accuracy_percent: Math.round(accuracyPercent * 100) / 100,
      deviation: Math.round(deviation * 10000) / 10000,
    };
  },

  /**
   * Helper: Calculate health factor
   */
  calculateHealthFactor(position: Position): number {
    if (position.debt === 0) return Number.POSITIVE_INFINITY;
    return position.collateral / position.debt;
  },

  /**
   * Helper: Calculate liquidation price
   */
  calculateLiquidationPrice(position: Position): number {
    if (position.debt === 0) return 0;
    return position.debt / position.collateral;
  },

  /**
   * Store simulation result for user
   */
  saveSimulation(userId: string, result: ScenarioResult): void {
    if (!simulationResults.has(userId)) {
      simulationResults.set(userId, []);
    }
    simulationResults.get(userId)!.push(result);
    logger.info(`Simulation saved for user: ${userId}`);
  },

  /**
   * Get user's simulation history
   */
  getUserSimulations(userId: string): ScenarioResult[] {
    return simulationResults.get(userId) ?? [];
  },

  /**
   * Complex scenario: combined price drop + rate increase
   */
  simulateComplexScenario(
    position: Position,
    priceDropPercent: number,
    rateIncreasePercent: number,
  ): ScenarioResult {
    const collateralAfterDrop = position.collateral * (1 - priceDropPercent / 100);
    const debtAfterRate = position.debt * (1 + rateIncreasePercent / 100);

    const complexPosition: Position = {
      collateral: collateralAfterDrop,
      debt: debtAfterRate,
      asset: position.asset,
    };

    const currentHealth = this.calculateHealthFactor(position);
    const complexHealth = this.calculateHealthFactor(complexPosition);

    return {
      scenario_name: `Complex: ${priceDropPercent}% price drop + ${rateIncreasePercent}% rate increase`,
      initial_health: currentHealth,
      final_health: complexHealth,
      collateral_change: collateralAfterDrop - position.collateral,
      debt_change: debtAfterRate - position.debt,
      is_liquidatable: complexHealth < 1.0,
      estimated_gas_cost: 60000,
      timestamp: Date.now(),
    };
  },
};
