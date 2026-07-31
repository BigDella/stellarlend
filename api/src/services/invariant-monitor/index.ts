import logger from '../../utils/logger';

export interface InvariantCheckResult {
  name: string;
  passed: boolean;
  message: string;
  severity: 'warning' | 'critical' | 'incident';
  details?: Record<string, any>;
}

export interface InvariantCheck {
  name: string;
  runCheck: () => Promise<InvariantCheckResult>;
}

export class InvariantMonitorService {
  private checks: InvariantCheck[] = [];
  private intervalId: NodeJS.Timeout | null = null;
  
  constructor() {}

  registerCheck(check: InvariantCheck) {
    this.checks.push(check);
  }

  async runAllChecks() {
    logger.info('Running all invariant checks...');
    for (const check of this.checks) {
      try {
        const result = await check.runCheck();
        if (!result.passed) {
          await this.handleViolation(result);
        }
      } catch (error: any) {
        logger.error(`Error running invariant check ${check.name}:`, error.message);
      }
    }
  }

  private async handleViolation(result: InvariantCheckResult) {
    logger.error(`INVARIANT VIOLATION [${result.severity.toUpperCase()}]: ${result.name} - ${result.message}`, result.details);
    
    // Alert escalation based on severity
    if (result.severity === 'incident' || result.severity === 'critical') {
      await this.triggerPagerDutyAlert(result);
    }
    
    await this.sendSlackAlert(result);
  }

  private async triggerPagerDutyAlert(result: InvariantCheckResult) {
    // Stub for PagerDuty Webhook
    logger.info(`[PagerDuty] Triggered alert for ${result.name}`);
  }

  private async sendSlackAlert(result: InvariantCheckResult) {
    // Stub for Slack Webhook
    logger.info(`[Slack] Sent alert for ${result.name}`);
  }

  start(intervalMs: number = 60000) {
    if (this.intervalId) {
      clearInterval(this.intervalId);
    }
    this.intervalId = setInterval(() => this.runAllChecks(), intervalMs);
    logger.info(`Invariant Monitor started with interval ${intervalMs}ms`);
  }

  stop() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
      this.intervalId = null;
      logger.info('Invariant Monitor stopped');
    }
  }
}

export const invariantMonitorService = new InvariantMonitorService();
