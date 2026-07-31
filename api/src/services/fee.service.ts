import logger from '../utils/logger';

interface FeeSnapshot {
  asset: string;
  spreadFeeBps: number;
  liquidationPenaltyBps: number;
  flashLoanFeeBps: number;
  utilizationPremiumBps: number;
  volatilityPremiumBps: number;
  computedAt: number;
}

interface FeeConfig {
  baseSpreadBps: number;
  baseLiquidationPenaltyBps: number;
  baseFlashLoanFeeBps: number;
  minFeeBps: number;
  maxFeeBps: number;
  maxChangePerHourBps: number;
  utilizationWeightBps: number;
  volatilityWeightBps: number;
}

const DEFAULT_CONFIG: FeeConfig = {
  baseSpreadBps: 30,
  baseLiquidationPenaltyBps: 500,
  baseFlashLoanFeeBps: 9,
  minFeeBps: 5,
  maxFeeBps: 500,
  maxChangePerHourBps: 50,
  utilizationWeightBps: 5000,
  volatilityWeightBps: 3000,
};

const feeHistory = new Map<string, FeeSnapshot[]>();
const lastFee = new Map<string, FeeSnapshot>();

function clamp(value: number, min: number, max: number): number {
  return Math.max(min, Math.min(max, value));
}

export const feeService = {
  getCurrentFees(asset?: string): FeeSnapshot | FeeSnapshot[] {
    if (asset) return lastFee.get(asset) ?? this.computeAndStore(asset, 5000, 200);
    return Array.from(lastFee.values());
  },

  getFeeHistory(asset: string, limit: number): FeeSnapshot[] {
    const history = feeHistory.get(asset) ?? [];
    return history.slice(-limit);
  },

  computeFee(asset: string, operation: string, amount: number): { feeBps: number; feeAmount: number } {
    const snapshot = (lastFee.get(asset) ?? this.computeAndStore(asset, 5000, 200)) as FeeSnapshot;

    let feeBps: number;
    switch (operation) {
      case 'borrow':
      case 'deposit':
        feeBps = snapshot.spreadFeeBps;
        break;
      case 'liquidation':
        feeBps = snapshot.liquidationPenaltyBps;
        break;
      case 'flash_loan':
        feeBps = snapshot.flashLoanFeeBps;
        break;
      default:
        feeBps = snapshot.spreadFeeBps;
    }

    return {
      feeBps,
      feeAmount: Math.floor((amount * feeBps) / 10_000),
    };
  },

  computeAndStore(asset: string, utilizationBps: number, volatilityBps: number): FeeSnapshot {
    const config = DEFAULT_CONFIG;

    const utilPremium = Math.floor((utilizationBps * config.utilizationWeightBps) / 10_000);
    const volPremium = Math.floor((volatilityBps * config.volatilityWeightBps) / 10_000);

    const rawSpread = config.baseSpreadBps + utilPremium + volPremium;
    let spreadFee = clamp(rawSpread, config.minFeeBps, config.maxFeeBps);

    const prev = lastFee.get(asset);
    if (prev) {
      const change = Math.abs(spreadFee - prev.spreadFeeBps);
      if (change > config.maxChangePerHourBps) {
        spreadFee = spreadFee > prev.spreadFeeBps
          ? prev.spreadFeeBps + config.maxChangePerHourBps
          : prev.spreadFeeBps - config.maxChangePerHourBps;
        spreadFee = clamp(spreadFee, config.minFeeBps, config.maxFeeBps);
      }
    }

    const snapshot: FeeSnapshot = {
      asset,
      spreadFeeBps: spreadFee,
      liquidationPenaltyBps: clamp(config.baseLiquidationPenaltyBps + Math.floor(volPremium / 2), config.minFeeBps, config.maxFeeBps),
      flashLoanFeeBps: clamp(config.baseFlashLoanFeeBps + Math.floor(utilPremium / 2), config.minFeeBps, config.maxFeeBps),
      utilizationPremiumBps: utilPremium,
      volatilityPremiumBps: volPremium,
      computedAt: Date.now(),
    };

    lastFee.set(asset, snapshot);
    const history = feeHistory.get(asset) ?? [];
    history.push(snapshot);
    if (history.length > 1000) history.splice(0, history.length - 1000);
    feeHistory.set(asset, history);

    return snapshot;
  },
};
