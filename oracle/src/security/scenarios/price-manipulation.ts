import { AttackScenario, SimulationResult } from "../types";

export class PriceManipulationScenario implements AttackScenario {
  name = "Oracle Price Manipulation";

  async run(protocolState: any): Promise<SimulationResult> {
    const { assets, oracleConfidence } = protocolState;
    
    // Simulate a 30% price deviation in a major asset
    const manipulatedAsset = assets[0];
    const originalPrice = manipulatedAsset.price;
    const manipulatedPrice = originalPrice * 1.3;
    
    // Calculate potential bad debt if this price was accepted
    const potentialBadDebt = (manipulatedPrice - originalPrice) * manipulatedAsset.totalBorrowed;
    
    const riskScore = (potentialBadDebt / protocolState.totalCollateralValue) * 100;
    
    return {
      scenarioName: this.name,
      success: riskScore > 10,
      impactValue: potentialBadDebt,
      riskScore: Math.min(riskScore, 100),
      recommendations: [
        "Increase oracle heartbeat frequency",
        "Implement a price deviation circuit breaker",
      ],
    };
  }
}
