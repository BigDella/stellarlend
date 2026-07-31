import { SecurityEngine } from "./simulation-engine";
import { RiskLevel } from "./types";
export class MonitoringService {
    engine;
    constructor() {
        this.engine = new SecurityEngine();
    }
    async monitor(protocolState) {
        const score = await this.engine.runSecurityAudit(protocolState);
        const alerts = [];
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
//# sourceMappingURL=monitoring-service.js.map