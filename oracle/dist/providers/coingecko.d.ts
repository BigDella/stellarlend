/**
 * CoinGecko Price Provider
 *
 * Fallback price source using CoinGecko's API.
 *
 * Supports:
 * - Free tier (no API key): api.coingecko.com, 10-30 calls/min
 * - Demo tier (CG-* key): api.coingecko.com with x-cg-demo-api-key header
 * - Pro tier (other key): pro-api.coingecko.com with x-cg-pro-api-key header
 *
 * @see https://docs.coingecko.com/reference/simple-price
 */
import { BasePriceProvider } from './base-provider.js';
import type { RawPriceData, ProviderConfig } from '../types/index.js';
/**
 * CoinGecko Price Provider
 */
export declare class CoinGeckoProvider extends BasePriceProvider {
    private apiKey?;
    private tier;
    constructor(config: ProviderConfig);
    /**
     * Get the correct header name for the API key
     */
    private getApiKeyHeader;
    /**
     * Map asset symbol to CoinGecko ID
     */
    private getCoingeckoId;
    /**
     * Fetch price for a specific asset
     */
    fetchPrice(asset: string): Promise<RawPriceData>;
    /**
     * Fetch prices for multiple assets (batch API call)
     */
    fetchPrices(assets: string[]): Promise<RawPriceData[]>;
    /**
     * Get supported assets
     */
    getSupportedAssets(): string[];
}
/**
 * Create a CoinGecko provider with default configuration
 *
 * API Key Types:
 * - No key: Free tier (api.coingecko.com, 10-30 calls/min)
 * - CG-* key: Demo tier (api.coingecko.com with demo header)
 * - Other key: Pro tier (pro-api.coingecko.com with pro header)
 */
export declare function createCoinGeckoProvider(apiKey?: string): CoinGeckoProvider;
//# sourceMappingURL=coingecko.d.ts.map