/**
 * Insurance Claims Module — Type Definitions
 *
 * All domain types, enums, and interfaces for the automated
 * insurance claim lifecycle in StellarLend.
 */

// ─────────────────────────────────────────────────────────────────────────────
// Enumerations
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Lifecycle status of an insurance claim.
 */
export enum ClaimStatus {
  /** Claim submitted, awaiting oracle verification. */
  PENDING = 'PENDING',
  /** Oracle verification in progress. */
  VERIFYING = 'VERIFYING',
  /** Verified and awaiting payout processing. */
  APPROVED = 'APPROVED',
  /** Claim rejected (failed verification, fraud detected, etc.). */
  REJECTED = 'REJECTED',
  /** Payout dispatched to the claimant. */
  PAID_OUT = 'PAID_OUT',
  /** Claim is under dispute review. */
  DISPUTED = 'DISPUTED',
  /** Oracle temporarily unavailable — pending manual review. */
  PENDING_MANUAL = 'PENDING_MANUAL',
  /** Claim cancelled by the claimant before resolution. */
  CANCELLED = 'CANCELLED',
}

/**
 * Reasons a claim can be rejected.
 */
export enum RejectionReason {
  FRAUD_DETECTED = 'FRAUD_DETECTED',
  ORACLE_PRICE_UNAVAILABLE = 'ORACLE_PRICE_UNAVAILABLE',
  INSUFFICIENT_COVERAGE = 'INSUFFICIENT_COVERAGE',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  UNSUPPORTED_ASSET = 'UNSUPPORTED_ASSET',
  STALE_ORACLE_PRICE = 'STALE_ORACLE_PRICE',
  CLAIM_EXPIRED = 'CLAIM_EXPIRED',
  DUPLICATE_CLAIM = 'DUPLICATE_CLAIM',
  POLICY_NOT_ACTIVE = 'POLICY_NOT_ACTIVE',
}

/**
 * Severity level of a fraud signal.
 */
export enum FraudSeverity {
  LOW = 'LOW',
  MEDIUM = 'MEDIUM',
  HIGH = 'HIGH',
  CRITICAL = 'CRITICAL',
}

/**
 * Type of fraud detected.
 */
export enum FraudSignalType {
  VELOCITY = 'VELOCITY',
  AMOUNT_ANOMALY = 'AMOUNT_ANOMALY',
  SUSPICIOUS_TIMING = 'SUSPICIOUS_TIMING',
  DUPLICATE_CLAIM = 'DUPLICATE_CLAIM',
  BLACKLISTED_ADDRESS = 'BLACKLISTED_ADDRESS',
}

/**
 * How a dispute was resolved.
 */
export enum DisputeResolution {
  APPROVED = 'APPROVED',
  REJECTED = 'REJECTED',
  ESCALATED = 'ESCALATED',
}

// ─────────────────────────────────────────────────────────────────────────────
// Core Domain Types
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A submitted insurance claim.
 */
export interface InsuranceClaim {
  /** Unique claim identifier (UUID-like). */
  id: string;
  /** Stellar address of the claimant. */
  claimantAddress: string;
  /** Asset symbol the claim is denominated in (e.g. 'XLM', 'USDC'). */
  asset: string;
  /** Claimed loss amount in the asset's smallest unit (7 decimal places). */
  claimedAmount: bigint;
  /** Maximum coverage the policy provides in asset units. */
  coverageLimit: bigint;
  /** Description of the loss event. */
  lossDescription: string;
  /** Unix timestamp when the loss occurred. */
  lossTimestamp: number;
  /** Unix timestamp when the claim was submitted. */
  submittedAt: number;
  /** Unix timestamp when the coverage policy was purchased. */
  coveragePurchasedAt: number;
  /** Current lifecycle status. */
  status: ClaimStatus;
  /** Oracle verification snapshot (populated after verification). */
  verificationResult?: ClaimVerificationResult;
  /** Calculated payout (populated after APPROVED). */
  payoutResult?: PayoutResult;
  /** Rejection reason (populated after REJECTED). */
  rejectionReason?: RejectionReason;
  /** Rejection details (human-readable). */
  rejectionDetails?: string;
  /** Fraud signals detected (may be empty). */
  fraudSignals: FraudSignal[];
  /** Full audit trail of status transitions. */
  history: ClaimHistoryEntry[];
  /** On-chain transaction hash after payout (populated after PAID_OUT). */
  payoutTransactionHash?: string;
}

/**
 * Input required to submit a new claim.
 */
export interface ClaimSubmissionRequest {
  claimantAddress: string;
  asset: string;
  claimedAmount: bigint;
  coverageLimit: bigint;
  lossDescription: string;
  lossTimestamp: number;
  coveragePurchasedAt: number;
  /** Optional: external policy ID for reference. */
  policyId?: string;
}

// ─────────────────────────────────────────────────────────────────────────────
// Oracle Verification
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Data captured from oracle at the time of claim verification.
 */
export interface OracleVerificationData {
  /** Oracle price in 7-decimal fixed-point (e.g. 1_0000000 = 1.0 USD). */
  priceAtVerification: bigint;
  /** Unix timestamp of the oracle price. */
  priceTimestamp: number;
  /** Oracle confidence score (0–100). */
  confidence: number;
  /** Oracle data sources used. */
  sources: string[];
  /** Unix timestamp when verification was performed. */
  verifiedAt: number;
}

/**
 * Result of the oracle-based claim verification step.
 */
export interface ClaimVerificationResult {
  isValid: boolean;
  oracleData?: OracleVerificationData;
  errors: ClaimVerificationError[];
}

/**
 * A single verification failure.
 */
export interface ClaimVerificationError {
  code: VerificationErrorCode;
  message: string;
  details?: Record<string, unknown>;
}

/**
 * Verification error codes.
 */
export enum VerificationErrorCode {
  ORACLE_UNAVAILABLE = 'ORACLE_UNAVAILABLE',
  PRICE_STALE = 'PRICE_STALE',
  AMOUNT_EXCEEDS_COVERAGE = 'AMOUNT_EXCEEDS_COVERAGE',
  INVALID_AMOUNT = 'INVALID_AMOUNT',
  UNSUPPORTED_ASSET = 'UNSUPPORTED_ASSET',
  LOSS_TIMESTAMP_IN_FUTURE = 'LOSS_TIMESTAMP_IN_FUTURE',
  LOSS_BEFORE_COVERAGE = 'LOSS_BEFORE_COVERAGE',
  LOW_ORACLE_CONFIDENCE = 'LOW_ORACLE_CONFIDENCE',
}

// ─────────────────────────────────────────────────────────────────────────────
// Payout
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Detailed payout calculation breakdown.
 */
export interface PayoutResult {
  /** Gross claim amount in asset units (before caps / deductible). */
  grossAmount: bigint;
  /** Amount after applying the coverage cap. */
  cappedAmount: bigint;
  /** Deductible amount subtracted. */
  deductibleAmount: bigint;
  /** Final net payout in asset units. */
  netPayoutAmount: bigint;
  /** Oracle price used (7-decimal fixed-point). */
  oraclePriceUsed: bigint;
  /** Payout value in USD (7-decimal fixed-point). */
  usdValue: bigint;
  /** Confidence discount applied (0.0 – 1.0). */
  confidenceDiscount: number;
  /** Unix timestamp of calculation. */
  calculatedAt: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Fraud Detection
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A single fraud indicator.
 */
export interface FraudSignal {
  type: FraudSignalType;
  severity: FraudSeverity;
  message: string;
  details?: Record<string, unknown>;
  detectedAt: number;
}

/**
 * Aggregated fraud detection result.
 */
export interface FraudDetectionResult {
  isFraudulent: boolean;
  signals: FraudSignal[];
  /** Highest severity across all signals (undefined if no signals). */
  maxSeverity?: FraudSeverity;
  /** Overall fraud risk score (0–100). */
  riskScore: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// Disputes
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A dispute record attached to a claim.
 */
export interface DisputeRecord {
  claimId: string;
  /** Stellar address of the party opening the dispute. */
  disputantAddress: string;
  /** Human-readable reason for dispute. */
  reason: string;
  /** Supporting evidence (URLs, hashes, descriptions). */
  evidence: string[];
  /** Unix timestamp when dispute was opened. */
  openedAt: number;
  /** Resolution outcome (undefined while pending). */
  resolution?: DisputeResolution;
  /** Admin Stellar address who resolved the dispute. */
  resolvedBy?: string;
  /** Admin notes on resolution. */
  resolutionNotes?: string;
  /** Unix timestamp of resolution. */
  resolvedAt?: number;
}

// ─────────────────────────────────────────────────────────────────────────────
// History / Audit Trail
// ─────────────────────────────────────────────────────────────────────────────

/**
 * A single immutable audit entry in the claim's history.
 */
export interface ClaimHistoryEntry {
  /** Previous status (undefined for initial submission). */
  fromStatus?: ClaimStatus;
  /** New status after this transition. */
  toStatus: ClaimStatus;
  /** Actor that triggered the transition ('oracle', 'system', or a Stellar address). */
  actor: string;
  /** Human-readable description of the event. */
  description: string;
  /** Unix timestamp of the event. */
  timestamp: number;
  /** Optional structured metadata. */
  metadata?: Record<string, unknown>;
}

// ─────────────────────────────────────────────────────────────────────────────
// Service Configuration
// ─────────────────────────────────────────────────────────────────────────────

/**
 * Configuration for the ClaimsService.
 */
export interface ClaimsServiceConfig {
  /**
   * Maximum number of claims to keep in-memory before LRU eviction.
   * Default: 10_000
   */
  maxClaimsInMemory: number;
  /**
   * Oracle confidence threshold below which a low-confidence discount is applied.
   * Default: 80
   */
  minOracleConfidence: number;
  /**
   * How many seconds old an oracle price can be before it's considered stale.
   * Default: 300
   */
  maxPriceAgeSeconds: number;
  /**
   * Deductible percentage applied to payouts (0–100).
   * Default: 5
   */
  deductiblePercent: number;
  /**
   * Number of claims per address per window that triggers velocity fraud.
   * Default: 3
   */
  velocityThreshold: number;
  /**
   * Time window (seconds) for velocity check.
   * Default: 3600 (1 hour)
   */
  velocityWindowSeconds: number;
  /**
   * Minimum seconds between coverage purchase and claim before
   * "suspicious timing" fraud signal fires.
   * Default: 300 (5 minutes)
   */
  minCoverageAgeSeconds: number;
  /**
   * Multiplier on historical average — claims larger than
   * (averageAmount * anomalyMultiplier) trigger amount anomaly.
   * Default: 5
   */
  amountAnomalyMultiplier: number;
  /**
   * If true, oracle failures put the claim in PENDING_MANUAL rather than REJECTED.
   * Default: true
   */
  fallbackToPendingManual: boolean;
}

/**
 * Aggregate statistics reported by ClaimsService.getStats().
 */
export interface ClaimsStats {
  totalClaims: number;
  byStatus: Record<ClaimStatus, number>;
  totalPayoutAmount: bigint;
  fraudDetectionRate: number;
  averageVerificationTimeMs: number;
  disputeRate: number;
}
