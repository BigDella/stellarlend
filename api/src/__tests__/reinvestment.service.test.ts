import { reinvestmentService, resetReinvestmentStore } from '../services/earnings/reinvestment.service';

jest.mock('../utils/logger');

const USER = 'GA6T6URCJEEWVTUFCFBP3OONDUTFOSAFUQDIITIUU2PYNTS4YEQKGP5E';
const OTHER_USER = 'GCTC7JUZWBLSTM5N43G3EO2OE53NAIAAU7OKRQTS5XVNLWZVBVKCK5AV';
const POOL_A = 'GBNNVJG4O3HMCGM5C4ORI4O4H3K5CQA6OTKIW2KKWY2EEG2IL3TRXK32';
const POOL_B = 'GBBGACGJBGZDU2CXSUXLEMOJ4VU7MY3PD4Q24HUPMBXF4IV3QRGT47ZW';

beforeEach(() => {
  resetReinvestmentStore();
});

describe('reinvestmentService.createPlan()', () => {
  it('creates a same_pool plan', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: 'real_time',
      thresholdAmount: '100',
    });
    expect(plan.id).toBeTruthy();
    expect(plan.paused).toBe(false);
    expect(plan.totalSweeps).toBe(0);
  });

  it('rejects an invalid userAddress', () => {
    expect(() =>
      reinvestmentService.createPlan({
        userAddress: 'not-an-address',
        sourcePool: POOL_A,
        strategy: 'same_pool',
        schedule: 'real_time',
        thresholdAmount: '0',
      })
    ).toThrow(/valid Stellar address/);
  });

  it('rejects an unknown strategy', () => {
    expect(() =>
      reinvestmentService.createPlan({
        userAddress: USER,
        sourcePool: POOL_A,
        strategy: 'yolo' as never,
        schedule: 'real_time',
        thresholdAmount: '0',
      })
    ).toThrow(/strategy must be one of/);
  });

  it('requires weightedTargets to sum to 10000 bps for weighted strategy', () => {
    expect(() =>
      reinvestmentService.createPlan({
        userAddress: USER,
        sourcePool: POOL_A,
        strategy: 'weighted',
        schedule: 'real_time',
        thresholdAmount: '0',
        weightedTargets: [
          { pool: POOL_A, weightBps: 5_000 },
          { pool: POOL_B, weightBps: 4_000 },
        ],
      })
    ).toThrow(/weightBps must sum to 10000/);
  });

  it('rejects weightedTargets for a non-weighted strategy', () => {
    expect(() =>
      reinvestmentService.createPlan({
        userAddress: USER,
        sourcePool: POOL_A,
        strategy: 'same_pool',
        schedule: 'real_time',
        thresholdAmount: '0',
        weightedTargets: [{ pool: POOL_A, weightBps: 10_000 }],
      })
    ).toThrow(/only allowed when strategy is "weighted"/);
  });
});

describe('reinvestmentService pause/resume', () => {
  it('pauses and resumes without touching the strategy', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'best_apy',
      schedule: 'weekly',
      thresholdAmount: '0',
    });

    const paused = reinvestmentService.pause(plan.id, USER);
    expect(paused.paused).toBe(true);
    expect(paused.strategy).toBe('best_apy');
    expect(paused.schedule).toBe('weekly');

    expect(() => reinvestmentService.pause(plan.id, USER)).toThrow(/already paused/);

    const resumed = reinvestmentService.resume(plan.id, USER);
    expect(resumed.paused).toBe(false);
    expect(() => reinvestmentService.resume(plan.id, USER)).toThrow(/not paused/);
  });

  it('rejects pause/resume by a non-owner', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: 'real_time',
      thresholdAmount: '0',
    });
    expect(() => reinvestmentService.pause(plan.id, OTHER_USER)).toThrow(/does not own/);
  });

  it('throws NotFoundError for an unknown plan id', () => {
    expect(() => reinvestmentService.getPlan('nonexistent')).toThrow(/not found/);
  });
});

describe('reinvestmentService.recordSweep()', () => {
  function samePoolPlan(overrides: Partial<{ threshold: string; schedule: 'real_time' | 'daily' }> = {}) {
    return reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: overrides.schedule ?? 'real_time',
      thresholdAmount: overrides.threshold ?? '100',
    });
  }

  it('rejects a sweep below the threshold', () => {
    const plan = samePoolPlan({ threshold: '1000' });
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '50',
        estimatedGasCost: '1',
        poolPaused: false,
      })
    ).toThrow(/below plan threshold/);
  });

  it('rejects a sweep where gas cost meets or exceeds earnings', () => {
    const plan = samePoolPlan({ threshold: '0' });
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '100',
        estimatedGasCost: '100',
        poolPaused: false,
      })
    ).toThrow(/not economical/);
  });

  it('rejects a sweep against a paused pool', () => {
    const plan = samePoolPlan({ threshold: '0' });
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '500',
        estimatedGasCost: '1',
        poolPaused: true,
      })
    ).toThrow(/pool is currently paused/i);
  });

  it('rejects a sweep on a paused plan', () => {
    const plan = samePoolPlan({ threshold: '0' });
    reinvestmentService.pause(plan.id, USER);
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '500',
        estimatedGasCost: '1',
        poolPaused: false,
      })
    ).toThrow(/plan is paused/i);
  });

  it('records a same_pool sweep and updates plan totals', () => {
    const plan = samePoolPlan({ threshold: '0' });
    const events = reinvestmentService.recordSweep(plan.id, {
      earnedAmount: '500',
      estimatedGasCost: '1',
      poolPaused: false,
    });
    expect(events).toHaveLength(1);
    expect(events[0]?.pool).toBe(POOL_A);
    expect(events[0]?.amount).toBe('500');
    expect(events[0]?.costBasis).toBe('500');

    const updated = reinvestmentService.getPlan(plan.id);
    expect(updated.totalReinvested).toBe('500');
    expect(updated.totalSweeps).toBe(1);
    expect(updated.lastSweptAt).toBeTruthy();
  });

  it('splits a weighted sweep across targets without losing dust', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'weighted',
      schedule: 'real_time',
      thresholdAmount: '0',
      weightedTargets: [
        { pool: POOL_A, weightBps: 3_333 },
        { pool: POOL_B, weightBps: 6_667 },
      ],
    });

    const events = reinvestmentService.recordSweep(plan.id, {
      earnedAmount: '1000',
      estimatedGasCost: '1',
      poolPaused: false,
    });

    expect(events).toHaveLength(2);
    const total = events.reduce((sum, e) => sum + BigInt(e.amount), BigInt(0));
    expect(total.toString()).toBe('1000');
  });

  it('requires targetPool for best_apy sweeps', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'best_apy',
      schedule: 'real_time',
      thresholdAmount: '0',
    });
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '500',
        estimatedGasCost: '1',
        poolPaused: false,
      })
    ).toThrow(/targetPool is required/);
  });

  it('enforces cadence for daily-scheduled plans', () => {
    const plan = samePoolPlan({ threshold: '0', schedule: 'daily' });
    reinvestmentService.recordSweep(plan.id, {
      earnedAmount: '100',
      estimatedGasCost: '1',
      poolPaused: false,
    });
    expect(() =>
      reinvestmentService.recordSweep(plan.id, {
        earnedAmount: '100',
        estimatedGasCost: '1',
        poolPaused: false,
      })
    ).toThrow(/not eligible again until/);
  });
});

describe('reinvestmentService.getHistory() and getAnalytics()', () => {
  it('returns swept events newest-first and computes analytics', () => {
    const plan = reinvestmentService.createPlan({
      userAddress: USER,
      sourcePool: POOL_A,
      strategy: 'same_pool',
      schedule: 'real_time',
      thresholdAmount: '0',
    });

    reinvestmentService.recordSweep(plan.id, { earnedAmount: '100', estimatedGasCost: '1', poolPaused: false });
    reinvestmentService.recordSweep(plan.id, { earnedAmount: '200', estimatedGasCost: '1', poolPaused: false });

    const history = reinvestmentService.getHistory(plan.id);
    expect(history).toHaveLength(2);
    expect(history[0]?.amount).toBe('200'); // most recent first

    const analytics = reinvestmentService.getAnalytics(plan.id, 500);
    expect(analytics.totalSweeps).toBe(2);
    expect(Number(analytics.compoundedValue)).toBeGreaterThanOrEqual(Number(analytics.manualValue));
    expect(analytics.byPool[POOL_A]).toBe('300');
  });

  it('throws for analytics on an unknown plan', () => {
    expect(() => reinvestmentService.getAnalytics('nonexistent')).toThrow(/not found/);
  });
});
