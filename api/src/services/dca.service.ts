import logger from '../utils/logger';

type Frequency = 'daily' | 'weekly' | 'monthly';
type Direction = 'buy' | 'sell';
type PlanStatus = 'active' | 'paused' | 'completed' | 'cancelled';

interface DcaPlan {
  id: string;
  userAddress: string;
  asset: string;
  amountPerExecution: number;
  frequency: Frequency;
  direction: Direction;
  maxExecutions: number;
  totalExecutions: number;
  fundedAmount: number;
  spentAmount: number;
  status: PlanStatus;
  nextExecutionAt: number;
  createdAt: number;
}

interface DcaExecution {
  planId: string;
  executionNumber: number;
  amount: number;
  executedAt: number;
}

const plans = new Map<string, DcaPlan>();
const userPlans = new Map<string, string[]>();
const executionHistory = new Map<string, DcaExecution[]>();
let nextId = 1;

const FREQUENCY_MS: Record<Frequency, number> = {
  daily: 24 * 60 * 60 * 1000,
  weekly: 7 * 24 * 60 * 60 * 1000,
  monthly: 30 * 24 * 60 * 60 * 1000,
};

export const dcaService = {
  createPlan(input: {
    userAddress: string;
    asset: string;
    amountPerExecution: number;
    frequency: Frequency;
    direction: Direction;
    maxExecutions: number;
    fundedAmount: number;
  }): DcaPlan {
    if (!input.userAddress) throw new Error('userAddress is required');
    if (input.amountPerExecution <= 0) throw new Error('amountPerExecution must be positive');
    if (input.maxExecutions <= 0) throw new Error('maxExecutions must be positive');

    const id = `dca_${nextId++}`;
    const now = Date.now();

    const plan: DcaPlan = {
      id,
      userAddress: input.userAddress,
      asset: input.asset,
      amountPerExecution: input.amountPerExecution,
      frequency: input.frequency,
      direction: input.direction,
      maxExecutions: input.maxExecutions,
      totalExecutions: 0,
      fundedAmount: input.fundedAmount,
      spentAmount: 0,
      status: 'active',
      nextExecutionAt: now + FREQUENCY_MS[input.frequency],
      createdAt: now,
    };

    plans.set(id, plan);

    const existing = userPlans.get(input.userAddress) ?? [];
    existing.push(id);
    userPlans.set(input.userAddress, existing);

    logger.info(`DCA plan created: ${id} for ${input.userAddress}`);
    return plan;
  },

  getPlan(planId: string): DcaPlan {
    const plan = plans.get(planId);
    if (!plan) throw new Error('Plan not found');
    return plan;
  },

  getUserPlans(userAddress: string): DcaPlan[] {
    const ids = userPlans.get(userAddress) ?? [];
    return ids.map((id) => plans.get(id)!).filter(Boolean);
  },

  pause(planId: string, userAddress: string): void {
    const plan = plans.get(planId);
    if (!plan) throw new Error('Plan not found');
    if (plan.userAddress !== userAddress) throw new Error('Unauthorized');
    if (plan.status !== 'active') throw new Error('Plan is not active');
    plan.status = 'paused';
  },

  resume(planId: string, userAddress: string): void {
    const plan = plans.get(planId);
    if (!plan) throw new Error('Plan not found');
    if (plan.userAddress !== userAddress) throw new Error('Unauthorized');
    if (plan.status !== 'paused') throw new Error('Plan is not paused');
    plan.status = 'active';
    plan.nextExecutionAt = Date.now() + FREQUENCY_MS[plan.frequency];
  },

  cancel(planId: string, userAddress: string): { refundAmount: number } {
    const plan = plans.get(planId);
    if (!plan) throw new Error('Plan not found');
    if (plan.userAddress !== userAddress) throw new Error('Unauthorized');
    if (plan.status === 'completed' || plan.status === 'cancelled') {
      throw new Error('Plan already finalized');
    }

    const refund = plan.fundedAmount - plan.spentAmount;
    plan.status = 'cancelled';

    logger.info(`DCA plan cancelled: ${planId}, refund: ${refund}`);
    return { refundAmount: refund };
  },

  getExecutionHistory(planId: string): DcaExecution[] {
    return executionHistory.get(planId) ?? [];
  },

  executeDue(): DcaExecution[] {
    const now = Date.now();
    const results: DcaExecution[] = [];

    for (const plan of plans.values()) {
      if (plan.status !== 'active') continue;
      if (now < plan.nextExecutionAt) continue;
      if (plan.totalExecutions >= plan.maxExecutions) {
        plan.status = 'completed';
        continue;
      }

      const remaining = plan.fundedAmount - plan.spentAmount;
      if (remaining < plan.amountPerExecution) continue;

      plan.totalExecutions++;
      plan.spentAmount += plan.amountPerExecution;
      plan.nextExecutionAt = now + FREQUENCY_MS[plan.frequency];

      if (plan.totalExecutions >= plan.maxExecutions) {
        plan.status = 'completed';
      }

      const execution: DcaExecution = {
        planId: plan.id,
        executionNumber: plan.totalExecutions,
        amount: plan.amountPerExecution,
        executedAt: now,
      };

      const history = executionHistory.get(plan.id) ?? [];
      history.push(execution);
      executionHistory.set(plan.id, history);
      results.push(execution);
    }

    return results;
  },
};
