import crypto from 'crypto';
import logger from '../utils/logger';

interface ReferralRecord {
  referrerAddress: string;
  refereeAddress: string;
  code: string;
  registeredAt: number;
  totalFeesGenerated: number;
  referrerEarned: number;
}

interface ReferrerStats {
  code: string;
  totalReferrals: number;
  l2Referrals: number;
  totalEarned: number;
  totalClaimed: number;
  claimable: number;
  lastClaimAt: number;
  referees: string[];
  tier: number;
  totalDeposit: number;
}

interface TierConfig {
  tier1Threshold: number;
  tier1BonusBps: number;
  tier2Threshold: number;
  tier2BonusBps: number;
  minDepositQualify: number;
}

const FEE_SHARE_PCT = 10;
const L2_FEE_SHARE_PCT = 3;
const MATURITY_MS = 30 * 24 * 60 * 60 * 1000; // 30 days

const TIER_CONFIG: TierConfig = {
  tier1Threshold: 5,
  tier1BonusBps: 100, // +1% bonus
  tier2Threshold: 20,
  tier2BonusBps: 300, // +3% bonus
  minDepositQualify: 100 * 10 ** 7, // 100 tokens minimum
};

const codes = new Map<string, string>(); // userAddress -> code
const codeToAddress = new Map<string, string>(); // code -> userAddress
const referrals = new Map<string, ReferralRecord>(); // refereeAddress -> record
const stats = new Map<string, ReferrerStats>();

function generateUniqueCode(address: string): string {
  const hash = crypto.createHash('sha256').update(address + Date.now()).digest('hex');
  return hash.slice(0, 8).toUpperCase();
}

export const referralService = {
  generateCode(userAddress: string): string {
    const existing = codes.get(userAddress);
    if (existing) return existing;

    const code = generateUniqueCode(userAddress);
    codes.set(userAddress, code);
    codeToAddress.set(code, userAddress);

    if (!stats.has(userAddress)) {
      stats.set(userAddress, {
        code,
        totalReferrals: 0,
        l2Referrals: 0,
        totalEarned: 0,
        totalClaimed: 0,
        claimable: 0,
        lastClaimAt: 0,
        referees: [],
        tier: 0,
        totalDeposit: 0,
      });
    }

    logger.info(`Referral code generated: ${code} for ${userAddress}`);
    return code;
  },

  register(refereeAddress: string, referralCode: string): { referrer: string } {
    const referrerAddress = codeToAddress.get(referralCode);
    if (!referrerAddress) throw new Error('Invalid referral code');
    if (referrerAddress === refereeAddress) throw new Error('Self-referral not allowed');
    if (referrals.has(refereeAddress)) throw new Error('Already registered with a referral');

    const record: ReferralRecord = {
      referrerAddress,
      refereeAddress,
      code: referralCode,
      registeredAt: Date.now(),
      totalFeesGenerated: 0,
      referrerEarned: 0,
    };
    referrals.set(refereeAddress, record);

    const referrerStats = stats.get(referrerAddress)!;
    referrerStats.totalReferrals++;
    referrerStats.referees.push(refereeAddress);

    // L2: check if referrer was also referred
    const referrerRecord = referrals.get(referrerAddress);
    if (referrerRecord) {
      const l1Stats = stats.get(referrerRecord.referrerAddress);
      if (l1Stats) l1Stats.l2Referrals++;
    }

    logger.info(`Referral registered: ${refereeAddress} -> ${referrerAddress}`);
    return { referrer: referrerAddress };
  },

  accrueFee(refereeAddress: string, feeAmount: number): void {
    const record = referrals.get(refereeAddress);
    if (!record) return;

    const l1Share = (feeAmount * FEE_SHARE_PCT) / 100;
    record.totalFeesGenerated += feeAmount;
    record.referrerEarned += l1Share;

    const referrerStats = stats.get(record.referrerAddress);
    if (referrerStats) {
      referrerStats.totalEarned += l1Share;
      referrerStats.claimable += l1Share;
    }

    // L2 commission
    const l1Record = referrals.get(record.referrerAddress);
    if (l1Record) {
      const l2Share = (feeAmount * L2_FEE_SHARE_PCT) / 100;
      const l1Stats = stats.get(l1Record.referrerAddress);
      if (l1Stats) {
        l1Stats.totalEarned += l2Share;
        l1Stats.claimable += l2Share;
      }
    }
  },

  getStats(userAddress: string): ReferrerStats | null {
    return stats.get(userAddress) ?? null;
  },

  claim(userAddress: string): { amount: number } {
    const s = stats.get(userAddress);
    if (!s || s.claimable <= 0) throw new Error('Nothing to claim');

    const now = Date.now();
    if (s.lastClaimAt > 0 && now - s.lastClaimAt < MATURITY_MS) {
      throw new Error('30-day maturity period not reached');
    }

    const amount = s.claimable;
    s.totalClaimed += amount;
    s.claimable = 0;
    s.lastClaimAt = now;

    logger.info(`Referral claim: ${userAddress} claimed ${amount}`);
    return { amount };
  },

  getReferralLink(userAddress: string): string {
    const code = codes.get(userAddress);
    if (!code) throw new Error('No referral code found. Generate one first.');
    return `https://stellarlend.com?ref=${code}`;
  },

  calculateTier(totalReferrals: number): number {
    if (totalReferrals >= TIER_CONFIG.tier2Threshold) return 2;
    if (totalReferrals >= TIER_CONFIG.tier1Threshold) return 1;
    return 0;
  },

  getTierBonus(tier: number): number {
    if (tier === 2) return TIER_CONFIG.tier2BonusBps;
    if (tier === 1) return TIER_CONFIG.tier1BonusBps;
    return 0;
  },

  validateAntiSybil(userAddress: string, totalDeposit: number): boolean {
    return totalDeposit >= TIER_CONFIG.minDepositQualify;
  },

  getConversionFunnel(userAddress: string) {
    const s = stats.get(userAddress);
    if (!s) return null;

    return {
      referralCode: s.code,
      referralsGenerated: s.referees.length,
      referralsConverted: s.totalReferrals,
      conversionRate: s.referees.length > 0 ? (s.totalReferrals / s.referees.length * 100).toFixed(2) : '0',
      l2Referrals: s.l2Referrals,
    };
  },

  getAntiSybilStatus(userAddress: string, totalDeposit: number) {
    const isEligible = this.validateAntiSybil(userAddress, totalDeposit);
    const tier = this.calculateTier(stats.get(userAddress)?.totalReferrals ?? 0);

    return {
      isEligible,
      totalDeposit,
      minRequired: TIER_CONFIG.minDepositQualify,
      currentTier: tier,
      tierBonus: this.getTierBonus(tier),
    };
  },
};
