import { randomUUID } from 'crypto';

export interface StateDiffEntry {
  field: string;
  currentValue: number;
  proposedValue: number;
}

export interface ProposalDryRunResult {
  shareId: string;
  proposalId: string;
  wouldSucceed: boolean;
  tvlDelta: number;
  apyDeltaBps: number;
  riskScoreDelta: number;
  gasUnitsEstimate: number;
  diffs: StateDiffEntry[];
  currentState: Record<string, number | boolean>;
  proposedState: Record<string, number | boolean>;
  simulatedAt: string;
}

export type ProposalKind =
  | 'min_collateral_ratio'
  | 'risk_params'
  | 'interest_rate'
  | 'emergency_pause'
  | 'pause_switch';

interface SimulateInput {
  proposalId: string;
  kind: ProposalKind;
  wouldSucceed?: boolean;
  current?: Partial<{
    minCollateralRatio: number;
    liquidationThreshold: number;
    closeFactor: number;
    liquidationIncentive: number;
    borrowApyBps: number;
    tvl: number;
    emergencyPause: boolean;
  }>;
  proposed?: Partial<{
    minCollateralRatio: number;
    liquidationThreshold: number;
    closeFactor: number;
    liquidationIncentive: number;
    borrowApyBps: number;
    emergencyPause: boolean;
  }>;
}

const cache = new Map<string, ProposalDryRunResult>();
const byProposal = new Map<string, string>();

export function resetSimulationCache(): void {
  cache.clear();
  byProposal.clear();
}

const GAS: Record<ProposalKind, number> = {
  min_collateral_ratio: 45_000,
  risk_params: 72_000,
  interest_rate: 68_000,
  emergency_pause: 24_000,
  pause_switch: 28_000,
};

export function simulateProposal(input: SimulateInput): ProposalDryRunResult {
  const cachedId = byProposal.get(input.proposalId);
  if (cachedId) {
    const hit = cache.get(cachedId);
    if (hit) return hit;
  }

  const current = {
    minCollateralRatio: 11_000,
    liquidationThreshold: 10_500,
    closeFactor: 5_000,
    liquidationIncentive: 1_000,
    borrowApyBps: 500,
    tvl: 1_000_000,
    emergencyPause: false,
    ...input.current,
  };
  const proposed = { ...current, ...input.proposed };

  if (input.kind === 'emergency_pause' && input.proposed?.emergencyPause === undefined) {
    proposed.emergencyPause = true;
  }

  const diffs: StateDiffEntry[] = [];
  const fields: Array<keyof typeof current> = [
    'minCollateralRatio',
    'liquidationThreshold',
    'closeFactor',
    'liquidationIncentive',
    'borrowApyBps',
  ];
  for (const field of fields) {
    const cur = Number(current[field]);
    const next = Number(proposed[field]);
    if (cur !== next) {
      diffs.push({ field, currentValue: cur, proposedValue: next });
    }
  }
  if (current.emergencyPause !== proposed.emergencyPause) {
    diffs.push({
      field: 'emergencyPause',
      currentValue: current.emergencyPause ? 1 : 0,
      proposedValue: proposed.emergencyPause ? 1 : 0,
    });
  }

  let tvlDelta = 0;
  if (proposed.minCollateralRatio !== current.minCollateralRatio && current.minCollateralRatio > 0) {
    tvlDelta = -((current.tvl * (proposed.minCollateralRatio - current.minCollateralRatio)) / current.minCollateralRatio) / 10;
  }
  if (proposed.emergencyPause && !current.emergencyPause) {
    tvlDelta -= current.tvl / 20;
  }

  const result: ProposalDryRunResult = {
    shareId: randomUUID(),
    proposalId: input.proposalId,
    wouldSucceed: input.wouldSucceed ?? true,
    tvlDelta,
    apyDeltaBps: proposed.borrowApyBps - current.borrowApyBps,
    riskScoreDelta:
      (current.liquidationThreshold - proposed.liquidationThreshold) / 10 +
      (proposed.closeFactor - current.closeFactor) / 20,
    gasUnitsEstimate: GAS[input.kind],
    diffs,
    currentState: current,
    proposedState: proposed,
    simulatedAt: new Date().toISOString(),
  };

  cache.set(result.shareId, result);
  byProposal.set(input.proposalId, result.shareId);
  return result;
}

export function getSharedSimulation(shareId: string): ProposalDryRunResult | null {
  return cache.get(shareId) ?? null;
}
