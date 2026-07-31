import { AttackScenario, SimulationResult } from "../types";

export class LiquidationCascadeScenario implements AttackScenario {
  name = "Liquidation Cascade";

  async run(protocolState: any): Promise<SimulationResult> {
    const { totalCollateralValue, totalDebtValue, marketVolatility } = protocolState;
    
    // Simulate a 20% drop in market prices leading to cascading liquidations
    const dropPercentage = 0.20;
    const liquidatedCollateral = totalCollateralValue * dropPercentage;
    const slippage = marketVolatility * 0.5; // High volatility increases slippage
    
    const unrecoveredDebt = liquidatedCollateral * slippage;
    
    const insolvencyRisk = (unrecoveredDebt / totalCollateralValue) * 100;
    
    return {
      scenarioName: this.name,
      success: insolvencyRisk > 5,
      impactValue: unrecoveredDebt,
      riskScore: Math.min(insolvencyRisk * 5, 100),
      recommendations: [
        "Increase liquidation incentives to attract more liquidators",
        "Lower LTV for volatile assets",
      ],
    };
  }
}
