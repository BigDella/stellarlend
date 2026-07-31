import { randomUUID } from 'crypto';
import { StrKey } from '@stellar/stellar-sdk';
import logger from '../../utils/logger';
import { ConflictError, NotFoundError, ValidationError } from '../../utils/errors';
import {
  CreateReinvestmentPlanRequest,
  RecordSweepRequest,
  ReinvestmentAnalytics,
  ReinvestmentEvent,
  ReinvestmentPlan,
  ReinvestSchedule,
  ReinvestStrategy,
  WeightedTarget,
} from '../../types/reinvestment';

const STRATEGIES: ReinvestStrategy[] = ['same_pool', 'best_apy', 'weighted'];
const SCHEDULES: ReinvestSchedule[] = ['real_time', 'daily', 'weekly', 'threshold'];
const SCHEDULE_GAP_MS: Record<ReinvestSchedule, number> = {
  real_time: 0,
  threshold: 0,
  daily: 24 * 60 * 60 * 1000,
  weekly: 7 * 24 * 60 * 60 * 1000,
};
const BPS_DENOMINATOR = 10_000;
const MAX_HISTORY_LEN = 200;
const DEFAULT_ASSUMED_APY_BPS = 500; // 5%, used for compounded-vs-manual analytics when not overridden

const plans = new Map<string, ReinvestmentPlan>();
const userPlanIds = new Map<string, string[]>();
const history = new Map<string, ReinvestmentEvent[]>();
let nextSweepEligibleAt = new Map<string, number>();

function now(): string {
  return new Date().toISOString();
}

function assertValidAddress(address: string, field: string): void {
  if (!address || !StrKey.isValidEd25519PublicKey(address)) {
    throw new ValidationError(`${field} must be a valid Stellar address`);
  }
}

function assertPositiveAmountString(value: string, field: string): void {
  if (!/^\d+$/.test(value)) {
    throw new ValidationError(`${field} must be a non-negative integer string`);
  }
}

function validateWeightedTargets(strategy: ReinvestStrategy, targets: WeightedTarget[] | undefined): WeightedTarget[] {
  const list = targets ?? [];
  if (strategy === 'weighted') {
    if (list.length === 0) {
      throw new ValidationError('weightedTargets is required when strategy is "weighted"');
    }
    const total = list.reduce((sum, t) => sum + t.weightBps, 0);
    if (total !== BPS_DENOMINATOR) {
      throw new ValidationError(`weightedTargets weightBps must sum to ${BPS_DENOMINATOR}, got ${total}`);
    }
    list.forEach((t, i) => assertValidAddress(t.pool, `weightedTargets[${i}].pool`));
    return list;
  }
  if (list.length > 0) {
    throw new ValidationError('weightedTargets is only allowed when strategy is "weighted"');
  }
  return [];
}

function appendHistory(planId: string, events: ReinvestmentEvent[]): void {
  const existing = history.get(planId) ?? [];
  const updated = [...events.reverse(), ...existing];
  if (updated.length > MAX_HISTORY_LEN) {
    updated.length = MAX_HISTORY_LEN;
  }
  history.set(planId, updated);
}

function requireOwnedPlan(planId: string, userAddress: string): ReinvestmentPlan {
  const plan = plans.get(planId);
  if (!plan) throw new NotFoundError(`Reinvestment plan ${planId} not found`);
  if (plan.userAddress !== userAddress) {
    throw new ValidationError('userAddress does not own this reinvestment plan');
  }
  return plan;
}

function finalizeSweep(
  plan: ReinvestmentPlan,
  events: ReinvestmentEvent[],
  scheduleGapMs: number
): ReinvestmentEvent[] {
  const totalSwept = events.reduce((sum, e) => sum + BigInt(e.amount), BigInt(0));
  const updated: ReinvestmentPlan = {
    ...plan,
    totalReinvested: (BigInt(plan.totalReinvested) + totalSwept).toString(),
    totalSweeps: plan.totalSweeps + 1,
    lastSweptAt: now(),
    updatedAt: now(),
  };
  plans.set(plan.id, updated);
  appendHistory(plan.id, events);
  if (scheduleGapMs > 0) {
    nextSweepEligibleAt.set(plan.id, Date.now() + scheduleGapMs);
  }
  logger.info(`Reinvestment sweep recorded: ${plan.id}, events: ${events.length}, total: ${totalSwept.toString()}`);
  return events;
}

export const reinvestmentService = {
  createPlan(input: CreateReinvestmentPlanRequest): ReinvestmentPlan {
    assertValidAddress(input.userAddress, 'userAddress');
    assertValidAddress(input.sourcePool, 'sourcePool');
    if (!STRATEGIES.includes(input.strategy)) {
      throw new ValidationError(`strategy must be one of: ${STRATEGIES.join(', ')}`);
    }
    if (!SCHEDULES.includes(input.schedule)) {
      throw new ValidationError(`schedule must be one of: ${SCHEDULES.join(', ')}`);
    }
    assertPositiveAmountString(input.thresholdAmount, 'thresholdAmount');
    const weightedTargets = validateWeightedTargets(input.strategy, input.weightedTargets);

    const timestamp = now();
    const plan: ReinvestmentPlan = {
      id: randomUUID(),
      userAddress: input.userAddress,
      sourcePool: input.sourcePool,
      strategy: input.strategy,
      schedule: input.schedule,
      thresholdAmount: input.thresholdAmount,
      weightedTargets,
      paused: false,
      totalReinvested: '0',
      totalSweeps: 0,
      createdAt: timestamp,
      updatedAt: timestamp,
    };

    plans.set(plan.id, plan);
    const existing = userPlanIds.get(input.userAddress) ?? [];
    existing.push(plan.id);
    userPlanIds.set(input.userAddress, existing);

    logger.info(`Reinvestment plan created: ${plan.id} for ${input.userAddress}`);
    return plan;
  },

  getPlan(planId: string): ReinvestmentPlan {
    const plan = plans.get(planId);
    if (!plan) throw new NotFoundError(`Reinvestment plan ${planId} not found`);
    return plan;
  },

  getUserPlans(userAddress: string): ReinvestmentPlan[] {
    const ids = userPlanIds.get(userAddress) ?? [];
    return ids.map((id) => plans.get(id)).filter((p): p is ReinvestmentPlan => Boolean(p));
  },

  pause(planId: string, userAddress: string): ReinvestmentPlan {
    const plan = requireOwnedPlan(planId, userAddress);
    if (plan.paused) throw new ConflictError('Reinvestment plan is already paused');
    const updated: ReinvestmentPlan = { ...plan, paused: true, updatedAt: now() };
    plans.set(planId, updated);
    logger.info(`Reinvestment plan paused: ${planId}`);
    return updated;
  },

  resume(planId: string, userAddress: string): ReinvestmentPlan {
    const plan = requireOwnedPlan(planId, userAddress);
    if (!plan.paused) throw new ConflictError('Reinvestment plan is not paused');
    const updated: ReinvestmentPlan = { ...plan, paused: false, updatedAt: now() };
    plans.set(planId, updated);
    logger.info(`Reinvestment plan resumed: ${planId}`);
    return updated;
  },

  /**
   * Record a sweep that has already been executed on-chain (via the earnings-reinvest
   * contract's `sweep` call). Applies the same gating rules as the contract — threshold,
   * pause state, pool-paused state, gas-vs-earnings viability, and schedule cadence — so
   * API consumers get a consistent error before/without submitting a doomed transaction,
   * and records the resulting reinvestment event(s) for history/analytics.
   */
  recordSweep(planId: string, input: RecordSweepRequest): ReinvestmentEvent[] {
    const plan = plans.get(planId);
    if (!plan) throw new NotFoundError(`Reinvestment plan ${planId} not found`);
    if (plan.paused) throw new ConflictError('Reinvestment plan is paused');
    if (input.poolPaused) throw new ConflictError('Target pool is currently paused');

    assertPositiveAmountString(input.earnedAmount, 'earnedAmount');
    assertPositiveAmountString(input.estimatedGasCost, 'estimatedGasCost');
    const earned = BigInt(input.earnedAmount);
    const gasCost = BigInt(input.estimatedGasCost);
    const threshold = BigInt(plan.thresholdAmount);

    if (earned < threshold) {
      throw new ValidationError(
        `earnedAmount (${input.earnedAmount}) is below plan threshold (${plan.thresholdAmount})`
      );
    }
    if (gasCost >= earned) {
      throw new ValidationError('estimatedGasCost meets or exceeds earnedAmount; sweep is not economical');
    }

    const gap = SCHEDULE_GAP_MS[plan.schedule];
    if (gap > 0) {
      const eligibleAt = nextSweepEligibleAt.get(planId) ?? 0;
      if (Date.now() < eligibleAt) {
        throw new ConflictError(
          `Plan is on a ${plan.schedule} schedule; not eligible again until ${new Date(eligibleAt).toISOString()}`
        );
      }
    }

    if (plan.strategy === 'weighted') {
      let allocated = BigInt(0);
      const events: ReinvestmentEvent[] = [];
      plan.weightedTargets.forEach((target, i) => {
        const isLast = i === plan.weightedTargets.length - 1;
        const share = isLast
          ? earned - allocated
          : (earned * BigInt(target.weightBps)) / BigInt(BPS_DENOMINATOR);
        allocated += share;
        events.push({
          planId,
          pool: target.pool,
          amount: share.toString(),
          costBasis: share.toString(),
          sweptAt: now(),
          txHash: input.txHash,
        });
      });
      return finalizeSweep(plan, events, gap);
    }

    let pool = plan.sourcePool;
    if (plan.strategy === 'best_apy') {
      if (!input.targetPool) {
        throw new ValidationError('targetPool is required for the "best_apy" strategy');
      }
      assertValidAddress(input.targetPool, 'targetPool');
      pool = input.targetPool;
    }
    const event: ReinvestmentEvent = {
      planId,
      pool,
      amount: earned.toString(),
      costBasis: earned.toString(),
      sweptAt: now(),
      txHash: input.txHash,
    };
    return finalizeSweep(plan, [event], gap);
  },

  getHistory(planId: string): ReinvestmentEvent[] {
    if (!plans.has(planId)) throw new NotFoundError(`Reinvestment plan ${planId} not found`);
    return [...(history.get(planId) ?? [])];
  },

  getAnalytics(planId: string, assumedApyBps: number = DEFAULT_ASSUMED_APY_BPS): ReinvestmentAnalytics {
    const plan = plans.get(planId);
    if (!plan) throw new NotFoundError(`Reinvestment plan ${planId} not found`);
    if (assumedApyBps < 0 || assumedApyBps > BPS_DENOMINATOR) {
      throw new ValidationError('assumedApyBps must be between 0 and 10000');
    }

    const events = history.get(planId) ?? [];
    const dailyRate = assumedApyBps / BPS_DENOMINATOR / 365;
    const nowMs = Date.now();

    let compoundedValue = 0;
    let manualValue = 0;
    const byPool: Record<string, string> = {};

    for (const event of events) {
      const amount = Number(event.amount);
      manualValue += amount;
      const daysElapsed = Math.max(0, (nowMs - new Date(event.sweptAt).getTime()) / (24 * 60 * 60 * 1000));
      compoundedValue += amount * Math.pow(1 + dailyRate, daysElapsed);
      byPool[event.pool] = ((BigInt(byPool[event.pool] ?? '0')) + BigInt(event.amount)).toString();
    }

    return {
      planId,
      totalReinvested: plan.totalReinvested,
      totalSweeps: plan.totalSweeps,
      compoundedValue: compoundedValue.toFixed(0),
      manualValue: manualValue.toFixed(0),
      additionalYieldFromCompounding: (compoundedValue - manualValue).toFixed(0),
      assumedApyBps,
      byPool,
    };
  },
};

export function resetReinvestmentStore(): void {
  plans.clear();
  userPlanIds.clear();
  history.clear();
  nextSweepEligibleAt = new Map<string, number>();
}
