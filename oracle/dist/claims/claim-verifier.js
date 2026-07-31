/**
 * Claim Verifier
 *
 * Validates an insurance claim using live oracle price data.
 * Checks asset support, price freshness, oracle confidence,
 * amount validity, and coverage limits.
 */
import { VerificationErrorCode } from './types.js';
import { logger } from '../utils/logger.js';
const DEFAULT_CONFIG = {
    maxPriceAgeSeconds: 300,
    minOracleConfidence: 80,
    rejectOnLowConfidence: false,
};
/**
 * Oracle-based claim verifier.
 */
export class ClaimVerifier {
    aggregator;
    config;
    constructor(aggregator, config = {}) {
        this.aggregator = aggregator;
        this.config = { ...DEFAULT_CONFIG, ...config };
    }
    /**
     * Verify a claim against oracle data and business rules.
     */
    async verify(claim) {
        const errors = [];
        const now = Math.floor(Date.now() / 1000);
        // ── Business Rule Checks (no oracle needed) ────────────────────────────
        if (claim.claimedAmount <= 0n) {
            errors.push({
                code: VerificationErrorCode.INVALID_AMOUNT,
                message: 'Claimed amount must be greater than zero',
                details: { claimedAmount: claim.claimedAmount.toString() },
            });
        }
        if (claim.claimedAmount > claim.coverageLimit) {
            errors.push({
                code: VerificationErrorCode.AMOUNT_EXCEEDS_COVERAGE,
                message: 'Claimed amount exceeds coverage limit',
                details: {
                    claimedAmount: claim.claimedAmount.toString(),
                    coverageLimit: claim.coverageLimit.toString(),
                },
            });
        }
        if (claim.lossTimestamp > now) {
            errors.push({
                code: VerificationErrorCode.LOSS_TIMESTAMP_IN_FUTURE,
                message: 'Loss timestamp cannot be in the future',
                details: { lossTimestamp: claim.lossTimestamp, now },
            });
        }
        if (claim.lossTimestamp < claim.coveragePurchasedAt) {
            errors.push({
                code: VerificationErrorCode.LOSS_BEFORE_COVERAGE,
                message: 'Loss occurred before coverage was purchased',
                details: {
                    lossTimestamp: claim.lossTimestamp,
                    coveragePurchasedAt: claim.coveragePurchasedAt,
                },
            });
        }
        // Early exit on business rule failures (no need to hit oracle)
        if (errors.length > 0) {
            logger.warn('Claim failed business rule checks', {
                claimId: claim.id,
                errors: errors.map((e) => e.code),
            });
            return { isValid: false, errors };
        }
        // ── Oracle Verification ────────────────────────────────────────────────
        let oraclePrice;
        try {
            oraclePrice = await this.aggregator.getPrice(claim.asset);
        }
        catch (err) {
            logger.error('Oracle fetch threw an exception during claim verification', {
                claimId: claim.id,
                asset: claim.asset,
                error: err,
            });
            errors.push({
                code: VerificationErrorCode.ORACLE_UNAVAILABLE,
                message: 'Oracle threw an exception — cannot verify claim',
                details: { error: String(err) },
            });
            return { isValid: false, errors };
        }
        if (!oraclePrice) {
            logger.warn('Oracle returned no price for asset', {
                claimId: claim.id,
                asset: claim.asset,
            });
            errors.push({
                code: VerificationErrorCode.ORACLE_UNAVAILABLE,
                message: `Oracle could not provide a price for asset '${claim.asset}'`,
                details: { asset: claim.asset },
            });
            return { isValid: false, errors };
        }
        // Check price freshness
        const priceAgeSeconds = now - oraclePrice.timestamp;
        if (priceAgeSeconds > this.config.maxPriceAgeSeconds) {
            errors.push({
                code: VerificationErrorCode.PRICE_STALE,
                message: `Oracle price is ${priceAgeSeconds}s old — exceeds max ${this.config.maxPriceAgeSeconds}s`,
                details: { priceAgeSeconds, maxPriceAgeSeconds: this.config.maxPriceAgeSeconds },
            });
        }
        // Check oracle confidence
        const isLowConfidence = oraclePrice.confidence < this.config.minOracleConfidence;
        if (isLowConfidence) {
            const error = {
                code: VerificationErrorCode.LOW_ORACLE_CONFIDENCE,
                message: `Oracle confidence ${oraclePrice.confidence}% is below threshold ${this.config.minOracleConfidence}%`,
                details: { confidence: oraclePrice.confidence, threshold: this.config.minOracleConfidence },
            };
            if (this.config.rejectOnLowConfidence) {
                errors.push(error);
            }
            else {
                logger.warn('Low oracle confidence — proceeding with discount', {
                    claimId: claim.id,
                    confidence: oraclePrice.confidence,
                });
            }
        }
        if (errors.length > 0) {
            return { isValid: false, errors };
        }
        const oracleData = {
            priceAtVerification: oraclePrice.price,
            priceTimestamp: oraclePrice.timestamp,
            confidence: oraclePrice.confidence,
            sources: oraclePrice.sources.map((s) => s.source),
            verifiedAt: now,
        };
        logger.info('Claim verified successfully via oracle', {
            claimId: claim.id,
            asset: claim.asset,
            price: oraclePrice.price.toString(),
            confidence: oraclePrice.confidence,
        });
        return { isValid: true, oracleData, errors: [] };
    }
}
/**
 * Factory function.
 */
export function createClaimVerifier(aggregator, config) {
    return new ClaimVerifier(aggregator, config);
}
//# sourceMappingURL=claim-verifier.js.map