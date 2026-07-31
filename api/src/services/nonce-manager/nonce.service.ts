import { redisCacheService } from '../redisCache.service';
import logger from '../../utils/logger';
import type { NonceState, PendingNonce } from '../../types/nonce';

interface NonceAllocation {
  address: string;
  nonce: string;
  allocatedAt: number;
  status: 'pending' | 'confirmed' | 'failed';
  txHash?: string;
}

const NONCE_KEY_PREFIX = 'nonce';
const PENDING_NONCES_KEY_PREFIX = 'pending_nonces';
const ALLOCATION_LOCK_TTL = 30; // seconds

class NonceManager {
  private locks = new Map<string, Promise<void>>();

  async getNonceState(address: string): Promise<NonceState> {
    const cacheKey = this.buildKey(address);
    const cached = await redisCacheService.get<NonceState>(cacheKey);
    if (cached) return cached;

    const state: NonceState = {
      address,
      currentNonce: '0',
      nextNonce: '0',
      pendingNonces: [],
      failedNonces: [],
      gaps: [],
      lastUpdated: Date.now(),
    };

    await redisCacheService.set(cacheKey, state, 3600);
    return state;
  }

  async allocateNonce(address: string): Promise<{ nonce: string; allocatedAt: number }> {
    await this.acquireLock(address);
    try {
      const state = await this.getNonceState(address);
      const nextNonce = BigInt(state.nextNonce) + 1n;
      const nonceStr = nextNonce.toString();

      const allocation: NonceAllocation = {
        address,
        nonce: nonceStr,
        allocatedAt: Date.now(),
        status: 'pending',
      };

      const pendingKey = this.buildPendingKey(address);
      const pending = await redisCacheService.get<PendingNonce[]>(pendingKey) || [];
      pending.push({
        nonce: nonceStr,
        allocatedAt: allocation.allocatedAt,
        status: 'pending',
      });

      state.nextNonce = nonceStr;
      state.pendingNonces.push({
        nonce: nonceStr,
        allocatedAt: allocation.allocatedAt,
        status: 'pending',
      });
      state.lastUpdated = Date.now();

      await redisCacheService.set(this.buildKey(address), state, 3600);
      await redisCacheService.set(pendingKey, pending, 3600);

      logger.info('Nonce allocated', { address, nonce: nonceStr });
      return { nonce: nonceStr, allocatedAt: allocation.allocatedAt };
    } finally {
      this.releaseLock(address);
    }
  }

  async confirmNonce(
    address: string,
    nonce: string,
    txHash: string
  ): Promise<void> {
    await this.acquireLock(address);
    try {
      const state = await this.getNonceState(address);
      const pending = state.pendingNonces.find((p) => p.nonce === nonce);

      if (pending) {
        pending.status = 'confirmed';
        pending.txHash = txHash;
      }

      state.currentNonce = nonce;
      state.lastUpdated = Date.now();

      await redisCacheService.set(this.buildKey(address), state, 3600);
      logger.info('Nonce confirmed', { address, nonce, txHash });
    } finally {
      this.releaseLock(address);
    }
  }

  async recoverNonce(address: string, failedNonce: string): Promise<{ recovered: boolean }> {
    await this.acquireLock(address);
    try {
      const state = await this.getNonceState(address);
      const pendingIdx = state.pendingNonces.findIndex((p) => p.nonce === failedNonce);

      if (pendingIdx >= 0) {
        const pending = state.pendingNonces[pendingIdx];
        if (pending) pending.status = 'failed';
      }

      const currentBig = BigInt(state.currentNonce);
      const failedBig = BigInt(failedNonce);

      if (failedBig > currentBig) {
        state.gaps.push({
          start: currentBig.toString(),
          end: (failedBig - 1n).toString(),
          reason: 'transaction_failed',
          filledAt: undefined,
        });
      }

      state.lastUpdated = Date.now();
      await redisCacheService.set(this.buildKey(address), state, 3600);

      logger.info('Nonce recovered', { address, nonce: failedNonce });
      return { recovered: true };
    } finally {
      this.releaseLock(address);
    }
  }

  async fillGaps(address: string): Promise<{ filled: number }> {
    await this.acquireLock(address);
    try {
      const state = await this.getNonceState(address);
      let filled = 0;

      for (const gap of state.gaps) {
        if (!gap.filledAt) {
          gap.filledAt = Date.now();
          filled++;
        }
      }

      state.gaps = state.gaps.filter((g) => g.filledAt === undefined || Date.now() - g.filledAt > 300000); // Keep 5 min history

      state.lastUpdated = Date.now();
      await redisCacheService.set(this.buildKey(address), state, 3600);

      if (filled > 0) {
        logger.info('Gaps filled', { address, count: filled });
      }
      return { filled };
    } finally {
      this.releaseLock(address);
    }
  }

  async getPendingNonces(address: string): Promise<PendingNonce[]> {
    const state = await this.getNonceState(address);
    return state.pendingNonces;
  }

  async getNextNonce(address: string): Promise<string> {
    const state = await this.getNonceState(address);
    return (BigInt(state.nextNonce) + 1n).toString();
  }

  private buildKey(address: string): string {
    return `${NONCE_KEY_PREFIX}:${address}`;
  }

  private buildPendingKey(address: string): string {
    return `${PENDING_NONCES_KEY_PREFIX}:${address}`;
  }

  private async acquireLock(address: string): Promise<void> {
    const lockKey = `${address}_lock`;
    if (this.locks.has(lockKey)) {
      await this.locks.get(lockKey);
    }

    const lockPromise = new Promise<void>((resolve) => {
      setTimeout(resolve, 0);
    });
    this.locks.set(lockKey, lockPromise);

    await lockPromise;
  }

  private releaseLock(address: string): void {
    const lockKey = `${address}_lock`;
    this.locks.delete(lockKey);
  }
}

export const nonceManager = new NonceManager();
