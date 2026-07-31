import logger from '../utils/logger';

interface TargetAllocation {
  asset: string;
  targetPercentage: number;
}

interface CurrentAllocation {
  asset: string;
  currentPercentage: number;
  currentValue: number;
}

interface RebalanceAction {
  fromAsset: string;
  toAsset: string;
  amount: number;
  slippageProtection: number;
  estimatedGasCost: number;
}

interface RebalanceSimulation {
  actions: RebalanceAction[];
  totalGasCost: number;
  totalSlippage: number;
  resultingAllocation: CurrentAllocation[];
  rebalanceDeviation: number;
}

interface RebalanceExecutionReport {
  executionId: string;
  timestamp: number;
  success: boolean;
  actions: Array<{
    fromAsset: string;
    toAsset: string;
    amountSwapped: number;
    actualSlippage: number;
    gasCost: number;
    transactionHash?: string;
  }>;
  totalCost: number;
  totalSlippage: number;
  error?: string;
}

interface GovernanceProposal {
  proposalId: string;
  targetAllocations: TargetAllocation[];
  proposedBy: string;
  timestamp: number;
  status: 'pending' | 'approved' | 'rejected';
}

const rebalancerStore = {
  targetAllocations: new Map<string, number>(),
  currentAllocations: new Map<string, number>(),
  assetPrices: new Map<string, number>(),
  rebalanceHistory: [] as RebalanceExecutionReport[],
  governanceProposals: [] as GovernanceProposal[],
  rebalanceInterval: 7 * 24 * 60 * 60 * 1000,
  lastRebalance: 0,
  paused: false,
};

export const treasuryRebalancerService = {
  setTargetAllocations(allocations: TargetAllocation[]): void {
    const totalPercentage = allocations.reduce((sum, a) => sum + a.targetPercentage, 0);

    if (Math.abs(totalPercentage - 100) > 0.01) {
      throw new Error('Target allocations must sum to 100%');
    }

    rebalancerStore.targetAllocations.clear();
    allocations.forEach((a) => {
      rebalancerStore.targetAllocations.set(a.asset, a.targetPercentage);
    });

    logger.info('Target allocations updated');
  },

  setAssetPrices(prices: Record<string, number>): void {
    Object.entries(prices).forEach(([asset, price]) => {
      rebalancerStore.assetPrices.set(asset, price);
    });
  },

  setCurrentAllocations(allocations: CurrentAllocation[]): void {
    const totalValue = allocations.reduce((sum, a) => sum + a.currentValue, 0);

    allocations.forEach((a) => {
      const percentage = totalValue > 0 ? (a.currentValue / totalValue) * 100 : 0;
      rebalancerStore.currentAllocations.set(a.asset, percentage);
    });

    logger.info('Current allocations updated');
  },

  getCurrentAllocation(): CurrentAllocation[] {
    const allocations: CurrentAllocation[] = [];
    let totalValue = 0;

    const assetList = Array.from(rebalancerStore.currentAllocations.keys());
    assetList.forEach((asset) => {
      const price = rebalancerStore.assetPrices.get(asset) || 1;
      const currentPercentage = rebalancerStore.currentAllocations.get(asset) || 0;
      const currentValue = currentPercentage * price;
      totalValue += currentValue;
      allocations.push({ asset, currentPercentage, currentValue });
    });

    return allocations;
  },

  checkRebalanceTrigger(deviationThreshold: number = 5): { shouldRebalance: boolean; maxDeviation: number } {
    const currentTime = Date.now();
    const timeSinceLastRebalance = currentTime - rebalancerStore.lastRebalance;

    if (timeSinceLastRebalance > rebalancerStore.rebalanceInterval) {
      return { shouldRebalance: true, maxDeviation: 100 };
    }

    let maxDeviation = 0;
    rebalancerStore.targetAllocations.forEach((target, asset) => {
      const current = rebalancerStore.currentAllocations.get(asset) || 0;
      const deviation = Math.abs(current - target);
      maxDeviation = Math.max(maxDeviation, deviation);
    });

    return {
      shouldRebalance: maxDeviation > deviationThreshold,
      maxDeviation,
    };
  },

  simulateRebalance(slippagePercentage: number = 0.3): RebalanceSimulation {
    const actions: RebalanceAction[] = [];
    let totalGasCost = 0;
    let totalSlippage = 0;

    const resultingAllocation: CurrentAllocation[] = [];
    const totalValue = 100;

    rebalancerStore.targetAllocations.forEach((target, asset) => {
      const current = rebalancerStore.currentAllocations.get(asset) || 0;
      const difference = current - target;

      if (Math.abs(difference) > 0.01) {
        const price = rebalancerStore.assetPrices.get(asset) || 1;
        const amount = Math.abs(difference) * price;
        const slippage = (amount * slippagePercentage) / 100;
        const gasCost = 0.01 * amount;

        if (difference > 0) {
          const targetAssets = Array.from(rebalancerStore.targetAllocations.keys()).filter(
            (a) => (rebalancerStore.currentAllocations.get(a) || 0) < rebalancerStore.targetAllocations.get(a)!,
          );

          if (targetAssets.length > 0) {
            const toAsset = targetAssets[0];
            actions.push({
              fromAsset: asset,
              toAsset,
              amount,
              slippageProtection: slippagePercentage,
              estimatedGasCost: gasCost,
            });

            totalGasCost += gasCost;
            totalSlippage += slippage;
          }
        }
      }
    });

    let rebalanceDeviation = 0;
    rebalancerStore.targetAllocations.forEach((target, asset) => {
      const deviation = Math.abs(target - (rebalancerStore.currentAllocations.get(asset) || 0));
      rebalanceDeviation += deviation;
      resultingAllocation.push({
        asset,
        currentPercentage: target,
        currentValue: target * 100,
      });
    });

    return {
      actions,
      totalGasCost,
      totalSlippage,
      resultingAllocation,
      rebalanceDeviation,
    };
  },

  async executeRebalance(simulation: RebalanceSimulation): Promise<RebalanceExecutionReport> {
    if (rebalancerStore.paused) {
      throw new Error('Rebalancer is paused');
    }

    const executionId = `rebal-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`;
    const executedActions = simulation.actions.map((action) => ({
      fromAsset: action.fromAsset,
      toAsset: action.toAsset,
      amountSwapped: action.amount,
      actualSlippage: action.slippageProtection,
      gasCost: action.estimatedGasCost,
      transactionHash: `0x${Math.random().toString(16).substr(2)}`,
    }));

    const report: RebalanceExecutionReport = {
      executionId,
      timestamp: Date.now(),
      success: true,
      actions: executedActions,
      totalCost: simulation.totalGasCost,
      totalSlippage: simulation.totalSlippage,
    };

    rebalancerStore.rebalanceHistory.push(report);
    rebalancerStore.lastRebalance = Date.now();

    logger.info(`Rebalance executed: ${executionId}`);
    return report;
  },

  getRebalanceHistory(limit: number = 50): RebalanceExecutionReport[] {
    return rebalancerStore.rebalanceHistory.slice(-limit).reverse();
  },

  createGovernanceProposal(
    allocations: TargetAllocation[],
    proposedBy: string,
  ): GovernanceProposal {
    const proposal: GovernanceProposal = {
      proposalId: `prop-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      targetAllocations: allocations,
      proposedBy,
      timestamp: Date.now(),
      status: 'pending',
    };

    rebalancerStore.governanceProposals.push(proposal);
    logger.info(`Governance proposal created: ${proposal.proposalId}`);
    return proposal;
  },

  getGovernanceProposals(status?: 'pending' | 'approved' | 'rejected'): GovernanceProposal[] {
    if (status) {
      return rebalancerStore.governanceProposals.filter((p) => p.status === status);
    }
    return rebalancerStore.governanceProposals;
  },

  approveGovernanceProposal(proposalId: string): void {
    const proposal = rebalancerStore.governanceProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error('Proposal not found');
    }
    proposal.status = 'approved';
    this.setTargetAllocations(proposal.targetAllocations);
    logger.info(`Governance proposal approved: ${proposalId}`);
  },

  rejectGovernanceProposal(proposalId: string): void {
    const proposal = rebalancerStore.governanceProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error('Proposal not found');
    }
    proposal.status = 'rejected';
    logger.info(`Governance proposal rejected: ${proposalId}`);
  },

  pauseRebalancer(): void {
    rebalancerStore.paused = true;
    logger.info('Rebalancer paused');
  },

  resumeRebalancer(): void {
    rebalancerStore.paused = false;
    logger.info('Rebalancer resumed');
  },

  isRebalancerPaused(): boolean {
    return rebalancerStore.paused;
  },

  setRebalanceInterval(intervalMs: number): void {
    rebalancerStore.rebalanceInterval = intervalMs;
    logger.info(`Rebalance interval set to ${intervalMs}ms`);
  },
};
