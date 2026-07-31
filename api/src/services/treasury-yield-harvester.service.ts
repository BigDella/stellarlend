import logger from '../utils/logger';

interface ExternalProtocol {
  protocolId: string;
  name: string;
  tvl: number;
  auditStatus: 'audited' | 'unaudited';
  ageMonths: number;
  supportedStrategies: ('single_sided_lp' | 'curve' | 'convex' | 'staking')[];
  maxAllocationPercentage: number;
}

interface RiskScore {
  protocolId: string;
  score: number;
  tvlScore: number;
  auditScore: number;
  ageScore: number;
  contractRisk: number;
  slashingRisk: number;
  bridgeRisk: number;
}

interface YieldPosition {
  positionId: string;
  protocolId: string;
  asset: string;
  amount: number;
  strategy: 'single_sided_lp' | 'curve' | 'convex' | 'staking';
  deployedAt: number;
  expectedAPY: number;
  currentYield: number;
  status: 'active' | 'pending_withdrawal' | 'emergency_withdrawn';
}

interface YieldReport {
  date: number;
  totalPositionValue: number;
  totalYieldEarned: number;
  yieldByProtocol: Record<string, number>;
  pnl: number;
  positionCount: number;
}

interface WithdrawalSimulation {
  positionId: string;
  expectedReturn: number;
  estimatedWithdrawalCost: number;
  expectedPnL: number;
  estimatedGasCost: number;
}

interface EmergencyWithdrawal {
  positionId: string;
  timestamp: number;
  amountWithdrawn: number;
  transactionHash: string;
  reason: string;
}

const yieldHarvesterStore = {
  protocols: new Map<string, ExternalProtocol>(),
  positions: new Map<string, YieldPosition>(),
  yieldHistory: new Map<string, YieldReport[]>(),
  emergencyWithdrawals: [] as EmergencyWithdrawal[],
  whitelist: new Set<string>(),
  lastWhitelistUpdate: 0,
};

export const treasuryYieldHarvesterService = {
  registerProtocol(protocol: ExternalProtocol): void {
    yieldHarvesterStore.protocols.set(protocol.protocolId, protocol);
    logger.info(`Protocol registered: ${protocol.name}`);
  },

  updateProtocolWhitelist(protocolIds: string[]): void {
    yieldHarvesterStore.whitelist.clear();
    protocolIds.forEach((id) => yieldHarvesterStore.whitelist.add(id));
    yieldHarvesterStore.lastWhitelistUpdate = Date.now();
    logger.info(`Protocol whitelist updated with ${protocolIds.length} protocols`);
  },

  getProtocols(): ExternalProtocol[] {
    return Array.from(yieldHarvesterStore.protocols.values());
  },

  getWhitelistedProtocols(): ExternalProtocol[] {
    return Array.from(yieldHarvesterStore.protocols.values()).filter((p) =>
      yieldHarvesterStore.whitelist.has(p.protocolId),
    );
  },

  calculateRiskScore(protocolId: string): RiskScore {
    const protocol = yieldHarvesterStore.protocols.get(protocolId);
    if (!protocol) {
      throw new Error(`Protocol ${protocolId} not found`);
    }

    const tvlScore = Math.min((protocol.tvl / 1000000000) * 100, 100);
    const auditScore = protocol.auditStatus === 'audited' ? 100 : 40;
    const ageScore = Math.min((protocol.ageMonths / 36) * 100, 100);

    const score = (tvlScore * 0.4 + auditScore * 0.35 + ageScore * 0.25) / 100;

    return {
      protocolId,
      score,
      tvlScore,
      auditScore,
      ageScore,
      contractRisk: 100 - auditScore,
      slashingRisk: Math.max(20 - ageScore / 5, 0),
      bridgeRisk: 10,
    };
  },

  deployToProtocol(
    protocolId: string,
    asset: string,
    amount: number,
    strategy: 'single_sided_lp' | 'curve' | 'convex' | 'staking',
  ): YieldPosition {
    const whitelist = yieldHarvesterStore.whitelist;
    if (!whitelist.has(protocolId)) {
      throw new Error(`Protocol ${protocolId} is not whitelisted`);
    }

    const protocol = yieldHarvesterStore.protocols.get(protocolId);
    if (!protocol) {
      throw new Error(`Protocol ${protocolId} not found`);
    }

    if (!protocol.supportedStrategies.includes(strategy)) {
      throw new Error(`Strategy ${strategy} not supported by ${protocol.name}`);
    }

    const riskScore = this.calculateRiskScore(protocolId);
    const expectedAPY = Math.max(15 - riskScore.score * 10, 2);

    const position: YieldPosition = {
      positionId: `yield-${Date.now()}-${Math.random().toString(36).substr(2, 9)}`,
      protocolId,
      asset,
      amount,
      strategy,
      deployedAt: Date.now(),
      expectedAPY,
      currentYield: 0,
      status: 'active',
    };

    yieldHarvesterStore.positions.set(position.positionId, position);
    logger.info(`Deployed ${amount} ${asset} to ${protocol.name} using ${strategy} strategy`);
    return position;
  },

  getPositions(status?: 'active' | 'pending_withdrawal' | 'emergency_withdrawn'): YieldPosition[] {
    const positions = Array.from(yieldHarvesterStore.positions.values());
    if (status) {
      return positions.filter((p) => p.status === status);
    }
    return positions;
  },

  updateYieldEarnings(positionId: string, yieldAmount: number): void {
    const position = yieldHarvesterStore.positions.get(positionId);
    if (!position) {
      throw new Error(`Position ${positionId} not found`);
    }

    position.currentYield += yieldAmount;

    const history = yieldHarvesterStore.yieldHistory.get(positionId) || [];
    const lastReport = history[history.length - 1];

    if (!lastReport || Date.now() - lastReport.date > 24 * 60 * 60 * 1000) {
      const report: YieldReport = {
        date: Date.now(),
        totalPositionValue: position.amount + position.currentYield,
        totalYieldEarned: position.currentYield,
        yieldByProtocol: { [position.protocolId]: yieldAmount },
        pnl: yieldAmount,
        positionCount: 1,
      };
      history.push(report);
      yieldHarvesterStore.yieldHistory.set(positionId, history);
    }

    logger.info(`Yield updated for position ${positionId}: +${yieldAmount}`);
  },

  harvestYield(): { positionsHarvested: number; totalYieldHarvested: number } {
    const positions = this.getPositions('active');
    let totalYieldHarvested = 0;

    positions.forEach((position) => {
      const dailyYield = (position.amount * position.expectedAPY) / 36500;
      this.updateYieldEarnings(position.positionId, dailyYield);
      totalYieldHarvested += dailyYield;
    });

    logger.info(`Harvested yield from ${positions.length} positions: ${totalYieldHarvested}`);
    return {
      positionsHarvested: positions.length,
      totalYieldHarvested,
    };
  },

  simulateWithdrawal(positionId: string): WithdrawalSimulation {
    const position = yieldHarvesterStore.positions.get(positionId);
    if (!position) {
      throw new Error(`Position ${positionId} not found`);
    }

    const expectedReturn = position.amount + position.currentYield;
    const estimatedWithdrawalCost = position.amount * 0.002;
    const estimatedGasCost = 0.01 * position.amount;
    const expectedPnL = position.currentYield - estimatedWithdrawalCost - estimatedGasCost;

    return {
      positionId,
      expectedReturn,
      estimatedWithdrawalCost,
      expectedPnL,
      estimatedGasCost,
    };
  },

  withdrawFromProtocol(positionId: string): { success: boolean; amountWithdrawn: number; pnl: number } {
    const position = yieldHarvesterStore.positions.get(positionId);
    if (!position) {
      throw new Error(`Position ${positionId} not found`);
    }

    if (position.status !== 'active') {
      throw new Error(`Position ${positionId} is not active`);
    }

    const simulation = this.simulateWithdrawal(positionId);
    position.status = 'pending_withdrawal';

    setTimeout(() => {
      position.status = 'active';
    }, 5000);

    logger.info(`Withdrawal initiated for position ${positionId}`);
    return {
      success: true,
      amountWithdrawn: simulation.expectedReturn,
      pnl: simulation.expectedPnL,
    };
  },

  emergencyWithdraw(positionId: string, reason: string): EmergencyWithdrawal {
    const position = yieldHarvesterStore.positions.get(positionId);
    if (!position) {
      throw new Error(`Position ${positionId} not found`);
    }

    position.status = 'emergency_withdrawn';

    const emergency: EmergencyWithdrawal = {
      positionId,
      timestamp: Date.now(),
      amountWithdrawn: position.amount + position.currentYield,
      transactionHash: `0x${Math.random().toString(16).substr(2)}`,
      reason,
    };

    yieldHarvesterStore.emergencyWithdrawals.push(emergency);
    logger.warn(`Emergency withdrawal executed for position ${positionId}: ${reason}`);
    return emergency;
  },

  getYieldReport(days: number = 30): YieldReport {
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    const allPositions = this.getPositions();

    let totalPositionValue = 0;
    let totalYieldEarned = 0;
    const yieldByProtocol: Record<string, number> = {};
    let totalPnL = 0;

    allPositions.forEach((position) => {
      const value = position.amount + position.currentYield;
      totalPositionValue += value;
      totalYieldEarned += position.currentYield;
      totalPnL += position.currentYield;

      yieldByProtocol[position.protocolId] = (yieldByProtocol[position.protocolId] || 0) + position.currentYield;
    });

    return {
      date: Date.now(),
      totalPositionValue,
      totalYieldEarned,
      yieldByProtocol,
      pnl: totalPnL,
      positionCount: allPositions.length,
    };
  },

  getEmergencyWithdrawals(limit: number = 50): EmergencyWithdrawal[] {
    return yieldHarvesterStore.emergencyWithdrawals.slice(-limit).reverse();
  },
};
