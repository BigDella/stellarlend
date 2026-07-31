import { randomUUID } from 'crypto';
import type {
  MultiStepTransaction,
  TransactionStep,
  CreateTransactionRequest,
  ApproveStepRequest,
  RejectStepRequest,
} from '../types/transaction';
import { StellarService } from './stellar.service';
import logger from '../utils/logger';

const DEFAULT_TTL_SECONDS = 600; // 10 minutes
const MAX_STEPS = 10;

// In-memory store. Replace with Redis/DB in production.
const transactions = new Map<string, MultiStepTransaction>();

function now(): string {
  return new Date().toISOString();
}

function isExpired(tx: MultiStepTransaction): boolean {
  return Date.now() > new Date(tx.expiresAt).getTime();
}

export class TransactionBuilderService {
  private stellar = new StellarService();

  create(req: CreateTransactionRequest): MultiStepTransaction {
    if (!req.steps || req.steps.length === 0) {
      throw Object.assign(new Error('At least one step is required'), { status: 400 });
    }
    if (req.steps.length > MAX_STEPS) {
      throw Object.assign(new Error(`Maximum ${MAX_STEPS} steps allowed`), { status: 400 });
    }

    const txId = randomUUID();
    const ttl = req.ttlSeconds ?? DEFAULT_TTL_SECONDS;
    const timestamp = now();

    const steps: TransactionStep[] = req.steps.map((s, i) => ({
      stepId: randomUUID(),
      index: i,
      operation: s.operation,
      userAddress: req.userAddress,
      amount: s.amount,
      assetAddress: s.assetAddress,
      status: 'pending',
      createdAt: timestamp,
      updatedAt: timestamp,
    }));

    const tx: MultiStepTransaction = {
      txId,
      userAddress: req.userAddress,
      description: req.description ?? `Multi-step transaction (${steps.length} steps)`,
      steps,
      currentStepIndex: 0,
      status: 'building',
      metadata: {},
      createdAt: timestamp,
      updatedAt: timestamp,
      expiresAt: new Date(Date.now() + ttl * 1000).toISOString(),
    };

    transactions.set(txId, tx);
    logger.info('Multi-step transaction created', { txId, steps: steps.length });
    return tx;
  }

  async prepareStep(txId: string, stepId: string): Promise<MultiStepTransaction> {
    const tx = this.assertActive(txId);
    const step = this.findStep(tx, stepId);

    if (step.index !== tx.currentStepIndex) {
      throw Object.assign(new Error('Steps must be prepared in order'), { status: 400 });
    }
    if (step.status !== 'pending') {
      throw Object.assign(new Error(`Step is already in status: ${step.status}`), { status: 400 });
    }

    const unsignedXdr = await this.stellar.buildUnsignedTransaction(
      step.operation,
      step.userAddress,
      step.assetAddress,
      step.amount
    );

    step.unsignedXdr = unsignedXdr;
    step.status = 'approved'; // awaiting signature from client
    step.updatedAt = now();
    tx.status = 'pending_approval';
    tx.updatedAt = now();

    transactions.set(txId, tx);
    logger.info('Step prepared', { txId, stepId });
    return tx;
  }

  async approveStep(req: ApproveStepRequest): Promise<MultiStepTransaction> {
    const tx = this.assertActive(req.txId);
    const step = this.findStep(tx, req.stepId);

    if (step.status !== 'approved') {
      throw Object.assign(new Error('Step must be in approved status before execution'), {
        status: 400,
      });
    }

    step.signedXdr = req.signedXdr;
    step.status = 'executing';
    step.updatedAt = now();
    tx.status = 'executing';
    tx.updatedAt = now();
    transactions.set(req.txId, tx);

    try {
      const result = await this.stellar.submitTransaction(req.signedXdr);
      step.txHash = result.transactionHash;
      step.status = result.success ? 'completed' : 'failed';
      step.error = result.success ? undefined : result.error;
      step.updatedAt = now();

      if (result.success) {
        const nextIndex = step.index + 1;
        tx.currentStepIndex = nextIndex;
        tx.status = nextIndex >= tx.steps.length ? 'completed' : 'pending_approval';
      } else {
        tx.status = 'failed';
      }
    } catch (err: unknown) {
      step.status = 'failed';
      step.error = err instanceof Error ? err.message : String(err);
      step.updatedAt = now();
      tx.status = 'failed';
    }

    tx.updatedAt = now();
    transactions.set(req.txId, tx);
    logger.info('Step executed', { txId: req.txId, stepId: req.stepId, status: step.status });
    return tx;
  }

  rejectStep(req: RejectStepRequest): MultiStepTransaction {
    const tx = this.assertActive(req.txId);
    const step = this.findStep(tx, req.stepId);

    step.status = 'rejected';
    step.error = req.reason;
    step.updatedAt = now();
    tx.status = 'failed';
    tx.updatedAt = now();

    transactions.set(req.txId, tx);
    logger.info('Step rejected', { txId: req.txId, stepId: req.stepId });
    return tx;
  }

  getTransaction(txId: string): MultiStepTransaction {
    const tx = transactions.get(txId);
    if (!tx) {
      throw Object.assign(new Error('Transaction not found'), { status: 404 });
    }
    if (isExpired(tx) && tx.status !== 'completed' && tx.status !== 'failed') {
      this.markExpired(tx);
    }
    return tx;
  }

  listForUser(userAddress: string): MultiStepTransaction[] {
    const result: MultiStepTransaction[] = [];
    for (const tx of transactions.values()) {
      if (tx.userAddress !== userAddress) continue;
      if (isExpired(tx) && tx.status !== 'completed' && tx.status !== 'failed') {
        this.markExpired(tx);
      }
      result.push(tx);
    }
    return result.sort((a, b) => new Date(b.createdAt).getTime() - new Date(a.createdAt).getTime());
  }

  cleanupExpired(): number {
    let count = 0;
    const cutoff = Date.now() - 24 * 60 * 60 * 1000; // keep 24h of completed/failed
    for (const [txId, tx] of transactions.entries()) {
      const isOldTerminal =
        (tx.status === 'completed' || tx.status === 'failed') &&
        new Date(tx.updatedAt).getTime() < cutoff;
      if (isOldTerminal || (isExpired(tx) && tx.status === 'expired')) {
        transactions.delete(txId);
        count++;
      }
    }
    if (count > 0) logger.info('Cleaned up expired transactions', { count });
    return count;
  }

  private assertActive(txId: string): MultiStepTransaction {
    const tx = transactions.get(txId);
    if (!tx) throw Object.assign(new Error('Transaction not found'), { status: 404 });
    if (isExpired(tx)) {
      this.markExpired(tx);
      throw Object.assign(new Error('Transaction has expired'), { status: 410 });
    }
    if (tx.status === 'completed') {
      throw Object.assign(new Error('Transaction is already completed'), { status: 400 });
    }
    if (tx.status === 'failed') {
      throw Object.assign(new Error('Transaction has failed'), { status: 400 });
    }
    return tx;
  }

  private findStep(tx: MultiStepTransaction, stepId: string): TransactionStep {
    const step = tx.steps.find((s) => s.stepId === stepId);
    if (!step) throw Object.assign(new Error('Step not found'), { status: 404 });
    return step;
  }

  private markExpired(tx: MultiStepTransaction): void {
    tx.status = 'expired';
    tx.steps.forEach((s) => {
      if (s.status === 'pending' || s.status === 'approved') {
        s.status = 'expired';
        s.updatedAt = now();
      }
    });
    tx.updatedAt = now();
    transactions.set(tx.txId, tx);
  }
}

export const transactionBuilderService = new TransactionBuilderService();

// Periodic cleanup every 15 minutes
setInterval(() => transactionBuilderService.cleanupExpired(), 15 * 60 * 1000);
