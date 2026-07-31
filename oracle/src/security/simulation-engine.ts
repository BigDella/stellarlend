import { AttackScenario, RiskLevel, RiskScore, SimulationResult } from "./types";
import { PriceManipulationScenario } from "./scenarios/price-manipulation";
import { LiquidationCascadeScenario } from "./scenarios/liquidation-cascade";

export class SecurityEngine {
  private scenarios: AttackScenario[] = [];

  constructor() {
    this.scenarios.push(new PriceManipulationScenario());
    this.scenarios.push(new LiquidationCascadeScenario());
  }

  async runSecurityAudit(protocolState: any): Promise<RiskScore> {
    const results = await Promise.all(
      this.scenarios.map((s) => s.run(protocolState))
    );

    const aggregateScore =
      results.reduce((acc, res) => acc + res.riskScore, 0) / results.length;

    return {
      aggregateScore,
      level: this.mapScoreToLevel(aggregateScore),
      breakdown: {
        oracleRisk: results.find((r) => r.scenarioName === "Oracle Price Manipulation")?.riskScore || 0,
        liquidityRisk: results.find((r) => r.scenarioName === "Liquidation Cascade")?.riskScore || 0,
        insolvencyRisk: (aggregateScore * 0.8), // Heuristic
      },
      timestamp: Date.now(),
    };
  }

  private mapScoreToLevel(score: number): RiskLevel {
    if (score < 20) return RiskLevel.LOW;
    if (score < 50) return RiskLevel.MEDIUM;
    if (score < 80) return RiskLevel.HIGH;
    return RiskLevel.CRITICAL;
  }
}
