/**
 * Fraud Detector
 *
 * Heuristic fraud detection for insurance claims.
 *
 * Checks:
 *   1. Velocity   — too many claims from same address in a sliding window.
 *   2. Amount anomaly — claim >> historical average for that asset.
 *   3. Suspicious timing — claim submitted very soon after coverage purchase.
 *   4. Duplicate claim — identical (address, asset, lossTimestamp) tuple.
 */
import type { InsuranceClaim, FraudDetectionResult } from './types.js';
/**
 * Fraud detector configuration.
 */
export interface FraudDetectorConfig {
    /** Max claims per address per window before VELOCITY fires. Default: 3 */
    velocityThreshold: number;
    /** Sliding window in seconds for velocity check. Default: 3600 */
    velocityWindowSeconds: number;
    /**
     * Min seconds between coverage purchase and claim submission.
     * Claims submitted faster than this trigger SUSPICIOUS_TIMING. Default: 300
     */
    minCoverageAgeSeconds: number;
    /**
     * Multiplier on per-asset average. Claims > (avg * multiplier) trigger
     * AMOUNT_ANOMALY. Default: 5
     */
    amountAnomalyMultiplier: number;
    /**
     * Minimum number of historical data points required before the amount
     * anomaly check is applied. Default: 3
     */
    anomalyMinDataPoints: number;
    /** Risk score threshold above which isFraudulent = true. Default: 60 */
    fraudRiskThreshold: number;
}
/**
 * Fraud detector.
 *
 * Receives the full current claim list so it can compute per-address
 * and per-asset statistics without an external database dependency.
 */
export declare class FraudDetector {
    private config;
    constructor(config?: Partial<FraudDetectorConfig>);
    /**
     * Analyse a claim for fraud signals.
     *
     * @param claim        - The claim being evaluated (not yet persisted).
     * @param allClaims    - All existing claims in the repository.
     */
    detect(claim: InsuranceClaim, allClaims: InsuranceClaim[]): FraudDetectionResult;
    private checkVelocity;
    private checkAmountAnomaly;
    private checkSuspiciousTiming;
    private checkDuplicate;
    private makeSignal;
    private highestSeverity;
}
/**
 * Factory function.
 */
export declare function createFraudDetector(config?: Partial<FraudDetectorConfig>): FraudDetector;
//# sourceMappingURL=fraud-detector.d.ts.map