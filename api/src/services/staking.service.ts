import { randomUUID } from 'crypto';
import type {
  StakingPosition,
  StakeRequest,
  UnstakeRequest,
  DelegateRequest,
  RevokeDelegationRequest,
  StakingRewardConfig,
} from '../types/staking';
import logger from '../utils/logger';

const REWARD_CONFIG: StakingRewardConfig = {
  baseAprBps: 500, // 5% base APR
  lockupBonusBps: 200, // +2% per 30-day lockup tier
  epochDurationSeconds: 86_400, // daily epochs
};

const MIN_STAKE_AMOUNT = BigInt(1_000_000); // 1 token in stroops
const LOCKUP_OPTIONS = [0, 30, 90, 180, 365];

// In-memory store. Replace with DB/Redis in production.
const positions = new Map<string, StakingPosition>();

function now(): string {
  return new Date().toISOString();
}

function computeVotingPower(stakedAmount: bigint, lockupDays: number): string {
  // Voting power = staked * (1 + lockup_multiplier)
  // Each 30-day tier adds 0.25x
  const tierMultiplierBps = Math.floor(lockupDays / 30) * 25; // bps
  const base = stakedAmount * BigInt(10_000 + tierMultiplierBps);
  return (base / BigInt(10_000)).toString();
}

function computeLockupEnd(lockupDays: number): string {
  const ms = lockupDays * 24 * 60 * 60 * 1000;
  return new Date(Date.now() + ms).toISOString();
}

function accrueRewards(position: StakingPosition): string {
  const staked = BigInt(position.stakedAmount);
  if (staked === BigInt(0)) return position.earnedRewards;

  const lockupBonus = Math.floor(position.lockupDays / 30) * REWARD_CONFIG.lockupBonusBps;
  const totalAprBps = REWARD_CONFIG.baseAprBps + lockupBonus;

  const updatedAt = new Date(position.updatedAt).getTime();
  const elapsed = (Date.now() - updatedAt) / 1000; // seconds
  const epochsElapsed = elapsed / REWARD_CONFIG.epochDurationSeconds;

  // rewards = staked * apr * elapsed_epochs / epochs_per_year
  const epochsPerYear = 365;
  const reward =
    (staked * BigInt(Math.round(epochsElapsed * totalAprBps * 100))) /
    BigInt(epochsPerYear * 10_000 * 100);

  const current = BigInt(position.earnedRewards);
  return (current + reward).toString();
}

export class StakingService {
  stake(req: StakeRequest): StakingPosition {
    const amount = BigInt(req.amount);
    if (amount < MIN_STAKE_AMOUNT) {
      throw Object.assign(new Error('Stake amount below minimum'), { status: 400 });
    }

    const lockupDays = req.lockupDays ?? 0;
    if (!LOCKUP_OPTIONS.includes(lockupDays)) {
      throw Object.assign(
        new Error(`Invalid lockup period. Allowed: ${LOCKUP_OPTIONS.join(', ')} days`),
        { status: 400 }
      );
    }

    const existing = positions.get(req.userAddress);

    if (existing) {
      // Merge into existing position, accruing rewards first
      const accrued = accrueRewards(existing);
      const newStaked = BigInt(existing.stakedAmount) + amount;
      const updated: StakingPosition = {
        ...existing,
        stakedAmount: newStaked.toString(),
        lockupDays: Math.max(existing.lockupDays, lockupDays),
        lockupEndTime: computeLockupEnd(Math.max(existing.lockupDays, lockupDays)),
        votingPower: computeVotingPower(newStaked, Math.max(existing.lockupDays, lockupDays)),
        earnedRewards: accrued,
        updatedAt: now(),
      };
      positions.set(req.userAddress, updated);
      logger.info('Staking position increased', {
        userAddress: req.userAddress,
        amount: req.amount,
      });
      return updated;
    }

    const position: StakingPosition = {
      userAddress: req.userAddress,
      stakedAmount: amount.toString(),
      lockupDays,
      lockupEndTime: computeLockupEnd(lockupDays),
      votingPower: computeVotingPower(amount, lockupDays),
      delegatedFrom: [],
      earnedRewards: '0',
      createdAt: now(),
      updatedAt: now(),
    };
    positions.set(req.userAddress, position);
    logger.info('New staking position created', { userAddress: req.userAddress });
    return position;
  }

  unstake(req: UnstakeRequest): StakingPosition {
    const position = positions.get(req.userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }

    const lockupEnd = new Date(position.lockupEndTime).getTime();
    const isEarlyUnstake = position.lockupDays > 0 && Date.now() < lockupEnd;

    let amount = BigInt(req.amount);
    const staked = BigInt(position.stakedAmount);
    if (amount > staked) {
      throw Object.assign(new Error('Unstake amount exceeds staked balance'), { status: 400 });
    }

    // Early unstake penalty: 10% of unstaked amount
    if (isEarlyUnstake) {
      const penalty = amount / BigInt(10);
      amount -= penalty;
      logger.info('Early unstake penalty applied', {
        userAddress: req.userAddress,
        penalty: penalty.toString(),
      });
    }

    const accrued = accrueRewards(position);
    const newStaked = staked - BigInt(req.amount);
    const updated: StakingPosition = {
      ...position,
      stakedAmount: newStaked.toString(),
      votingPower: computeVotingPower(newStaked, position.lockupDays),
      earnedRewards: accrued,
      updatedAt: now(),
    };
    positions.set(req.userAddress, updated);
    logger.info('Unstaked tokens', { userAddress: req.userAddress, amount: req.amount });
    return updated;
  }

  delegate(req: DelegateRequest): { delegator: StakingPosition; delegate: StakingPosition } {
    const delegatorPos = positions.get(req.userAddress);
    if (!delegatorPos) {
      throw Object.assign(new Error('No staking position for delegator'), { status: 404 });
    }
    if (req.userAddress === req.delegateTo) {
      throw Object.assign(new Error('Cannot delegate to self'), { status: 400 });
    }

    const delegatePos = positions.get(req.delegateTo);
    if (!delegatePos) {
      throw Object.assign(new Error('Delegate address has no staking position'), { status: 404 });
    }

    // Remove from previous delegate if any
    if (delegatorPos.delegatedTo) {
      const prevDelegate = positions.get(delegatorPos.delegatedTo);
      if (prevDelegate) {
        const filtered = prevDelegate.delegatedFrom.filter((a) => a !== req.userAddress);
        const prevDelegateVP = computeVotingPower(
          BigInt(prevDelegate.stakedAmount),
          prevDelegate.lockupDays
        );
        positions.set(delegatorPos.delegatedTo, {
          ...prevDelegate,
          delegatedFrom: filtered,
          votingPower: prevDelegateVP,
          updatedAt: now(),
        });
      }
    }

    const updatedDelegator: StakingPosition = {
      ...delegatorPos,
      delegatedTo: req.delegateTo,
      votingPower: '0', // delegator gives up their voting power
      updatedAt: now(),
    };

    const delegatedPower = BigInt(delegatorPos.votingPower || delegatorPos.stakedAmount);
    const delegateNewVP = BigInt(delegatePos.votingPower) + delegatedPower;

    const updatedDelegate: StakingPosition = {
      ...delegatePos,
      delegatedFrom: [...new Set([...delegatePos.delegatedFrom, req.userAddress])],
      votingPower: delegateNewVP.toString(),
      updatedAt: now(),
    };

    positions.set(req.userAddress, updatedDelegator);
    positions.set(req.delegateTo, updatedDelegate);

    logger.info('Vote delegation set', { from: req.userAddress, to: req.delegateTo });
    return { delegator: updatedDelegator, delegate: updatedDelegate };
  }

  revokeDelegation(req: RevokeDelegationRequest): StakingPosition {
    const position = positions.get(req.userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }
    if (!position.delegatedTo) {
      throw Object.assign(new Error('No active delegation to revoke'), { status: 400 });
    }

    const delegatePos = positions.get(position.delegatedTo);
    if (delegatePos) {
      const reclaimed = BigInt(position.stakedAmount);
      const delegateNewVP = BigInt(delegatePos.votingPower) - reclaimed;
      positions.set(position.delegatedTo, {
        ...delegatePos,
        delegatedFrom: delegatePos.delegatedFrom.filter((a) => a !== req.userAddress),
        votingPower: (delegateNewVP > BigInt(0) ? delegateNewVP : BigInt(0)).toString(),
        updatedAt: now(),
      });
    }

    const restored: StakingPosition = {
      ...position,
      delegatedTo: undefined,
      votingPower: computeVotingPower(BigInt(position.stakedAmount), position.lockupDays),
      updatedAt: now(),
    };
    positions.set(req.userAddress, restored);
    logger.info('Delegation revoked', { userAddress: req.userAddress });
    return restored;
  }

  claimRewards(userAddress: string): { position: StakingPosition; claimed: string } {
    const position = positions.get(userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }

    const totalRewards = accrueRewards(position);
    const updated: StakingPosition = {
      ...position,
      earnedRewards: '0',
      updatedAt: now(),
    };
    positions.set(userAddress, updated);
    logger.info('Rewards claimed', { userAddress, amount: totalRewards });
    return { position: updated, claimed: totalRewards };
  }

  getPosition(userAddress: string): StakingPosition {
    const position = positions.get(userAddress);
    if (!position) {
      throw Object.assign(new Error('No staking position found'), { status: 404 });
    }
    return { ...position, earnedRewards: accrueRewards(position) };
  }

  getAllPositions(): StakingPosition[] {
    return Array.from(positions.values()).map((p) => ({
      ...p,
      earnedRewards: accrueRewards(p),
    }));
  }
}

export const stakingService = new StakingService();
