import logger from '../utils/logger';

interface LiquidityPosition {
  positionId: string;
  pool: string;
  asset: string;
  amount: number;
  deployedAt: number;
  yieldEarned: number;
  status: 'active' | 'pending_withdrawal' | 'withdrawn';
}

interface PoolLiquidityInfo {
  pool: string;
  totalLiquidity: number;
  polLiquidity: number;
  externalLiquidity: number;
  polPercentage: number;
  utilizationRate: number;
  currentAPY: number;
}

interface POLDeploymentProposal {
  proposalId: string;
  poolId: string;
  amount: number;
  proposedBy: string;
  timestamp: number;
  status: 'pending' | 'approved' | 'executed' | 'rejected';
  timelockExpiry?: number;
  timelockDuration: number;
}

interface POLWithdrawalProposal {
  proposalId: string;
  positionId: string;
  proposedBy: string;
  timestamp: number;
  status: 'pending' | 'approved' | 'executed' | 'rejected';
  timelockExpiry?: number;
  timelockDuration: number;
}

interface POLRebalanceProposal {
  proposalId: string;
  movements: Array<{
    fromPool: string;
    toPool: string;
    amount: number;
  }>;
  proposedBy: string;
  timestamp: number;
  status: 'pending' | 'approved' | 'executed' | 'rejected';
}

interface POLDashboard {
  totalPOLValue: number;
  totalPOLYield: number;
  polPositionCount: number;
  poolBreakdown: PoolLiquidityInfo[];
  totalLiquidity: number;
  polPercentageOfTotal: number;
}

interface POLHistory {
  timestamp: number;
  action: 'deploy' | 'withdraw' | 'harvest_yield' | 'rebalance';
  positionId?: string;
  amount: number;
  pool?: string;
  yieldAmount?: number;
}

const polStore = {
  positions: new Map<string, LiquidityPosition>(),
  pools: new Map<string, PoolLiquidityInfo>(),
  deploymentProposals: [] as POLDeploymentProposal[],
  withdrawalProposals: [] as POLWithdrawalProposal[],
  rebalanceProposals: [] as POLRebalanceProposal[],
  history: [] as POLHistory[],
  totalDeployed: 0,
};

export const protocolOwnedLiquidityService = {
  initializePool(pool: string, totalLiquidity: number, utilizationRate: number, currentAPY: number): void {
    polStore.pools.set(pool, {
      pool,
      totalLiquidity,
      polLiquidity: 0,
      externalLiquidity: totalLiquidity,
      polPercentage: 0,
      utilizationRate,
      currentAPY,
    });
    logger.info(`Pool initialized: ${pool}`);
  },

  createDeploymentProposal(
    poolId: string,
    amount: number,
    proposedBy: string,
    timelockDuration: number = 2 * 24 * 60 * 60 * 1000,
  ): POLDeploymentProposal {
    const proposal: POLDeploymentProposal = {
      proposalId: `pol-deploy-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      poolId,
      amount,
      proposedBy,
      timestamp: Date.now(),
      status: 'pending',
      timelockDuration,
    };

    polStore.deploymentProposals.push(proposal);
    logger.info(`Deployment proposal created: ${proposal.proposalId} for pool ${poolId}`);
    return proposal;
  },

  approveDeploymentProposal(proposalId: string): void {
    const proposal = polStore.deploymentProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    proposal.status = 'approved';
    proposal.timelockExpiry = Date.now() + proposal.timelockDuration;
    logger.info(`Deployment proposal approved: ${proposalId}, timelock expires at ${proposal.timelockExpiry}`);
  },

  executeDeploymentProposal(proposalId: string): LiquidityPosition {
    const proposal = polStore.deploymentProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    if (proposal.status !== 'approved') {
      throw new Error(`Proposal ${proposalId} is not approved`);
    }

    if (!proposal.timelockExpiry || Date.now() < proposal.timelockExpiry) {
      throw new Error('Timelock has not expired');
    }

    const pool = polStore.pools.get(proposal.poolId);
    if (!pool) {
      throw new Error(`Pool ${proposal.poolId} not found`);
    }

    const position: LiquidityPosition = {
      positionId: `pol-pos-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      pool: proposal.poolId,
      asset: 'POL-share',
      amount: proposal.amount,
      deployedAt: Date.now(),
      yieldEarned: 0,
      status: 'active',
    };

    polStore.positions.set(position.positionId, position);
    polStore.totalDeployed += proposal.amount;

    pool.polLiquidity += proposal.amount;
    pool.externalLiquidity = pool.totalLiquidity - pool.polLiquidity;
    pool.polPercentage = (pool.polLiquidity / pool.totalLiquidity) * 100;

    proposal.status = 'executed';

    polStore.history.push({
      timestamp: Date.now(),
      action: 'deploy',
      positionId: position.positionId,
      amount: proposal.amount,
      pool: proposal.poolId,
    });

    logger.info(`Deployment executed: ${proposal.amount} deployed to ${proposal.poolId}`);
    return position;
  },

  createWithdrawalProposal(
    positionId: string,
    proposedBy: string,
    timelockDuration: number = 2 * 24 * 60 * 60 * 1000,
  ): POLWithdrawalProposal {
    const position = polStore.positions.get(positionId);
    if (!position) {
      throw new Error(`Position ${positionId} not found`);
    }

    const proposal: POLWithdrawalProposal = {
      proposalId: `pol-withdraw-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      positionId,
      proposedBy,
      timestamp: Date.now(),
      status: 'pending',
      timelockDuration,
    };

    polStore.withdrawalProposals.push(proposal);
    logger.info(`Withdrawal proposal created: ${proposal.proposalId}`);
    return proposal;
  },

  approveWithdrawalProposal(proposalId: string): void {
    const proposal = polStore.withdrawalProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    proposal.status = 'approved';
    proposal.timelockExpiry = Date.now() + proposal.timelockDuration;
    logger.info(`Withdrawal proposal approved: ${proposalId}`);
  },

  executeWithdrawalProposal(proposalId: string): { success: boolean; amountWithdrawn: number } {
    const proposal = polStore.withdrawalProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    if (proposal.status !== 'approved') {
      throw new Error(`Proposal ${proposalId} is not approved`);
    }

    if (!proposal.timelockExpiry || Date.now() < proposal.timelockExpiry) {
      throw new Error('Timelock has not expired');
    }

    const position = polStore.positions.get(proposal.positionId);
    if (!position) {
      throw new Error(`Position ${proposal.positionId} not found`);
    }

    const pool = polStore.pools.get(position.pool);
    if (!pool) {
      throw new Error(`Pool ${position.pool} not found`);
    }

    const amountWithdrawn = position.amount + position.yieldEarned;
    position.status = 'withdrawn';

    pool.polLiquidity -= position.amount;
    pool.externalLiquidity = pool.totalLiquidity - pool.polLiquidity;
    pool.polPercentage = pool.totalLiquidity > 0 ? (pool.polLiquidity / pool.totalLiquidity) * 100 : 0;

    polStore.totalDeployed -= position.amount;

    proposal.status = 'executed';

    polStore.history.push({
      timestamp: Date.now(),
      action: 'withdraw',
      positionId: proposal.positionId,
      amount: amountWithdrawn,
      pool: position.pool,
    });

    logger.info(`Withdrawal executed: ${amountWithdrawn} withdrawn from ${position.pool}`);
    return {
      success: true,
      amountWithdrawn,
    };
  },

  getPositions(status?: 'active' | 'pending_withdrawal' | 'withdrawn'): LiquidityPosition[] {
    const positions = Array.from(polStore.positions.values());
    if (status) {
      return positions.filter((p) => p.status === status);
    }
    return positions;
  },

  harvestYield(): { positionsHarvested: number; totalYieldHarvested: number } {
    const activePositions = this.getPositions('active');
    let totalYieldHarvested = 0;

    activePositions.forEach((position) => {
      const pool = polStore.pools.get(position.pool);
      if (pool) {
        const dailyYield = (position.amount * pool.currentAPY) / 36500;
        position.yieldEarned += dailyYield;
        totalYieldHarvested += dailyYield;

        polStore.history.push({
          timestamp: Date.now(),
          action: 'harvest_yield',
          positionId: position.positionId,
          amount: position.amount,
          pool: position.pool,
          yieldAmount: dailyYield,
        });
      }
    });

    logger.info(`Harvested yield from ${activePositions.length} POL positions: ${totalYieldHarvested}`);
    return {
      positionsHarvested: activePositions.length,
      totalYieldHarvested,
    };
  },

  createRebalanceProposal(
    movements: Array<{ fromPool: string; toPool: string; amount: number }>,
    proposedBy: string,
  ): POLRebalanceProposal {
    const proposal: POLRebalanceProposal = {
      proposalId: `pol-rebal-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      movements,
      proposedBy,
      timestamp: Date.now(),
      status: 'pending',
    };

    polStore.rebalanceProposals.push(proposal);
    logger.info(`Rebalance proposal created: ${proposal.proposalId}`);
    return proposal;
  },

  approveRebalanceProposal(proposalId: string): void {
    const proposal = polStore.rebalanceProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    proposal.status = 'approved';
    logger.info(`Rebalance proposal approved: ${proposalId}`);
  },

  executeRebalanceProposal(proposalId: string): void {
    const proposal = polStore.rebalanceProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }

    if (proposal.status !== 'approved') {
      throw new Error(`Proposal ${proposalId} is not approved`);
    }

    proposal.movements.forEach((movement) => {
      const fromPool = polStore.pools.get(movement.fromPool);
      const toPool = polStore.pools.get(movement.toPool);

      if (!fromPool || !toPool) {
        throw new Error('One or more pools not found');
      }

      fromPool.polLiquidity -= movement.amount;
      toPool.polLiquidity += movement.amount;

      fromPool.polPercentage = fromPool.totalLiquidity > 0 ? (fromPool.polLiquidity / fromPool.totalLiquidity) * 100 : 0;
      toPool.polPercentage = toPool.totalLiquidity > 0 ? (toPool.polLiquidity / toPool.totalLiquidity) * 100 : 0;

      polStore.history.push({
        timestamp: Date.now(),
        action: 'rebalance',
        amount: movement.amount,
        pool: movement.toPool,
      });
    });

    proposal.status = 'executed';
    logger.info(`Rebalance executed: ${proposal.proposalId}`);
  },

  getDashboard(): POLDashboard {
    const activePositions = this.getPositions('active');

    let totalPOLValue = 0;
    let totalPOLYield = 0;

    activePositions.forEach((position) => {
      totalPOLValue += position.amount;
      totalPOLYield += position.yieldEarned;
    });

    let totalLiquidity = 0;
    const poolBreakdown: PoolLiquidityInfo[] = [];

    polStore.pools.forEach((pool) => {
      totalLiquidity += pool.totalLiquidity;
      poolBreakdown.push(pool);
    });

    return {
      totalPOLValue,
      totalPOLYield,
      polPositionCount: activePositions.length,
      poolBreakdown,
      totalLiquidity,
      polPercentageOfTotal: totalLiquidity > 0 ? (totalPOLValue / totalLiquidity) * 100 : 0,
    };
  },

  getHistory(limit: number = 100): POLHistory[] {
    return polStore.history.slice(-limit).reverse();
  },

  getDeploymentProposals(status?: 'pending' | 'approved' | 'executed' | 'rejected'): POLDeploymentProposal[] {
    if (status) {
      return polStore.deploymentProposals.filter((p) => p.status === status);
    }
    return polStore.deploymentProposals;
  },

  getWithdrawalProposals(status?: 'pending' | 'approved' | 'executed' | 'rejected'): POLWithdrawalProposal[] {
    if (status) {
      return polStore.withdrawalProposals.filter((p) => p.status === status);
    }
    return polStore.withdrawalProposals;
  },

  getRebalanceProposals(status?: 'pending' | 'approved' | 'executed' | 'rejected'): POLRebalanceProposal[] {
    if (status) {
      return polStore.rebalanceProposals.filter((p) => p.status === status);
    }
    return polStore.rebalanceProposals;
  },

  rejectDeploymentProposal(proposalId: string): void {
    const proposal = polStore.deploymentProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }
    proposal.status = 'rejected';
    logger.info(`Deployment proposal rejected: ${proposalId}`);
  },

  rejectWithdrawalProposal(proposalId: string): void {
    const proposal = polStore.withdrawalProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }
    proposal.status = 'rejected';
    logger.info(`Withdrawal proposal rejected: ${proposalId}`);
  },

  rejectRebalanceProposal(proposalId: string): void {
    const proposal = polStore.rebalanceProposals.find((p) => p.proposalId === proposalId);
    if (!proposal) {
      throw new Error(`Proposal ${proposalId} not found`);
    }
    proposal.status = 'rejected';
    logger.info(`Rebalance proposal rejected: ${proposalId}`);
  },
};
