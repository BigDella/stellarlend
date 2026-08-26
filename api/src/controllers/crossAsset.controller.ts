import { Request, Response, NextFunction } from 'express';
import { StellarService } from '../services/stellar.service';
import { config } from '../config/index';
import logger from '../utils/logger';
import { emergencyPauseService } from '../services/emergencyPause.service';
import { redisCacheService } from '../services/redisCache.service';
import { auditLogService } from '../services/auditLog.service';
import { parsePaginationParams } from '../utils/pagination';

// Cross-Asset Lending Controller
// Handles cross-asset lending operations including multi-collateral borrowing,
// collateral management, and position summaries.

export const getCrossAssetPositionSummary = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress } = req.query as any;

    logger.info('Cross-asset position summary request', { userAddress });

    // TODO: Call contract method when cross-asset deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.getCrossAssetPositionSummary(userAddress);

    const response = {
      success: true,
      user: userAddress,
      totalCollateralValue: 0, // Would be actual values from contract
      weightedCollateralValue: 0,
      totalDebtValue: 0,
      weightedDebtValue: 0,
      healthFactor: 0,
      isLiquidatable: false,
      borrowCapacity: 0,
    };

    await redisCacheService.delByPrefix('stellarlend:cross_position:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const depositCrossAsset = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress, assetAddress, amount } = req.body as any;

    logger.info('Cross-asset deposit request', { userAddress, assetAddress, amount });

    // TODO: Call contract method when cross-asset deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.depositCrossAsset(userAddress, assetAddress, amount);

    const response = {
      success: true,
      transactionHash: 'pending', // Would be actual hash from contract
      amount,
      asset: assetAddress,
      user: userAddress,
    };

    await redisCacheService.delByPrefix('stellarlend:cross_position:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const borrowCrossAsset = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress, assetAddress, amount } = req.body as any;

    logger.info('Cross-asset borrow request', { userAddress, assetAddress, amount });

    // TODO: Call contract method when cross-asset deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.borrowCrossAsset(userAddress, assetAddress, amount);

    const response = {
      success: true,
      transactionHash: 'pending', // Would be actual hash from contract
      amount,
      asset: assetAddress,
      user: userAddress,
    };

    await redisCacheService.delByPrefix('stellarlend:cross_position:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const withdrawCrossAsset = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress, assetAddress, amount } = req.body as any;

    logger.info('Cross-asset withdraw request', { userAddress, assetAddress, amount });

    // TODO: Call contract method when cross-asset deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.withdrawCrossAsset(userAddress, assetAddress, amount);

    const response = {
      success: true,
      transactionHash: 'pending', // Would be actual hash from contract
      amount,
      asset: assetAddress,
      user: userAddress,
    };

    await redisCacheService.delByPrefix('stellarlend:cross_position:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const liquidateCrossAsset = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { liquidator, user, debtAsset, collateralAsset, debtToRepay, collateralToReceive } = req.body as any;

    logger.info('Cross-asset liquidation request', { liquidator, user, debtAsset, collateralAsset, debtToRepay, collateralToReceive });

    // TODO: Call contract method when cross-asset deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.liquidateCrossAsset(liquidator, user, debtAsset, collateralAsset, debtToRepay, collateralToReceive);

    const response = {
      success: true,
      transactionHash: 'pending', // Would be actual hash from contract
      liquidator,
      user,
      debtAsset,
      collateralAsset,
      debtToRepay,
      collateralReceived: collateralToReceive,
    };

    await redisCacheService.delByPrefix('stellarlend:cross_position:');

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const computeUnifiedHealth = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { computeUnifiedHealth: compute, portfolioRiskScore } = await import(
      '../services/crossAssetRisk.service'
    );
    const { positions } = req.body as { positions?: unknown };
    if (!Array.isArray(positions) || positions.length === 0) {
      return res.status(400).json({ success: false, error: 'positions array required' });
    }
    const health = compute(positions);
    return res.status(200).json({
      success: true,
      data: { ...health, portfolioRiskScore: portfolioRiskScore(health.healthFactor) },
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const getCorrelationMatrix = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { setCorrelation, getCorrelation } = await import('../services/crossAssetRisk.service');
    const { pairs, assets } = req.body ?? {};
    if (Array.isArray(pairs)) {
      for (const p of pairs) {
        setCorrelation(p.a, p.b, p.bps);
      }
    }
    const assetList: string[] = Array.isArray(assets) ? assets : [];
    const matrix: Record<string, number> = {};
    for (let i = 0; i < assetList.length; i++) {
      for (let j = i + 1; j < assetList.length; j++) {
        matrix[`${assetList[i]}|${assetList[j]}`] = getCorrelation(assetList[i]!, assetList[j]!);
      }
    }
    return res.status(200).json({ success: true, data: { matrix } });
  } catch (error) {
    next(error);
    return;
  }
};

export const getArbitrageOpportunities = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { detectArbitrage } = await import('../services/crossAssetRisk.service');
    const pools = req.body?.pools as Array<{ asset: string; utilizationBps: number }> | undefined;
    if (!Array.isArray(pools)) {
      return res.status(400).json({ success: false, error: 'pools array required' });
    }
    return res.status(200).json({ success: true, data: detectArbitrage(pools) });
  } catch (error) {
    next(error);
    return;
  }
};

export const getPairThreshold = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { pairLiquidationThreshold } = await import('../services/crossAssetRisk.service');
    const { debtAsset, collateralAsset, debtLt, collLt } = req.body ?? {};
    if (!debtAsset || !collateralAsset) {
      return res.status(400).json({ success: false, error: 'debtAsset and collateralAsset required' });
    }
    const thresholdBps = pairLiquidationThreshold(
      Number(debtLt) || 8000,
      Number(collLt) || 8000,
      String(debtAsset),
      String(collateralAsset)
    );
    return res.status(200).json({ success: true, data: { debtAsset, collateralAsset, thresholdBps } });
  } catch (error) {
    next(error);
    return;
  }
};
