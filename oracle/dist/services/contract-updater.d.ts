/**
 * Contract Updater Service
 */
import type { ContractUpdateResult, AggregatedPrice } from '../types/index.js';
/**
 * Contract updater configuration
 */
export interface ContractUpdaterConfig {
    network: 'testnet' | 'mainnet';
    rpcUrl: string;
    /** StellarLend contract ID */
    contractId: string;
    /** Admin secret key for signing */
    adminSecretKey: string;
    baseFee: number;
    maxFee: number;
    maxRetries: number;
    retryDelayMs: number;
}
/**
 * Contract Updater
 */
export declare class ContractUpdater {
    private config;
    private server;
    private adminKeypair;
    private networkPassphrase;
    constructor(config: ContractUpdaterConfig);
    /**
     * Update price for a single asset
     */
    updatePrice(asset: string, price: bigint, timestamp: number): Promise<ContractUpdateResult>;
    /**
     * Update prices for multiple assets
     */
    updatePrices(prices: AggregatedPrice[]): Promise<ContractUpdateResult[]>;
    /**
     * Submit a price update transaction to the contract
     */
    private submitPriceUpdate;
    /**
     * Comprehensive health check with detailed status
     */
    healthCheck(): Promise<{
        overall: boolean;
        rpc: boolean;
        admin: boolean;
        contract: boolean;
        details: {
            rpc?: string;
            admin?: {
                balance: string;
                exists: boolean;
            };
            contract?: string;
        };
    }>;
    /**
     * Get the admin public key
     */
    getAdminPublicKey(): string;
    /**
     * Sleep utility
     */
    private sleep;
}
/**
 * Create a contract updater
 */
export declare function createContractUpdater(config: ContractUpdaterConfig): ContractUpdater;
//# sourceMappingURL=contract-updater.d.ts.map