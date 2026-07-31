import { SecurityEngine } from "../src/security/simulation-engine";
import { MonitoringService } from "../src/security/monitoring-service";
import { RiskLevel } from "../src/security/types";

describe("Economic Security Simulation Framework", () => {
  let engine: SecurityEngine;
  let monitor: MonitoringService;

  beforeEach(() => {
    engine = new SecurityEngine();
    monitor = new MonitoringService();
  });

  const mockProtocolState = {
    assets: [
      { price: 100, totalBorrowed: 1000000 },
      { price: 10, totalBorrowed: 5000000 },
    ],
    totalCollateralValue: 10000000,
    totalDebtValue: 6000000,
    marketVolatility: 0.1,
  };

  test("should calculate a risk score based on protocol state", async () => {
    const score = await engine.runSecurityAudit(mockProtocolState);
    expect(score.aggregateScore).toBeGreaterThan(0);
    expect(score.level).toBeDefined();
  });

  test("should detect high risk in a volatile market", async () => {
    const highVolatilityState = {
      ...mockProtocolState,
      marketVolatility: 0.8,
      totalDebtValue: 9000000, // Near liquidation
    };
    const score = await engine.runSecurityAudit(highVolatilityState);
    expect(score.level).toBe(RiskLevel.HIGH);
  });

  test("should trigger alerts when risk level is critical", async () => {
    const criticalState = {
      ...mockProtocolState,
      assets: [{ price: 200, totalBorrowed: 10000000 }], // Massive borrowing against inflated price
    };
    const alerts = await monitor.monitor(criticalState);
    expect(alerts.length).toBeGreaterThan(0);
    expect(alerts[0].level).toBe(RiskLevel.HIGH);
  });
});
