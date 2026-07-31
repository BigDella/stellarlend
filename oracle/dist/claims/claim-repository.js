/**
 * Claim Repository
 *
 * In-memory store for insurance claims with LRU eviction and an
 * append-only audit trail per claim.
 */
import { randomUUID } from 'node:crypto';
import { ClaimStatus } from './types.js';
import { logger } from '../utils/logger.js';
const DEFAULT_CONFIG = {
    maxEntries: 10_000,
};
/**
 * In-memory insurance claim store.
 *
 * Uses a Map (insertion-order preserved) to track LRU position.
 * The first key in the Map is always the oldest / least-recently touched.
 */
export class ClaimRepository {
    store = new Map();
    config;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
        logger.info('ClaimRepository initialized', { maxEntries: this.config.maxEntries });
    }
    // ── Write Operations ────────────────────────────────────────────────────────
    /**
     * Create a new claim from a submission request.
     * Returns the persisted claim.
     */
    create(request) {
        const now = Math.floor(Date.now() / 1000);
        const id = randomUUID();
        const initialEntry = {
            toStatus: ClaimStatus.PENDING,
            actor: request.claimantAddress,
            description: 'Claim submitted by claimant',
            timestamp: now,
        };
        const claim = {
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
    save(claim) {
        this.persist(claim.id, claim);
    }
    /**
     * Append a history entry and persist.
     */
    appendHistory(claimId, entry) {
        const claim = this.findById(claimId);
        if (!claim)
            return null;
        claim.history.push(entry);
        this.persist(claimId, claim);
        return claim;
    }
    /**
     * Transition a claim to a new status, recording the history entry.
     */
    transition(claimId, toStatus, actor, description, metadata) {
        const claim = this.findById(claimId);
        if (!claim)
            return null;
        const entry = {
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
    findById(id) {
        const claim = this.store.get(id);
        if (!claim)
            return null;
        // Refresh LRU position
        this.store.delete(id);
        this.store.set(id, claim);
        return claim;
    }
    findByAddress(address) {
        return Array.from(this.store.values()).filter((c) => c.claimantAddress === address);
    }
    findByStatus(status) {
        return Array.from(this.store.values()).filter((c) => c.status === status);
    }
    /**
     * Return all claims submitted since a given Unix timestamp.
     */
    findSince(sinceTimestamp) {
        return Array.from(this.store.values()).filter((c) => c.submittedAt >= sinceTimestamp);
    }
    /**
     * Return the full audit history for a claim.
     */
    getHistory(claimId) {
        return this.findById(claimId)?.history ?? [];
    }
    count() {
        return this.store.size;
    }
    getAll() {
        return Array.from(this.store.values());
    }
    // ── Private Helpers ─────────────────────────────────────────────────────────
    persist(id, claim) {
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
export function createClaimRepository(config) {
    return new ClaimRepository(config);
}
//# sourceMappingURL=claim-repository.js.map