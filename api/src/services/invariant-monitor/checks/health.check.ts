import { InvariantCheck, InvariantCheckResult } from '../index';

export class HealthCheck implements InvariantCheck {
  name = 'Health Factor Invariants';

  async runCheck(): Promise<InvariantCheckResult> {
    // Stub logic simulating checking all active positions
    const activePositions = [
      { id: '1', healthFactor: 1.5 },
      { id: '2', healthFactor: 1.2 },
      { id: '3', healthFactor: 2.0 },
    ];

    const violations = activePositions.filter(p => p.healthFactor < 1.0);

    if (violations.length > 0) {
      return {
        name: this.name,
        passed: false,
        message: 'Found positions with health factor < 1.0',
        severity: 'critical',
        details: { violationsCount: violations.length, sample: violations.slice(0, 5) },
      };
    }

    return {
      name: this.name,
      passed: true,
      message: 'All positions have health factor >= 1.0',
      severity: 'warning'
    };
  }
}
