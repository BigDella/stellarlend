/**
 * Claim Repository
 *
 * In-memory store for insurance claims with LRU eviction and an
 * append-only audit trail per claim.
 */

import { randomUUID } from 'node:crypto';
import type {
  InsuranceClaim,
  ClaimHistoryEntry,
  ClaimSubmissionRequest,
} from './types.js';
import { ClaimStatus, FraudSignal } from './types.js';
import { logger } from '../utils/logger.js';

/**
 * Configuration for the repository.
 */
export interface ClaimRepositoryConfig {
  /** Maximum number of claims to hold before evicting oldest. Default: 10_000 */
  maxEntries: number;
}

const DEFAULT_CONFIG: ClaimRepositoryConfig = {
  maxEntries: 10_000,
};

/**
 * In-memory insurance claim store.
 *
 * Uses a Map (insertion-order preserved) to track LRU position.
 * The first key in the Map is always the oldest / least-recently touched.
 */
export class ClaimRepository {
  private store: Map<string, InsuranceClaim> = new Map();
  private config: ClaimRepositoryConfig;

  constructor(config: Partial<ClaimRepositoryConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };
    logger.info('ClaimRepository initialized', { maxEntries: this.config.maxEntries });
  }

  // ── Write Operations ────────────────────────────────────────────────────────

  /**
   * Create a new claim from a submission request.
   * Returns the persisted claim.
   */
  create(request: ClaimSubmissionRequest): InsuranceClaim {
    const now = Math.floor(Date.now() / 1000);
    const id = randomUUID();

    const initialEntry: ClaimHistoryEntry = {
      toStatus: ClaimStatus.PENDING,
      actor: request.claimantAddress,
      description: 'Claim submitted by claimant',
      timestamp: now,
    };

    const claim: InsuranceClaim = {
      id,
      claimantAddress: request.claimantAddress,
      asset: request.asset.toUpperCase(),
      claimedAmount: request.claimedAmount,
      coverageLimit: request.coverageLimit,
      lossDescription: request.lossDescription,
      lossTimestamp: request.lossTimestamp,
      submittedAt: now,
      coveragePurchasedAt: request.coveragePurchasedAt,
      status: ClaimStatus.PENDING,
      fraudSignals: [],
      history: [initialEntry],
    };

    this.persist(id, claim);
    logger.info('Claim created', { claimId: id, asset: claim.asset, claimant: claim.claimantAddress });
    return claim;
  }

  /**
   * Persist (insert or update) a claim.
   */
  save(claim: InsuranceClaim): void {
    this.persist(claim.id, claim);
  }

  /**
   * Append a history entry and persist.
   */
  appendHistory(claimId: string, entry: ClaimHistoryEntry): InsuranceClaim | null {
    const claim = this.findById(claimId);
    if (!claim) return null;

    claim.history.push(entry);
    this.persist(claimId, claim);
    return claim;
  }

  /**
   * Transition a claim to a new status, recording the history entry.
   */
  transition(
    claimId: string,
    toStatus: ClaimStatus,
    actor: string,
    description: string,
    metadata?: Record<string, unknown>
  ): InsuranceClaim | null {
    const claim = this.findById(claimId);
    if (!claim) return null;

    const entry: ClaimHistoryEntry = {
      fromStatus: claim.status,
      toStatus,
      actor,
      description,
      timestamp: Math.floor(Date.now() / 1000),
      metadata,
    };

    claim.history.push(entry);
    claim.status = toStatus;
    this.persist(claimId, claim);

    logger.info('Claim status transition', {
      claimId,
      from: entry.fromStatus,
      to: toStatus,
      actor,
    });

    return claim;
  }

  // ── Read Operations ─────────────────────────────────────────────────────────

  findById(id: string): InsuranceClaim | null {
    const claim = this.store.get(id);
    if (!claim) return null;

    // Refresh LRU position
    this.store.delete(id);
    this.store.set(id, claim);
    return claim;
  }

  findByAddress(address: string): InsuranceClaim[] {
    return Array.from(this.store.values()).filter(
      (c) => c.claimantAddress === address
    );
  }

  findByStatus(status: ClaimStatus): InsuranceClaim[] {
    return Array.from(this.store.values()).filter((c) => c.status === status);
  }

  /**
   * Return all claims submitted since a given Unix timestamp.
   */
  findSince(sinceTimestamp: number): InsuranceClaim[] {
    return Array.from(this.store.values()).filter(
      (c) => c.submittedAt >= sinceTimestamp
    );
  }

  /**
   * Return the full audit history for a claim.
   */
  getHistory(claimId: string): ClaimHistoryEntry[] {
    return this.findById(claimId)?.history ?? [];
  }

  count(): number {
    return this.store.size;
  }

  getAll(): InsuranceClaim[] {
    return Array.from(this.store.values());
  }

  // ── Private Helpers ─────────────────────────────────────────────────────────

  private persist(id: string, claim: InsuranceClaim): void {
    // Evict oldest entry if at capacity
    if (!this.store.has(id) && this.store.size >= this.config.maxEntries) {
      const oldestKey = this.store.keys().next().value;
      if (oldestKey) {
        this.store.delete(oldestKey);
        logger.debug('ClaimRepository: evicted oldest claim', { evicted: oldestKey });
      }
    }

    // Refresh LRU position for existing keys
    if (this.store.has(id)) {
      this.store.delete(id);
    }

    this.store.set(id, claim);
  }
}

/**
 * Factory function.
 */
export function createClaimRepository(
  config?: Partial<ClaimRepositoryConfig>
): ClaimRepository {
  return new ClaimRepository(config);
}
