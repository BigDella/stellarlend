import { SecurityEngine } from "./simulation-engine";
import { RiskLevel, SecurityAlert } from "./types";

export class MonitoringService {
  private engine: SecurityEngine;

  constructor() {
    this.engine = new SecurityEngine();
  }

  async monitor(protocolState: any): Promise<SecurityAlert[]> {
    const score = await this.engine.runSecurityAudit(protocolState);
    const alerts: SecurityAlert[] = [];

    if (score.level === RiskLevel.HIGH || score.level === RiskLevel.CRITICAL) {
      alerts.push({
        id: `alert-${Date.now()}`,
        level: score.level,
        message: `High protocol risk detected: ${score.aggregateScore.toFixed(2)}%`,
        data: score,
        timestamp: score.timestamp,
      });
    }

    return alerts;
  }
}
