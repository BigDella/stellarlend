import { RiskScore } from "./types";
export declare class SecurityEngine {
    private scenarios;
    constructor();
    runSecurityAudit(protocolState: any): Promise<RiskScore>;
    private mapScoreToLevel;
}
//# sourceMappingURL=simulation-engine.d.ts.map