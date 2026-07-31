/**
 * Claims Service
 *
 * Top-level facade for the insurance claims automation pipeline.
 *
 * Flow:
 *   submitClaim()
 *     → fraud detection
 *     → oracle verification  (ClaimVerifier)
 *     → payout calculation   (PayoutCalculator)
 *     → persistence          (ClaimRepository)
 *
 *   processPayout()          — register approved payout on-chain
 *   openDispute()            — delegate to DisputeManager
 *   resolveDispute()         — admin override
 *   getClaimHistory()        — full audit trail
 *   getStats()               — aggregate metrics
 */

import type { PriceAggregator } from '@/services/price-aggregator';
import type {
  ClaimSubmissionRequest,
  InsuranceClaim,
  ClaimHistoryEntry,
  DisputeRecord,
  ClaimsStats,
  ClaimsServiceConfig,
} from './types';
import { ClaimStatus, RejectionReason, DisputeResolution } from './types';
import {
  ClaimRepository,
  createClaimRepository,
} from './claim-repository';
import {
  ClaimVerifier,
  createClaimVerifier,
} from './claim-verifier';
import {
  PayoutCalculator,
  createPayoutCalculator,
} from './payout-calculator';
import {
  FraudDetector,
  createFraudDetector,
} from './fraud-detector';
import {
  DisputeManager,
  createDisputeManager,
  DisputeError,
} from './dispute-manager';
import { logger } from '@/utils/logger';

/**
 * Default service configuration.
 */
const DEFAULT_CONFIG: ClaimsServiceConfig = {
  maxClaimsInMemory: 10_000,
  minOracleConfidence: 80,
  maxPriceAgeSeconds: 300,
  deductiblePercent: 5,
  velocityThreshold: 3,
  velocityWindowSeconds: 3600,
  minCoverageAgeSeconds: 300,
  amountAnomalyMultiplier: 5,
  fallbackToPendingManual: true,
};

/**
 * Result of a claim submission.
 */
export interface SubmitClaimResult {
  claim: InsuranceClaim;
  /** true if the claim was auto-approved (oracle verified, no fraud) */
  autoApproved: boolean;
  /** true if the claim was flagged as fraudulent and rejected */
  fraudRejected: boolean;
  /** true if the oracle was unavailable and claim is pending manual review */
  pendingManual: boolean;
}

/**
 * Insurance Claims Service.
 */
export class ClaimsService {
  private repository: ClaimRepository;
  private verifier: ClaimVerifier;
  private calculator: PayoutCalculator;
  private fraudDetector: FraudDetector;
  private disputeManager: DisputeManager;
  private config: ClaimsServiceConfig;

  /** Tracks verification durations for stats. */
  private verificationDurationsMs: number[] = [];

  constructor(aggregator: PriceAggregator, config: Partial<ClaimsServiceConfig> = {}) {
    this.config = { ...DEFAULT_CONFIG, ...config };

    this.repository = createClaimRepository({ maxEntries: this.config.maxClaimsInMemory });

    this.verifier = createClaimVerifier(aggregator, {
      maxPriceAgeSeconds: this.config.maxPriceAgeSeconds,
      minOracleConfidence: this.config.minOracleConfidence,
    });

    this.calculator = createPayoutCalculator({
      deductiblePercent: this.config.deductiblePercent,
      minOracleConfidence: this.config.minOracleConfidence,
    });

    this.fraudDetector = createFraudDetector({
      velocityThreshold: this.config.velocityThreshold,
      velocityWindowSeconds: this.config.velocityWindowSeconds,
      minCoverageAgeSeconds: this.config.minCoverageAgeSeconds,
      amountAnomalyMultiplier: this.config.amountAnomalyMultiplier,
    });

    this.disputeManager = createDisputeManager(this.repository);

    logger.info('ClaimsService initialized', {
      maxClaimsInMemory: this.config.maxClaimsInMemory,
      fallbackToPendingManual: this.config.fallbackToPendingManual,
    });
  }

  // ── Submit Claim ────────────────────────────────────────────────────────────

  /**
   * Submit a new insurance claim and run the automated verification pipeline.
   */
  async submitClaim(request: ClaimSubmissionRequest): Promise<SubmitClaimResult> {
    logger.info('Processing claim submission', {
      claimant: request.claimantAddress,
      asset: request.asset,
      amount: request.claimedAmount.toString(),
    });

    // 1. Persist the claim in PENDING state
    const claim = this.repository.create(request);

    // 2. Fraud detection
    this.repository.transition(
      claim.id, ClaimStatus.VERIFYING, 'system', 'Starting fraud analysis and oracle verification'
    );

    const allClaims = this.repository.getAll();
    const fraudResult = this.fraudDetector.detect(claim, allClaims.filter((c) => c.id !== claim.id));

    if (fraudResult.signals.length > 0) {
      claim.fraudSignals = fraudResult.signals;
      this.repository.save(claim);
    }

    if (fraudResult.isFraudulent) {
      this.repository.transition(
        claim.id,
        ClaimStatus.REJECTED,
        'system',
        `Fraud detected (risk score: ${fraudResult.riskScore})`,
        { signals: fraudResult.signals.map((s) => s.type), riskScore: fraudResult.riskScore }
      );

      const updated = this.repository.findById(claim.id)!;
      updated.rejectionReason = RejectionReason.FRAUD_DETECTED;
      updated.rejectionDetails = `Fraud risk score ${fraudResult.riskScore}/100. Signals: ${fraudResult.signals.map((s) => s.type).join(', ')}`;
      this.repository.save(updated);

      return { claim: updated, autoApproved: false, fraudRejected: true, pendingManual: false };
    }

    // 3. Oracle verification
    const verifyStart = Date.now();
    const verificationResult = await this.verifier.verify(claim);
    this.verificationDurationsMs.push(Date.now() - verifyStart);

    const freshClaim = this.repository.findById(claim.id)!;
    freshClaim.verificationResult = verificationResult;
    this.repository.save(freshClaim);

    if (!verificationResult.isValid) {
      const isOracleUnavailable = verificationResult.errors.some(
        (e) => e.code === 'ORACLE_UNAVAILABLE' || e.code === 'PRICE_STALE'
      );

      if (isOracleUnavailable && this.config.fallbackToPendingManual) {
        this.repository.transition(
          claim.id,
          ClaimStatus.PENDING_MANUAL,
          'system',
          'Oracle unavailable — claim requires manual review',
          { errors: verificationResult.errors.map((e) => e.code) }
        );
        return {
          claim: this.repository.findById(claim.id)!,
          autoApproved: false,
          fraudRejected: false,
          pendingManual: true,
        };
      }

      // Determine rejection reason
      const firstError = verificationResult.errors[0];
      const rejectionReason = this.errorCodeToRejectionReason(firstError?.code);

      this.repository.transition(
        claim.id,
        ClaimStatus.REJECTED,
        'system',
        `Verification failed: ${firstError?.message ?? 'unknown'}`,
        { errors: verificationResult.errors }
      );

      const rejected = this.repository.findById(claim.id)!;
      rejected.rejectionReason = rejectionReason;
      rejected.rejectionDetails = verificationResult.errors.map((e) => e.message).join('; ');
      this.repository.save(rejected);

      return { claim: rejected, autoApproved: false, fraudRejected: false, pendingManual: false };
    }

    // 4. Calculate payout
    const payoutResult = this.calculator.calculate(freshClaim, verificationResult.oracleData!);
    freshClaim.payoutResult = payoutResult;
    this.repository.save(freshClaim);

    // 5. Auto-approve
    this.repository.transition(
      claim.id,
      ClaimStatus.APPROVED,
      'system',
      `Oracle verification passed. Payout: ${payoutResult.netPayoutAmount} ${freshClaim.asset}`,
      { netPayoutAmount: payoutResult.netPayoutAmount.toString() }
    );

    return {
      claim: this.repository.findById(claim.id)!,
      autoApproved: true,
      fraudRejected: false,
      pendingManual: false,
    };
  }

  // ── Payout ──────────────────────────────────────────────────────────────────

  /**
   * Mark an approved claim as PAID_OUT.
   * In a full integration this would call ContractUpdater; here it records
   * the transaction hash supplied by the caller after on-chain dispatch.
   *
   * @param claimId     - ID of the APPROVED claim.
   * @param txHash      - On-chain transaction hash of the payout.
   * @param adminAddress - Admin or system address initiating the payout.
   */
  processPayout(claimId: string, txHash: string, adminAddress: string): InsuranceClaim {
    const claim = this.repository.findById(claimId);
    if (!claim) throw new Error(`Claim '${claimId}' not found`);
    if (claim.status !== ClaimStatus.APPROVED) {
      throw new Error(
        `Claim '${claimId}' must be APPROVED to process payout, got '${claim.status}'`
      );
    }

    claim.payoutTransactionHash = txHash;
    this.repository.save(claim);

    this.repository.transition(
      claimId,
      ClaimStatus.PAID_OUT,
      adminAddress,
      `Payout dispatched on-chain`,
      { txHash, netPayoutAmount: claim.payoutResult?.netPayoutAmount.toString() }
    );

    logger.info('Payout processed', { claimId, txHash, admin: adminAddress });
    return this.repository.findById(claimId)!;
  }

  // ── Re-verification ──────────────────────────────────────────────────────────

  /**
   * Re-run oracle verification on a PENDING or PENDING_MANUAL claim
   * (e.g. after oracle comes back online).
   */
  async verifyClaim(claimId: string): Promise<InsuranceClaim> {
    const claim = this.repository.findById(claimId);
    if (!claim) throw new Error(`Claim '${claimId}' not found`);

    const allowedStatuses = [ClaimStatus.PENDING, ClaimStatus.PENDING_MANUAL, ClaimStatus.VERIFYING];
    if (!allowedStatuses.includes(claim.status)) {
      throw new Error(
        `Claim '${claimId}' cannot be re-verified in status '${claim.status}'`
      );
    }

    // Transition back to VERIFYING
    this.repository.transition(claim.id, ClaimStatus.VERIFYING, 'system', 'Re-verification triggered');

    const verificationResult = await this.verifier.verify(claim);
    claim.verificationResult = verificationResult;
    this.repository.save(claim);

    if (!verificationResult.isValid) {
      if (this.config.fallbackToPendingManual) {
        this.repository.transition(claim.id, ClaimStatus.PENDING_MANUAL, 'system', 'Oracle still unavailable');
      } else {
        this.repository.transition(claim.id, ClaimStatus.REJECTED, 'system', 'Re-verification failed');
        claim.rejectionReason = this.errorCodeToRejectionReason(verificationResult.errors[0]?.code);
        this.repository.save(claim);
      }
    } else {
      const payout = this.calculator.calculate(claim, verificationResult.oracleData!);
      claim.payoutResult = payout;
      this.repository.save(claim);
      this.repository.transition(claim.id, ClaimStatus.APPROVED, 'system', 'Re-verification passed');
    }

    return this.repository.findById(claimId)!;
  }

  // ── Disputes ────────────────────────────────────────────────────────────────

  openDispute(
    claimId: string,
    disputantAddress: string,
    reason: string,
    evidence: string[] = []
  ): DisputeRecord {
    return this.disputeManager.openDispute(claimId, disputantAddress, reason, evidence);
  }

  resolveDispute(
    claimId: string,
    resolution: DisputeResolution,
    adminAddress: string,
    resolutionNotes?: string
  ): { claim: InsuranceClaim; dispute: DisputeRecord } {
    return this.disputeManager.resolveDispute(claimId, resolution, adminAddress, resolutionNotes);
  }

  getDispute(claimId: string): DisputeRecord | null {
    return this.disputeManager.getDispute(claimId);
  }

  // ── History & Query ─────────────────────────────────────────────────────────

  getClaim(claimId: string): InsuranceClaim | null {
    return this.repository.findById(claimId);
  }

  getClaimsByAddress(address: string): InsuranceClaim[] {
    return this.repository.findByAddress(address);
  }

  getClaimHistory(claimId: string): ClaimHistoryEntry[] {
    return this.repository.getHistory(claimId);
  }

  getClaimsByStatus(status: ClaimStatus): InsuranceClaim[] {
    return this.repository.findByStatus(status);
  }

  // ── Stats ────────────────────────────────────────────────────────────────────

  getStats(): ClaimsStats {
    const all = this.repository.getAll();
    const byStatus = Object.fromEntries(
      Object.values(ClaimStatus).map((s) => [s, 0])
    ) as Record<ClaimStatus, number>;

    let totalPayoutAmount = 0n;
    let fraudCount = 0;

    for (const c of all) {
      byStatus[c.status] = (byStatus[c.status] ?? 0) + 1;
      if (c.payoutResult?.netPayoutAmount) {
        totalPayoutAmount += c.payoutResult.netPayoutAmount;
      }
      if (c.fraudSignals.length > 0) fraudCount++;
    }

    const avgVerificationMs =
      this.verificationDurationsMs.length > 0
        ? this.verificationDurationsMs.reduce((a, b) => a + b, 0) /
          this.verificationDurationsMs.length
        : 0;

    const disputeCount = this.disputeManager.getAllDisputes().length;

    return {
      totalClaims: all.length,
      byStatus,
      totalPayoutAmount,
      fraudDetectionRate: all.length > 0 ? fraudCount / all.length : 0,
      averageVerificationTimeMs: avgVerificationMs,
      disputeRate: all.length > 0 ? disputeCount / all.length : 0,
    };
  }

  // ── Private Helpers ─────────────────────────────────────────────────────────

  private errorCodeToRejectionReason(
    code: string | undefined
  ): RejectionReason {
    switch (code) {
      case 'ORACLE_UNAVAILABLE':
        return RejectionReason.ORACLE_PRICE_UNAVAILABLE;
      case 'PRICE_STALE':
        return RejectionReason.STALE_ORACLE_PRICE;
      case 'AMOUNT_EXCEEDS_COVERAGE':
        return RejectionReason.INSUFFICIENT_COVERAGE;
      case 'INVALID_AMOUNT':
        return RejectionReason.INVALID_AMOUNT;
      case 'UNSUPPORTED_ASSET':
        return RejectionReason.UNSUPPORTED_ASSET;
      case 'LOSS_BEFORE_COVERAGE':
      case 'LOSS_TIMESTAMP_IN_FUTURE':
        return RejectionReason.POLICY_NOT_ACTIVE;
      default:
        return RejectionReason.ORACLE_PRICE_UNAVAILABLE;
    }
  }
}

/**
 * Factory function.
 */
export function createClaimsService(
  aggregator: PriceAggregator,
  config?: Partial<ClaimsServiceConfig>
): ClaimsService {
  return new ClaimsService(aggregator, config);
}

export { DisputeError };
