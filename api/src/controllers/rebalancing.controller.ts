import { Request, Response, NextFunction } from 'express';
import { StellarService } from '../services/stellar.service';
import { config } from '../config/index';
import logger from '../utils/logger';
import { emergencyPauseService } from '../services/emergencyPause.service';
import { redisCacheService } from '../services/redisCache.service';
import { auditLogService } from '../services/auditLog.service';

// Rebalancing Controller
// Handles automated collateral rebalancing operations including configuration,
// execution, and emergency controls.

export const configureRebalancing = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const {
      userAddress,
      targetHealthFactorMin,
      targetHealthFactorMax,
      maxGasCost,
      autoRebalanceEnabled,
      minSwapSize,
      maxSlippageBps,
      rebalanceCooldown,
    } = req.body as any;

    logger.info('Rebalancing configuration request', {
      userAddress,
      targetHealthFactorMin,
      targetHealthFactorMax,
    });

    // TODO: Call contract method when rebalancing deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.configureRebalancing(userAddress, targetHealthFactorMin, targetHealthFactorMax, maxGasCost, autoRebalanceEnabled, minSwapSize, maxSlippageBps, rebalanceCooldown);

    const response = {
      success: true,
      user: userAddress,
      message: 'Rebalancing configuration updated',
    };

    await redisCacheService.delByPrefix('stellarlend:rebalancing:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const executeRebalancing = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress } = req.body as any;

    logger.info('Rebalancing execution request', { userAddress });

    // TODO: Call contract method when rebalancing deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.executeRebalancing(userAddress);

    const response = {
      success: true,
      user: userAddress,
      message: 'Rebalancing executed successfully',
    };

    await redisCacheService.delByPrefix('stellarlend:rebalancing:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const getRebalancingConfig = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { userAddress } = req.query as any;

    logger.info('Get rebalancing configuration request', { userAddress });

    // TODO: Call contract method when rebalancing deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.getRebalancingConfig(userAddress);

    const response = {
      success: true,
      user: userAddress,
      targetHealthFactorMin: 15000, // Default 1.5x
      targetHealthFactorMax: 25000, // Default 2.5x
      maxGasCost: 1000000,
      autoRebalanceEnabled: false,
      minSwapSize: 1000000,
      maxSlippageBps: 500,
      rebalanceCooldown: 3600,
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const setRebalancingEmergencyStop = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { stopped } = req.body as any;

    logger.info('Set rebalancing emergency stop request', { stopped });

    // TODO: Call contract method when rebalancing deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.setRebalancingEmergencyStop(stopped);

    const response = {
      success: true,
      stopped,
      message: stopped
        ? 'Rebalancing emergency stop activated'
        : 'Rebalancing emergency stop deactivated',
    };

    auditLogService.record({
      action: 'REBALANCING_EMERGENCY_STOP',
      actor: req.ip ?? 'SYSTEM',
      status: 'success',
      ip: req.ip,
    });

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const setRebalancingPause = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { paused } = req.body as any;

    logger.info('Set rebalancing pause request', { paused });

    // TODO: Call contract method when rebalancing deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.setRebalancingPause(paused);

    const response = {
      success: true,
      paused,
      message: paused ? 'Rebalancing paused' : 'Rebalancing resumed',
    };

    auditLogService.record({
      action: 'REBALANCING_PAUSE',
      actor: req.ip ?? 'SYSTEM',
      status: 'success',
      ip: req.ip,
    });

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};
