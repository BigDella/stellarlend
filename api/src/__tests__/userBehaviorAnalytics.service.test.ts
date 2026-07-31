import * as service from '../services/userBehaviorAnalytics.service';
import { ValidationError } from '../utils/errors';

describe('userBehaviorAnalytics.service', () => {
  beforeEach(() => {
    service.resetForTests();
  });

  function seedFunnel(): void {
    const now = Date.now();
    // 3 visitors
    service.recordEvent({ userId: 'u1', sessionId: 's1', stage: 'visit', timestamp: now });
    service.recordEvent({ userId: 'u2', sessionId: 's2', stage: 'visit', timestamp: now });
    service.recordEvent({ userId: 'u3', sessionId: 's3', stage: 'visit', timestamp: now });
    // 2 connect wallets
    service.recordEvent({ userId: 'u1', sessionId: 's1', stage: 'wallet_connect', timestamp: now });
    service.recordEvent({ userId: 'u2', sessionId: 's2', stage: 'wallet_connect', timestamp: now });
    // 1 deposits
    service.recordEvent({
      userId: 'u1',
      sessionId: 's1',
      stage: 'first_deposit',
      timestamp: now,
      volumeUsd: 5000,
    });
  }

  describe('recordEvent', () => {
    it('rejects events missing required fields', () => {
      expect(() =>
        service.recordEvent({ userId: '', sessionId: 's1', stage: 'visit' })
      ).toThrow(ValidationError);
    });

    it('rejects an invalid funnel stage', () => {
      expect(() =>
        service.recordEvent({ userId: 'u1', sessionId: 's1', stage: 'bogus' as any })
      ).toThrow(ValidationError);
    });

    it('silently discards opted-out events (GDPR opt-out)', () => {
      service.recordEvent({ userId: 'u1', sessionId: 's1', stage: 'visit', optedOut: true });
      expect(service.getFunnel()[0]!.uniqueUsers).toBe(0);
    });
  });

  describe('getFunnel', () => {
    it('computes unique users and drop-off per stage', () => {
      seedFunnel();
      const funnel = service.getFunnel();

      expect(funnel[0]!.stage).toBe('visit');
      expect(funnel[0]!.uniqueUsers).toBe(3);
      expect(funnel[1]!.uniqueUsers).toBe(2);
      expect(funnel[1]!.conversionFromPreviousPercent).toBeCloseTo((2 / 3) * 100);
      expect(funnel[1]!.dropOffPercent).toBeCloseTo(100 - (2 / 3) * 100);
      expect(funnel[2]!.uniqueUsers).toBe(1);
    });
  });

  describe('getConversionRates', () => {
    it('computes visitor-to-depositor and depositor-to-borrower rates', () => {
      seedFunnel();
      const rates = service.getConversionRates();
      expect(rates.visitorToDepositorPercent).toBeCloseTo((1 / 3) * 100);
      expect(rates.depositorToBorrowerPercent).toBe(0);
    });
  });

  describe('getPowerUsers', () => {
    it('ranks users by volume and returns the top percentile', () => {
      service.recordEvent({ userId: 'whale', sessionId: 's1', stage: 'first_deposit', volumeUsd: 1_000_000 });
      service.recordEvent({ userId: 'retail1', sessionId: 's2', stage: 'first_deposit', volumeUsd: 100 });
      service.recordEvent({ userId: 'retail2', sessionId: 's3', stage: 'first_deposit', volumeUsd: 200 });

      const powerUsers = service.getPowerUsers(10);
      expect(powerUsers[0]!.userId).toBe('whale');
      expect(powerUsers[0]!.volumeUsd).toBe(1_000_000);
    });
  });

  describe('getChurnRisk', () => {
    it('flags users inactive beyond the churn threshold', () => {
      const now = Date.now();
      const twentyDaysAgo = now - 20 * 24 * 60 * 60 * 1000;
      service.recordEvent({ userId: 'active', sessionId: 's1', stage: 'visit', timestamp: now });
      service.recordEvent({ userId: 'churned', sessionId: 's2', stage: 'visit', timestamp: twentyDaysAgo });

      const churnRisk = service.getChurnRisk(now);
      const userIds = churnRisk.map((u) => u.userId);
      expect(userIds).toContain('churned');
      expect(userIds).not.toContain('active');
    });
  });

  describe('getAbTestMetrics', () => {
    it('aggregates conversion rate per experiment variant', () => {
      service.recordEvent({
        userId: 'u1',
        sessionId: 's1',
        stage: 'first_deposit',
        experimentVariant: 'control',
      });
      service.recordEvent({
        userId: 'u2',
        sessionId: 's2',
        stage: 'visit',
        experimentVariant: 'control',
      });
      service.recordEvent({
        userId: 'u3',
        sessionId: 's3',
        stage: 'first_deposit',
        experimentVariant: 'treatment',
      });

      const metrics = service.getAbTestMetrics('first_deposit');
      const control = metrics.find((m) => m.variant === 'control')!;
      const treatment = metrics.find((m) => m.variant === 'treatment')!;

      expect(control.users).toBe(2);
      expect(control.conversions).toBe(1);
      expect(control.conversionRatePercent).toBeCloseTo(50);
      expect(treatment.conversionRatePercent).toBeCloseTo(100);
    });
  });

  describe('getCohortRetention', () => {
    it('groups users into cohorts and computes per-period retention', () => {
      const now = Date.now();
      service.recordEvent({ userId: 'u1', sessionId: 's1', stage: 'visit', timestamp: now });
      service.recordEvent({ userId: 'u2', sessionId: 's2', stage: 'visit', timestamp: now });

      const cohorts = service.getCohortRetention('weekly', 2);
      expect(cohorts.length).toBeGreaterThan(0);
      expect(cohorts[0]!.cohortSize).toBe(2);
      expect(cohorts[0]!.retentionByPeriod[0]).toBeCloseTo(100);
    });
  });
});
