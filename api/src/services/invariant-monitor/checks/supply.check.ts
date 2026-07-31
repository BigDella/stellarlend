import { InvariantCheck, InvariantCheckResult } from '../index';

export class SupplyCheck implements InvariantCheck {
  name = 'Supply Invariants';

  async runCheck(): Promise<InvariantCheckResult> {
    // Stub logic simulating fetching from indexer/contracts
    const totalSupply = 1000000;
    const sumUserPositions = 1000000;
    const globalCap = 2000000;

    if (totalSupply !== sumUserPositions) {
      return {
        name: this.name,
        passed: false,
        message: 'Total supply does not match sum of user positions',
        severity: 'critical',
        details: { totalSupply, sumUserPositions },
      };
    }

    if (totalSupply > globalCap) {
      return {
        name: this.name,
        passed: false,
        message: 'Total supply exceeds global cap',
        severity: 'incident',
        details: { totalSupply, globalCap },
      };
    }

    return {
      name: this.name,
      passed: true,
      message: 'Supply invariants passed',
      severity: 'warning'
    };
  }
}
