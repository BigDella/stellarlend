import { Request, Response, NextFunction } from 'express';
import { reputationService } from '../services/reputation.service';

/**
 * GET /api/reputation/:address
 * Fetch the reputation score and details for a given borrower address.
 * Calls the on-chain Soroban reputation contract via simulation; falls back
 * to empty defaults when the contract is not configured.
 */
export const getReputation = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { address } = req.params;

    if (!address) {
      return res.status(400).json({ success: false, error: 'Address parameter is required' });
    }

    const reputation = await reputationService.getReputation(address);
    return res.status(200).json({ success: true, reputation });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/reputation/tiers
 * List all reputation tiers and their associated benefits.
 */
export const getReputationTiers = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const tiers = [
      {
        tier: 'Bronze',
        min_score: 0,
        max_score: 249,
        benefits: {
          interest_rate_discount_bps: 0,
          borrowing_limit_multiplier_bps: 10_000,
          collateral_reduction_bps: 0,
        },
      },
      {
        tier: 'Silver',
        min_score: 250,
        max_score: 499,
        benefits: {
          interest_rate_discount_bps: 25,
          borrowing_limit_multiplier_bps: 11_000,
          collateral_reduction_bps: 100,
        },
      },
      {
        tier: 'Gold',
        min_score: 500,
        max_score: 749,
        benefits: {
          interest_rate_discount_bps: 50,
          borrowing_limit_multiplier_bps: 12_500,
          collateral_reduction_bps: 200,
        },
      },
      {
        tier: 'Platinum',
        min_score: 750,
        max_score: 1000,
        benefits: {
          interest_rate_discount_bps: 100,
          borrowing_limit_multiplier_bps: 15_000,
          collateral_reduction_bps: 300,
        },
      },
    ];

    return res.status(200).json({ success: true, tiers });
  } catch (err) {
    next(err);
    return;
  }
};

/**
 * GET /api/reputation/leaderboard
 * Return the top reputed borrowers ordered by score descending.
 * Populated from in-memory cache built as addresses are queried.
 */
export const getLeaderboard = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const limit = Math.min(Math.max(Number(req.query.limit) || 10, 1), 100);
    const leaderboard = reputationService.getLeaderboard(limit);
    return res.status(200).json({ success: true, leaderboard, total: leaderboard.length, limit });
  } catch (err) {
    next(err);
    return;
  }
};
