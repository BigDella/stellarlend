/**
 * Price Validator Service
 *
 * Validates and sanitizes price data before it's used for
 * contract updates. Implements multiple validation checks:
 */
import type { RawPriceData, ValidationResult } from '../types/index.js';
/**
 * Validator configuration
 */
export interface ValidatorConfig {
    maxDeviationPercent: number;
    maxStalenessSeconds: number;
    minPrice: number;
    maxPrice: number;
    sourceWeights: Record<string, number>;
}
/**
 * Price Validator
 */
export declare class PriceValidator {
    private config;
    private cachedPrices;
    constructor(config?: Partial<ValidatorConfig>);
    /**
     * Validate raw price data and convert to validated PriceData
     */
    validate(raw: RawPriceData): ValidationResult;
    /**
     * Validate multiple prices
     */
    validateMany(prices: RawPriceData[]): ValidationResult[];
    /**
     * Calculate confidence score based on various factors
     */
    private calculateConfidence;
    /**
     * Update cached price manually (e.g., after successful contract update)
     */
    updateCache(asset: string, price: number): void;
    /**
     * Clear cached price for an asset
     */
    clearCache(asset?: string): void;
    /**
     * Get current cache state (for debugging)
     */
    getCacheState(): Record<string, number>;
}
/**
 * Create a validator with custom configuration
 */
export declare function createValidator(config?: Partial<ValidatorConfig>): PriceValidator;
//# sourceMappingURL=price-validator.d.ts.map