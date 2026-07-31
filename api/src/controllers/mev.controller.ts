/**
 * MEV Protection Controller
 *
 * Endpoints:
 *  POST /mev/commit              — build unsigned commit transaction
 *  POST /mev/reveal              — build unsigned reveal transaction
 *  POST /mev/auction/bid         — build unsigned auction bid transaction
 *  POST /mev/auction/settle      — build unsigned auction settle transaction
 *  GET  /mev/auction/:slotId     — query settled auction result
 *  GET  /mev/auction/current     — current open auction slot
 *  GET  /mev/dashboard           — MEV extraction monitoring dashboard
 *  GET  /mev/gas-analysis        — gas price bidding analysis
 *  GET  /mev/route               — private mempool routing hint
 *  GET  /mev/fee-preview         — preview effective MEV fee
 */

import { Request, Response, NextFunction } from 'express';
import { mevService, SensitiveOperation, TxOrderingHint } from '../services/mev.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

// ─── Commit ───────────────────────────────────────────────────────────────────

export const buildCommit = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const {
      userAddress,
      operation,
      assetAddress,
      secondaryAssetAddress,
      borrowerAddress,
      amount,
      maxFeeBps,
      hint,
      maxSlippageBps,
      deadline,
    } = req.body as any;

    if (!userAddress || !operation || !amount || maxFeeBps === undefined) {
      throw new ValidationError('userAddress, operation, amount, and maxFeeBps are required');
    }

    const validOps: SensitiveOperation[] = ['Borrow', 'Withdraw', 'Liquidate'];
    if (!validOps.includes(operation)) {
      throw new ValidationError(`operation must be one of: ${validOps.join(', ')}`);
    }

    if (operation === 'Liquidate' && !borrowerAddress) {
      throw new ValidationError('borrowerAddress is required for Liquidate operations');
    }

    logger.info('Building MEV commit transaction', { userAddress, operation, amount });

    const result = await mevService.buildCommitTransaction({
      userAddress,
      operation,
      assetAddress,
      secondaryAssetAddress,
      borrowerAddress,
      amount: String(amount),
      maxFeeBps: Number(maxFeeBps),
      hint: (hint as TxOrderingHint) ?? 'Default',
      maxSlippageBps: maxSlippageBps !== undefined ? Number(maxSlippageBps) : undefined,
      deadline: deadline !== undefined ? Number(deadline) : undefined,
    });

    res.status(200).json({ success: true, data: result });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Reveal ───────────────────────────────────────────────────────────────────

export const buildReveal = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { userAddress, commitId, operation } = req.body as any;

    if (!userAddress || !commitId || !operation) {
      throw new ValidationError('userAddress, commitId, and operation are required');
    }

    logger.info('Building MEV reveal transaction', { userAddress, commitId, operation });

    const unsignedXdr = await mevService.buildRevealTransaction({
      userAddress,
      commitId: String(commitId),
      operation: operation as SensitiveOperation,
    });

    res.status(200).json({ success: true, data: { unsignedXdr, commitId, operation } });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Batch Auction ────────────────────────────────────────────────────────────

export const buildAuctionBid = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const {
      bidderAddress,
      borrowerAddress,
      debtAmount,
      minCollateralOut,
      maxFeeBps,
      deadline,
    } = req.body as any;

    if (!bidderAddress || !borrowerAddress || !debtAmount || !minCollateralOut || maxFeeBps === undefined) {
      throw new ValidationError(
        'bidderAddress, borrowerAddress, debtAmount, minCollateralOut, and maxFeeBps are required',
      );
    }

    logger.info('Building batch auction bid', { bidderAddress, borrowerAddress, debtAmount });

    const result = await mevService.buildPlaceAuctionBidTransaction({
      bidderAddress,
      borrowerAddress,
      debtAmount: String(debtAmount),
      minCollateralOut: String(minCollateralOut),
      maxFeeBps: Number(maxFeeBps),
      deadline: deadline !== undefined ? Number(deadline) : undefined,
    });

    res.status(200).json({ success: true, data: result });
  } catch (err) {
    next(err);
    return;
  }
};

export const buildSettleAuction = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { callerAddress, slotId } = req.body as any;

    if (!callerAddress || slotId === undefined) {
      throw new ValidationError('callerAddress and slotId are required');
    }

    logger.info('Building settle auction transaction', { callerAddress, slotId });

    const unsignedXdr = await mevService.buildSettleAuctionTransaction(
      callerAddress,
      String(slotId),
    );

    res.status(200).json({ success: true, data: { unsignedXdr, slotId } });
  } catch (err) {
    next(err);
    return;
  }
};

export const getAuctionResult = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { slotId } = req.params!;
    const result = await mevService.getAuctionResult(slotId!);

    if (!result) {
      res.status(404).json({ success: false, error: 'Auction slot not found or not yet settled' });
      return;
    }

    res.status(200).json({ success: true, data: result });
  } catch (err) {
    next(err);
    return;
  }
};

export const getCurrentAuctionSlot = async (
  _req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const slotId = await mevService.getCurrentAuctionSlot();
    res.status(200).json({ success: true, data: { currentSlotId: slotId.toString() } });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Monitoring Dashboard ─────────────────────────────────────────────────────

export const getDashboard = async (
  _req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const data = await mevService.getDashboard();
    res.status(200).json({ success: true, data });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Gas Bidding Analysis ─────────────────────────────────────────────────────

export const getGasBidAnalysis = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { operation, assetAddress, amount } = req.query as {
      operation?: string;
      assetAddress?: string;
      amount?: string;
    };

    const validOps: SensitiveOperation[] = ['Borrow', 'Withdraw', 'Liquidate'];
    const op: SensitiveOperation = validOps.includes(operation as SensitiveOperation)
      ? (operation as SensitiveOperation)
      : 'Borrow';

    const analysis = await mevService.getGasBidAnalysis(op, assetAddress, amount ?? '0');
    res.status(200).json({ success: true, data: analysis });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Private Mempool Routing ──────────────────────────────────────────────────

export const getPrivateMempoolRoute = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { operation, hint } = req.query as { operation?: string; hint?: string };

    const validOps: SensitiveOperation[] = ['Borrow', 'Withdraw', 'Liquidate'];
    const op: SensitiveOperation = validOps.includes(operation as SensitiveOperation)
      ? (operation as SensitiveOperation)
      : 'Borrow';

    const validHints: TxOrderingHint[] = ['Default', 'PrivateMempool', 'BatchAuction', 'DelayedReveal'];
    const requestedHint: TxOrderingHint = validHints.includes(hint as TxOrderingHint)
      ? (hint as TxOrderingHint)
      : 'Default';

    const route = await mevService.getPrivateMempoolRoute(op, requestedHint);
    res.status(200).json({ success: true, data: route });
  } catch (err) {
    next(err);
    return;
  }
};

// ─── Fee Preview ──────────────────────────────────────────────────────────────

export const getFeePreview = async (
  req: Request,
  res: Response,
  next: NextFunction,
): Promise<void> => {
  try {
    const { operation, assetAddress, amount } = req.query as {
      operation?: string;
      assetAddress?: string;
      amount?: string;
    };

    const validOps: SensitiveOperation[] = ['Borrow', 'Withdraw', 'Liquidate'];
    const op: SensitiveOperation = validOps.includes(operation as SensitiveOperation)
      ? (operation as SensitiveOperation)
      : 'Borrow';

    const feeBps = await mevService.previewFeeBps(op, assetAddress, amount ?? '0');
    res.status(200).json({
      success: true,
      data: {
        operation: op,
        assetAddress,
        amount: amount ?? '0',
        effectiveFeeBps: feeBps,
        effectiveFeePercent: (feeBps / 100).toFixed(2),
      },
    });
  } catch (err) {
    next(err);
    return;
  }
};
