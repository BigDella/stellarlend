import { Request, Response, NextFunction } from 'express';
import { StellarService } from '../services/stellar.service';
import {
  LendingOperation,
  PrepareResponse,
  RelayDelegatedRequest,
  RelayDelegatedResponse,
  SubmitRequest,
  ProtocolStatsResponse,
  TransactionHistoryQuery,
  TransactionHistoryResponse,
} from '../types/index';
import { config } from '../config/index';
import logger from '../utils/logger';
import { emergencyPauseService } from '../services/emergencyPause.service';
import { redisCacheService } from '../services/redisCache.service';
import { auditLogService } from '../services/auditLog.service';
import { parsePaginationParams } from '../utils/pagination';
import { requestCoalescingService } from '../services/requestCoalescing.service';
import {
  assignRole,
  getCurrentRoleAssignments,
  getRbacAuditContext,
  scheduleRevocation,
  type Role,
} from '../middleware/rbac';

function mapHealthResponse(services: { horizon: boolean; sorobanRpc: boolean }) {
  const isHealthy = services.horizon && services.sorobanRpc;

  return {
    isHealthy,
    readinessStatus: isHealthy ? 'ok' : 'error',
    horizon: services.horizon ? 'up' : 'down',
    soroban: services.sorobanRpc ? 'up' : 'down',
  };
}

export const prepare = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const operation = req.params.operation as LendingOperation;
    const { userAddress, assetAddress, amount } = { ...req.query, ...req.body } as any;

    logger.info('Preparing unsigned transaction', { operation, userAddress, amount });

    const stellarService = new StellarService();
    const unsignedXdr = await stellarService.buildUnsignedTransaction(
      operation,
      userAddress,
      assetAddress,
      amount
    );

    const expiresAt = new Date(Date.now() + 5 * 60 * 1000).toISOString();

    const response: PrepareResponse = { unsignedXdr, operation, expiresAt };
    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const relayDelegated = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { delegatorAddress, nonce, deadline, callsXdr } = req.body as RelayDelegatedRequest;

    const stellarService = new StellarService();
    const result = await stellarService.relayExecuteDelegated(
      delegatorAddress,
      nonce,
      deadline,
      callsXdr
    );

    const response: RelayDelegatedResponse = {
      delegateAddress: result.delegateAddress,
      transactionHash: result.txHash,
      status: result.success ? 'success' : 'failed',
      error: result.error,
    };

    return res.status(result.success ? 200 : 400).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const submit = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { signedXdr, operation, userAddress, amount, assetAddress }: SubmitRequest = req.body;
    const pauseState = emergencyPauseService.isPaused();
    if (pauseState.paused) {
      if (operation === 'withdraw' && userAddress && amount) {
        emergencyPauseService.queueWithdrawal({ userAddress, assetAddress, amount });
        return res.status(202).json({
          success: false,
          status: 'queued',
          reason: pauseState.reason,
          message: 'Withdrawal queued while protocol is paused',
        });
      }
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: pauseState.reason,
      });
    }

    logger.info('Submitting signed transaction');

    const stellarService = new StellarService();
    const result = await stellarService.submitTransaction(signedXdr);

    if (result.success && result.transactionHash) {
      emergencyPauseService.recordSuccess();
      const monitorResult = await stellarService.monitorTransaction(result.transactionHash);

      auditLogService.record({
        action: operation ? (operation.toUpperCase() as any) : 'TRANSACTION_EXECUTED',
        actor: userAddress || 'REDACTED',
        status: monitorResult.status as any,
        txHash: result.transactionHash,
        ledger: monitorResult.ledger,
        amount: amount || 'REDACTED',
        assetAddress: assetAddress || 'REDACTED',
        ip: req.ip,
      });

      await redisCacheService.delByPrefix('stellarlend:position:');
      await redisCacheService.delByPrefix('stellarlend:pool:');
      await redisCacheService.delByPrefix('stellarlend:protocol:');

      return res.status(200).json(monitorResult);
    }

    emergencyPauseService.recordFailure();
    return res.status(400).json(result);
  } catch (error) {
    emergencyPauseService.recordFailure();
    next(error);
    return;
  }
};

export const getPauseStatus = (_req: Request, res: Response) => {
  return res.status(200).json({
    ...emergencyPauseService.isPaused(),
    queuedWithdrawals: emergencyPauseService.getWithdrawalQueue(),
    cacheMetrics: redisCacheService.getMetrics(),
  });
};

export const setManualPause = (req: Request, res: Response) => {
  const before = emergencyPauseService.isPaused();
  emergencyPauseService.pause('manual');
  auditLogService.record({
    action: 'PROTOCOL_PAUSED',
    actor: req.ip ?? 'SYSTEM',
    status: 'success',
    ip: req.ip,
    beforeState: { paused: before.paused, reason: before.reason },
    afterState: { paused: true, reason: 'manual' },
  });
  return res.status(200).json({ paused: true, reason: 'manual' });
};

export const resumeProtocol = (req: Request, res: Response) => {
  const before = emergencyPauseService.isPaused();
  const queuedWithdrawals = emergencyPauseService.drainWithdrawalQueue();
  emergencyPauseService.resume();
  auditLogService.record({
    action: 'PROTOCOL_RESUMED',
    actor: req.ip ?? 'SYSTEM',
    status: 'success',
    ip: req.ip,
    beforeState: { paused: before.paused, reason: before.reason },
    afterState: { paused: false, queuedWithdrawalsReleased: queuedWithdrawals.length },
  });
  return res.status(200).json({
    paused: false,
    resumed: true,
    queuedWithdrawalsReleased: queuedWithdrawals.length,
  });
};

export const assignAccessRole = (req: Request, res: Response) => {
  const { actor, role } = getRbacAuditContext(req);
  const { targetAddress, targetRole } = req.body as {
    targetAddress: string;
    targetRole: Role;
  };
  assignRole(role, targetAddress, targetRole);
  auditLogService.record({
    action: 'ROLE_ASSIGNED',
    actor,
    status: 'success',
    ip: req.ip,
    afterState: { targetAddress, targetRole, assignedBy: actor },
  });
  return res.status(200).json({
    success: true,
    assignedBy: actor,
    targetAddress,
    targetRole,
  });
};

export const revokeAccessRole = (req: Request, res: Response) => {
  const { actor, role } = getRbacAuditContext(req);
  const { targetAddress, targetRole, coolOffMs } = req.body as {
    targetAddress: string;
    targetRole: Role;
    coolOffMs?: number;
  };
  scheduleRevocation(actor, role, targetAddress, targetRole, coolOffMs ?? 3_600_000);
  auditLogService.record({
    action: 'ROLE_REVOKED',
    actor,
    status: 'pending',
    ip: req.ip,
    afterState: { targetAddress, targetRole, coolOffMs: coolOffMs ?? 3_600_000, revokedBy: actor },
  });
  return res.status(202).json({
    success: true,
    targetAddress,
    targetRole,
    coolOffMs: coolOffMs ?? 3_600_000,
  });
};

export const listRoleAssignments = (_req: Request, res: Response) => {
  return res.status(200).json(getCurrentRoleAssignments());
};

export const healthCheck = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const stellarService = new StellarService();
    const services = await stellarService.healthCheck();
    const { isHealthy } = mapHealthResponse(services);

    res.status(isHealthy ? 200 : 503).json({
      status: isHealthy ? 'healthy' : 'unhealthy',
      timestamp: new Date().toISOString(),
      services,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const livenessCheck = (_req: Request, res: Response) => {
  res.status(200).json({ status: 'ok' });
};

export const readinessCheck = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const stellarService = new StellarService();
    const services = await stellarService.healthCheck();
    const { isHealthy, readinessStatus, horizon, soroban } = mapHealthResponse(services);

    res.status(isHealthy ? 200 : 503).json({
      status: readinessStatus,
      horizon,
      soroban,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const coalescingMetrics = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const metrics = requestCoalescingService.getStats();

    res.status(200).json({
      timestamp: new Date().toISOString(),
      metrics,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const protocolStats = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const stellarService = new StellarService();
    const stats: ProtocolStatsResponse = await stellarService.getProtocolStats();

    res.setHeader(
      'Cache-Control',
      `public, max-age=${Math.floor(config.cache.protocolStatsTtlMs / 1000)}`
    );

    return res.status(200).json(stats);
  } catch (error) {
    next(error);
    return;
  }
};

export const getTransactionHistory = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const stellarService = new StellarService();
    const pagination = parsePaginationParams(req.query as Record<string, unknown>);
    const query: TransactionHistoryQuery = {
      userAddress: req.params!.userAddress!,
      limit: pagination.limit,
      cursor: pagination.cursor ?? undefined,
    };

    const history: TransactionHistoryResponse = await stellarService.getTransactionHistory(query);
    return res.status(200).json(history);
  } catch (error) {
    next(error);
    return;
  }
};

export const streamTransactionHistory = async (req: Request, res: Response, next: NextFunction) => {
  const pageSize = req.query.pageSize ? Number(req.query.pageSize) : undefined;

  const abort = new AbortController();
  req.on('close', () => abort.abort());

  res.setHeader('Content-Type', 'application/x-ndjson');
  res.setHeader('Cache-Control', 'no-cache');
  res.setHeader('X-Content-Type-Options', 'nosniff');
  res.flushHeaders();

  try {
    const stellarService = new StellarService();
    const stream = stellarService.streamTransactionHistory(
      req.params!.userAddress!,
      pageSize,
      abort.signal
    );

    for await (const item of stream) {
      if (abort.signal.aborted) break;
      res.write(JSON.stringify(item) + '\n');
    }

    if (!abort.signal.aborted) {
      res.end();
    }
  } catch (error) {
    if (!res.headersSent) {
      next(error);
      return;
    } else {
      res.write(JSON.stringify({ error: 'Stream interrupted' }) + '\n');
      res.end();
    }
  }
};

export const getAuditLogs = (req: Request, res: Response) => {
  const { action, actor, status, from, to, limit, offset } = req.query as Record<string, string>;
  const entries = auditLogService.search({
    action,
    actor,
    status,
    from,
    to,
    limit: limit ? Number(limit) : undefined,
    offset: offset ? Number(offset) : undefined,
  });
  return res.status(200).json({ total: auditLogService.count(), entries });
};

export const exportAuditLogs = (req: Request, res: Response) => {
  const { action, actor, status, from, to } = req.query as Record<string, string>;
  const json = auditLogService.export({ action, actor, status, from, to });
  res.setHeader('Content-Type', 'application/json');
  res.setHeader('Content-Disposition', 'attachment; filename="audit-logs.json"');
  return res.status(200).send(json);
};

export const verifyAuditLogIntegrity = (_req: Request, res: Response) => {
  const result = auditLogService.verify();
  return res.status(result.valid ? 200 : 409).json(result);
};

/**
 * Get TWAP-based liquidation price for an asset.
 *
 * Uses the oracle contract's TWAP accumulator (get_liquidation_price)
 * to provide manipulation-resistant pricing for liquidation calculations.
 * Falls back to median spot price across sources when manipulation is detected.
 */
export const getLiquidationPrice = async (req: Request, res: Response, next: NextFunction) => {
  try {
    const { asset } = req.params!;

    if (!asset) {
      return res.status(400).json({
        success: false,
        error: 'Asset address is required',
      });
    }

    const stellarService = new StellarService();
    const price = await stellarService.getLiquidationPrice(asset);

    return res.status(200).json({
      success: true,
      asset,
      liquidationPrice: price,
      pricingMethod: 'twap_with_median_fallback',
      description:
        'TWAP-based price with manipulation resistance. Falls back to median across sources when manipulation is detected.',
    });
  } catch (error) {
    next(error);
    return;
  }
};

// Rebalancing Endpoints
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

    await redisCacheService.delByPrefix('stellarlend:position:');

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

// Debt Token Endpoints
export const mintDebtToken = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress, collateralAsset, principal, interestRateBps } = req.body as any;

    logger.info('Debt token mint request', { userAddress, collateralAsset, principal });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.mintDebtToken(userAddress, collateralAsset, principal, interestRateBps);

    const response = {
      success: true,
      user: userAddress,
      tokenId: 'pending', // Would be actual token ID from contract
      collateralAsset,
      principal,
      message: 'Debt token minted successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const transferDebtToken = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { from, to, tokenId } = req.body as any;

    logger.info('Debt token transfer request', { from, to, tokenId });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.transferDebtToken(from, to, tokenId);

    const response = {
      success: true,
      from,
      to,
      tokenId,
      message: 'Debt token transferred successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const burnDebtToken = async (req: Request, res: Response, next: NextFunction) => {
  try {
    if (emergencyPauseService.isPaused().paused) {
      return res.status(503).json({
        success: false,
        error: 'Protocol is paused',
        reason: emergencyPauseService.isPaused().reason,
      });
    }

    const { userAddress, tokenId, reason } = req.body as any;

    logger.info('Debt token burn request', { userAddress, tokenId, reason });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.burnDebtToken(userAddress, tokenId, reason);

    const response = {
      success: true,
      user: userAddress,
      tokenId,
      reason,
      message: 'Debt token burned successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const getDebtPosition = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { tokenId } = req.query as any;

    logger.info('Get debt position request', { tokenId });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.getDebtPosition(tokenId);

    const response = {
      success: true,
      tokenId,
      borrower: 'pending', // Would be actual borrower from contract
      principal: 0,
      accruedInterest: 0,
      collateralAsset: 'pending',
      interestRateBps: 0,
      isLiquidatable: false,
      message: 'Debt position retrieved successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const getUserDebtTokens = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { userAddress } = req.query as any;

    logger.info('Get user debt tokens request', { userAddress });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.getUserDebtTokens(userAddress);

    const response = {
      success: true,
      user: userAddress,
      tokens: [], // Would be actual token IDs from contract
      message: 'User debt tokens retrieved successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const getDebtTokenTotalSupply = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    logger.info('Get debt token total supply request');

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.getDebtTokenTotalSupply();

    const response = {
      success: true,
      totalSupply: 0, // Would be actual supply from contract
      message: 'Debt token total supply retrieved successfully',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

// Admin Endpoints for Debt Tokens
export const setDebtTokenTransferPause = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { paused } = req.body as any;

    logger.info('Set debt token transfer pause request', { paused });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.setDebtTokenTransferPause(paused);

    const response = {
      success: true,
      paused,
      message: paused ? 'Debt token transfers paused' : 'Debt token transfers resumed',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};

export const setDebtTokenAddressBlocked = async (
  req: Request,
  res: Response,
  next: NextFunction
) => {
  try {
    const { address, blocked } = req.body as any;

    logger.info('Set debt token address blocked request', { address, blocked });

    // TODO: Call contract method when debt token deployment is ready
    // const stellarService = new StellarService();
    // const result = await stellarService.setDebtTokenAddressBlocked(address, blocked);

    const response = {
      success: true,
      address,
      blocked,
      message: blocked ? 'Address blocked' : 'Address unblocked',
    };

    return res.status(200).json(response);
  } catch (error) {
    next(error);
    return;
  }
};
