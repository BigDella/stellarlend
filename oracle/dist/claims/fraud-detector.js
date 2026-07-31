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
import { FraudSeverity, FraudSignalType } from './types.js';
import { logger } from '../utils/logger.js';
const DEFAULT_CONFIG = {
    velocityThreshold: 3,
    velocityWindowSeconds: 3600,
    minCoverageAgeSeconds: 300,
    amountAnomalyMultiplier: 5,
    anomalyMinDataPoints: 3,
    fraudRiskThreshold: 60,
};
/** Scoring weights per signal severity (0–100 scale). */
const SEVERITY_SCORE = {
    [FraudSeverity.LOW]: 10,
    [FraudSeverity.MEDIUM]: 25,
    [FraudSeverity.HIGH]: 50,
    [FraudSeverity.CRITICAL]: 100,
};
/**
 * Fraud detector.
 *
 * Receives the full current claim list so it can compute per-address
 * and per-asset statistics without an external database dependency.
 */
export class FraudDetector {
    config;
    constructor(config = {}) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }
    /**
     * Analyse a claim for fraud signals.
     *
     * @param claim        - The claim being evaluated (not yet persisted).
     * @param allClaims    - All existing claims in the repository.
     */
    detect(claim, allClaims) {
        const signals = [];
        const now = Math.floor(Date.now() / 1000);
        // 1. Velocity check
        const velocitySignal = this.checkVelocity(claim, allClaims, now);
        if (velocitySignal)
            signals.push(velocitySignal);
        // 2. Amount anomaly check
        const anomalySignal = this.checkAmountAnomaly(claim, allClaims);
        if (anomalySignal)
            signals.push(anomalySignal);
        // 3. Suspicious timing check
        const timingSignal = this.checkSuspiciousTiming(claim, now);
        if (timingSignal)
            signals.push(timingSignal);
        // 4. Duplicate claim check
        const duplicateSignal = this.checkDuplicate(claim, allClaims);
        if (duplicateSignal)
            signals.push(duplicateSignal);
        // Aggregate risk score (capped at 100)
        const riskScore = Math.min(100, signals.reduce((sum, s) => sum + SEVERITY_SCORE[s.severity], 0));
        const maxSeverity = this.highestSeverity(signals);
        const isFraudulent = riskScore >= this.config.fraudRiskThreshold;
        if (isFraudulent) {
            logger.warn('Fraud detected for claim', {
                claimId: claim.id,
                claimant: claim.claimantAddress,
                riskScore,
                signals: signals.map((s) => s.type),
            });
        }
        else if (signals.length > 0) {
            logger.info('Fraud signals detected (below threshold)', {
                claimId: claim.id,
                riskScore,
                signals: signals.map((s) => s.type),
            });
        }
        return { isFraudulent, signals, maxSeverity, riskScore };
    }
    // ── Private Checks ──────────────────────────────────────────────────────────
    checkVelocity(claim, allClaims, now) {
        const windowStart = now - this.config.velocityWindowSeconds;
        const recentClaims = allClaims.filter((c) => c.claimantAddress === claim.claimantAddress &&
            c.submittedAt >= windowStart &&
            c.id !== claim.id);
        if (recentClaims.length >= this.config.velocityThreshold) {
            return this.makeSignal(FraudSignalType.VELOCITY, FraudSeverity.HIGH, `Address submitted ${recentClaims.length + 1} claims within ${this.config.velocityWindowSeconds}s`, { count: recentClaims.length + 1, windowSeconds: this.config.velocityWindowSeconds });
        }
        return null;
    }
    checkAmountAnomaly(claim, allClaims) {
        const assetClaims = allClaims.filter((c) => c.asset === claim.asset && c.id !== claim.id);
        if (assetClaims.length < this.config.anomalyMinDataPoints) {
            return null; // Not enough data to establish a baseline
        }
        const total = assetClaims.reduce((sum, c) => sum + c.claimedAmount, 0n);
        const avg = total / BigInt(assetClaims.length);
        const threshold = avg * BigInt(this.config.amountAnomalyMultiplier);
        if (claim.claimedAmount > threshold) {
            return this.makeSignal(FraudSignalType.AMOUNT_ANOMALY, FraudSeverity.MEDIUM, `Claim amount (${claim.claimedAmount}) is ${this.config.amountAnomalyMultiplier}× above asset average (${avg})`, {
                claimedAmount: claim.claimedAmount.toString(),
                assetAverage: avg.toString(),
                threshold: threshold.toString(),
            });
        }
        return null;
    }
    checkSuspiciousTiming(claim, now) {
        const coverageAge = claim.submittedAt - claim.coveragePurchasedAt;
        if (coverageAge < this.config.minCoverageAgeSeconds) {
            return this.makeSignal(FraudSignalType.SUSPICIOUS_TIMING, FraudSeverity.MEDIUM, `Claim submitted only ${coverageAge}s after coverage was purchased (min: ${this.config.minCoverageAgeSeconds}s)`, { coverageAge, minCoverageAgeSeconds: this.config.minCoverageAgeSeconds });
        }
        return null;
    }
    checkDuplicate(claim, allClaims) {
        const duplicate = allClaims.find((c) => c.id !== claim.id &&
            c.claimantAddress === claim.claimantAddress &&
            c.asset === claim.asset &&
            c.lossTimestamp === claim.lossTimestamp);
        if (duplicate) {
            return this.makeSignal(FraudSignalType.DUPLICATE_CLAIM, FraudSeverity.CRITICAL, `Duplicate claim detected — same claimant, asset, and loss timestamp as claim ${duplicate.id}`, { duplicateClaimId: duplicate.id });
        }
        return null;
    }
    makeSignal(type, severity, message, details) {
        return { type, severity, message, details, detectedAt: Math.floor(Date.now() / 1000) };
    }
    highestSeverity(signals) {
        if (signals.length === 0)
            return undefined;
        const order = [
            FraudSeverity.LOW,
            FraudSeverity.MEDIUM,
            FraudSeverity.HIGH,
            FraudSeverity.CRITICAL,
        ];
        return signals.reduce((max, s) => order.indexOf(s.severity) > order.indexOf(max) ? s.severity : max, signals[0].severity);
    }
}
/**
 * Factory function.
 */
export function createFraudDetector(config) {
    return new FraudDetector(config);
}
//# sourceMappingURL=fraud-detector.js.map