/** Generic Result type for contract calls */
export type Result<T, E> = { ok: true; value: T } | { ok: false; error: E };

/** Contract metadata */
export const StakingMetadata = {
  contractName: "staking",
  wasmHash: "",
  version: "",
  extractedAt: "",
} as const;

// ---------------------------------------------------------------------------
// Struct types (from contract spec)
// ---------------------------------------------------------------------------

/** A user's stake position */
export interface StakePosition {
  user: string;
  amount: bigint;
  stakedAt: bigint;
  lastClaim: bigint;
}

/** Reward distribution period */
export interface RewardDistribution {
  periodStart: bigint;
  periodEnd: bigint;
  totalRewards: bigint;
  distributed: bigint;
}

// ---------------------------------------------------------------------------
// Error types
// ---------------------------------------------------------------------------

export enum StakingError {
  Unauthorized = 1,
  InvalidAmount = 2,
  InsufficientBalance = 3,
  NoStake = 4,
}

// ---------------------------------------------------------------------------
// Client interface
// ---------------------------------------------------------------------------

/** Type-safe client interface for the staking contract */
export interface StakingClient {
  /** Initialize staking contract with pool and reward tokens */
  initialize(admin: string, poolToken: string, rewardToken: string): Promise<Result<void, StakingError>>;

  /** Stake LP tokens */
  stake(user: string, amount: bigint): Promise<Result<void, StakingError>>;

  /** Unstake LP tokens */
  unstake(user: string, amount: bigint): Promise<Result<void, StakingError>>;

  /** Claim accrued rewards */
  claimRewards(user: string): Promise<Result<bigint, StakingError>>;

  /** Get user's stake position */
  getStake(user: string): Promise<StakePosition | null>;

  /** Set reward distribution rate */
  setRewardRate(rate: bigint): Promise<Result<void, StakingError>>;

  /** Get total amount staked */
  getTotalStaked(): Promise<bigint>;
}
