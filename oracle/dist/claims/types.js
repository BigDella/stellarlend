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
export var ClaimStatus;
(function (ClaimStatus) {
    /** Claim submitted, awaiting oracle verification. */
    ClaimStatus["PENDING"] = "PENDING";
    /** Oracle verification in progress. */
    ClaimStatus["VERIFYING"] = "VERIFYING";
    /** Verified and awaiting payout processing. */
    ClaimStatus["APPROVED"] = "APPROVED";
    /** Claim rejected (failed verification, fraud detected, etc.). */
    ClaimStatus["REJECTED"] = "REJECTED";
    /** Payout dispatched to the claimant. */
    ClaimStatus["PAID_OUT"] = "PAID_OUT";
    /** Claim is under dispute review. */
    ClaimStatus["DISPUTED"] = "DISPUTED";
    /** Oracle temporarily unavailable — pending manual review. */
    ClaimStatus["PENDING_MANUAL"] = "PENDING_MANUAL";
    /** Claim cancelled by the claimant before resolution. */
    ClaimStatus["CANCELLED"] = "CANCELLED";
})(ClaimStatus || (ClaimStatus = {}));
/**
 * Reasons a claim can be rejected.
 */
export var RejectionReason;
(function (RejectionReason) {
    RejectionReason["FRAUD_DETECTED"] = "FRAUD_DETECTED";
    RejectionReason["ORACLE_PRICE_UNAVAILABLE"] = "ORACLE_PRICE_UNAVAILABLE";
    RejectionReason["INSUFFICIENT_COVERAGE"] = "INSUFFICIENT_COVERAGE";
    RejectionReason["INVALID_AMOUNT"] = "INVALID_AMOUNT";
    RejectionReason["UNSUPPORTED_ASSET"] = "UNSUPPORTED_ASSET";
    RejectionReason["STALE_ORACLE_PRICE"] = "STALE_ORACLE_PRICE";
    RejectionReason["CLAIM_EXPIRED"] = "CLAIM_EXPIRED";
    RejectionReason["DUPLICATE_CLAIM"] = "DUPLICATE_CLAIM";
    RejectionReason["POLICY_NOT_ACTIVE"] = "POLICY_NOT_ACTIVE";
})(RejectionReason || (RejectionReason = {}));
/**
 * Severity level of a fraud signal.
 */
export var FraudSeverity;
(function (FraudSeverity) {
    FraudSeverity["LOW"] = "LOW";
    FraudSeverity["MEDIUM"] = "MEDIUM";
    FraudSeverity["HIGH"] = "HIGH";
    FraudSeverity["CRITICAL"] = "CRITICAL";
})(FraudSeverity || (FraudSeverity = {}));
/**
 * Type of fraud detected.
 */
export var FraudSignalType;
(function (FraudSignalType) {
    FraudSignalType["VELOCITY"] = "VELOCITY";
    FraudSignalType["AMOUNT_ANOMALY"] = "AMOUNT_ANOMALY";
    FraudSignalType["SUSPICIOUS_TIMING"] = "SUSPICIOUS_TIMING";
    FraudSignalType["DUPLICATE_CLAIM"] = "DUPLICATE_CLAIM";
    FraudSignalType["BLACKLISTED_ADDRESS"] = "BLACKLISTED_ADDRESS";
})(FraudSignalType || (FraudSignalType = {}));
/**
 * How a dispute was resolved.
 */
export var DisputeResolution;
(function (DisputeResolution) {
    DisputeResolution["APPROVED"] = "APPROVED";
    DisputeResolution["REJECTED"] = "REJECTED";
    DisputeResolution["ESCALATED"] = "ESCALATED";
})(DisputeResolution || (DisputeResolution = {}));
/**
 * Verification error codes.
 */
export var VerificationErrorCode;
(function (VerificationErrorCode) {
    VerificationErrorCode["ORACLE_UNAVAILABLE"] = "ORACLE_UNAVAILABLE";
    VerificationErrorCode["PRICE_STALE"] = "PRICE_STALE";
    VerificationErrorCode["AMOUNT_EXCEEDS_COVERAGE"] = "AMOUNT_EXCEEDS_COVERAGE";
    VerificationErrorCode["INVALID_AMOUNT"] = "INVALID_AMOUNT";
    VerificationErrorCode["UNSUPPORTED_ASSET"] = "UNSUPPORTED_ASSET";
    VerificationErrorCode["LOSS_TIMESTAMP_IN_FUTURE"] = "LOSS_TIMESTAMP_IN_FUTURE";
    VerificationErrorCode["LOSS_BEFORE_COVERAGE"] = "LOSS_BEFORE_COVERAGE";
    VerificationErrorCode["LOW_ORACLE_CONFIDENCE"] = "LOW_ORACLE_CONFIDENCE";
})(VerificationErrorCode || (VerificationErrorCode = {}));
//# sourceMappingURL=types.js.map