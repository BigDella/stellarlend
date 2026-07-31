export enum RiskLevel {
  LOW = "LOW",
  MEDIUM = "MEDIUM",
  HIGH = "HIGH",
  CRITICAL = "CRITICAL",
}

export interface SimulationResult {
  scenarioName: string;
  success: boolean;
  impactValue: number;
  riskScore: number;
  recommendations: string[];
}

export interface RiskScore {
  aggregateScore: number;
  level: RiskLevel;
  breakdown: {
    oracleRisk: number;
    liquidityRisk: number;
    insolvencyRisk: number;
  };
  timestamp: number;
}

export interface SecurityAlert {
  id: string;
  level: RiskLevel;
  message: string;
  data: any;
  timestamp: number;
}

export interface AttackScenario {
  name: string;
  run(protocolState: any): Promise<SimulationResult>;
}
