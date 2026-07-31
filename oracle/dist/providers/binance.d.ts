/**
 * Binance Price Provider
 *
 * Fallback price source using Binance's public API.
 * No API key required for public market data.
 *
 * @see https://binance-docs.github.io/apidocs/spot/en/
 */
import { BasePriceProvider } from './base-provider.js';
import type { RawPriceData, ProviderConfig } from '../types/index.js';
/**
 * Binance Price Provider
 */
export declare class BinanceProvider extends BasePriceProvider {
    constructor(config: ProviderConfig);
    /**
     * Map asset symbol to Binance trading pair
     */
    private getBinanceSymbol;
    /**
     * Fetch price for a specific asset
     */
    fetchPrice(asset: string): Promise<RawPriceData>;
    /**
     * Fetch prices for multiple assets
     * Uses batch ticker endpoint for efficiency
     */
    fetchPrices(assets: string[]): Promise<RawPriceData[]>;
    /**
     * Get supported assets
     */
    getSupportedAssets(): string[];
}
/**
 * Create a Binance provider with default configuration
 */
export declare function createBinanceProvider(): BinanceProvider;
//# sourceMappingURL=binance.d.ts.map