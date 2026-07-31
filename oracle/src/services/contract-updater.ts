/**
 * Contract Updater Service
 */

import {
  Account,
  Keypair,
  Contract,
  rpc,
  TransactionBuilder,
  Networks,
  xdr,
  Address,
  nativeToScVal,
} from '@stellar/stellar-sdk';
import type { ContractUpdateResult, AggregatedPrice } from '../types/index.js';
import { logger } from '../utils/logger.js';

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
 * Default configuration
 */
const DEFAULT_CONFIG: Partial<ContractUpdaterConfig> = {
  baseFee: 100000,
  maxFee: 1000000,
  maxRetries: 3,
  retryDelayMs: 1000,
};

const MIN_TRANSACTION_FEE = 100;

/**
 * Contract Updater
 */
export class ContractUpdater {
  private config: ContractUpdaterConfig;
  private server: rpc.Server;
  private adminKeypair: Keypair;
  private networkPassphrase: string;

  constructor(config: ContractUpdaterConfig) {
    this.config = { ...DEFAULT_CONFIG, ...config } as ContractUpdaterConfig;

    if (this.config.baseFee < MIN_TRANSACTION_FEE) {
      throw new Error(`baseFee must be at least ${MIN_TRANSACTION_FEE} stroops`);
    }

    if (this.config.baseFee > this.config.maxFee) {
      throw new Error('baseFee cannot exceed maxFee');
    }

    this.server = new rpc.Server(this.config.rpcUrl);
    this.adminKeypair = Keypair.fromSecret(this.config.adminSecretKey);
    this.networkPassphrase = this.config.network === 'testnet' ? Networks.TESTNET : Networks.PUBLIC;

    logger.info('Contract updater initialized', {
      network: this.config.network,
      contractId: this.config.contractId,
      baseFee: this.config.baseFee,
      maxFee: this.config.maxFee,
      adminPublicKey: this.adminKeypair.publicKey(),
    });
  }

  /**
   * Update price for a single asset
   */
  async updatePrice(
    asset: string,
    price: bigint,
    timestamp: number
  ): Promise<ContractUpdateResult> {
    const startTime = Date.now();
    let lastError: Error | undefined;

    for (let attempt = 1; attempt <= this.config.maxRetries; attempt++) {
      try {
        logger.info(`Updating price for ${asset} (attempt ${attempt})`, {
          price: price.toString(),
          timestamp,
        });

        const txHash = await this.submitPriceUpdate(asset, price, timestamp);

        const result: ContractUpdateResult = {
          success: true,
          transactionHash: txHash,
          asset,
          price,
          timestamp,
        };

        logger.info(`Price update successful for ${asset}`, {
          txHash,
          durationMs: Date.now() - startTime,
        });

        return result;
      } catch (error) {
        lastError = error instanceof Error ? error : new Error(String(error));

        logger.warn(`Price update attempt ${attempt} failed for ${asset}`, {
          error: lastError.message,
        });

        if (attempt < this.config.maxRetries) {
          const delay = this.config.retryDelayMs * Math.pow(2, attempt - 1);
          await this.sleep(delay);
        }
      }
    }

    logger.error(`All price update attempts failed for ${asset}`, {
      error: lastError?.message,
    });

    return {
      success: false,
      asset,
      price,
      timestamp,
      error: lastError?.message || 'Unknown error',
    };
  }

  /**
   * Update prices for multiple assets
   */
  async updatePrices(prices: AggregatedPrice[]): Promise<ContractUpdateResult[]> {
    const results: ContractUpdateResult[] = [];

    for (const price of prices) {
      const result = await this.updatePrice(price.asset, price.price, price.timestamp);
      results.push(result);

      await this.sleep(100);
    }

    return results;
  }

  /**
   * Submit a price update transaction to the contract
   */
  private async submitPriceUpdate(
    asset: string,
    price: bigint,
    timestamp: number
  ): Promise<string> {
    const contract = new Contract(this.config.contractId);
    const adminAddress = new Address(this.adminKeypair.publicKey());

    const operation = contract.call(
      'set_asset_price',
      adminAddress.toScVal(),
      xdr.ScVal.scvSymbol(asset),
      nativeToScVal(price, { type: 'i128' }),
      nativeToScVal(timestamp, { type: 'u64' })
    );

    const account = await this.server.getAccount(this.adminKeypair.publicKey());

    const transaction = new TransactionBuilder(account, {
      fee: String(this.config.baseFee),
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(operation)
      .setTimeout(parseInt(process.env.TX_TIMEOUT || "30"))
      .build();

    const simulated = await this.server.simulateTransaction(transaction);

    if (rpc.Api.isSimulationError(simulated)) {
      const message = String(simulated.error ?? 'Unknown simulation error');
      throw new Error(
        message.startsWith('Simulation failed:') ? message : `Simulation failed: ${message}`
      );
    }

    if (!rpc.Api.isSimulationSuccess(simulated)) {
      throw new Error('Simulation did not succeed');
    }

    const prepared = rpc.assembleTransaction(transaction, simulated).build();
    prepared.sign(this.adminKeypair);

    const response = await this.server.sendTransaction(prepared);

    if (response.status === 'ERROR') {
      throw new Error(`Transaction failed: ${response.errorResult}`);
    }

    const hash = response.hash;
    let getResponse = await this.server.getTransaction(hash);

    const MAX_POLL_ATTEMPTS = 30;
    let attempts = 0;

    while (
      getResponse.status === rpc.Api.GetTransactionStatus.NOT_FOUND &&
      attempts < MAX_POLL_ATTEMPTS
    ) {
      await this.sleep(1000);
      attempts++;
      getResponse = await this.server.getTransaction(hash);
    }

    if (attempts >= MAX_POLL_ATTEMPTS) {
      logger.error('Transaction polling timed out', {
        txHash: hash,
        asset,
        attempts,
      });
      throw new Error(`Transaction polling timed out after ${MAX_POLL_ATTEMPTS} attempts`);
    }

    if (getResponse.status === rpc.Api.GetTransactionStatus.FAILED) {
      throw new Error(`Transaction failed on-chain`);
    }

    return hash;
  }

  /**
   * Comprehensive health check with detailed status
   */
  async healthCheck(): Promise<{
    overall: boolean;
    rpc: boolean;
    admin: boolean;
    contract: boolean;
    details: {
      rpc?: string;
      admin?: { balance: string; exists: boolean };
      contract?: string;
    };
  }> {
    const startTime = Date.now();
    const result = {
      overall: false,
      rpc: false,
      admin: false,
      contract: false,
      details: {} as any,
    };

    try {
      // 1. Check RPC connectivity
      try {
        await this.server.getHealth();
        result.rpc = true;
        result.details.rpc = 'RPC endpoint reachable';
      } catch (error) {
        result.details.rpc = `RPC unreachable: ${error instanceof Error ? error.message : 'Unknown error'}`;
      }

      // 2. Check admin account exists and has funds
      try {
        const adminAccount = (await this.server.getAccount(this.adminKeypair.publicKey())) as any;
        result.admin = true;
        result.details.admin = {
          exists: true,
          balance:
            adminAccount.balances
              .filter((balance: any) => balance.asset_type === 'native')
              .map((balance: any) => balance.balance)
              .join('') || '0',
        };
      } catch (error) {
        result.details.admin = {
          exists: false,
          balance: '0',
        };
      }

      // 3. Check contract is deployed and accessible
      try {
        const contract = new Contract(this.config.contractId);
        // Try to read from contract to verify it's deployed
        await this.server.simulateTransaction(
          new TransactionBuilder(new Account(this.adminKeypair.publicKey(), '1'), {
            fee: '100',
            networkPassphrase: this.networkPassphrase,
          })
            .addOperation(contract.call('get_asset_price', xdr.ScVal.scvSymbol('XLM')))
            .setTimeout(0)
            .build()
        );
        result.contract = true;
        result.details.contract = 'Contract accessible';
      } catch (error) {
        result.details.contract = `Contract inaccessible: ${error instanceof Error ? error.message : 'Unknown error'}`;
      }

      // Overall health is true only if all checks pass
      result.overall = result.rpc && result.admin && result.contract;

      logger.info('Health check completed', {
        duration: Date.now() - startTime,
        overall: result.overall,
        rpc: result.rpc,
        admin: result.admin,
        contract: result.contract,
      });

      return result;
    } catch (error) {
      logger.error('Health check failed with unexpected error:', error);
      return {
        overall: false,
        rpc: false,
        admin: false,
        contract: false,
        details: {
          rpc: 'Health check failed',
          admin: { exists: false, balance: '0' },
          contract: 'Health check failed',
        },
      };
    }
  }

  /**
   * Get the admin public key
   */
  getAdminPublicKey(): string {
    return this.adminKeypair.publicKey();
  }

  /**
   * Sleep utility
   */
  private sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
  }
}

/**
 * Create a contract updater
 */
export function createContractUpdater(config: ContractUpdaterConfig): ContractUpdater {
  return new ContractUpdater(config);
}
