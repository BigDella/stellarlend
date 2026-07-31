/**
 * Oracle Service Configuration
 *
 * Handles loading and validating environment variables and
 * provides typed configuration for the oracle service.
 */
import type { OracleServiceConfig, AssetMapping, SupportedAsset } from './types/index.js';
export type { OracleServiceConfig } from './types/index.js';
/**
 * Asset mappings for different providers
 */
export declare const ASSET_MAPPINGS: AssetMapping[];
/**
 * Get asset mapping by symbol
 */
export declare function getAssetMapping(symbol: SupportedAsset): AssetMapping | undefined;
/**
 * Check if an asset is supported
 */
export declare function isSupportedAsset(symbol: string): symbol is SupportedAsset;
export declare function loadConfig(): OracleServiceConfig;
/**
 * Masks a secret key for safe logging.
 * Shows first 2 and last 2 characters only.
 * Handles edge cases: empty string, very short keys.
 */
export declare function maskSecret(key: string): string;
/**
 * Returns a safe (redacted) version of the config for logging.
 * Strips adminSecretKey entirely.
 */
export declare function getSafeConfig(config: OracleServiceConfig): Omit<OracleServiceConfig, 'adminSecretKey'> & {
    adminSecretKey: string;
};
export declare const PRICE_SCALE = 1000000n;
export declare function scalePrice(price: number): bigint;
export declare function unscalePrice(price: bigint): number;
//# sourceMappingURL=config.d.ts.map