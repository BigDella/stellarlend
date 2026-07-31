import {
  StressScenario,
  StressTestInput,
  StressTestResult,
  StressSummary,
  AffectedPosition,
  WaterfallStep,
  CascadingLiquidationResult,
  PositionSnapshot,
  RiskThresholds,
} from './types';

const DEFAULT_RISK_THRESHOLDS: RiskThresholds = {
  healthFactorMin: 1.0,
  maxBadDebtRatio: 0.1,
  maxLiquidationCascade: 5,
  solvencyRatioMin: 1.0,
};

const BPS_DIVISOR = 10000;

export class StressTester {
  execute(input: StressTestInput): StressTestResult {
    const startTime = Date.now();
    const thresholds = input.riskThresholds ?? DEFAULT_RISK_THRESHOLDS;
    const scenario = input.scenario;

    const priceChanges = new Map(
      scenario.priceChanges.map((pc) => [pc.asset, pc.changePercent])
    );

    const volMultipliers = new Map(
      scenario.volatilityMultipliers.map((vm) => [vm.asset, vm.multiplier])
    );

    const affectedPositions: AffectedPosition[] = [];
    const waterfallSteps: WaterfallStep[] = [];

    let totalShortfall = 0;
    let totalBadDebt = 0;
    let totalLiquidatedValue = 0;
    let totalLiquidations = 0;

    const steps = scenario.durationSteps ?? 5;
    const stepSize = 1 / steps;

    let remainingLiquidity = input.protocolLiquidity;
    let cumulativeShortfall = 0;

    for (let step = 1; step <= steps; step++) {
      const priceLevel = 1 - (stepSize * step);
      let stepLiquidations = 0;
      let stepShortfall = 0;

      const affectedPositions = input.positions.map((pos) =>
        this.simulatePositionImpact(pos, priceChanges, priceLevel)
      );

      for (const affected of affectedPositions) {
        const pos = input.positions.find((p) => p.user === affected.user);
        if (!pos) continue;

        const isLiquidatable = affected.healthFactor < thresholds.healthFactorMin;

        if (isLiquidatable) {
          stepLiquidations++;
          const collateralValue = pos.collateral.reduce(
            (sum, c) => sum + c.amount * c.price * priceLevel,
            0
          );
          const debtValue = pos.borrow.reduce(
            (sum, b) => sum + b.amount * b.price,
            0
          );

          const liquidatedCollateral = collateralValue * 0.5;
          const recoveredDebt = liquidatedCollateral * 0.95;
          const shortfall = debtValue - recoveredDebt;

          if (shortfall > 0) {
            stepShortfall += shortfall;
            remainingLiquidity -= shortfall;
            cumulativeShortfall += shortfall;
          }

          totalLiquidatedValue += liquidatedCollateral;
        }

        if (affected.isInsolvent) {
          stepShortfall += affected.shortfall;
        }

        affectedPositions.push({
          user: pos.user,
          previousHealthFactor: pos.healthFactor,
          afterScenarioHealthFactor: affected.healthFactor,
          liquidatedCollateral: isLiquidatable ? pos.collateral.reduce((s, c) => s + c.amount * c.price * priceLevel * 0.5, 0) : 0,
          remainingDebt: Math.max(0, affected.shortfall),
          shortfall: affected.shortfall,
          isInsolvent: affected.isInsolvent,
        });
      }

      totalLiquidations += stepLiquidations;
      totalShortfall += stepShortfall;
      totalBadDebt += stepShortfall;

      waterfallSteps.push({
        step,
        priceLevel,
        liquidationsTriggered: stepLiquidations,
        cumulativeShortfall,
        remainingLiquidity: Math.max(0, remainingLiquidity),
        protocolSolvent: remainingLiquidity > 0,
      });
    }

    const protocolInsolvent = remainingLiquidity < 0;
    const solvencyRatio = input.totalDebtValue > 0
      ? (input.totalCollateralValue - totalBadDebt) / input.totalDebtValue
      : 1;

    const maxRiskScore = input.totalCollateralValue > 0
      ? (totalShortfall / input.totalCollateralValue) * 100
      : 0;

    let riskLevel: StressSummary['riskLevel'] = 'LOW';
    if (maxRiskScore > 30 || protocolInsolvent) riskLevel = 'CRITICAL';
    else if (maxRiskScore > 15) riskLevel = 'HIGH';
    else if (maxRiskScore > 5) riskLevel = 'MEDIUM';

    const summary: StressSummary = {
      totalShortfall,
      badDebtTotal: totalBadDebt,
      protocolInsolvent,
      solvencyRatio,
      totalPositionsAffected: affectedPositions.length,
      totalLiquidationsTriggered: totalLiquidations,
      worstCaseRecovery: input.totalDebtValue > 0
        ? (input.totalCollateralValue - totalBadDebt) / input.totalDebtValue * 100
        : 100,
      riskLevel,
    };

    let cascadingResult: CascadingLiquidationResult | undefined;
    if (scenario.cascadingLiquidation && totalLiquidations > 0) {
      cascadingResult = this.simulateCascadingLiquidations(
        input,
        affectedPositions,
        thresholds
      );
    }

    const recommendations = this.generateRecommendations(summary, scenario);

    const durationMs = Date.now() - startTime;

    return {
      scenarioId: scenario.id,
      scenarioName: scenario.name,
      executedAt: Date.now(),
      durationMs,
      summary,
      affectedPositions,
      waterfallAnalysis: waterfallSteps,
      cascadingLiquidationImpact: cascadingResult,
      recommendations,
      passed: riskLevel !== 'CRITICAL',
    };
  }

  private simulatePositionImpact(
    position: PositionSnapshot,
    priceChanges: Map<string, number>,
    priceLevel: number
  ): { healthFactor: number; isInsolvent: boolean; shortfall: number } {
    let collateralValue = 0;
    for (const c of position.collateral) {
      const change = priceChanges.get(c.asset) ?? 0;
      const adjustedPrice = c.price * (1 + change / 100 * priceLevel);
      collateralValue += c.amount * adjustedPrice;
    }

    let debtValue = 0;
    for (const b of position.borrow) {
      const change = priceChanges.get(b.asset) ?? 0;
      const adjustedPrice = b.price * (1 + change / 100 * priceLevel);
      debtValue += b.amount * adjustedPrice;
    }

    const healthFactor = debtValue > 0 ? collateralValue / debtValue : Infinity;
    const isInsolvent = debtValue > collateralValue;
    const shortfall = isInsolvent ? debtValue - collateralValue : 0;

    return { healthFactor, isInsolvent, shortfall };
  }

  private simulateCascadingLiquidations(
    input: StressTestInput,
    initialAffected: AffectedPosition[],
    thresholds: RiskThresholds
  ): CascadingLiquidationResult {
    const MAX_CASCADE_ROUNDS = 10;
    let totalLiquidated = 0;
    let totalCascadeShortfall = 0;
    const affectedUsers = new Set<string>(initialAffected.map((a) => a.user));
    let maxDepth = 0;

    let currentAffected = initialAffected.filter((a) => a.afterScenarioHealthFactor < thresholds.healthFactorMin);

    for (let round = 1; round <= MAX_CASCADE_ROUNDS; round++) {
      if (currentAffected.length === 0) break;

      maxDepth = round;

      for (const affected of currentAffected) {
        affectedUsers.add(affected.user);
      }

      const roundLiquidatedValue = currentAffected.reduce(
        (sum, pos) => sum + pos.liquidatedCollateral,
        0
      );

      const cascadePressure = roundLiquidatedValue * 0.1;
      totalLiquidated += roundLiquidatedValue;
      totalCascadeShortfall += cascadePressure;

      const nextRound: AffectedPosition[] = [];
      for (const pos of input.positions) {
        if (affectedUsers.has(pos.user)) continue;

        const newHealthFactor = pos.healthFactor * (1 - (0.05 * round));
        if (newHealthFactor < thresholds.healthFactorMin) {
          nextRound.push({
            user: pos.user,
            previousHealthFactor: pos.healthFactor,
            afterScenarioHealthFactor: newHealthFactor,
            liquidatedCollateral: pos.collateral.reduce((s, c) => s + c.amount * c.price * 0.3, 0),
            remainingDebt: pos.borrow.reduce((s, b) => s + b.amount * b.price, 0) * 0.1,
            shortfall: pos.borrow.reduce((s, b) => s + b.amount * b.price, 0) * 0.05,
            isInsolvent: newHealthFactor < 0.8,
          });
        }
      }

      currentAffected = nextRound;
    }

    return {
      totalCascadeRounds: maxDepth,
      totalLiquidatedValue: totalLiquidated,
      totalShortfall: totalCascadeShortfall,
      affectedUsers: Array.from(affectedUsers),
      maxCascadeDepth: maxDepth,
    };
  }

  private generateRecommendations(
    summary: StressSummary,
    scenario: StressScenario
  ): string[] {
    const recommendations: string[] = [];

    if (summary.protocolInsolvent) {
      recommendations.push(
        `CRITICAL: Protocol becomes insolvent under ${scenario.name}. Immediate risk parameter adjustments required.`
      );
      recommendations.push('Increase minimum collateral ratios by at least 25%.');
      recommendations.push('Reduce maximum LTV ratios across all affected assets.');
    }

    if (summary.riskLevel === 'HIGH' || summary.riskLevel === 'CRITICAL') {
      recommendations.push(
        `Total shortfall of ${summary.totalShortfall.toFixed(2)} exceeds acceptable threshold. Consider establishing an insurance fund.`
      );
      recommendations.push('Implement dynamic liquidation thresholds that tighten during high volatility.');
    }

    if (summary.totalLiquidationsTriggered > 10) {
      recommendations.push(
        `${summary.totalLiquidationsTriggered} positions liquidated. Review liquidation bonus to ensure sufficient liquidator participation.`
      );
    }

    if (scenario.cascadingLiquidation) {
      recommendations.push(
        'Cascading liquidation risk detected. Implement circuit breakers to pause liquidations during extreme volatility.'
      );
    }

    if (summary.solvencyRatio < 0.8) {
      recommendations.push(
        `Solvency ratio of ${(summary.solvencyRatio * 100).toFixed(1)}% is critically low. Consider protocol recapitalization.`
      );
    }

    if (summary.badDebtTotal > 0) {
      recommendations.push(
        `Bad debt of ${summary.badDebtTotal.toFixed(2)} detected. Review close factor to limit exposure per liquidation.`
      );
    }

    if (recommendations.length === 0) {
      recommendations.push(`Protocol remains solvent under ${scenario.name} scenario.`);
    }

    return recommendations;
  }
}

export const stressTester = new StressTester();
