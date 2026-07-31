import { Router, Request, Response } from 'express';
import { nonceManager } from '../services/nonce-manager/nonce.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';
import type {
  NonceAllocationRequest,
  NonceRecoveryRequest,
  NonceStateResponse,
} from '../types/nonce';

const router = Router();

function validateAddress(address: string): void {
  if (!address || !/^G[A-Z0-9]{56}$/.test(address)) {
    throw new ValidationError('Invalid Stellar address format');
  }
}

router.get('/:address', async (req: Request, res: Response) => {
  try {
    const { address } = req.params!;
    validateAddress(address!);

    const state = await nonceManager.getNonceState(address!);
    const response: NonceStateResponse = {
      address: state.address,
      currentNonce: state.currentNonce,
      nextNonce: state.nextNonce,
      pendingNonceCount: state.pendingNonces.length,
      failedNonceCount: state.failedNonces.length,
      gapCount: state.gaps.length,
    };

    res.json(response);
  } catch (error) {
    logger.error('Error fetching nonce state:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to fetch nonce state' });
    }
  }
});

router.post('/next', async (req: Request, res: Response) => {
  try {
    const { address } = req.body as NonceAllocationRequest;
    validateAddress(address);

    const result = await nonceManager.allocateNonce(address);
    res.json({
      nonce: result.nonce,
      allocatedAt: result.allocatedAt,
    });
  } catch (error) {
    logger.error('Error allocating nonce:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to allocate nonce' });
    }
  }
});

router.post('/recover', async (req: Request, res: Response) => {
  try {
    const { address, nonce } = req.body as NonceRecoveryRequest;
    validateAddress(address);

    if (!nonce) {
      throw new ValidationError('Nonce is required');
    }

    const result = await nonceManager.recoverNonce(address, nonce);
    res.json(result);
  } catch (error) {
    logger.error('Error recovering nonce:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to recover nonce' });
    }
  }
});

router.post('/confirm', async (req: Request, res: Response) => {
  try {
    const { address, nonce, txHash } = req.body;
    validateAddress(address);

    if (!nonce || !txHash) {
      throw new ValidationError('Nonce and transaction hash are required');
    }

    await nonceManager.confirmNonce(address, nonce, txHash);
    res.json({ confirmed: true });
  } catch (error) {
    logger.error('Error confirming nonce:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to confirm nonce' });
    }
  }
});

router.post('/fill-gaps', async (req: Request, res: Response) => {
  try {
    const { address } = req.body as { address: string };
    validateAddress(address);

    const result = await nonceManager.fillGaps(address);
    res.json(result);
  } catch (error) {
    logger.error('Error filling gaps:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to fill gaps' });
    }
  }
});

router.get('/:address/pending', async (req: Request, res: Response) => {
  try {
    const { address } = req.params!;
    validateAddress(address!);

    const pending = await nonceManager.getPendingNonces(address!);
    res.json({ pending });
  } catch (error) {
    logger.error('Error fetching pending nonces:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to fetch pending nonces' });
    }
  }
});

router.get('/:address/next-available', async (req: Request, res: Response) => {
  try {
    const { address } = req.params!;
    validateAddress(address!);

    const nextNonce = await nonceManager.getNextNonce(address!);
    res.json({ nextNonce });
  } catch (error) {
    logger.error('Error fetching next nonce:', error);
    if (error instanceof ValidationError) {
      res.status(400).json({ error: error.message });
    } else {
      res.status(500).json({ error: 'Failed to fetch next nonce' });
    }
  }
});

export const nonceRouter = router;
