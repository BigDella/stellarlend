import { Request, Response, NextFunction } from 'express';
import { leaderboardService } from '../services/social-trading/leaderboard.service';
import { copyTradingService } from '../services/social-trading/copy-trading.service';
import { FollowRequestDto, UnfollowRequestDto, LeaderboardQueryDto } from '../dto/social.dto';
import logger from '../utils/logger';

export const getLeaderboard = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const query = LeaderboardQueryDto.fromQuery(req.query as Record<string, unknown>);
    const entries = leaderboardService.getLeaderboard(query as unknown as Record<string, unknown>);
    return res.status(200).json({ success: true, data: entries });
  } catch (error) {
    next(error);
    return;
  }
};

export const getLeaderProfile = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const address = req.params.address as string;
    const profile = leaderboardService.getLeaderProfile(address);
    if (!profile) {
      return res.status(404).json({ success: false, error: 'Leader not found or opted out' });
    }
    return res.status(200).json({ success: true, data: profile });
  } catch (error) {
    next(error);
    return;
  }
};

export const follow = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const dto = FollowRequestDto.fromBody(req.body as Record<string, unknown>);
    const result = FollowRequestDto.validate(req.body as Record<string, unknown>);
    if (!result.isValid) {
      return res.status(400).json({ success: false, error: result.toErrorString() });
    }
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const relation = copyTradingService.follow(userAddress, dto.leaderAddress, dto.amount);
    logger.info('User followed leader', { follower: userAddress, leader: dto.leaderAddress });
    return res.status(201).json({ success: true, data: relation });
  } catch (error) {
    next(error);
    return;
  }
};

export const unfollow = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const dto = UnfollowRequestDto.fromBody(req.body as Record<string, unknown>);
    const result = UnfollowRequestDto.validate(req.body as Record<string, unknown>);
    if (!result.isValid) {
      return res.status(400).json({ success: false, error: result.toErrorString() });
    }
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const relation = copyTradingService.unfollow(userAddress, dto.leaderAddress);
    if (!relation) {
      return res.status(404).json({ success: false, error: 'Follow relation not found' });
    }
    logger.info('User unfollowed leader', { follower: userAddress, leader: dto.leaderAddress });
    return res.status(200).json({ success: true, data: relation });
  } catch (error) {
    next(error);
    return;
  }
};

export const getMyFollowing = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const following = copyTradingService.getFollowing(userAddress);
    return res.status(200).json({ success: true, data: following });
  } catch (error) {
    next(error);
    return;
  }
};

export const setPrivacy = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const userAddress = req.headers['x-user-address'] as string;
    if (!userAddress) {
      return res.status(401).json({ success: false, error: 'Missing x-user-address header' });
    }
    const optOut = req.body.optOutCopying === true;
    leaderboardService.setOptOut(userAddress, optOut);
    return res.status(200).json({ success: true, data: { optOutCopying: optOut } });
  } catch (error) {
    next(error);
    return;
  }
};
