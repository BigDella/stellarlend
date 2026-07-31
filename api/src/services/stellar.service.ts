import {
  TransactionBuilder,
  Contract,
  xdr,
  Address,
  Keypair,
  nativeToScVal,
  Account,
  BASE_FEE,
  scValToNative,
} from '@stellar/stellar-sdk';
import { ValidationError } from '../utils/errors';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import axios from 'axios';
import type { AxiosResponse } from 'axios';
import { config } from '../config';
import logger from '../utils/logger';
import { InternalServerError } from '../utils/errors';
import { parsePaginationParams, buildPaginationMeta } from '../utils/pagination';
import {
  TransactionResponse,
  LendingOperation,
  ProtocolStatsResponse,
  PositionResponse,
  TransactionHistoryItem,
  TransactionHistoryQuery,
  TransactionHistoryResponse,
} from '../types';
import { BoundedTtlCache } from '../utils/boundedTtlCache';
import { redisCacheService } from './redisCache.service';
import { requestCoalescingService } from './requestCoalescing.service';

const CONTRACT_METHODS: Record<LendingOperation, string> = {
  deposit: 'deposit_collateral',
  borrow: 'borrow_asset',
  repay: 'repay_debt',
  withdraw: 'withdraw_collateral',
};

// Timeout generous enough for client-side signing (5 minutes)
const TX_TIMEOUT_SECONDS = 300;
const PROTOCOL_STATS_CACHE_KEY = 'protocol-stats';

const protocolStatsCache = new BoundedTtlCache<ProtocolStatsResponse>({
  ttlMs: config.cache.protocolStatsTtlMs,
  maxEntries: 1,
});

function toIntegerString(value: unknown): string {
  if (typeof value === 'bigint') {
    return value.toString();
  }

  if (typeof value === 'number') {
    if (!Number.isFinite(value)) {
      throw new InternalServerError('Invalid protocol stats value');
    }
    return Math.trunc(value).toString();
  }

  if (typeof value === 'string') {
    return value;
  }

  throw new InternalServerError('Unexpected protocol stats payload');
}

function toSafeNumber(value: unknown): number {
  if (typeof value === 'number') {
    return Math.trunc(value);
  }

  if (typeof value === 'bigint') {
    return Number(value);
  }

  if (typeof value === 'string') {
    return parseInt(value, 10);
  }

  return 0;
}

function formatBpsAsRatio(value: string): string {
  const bps = BigInt(value);
  const scaled = (bps * 100n) / 10000n;
  const whole = scaled / 100n;
  const fractional = (scaled % 100n).toString().padStart(2, '0');
  return `${whole}.${fractional}`;
}

function decodeSimulationResult(simulation: any): any {
  const rawValue =
    simulation?.result?.retval ??
    simulation?.retval ??
    simulation?.result?.xdr ??
    simulation?.results?.[0]?.xdr;

  if (!rawValue) {
    throw new InternalServerError('Missing Soroban simulation result');
  }

  if (typeof rawValue === 'string') {
    return scValToNative(xdr.ScVal.fromXDR(rawValue, 'base64'));
  }

  return scValToNative(rawValue);
}

export function clearProtocolStatsCache(): void {
  protocolStatsCache.clear();
}

/**
 * Safely create a Keypair from a secret key, throwing a descriptive error
 * for invalid key format without echoing the key value.
 */
function keypairFromSecret(secret: string): Keypair {
  try {
    return Keypair.fromSecret(secret);
  } catch (error) {
    if (error instanceof Error && /invalid|bad|decode|checksum/i.test(error.message)) {
      throw new ValidationError('Invalid secret key format');
    }
    throw error;
  }
}

/** Deterministic, non-cryptographic hash used to derive simulated values from an address string. */
function simpleHash(value: string): number {
  let hash = 0;
  for (let i = 0; i < value.length; i++) {
    hash = (hash * 31 + value.charCodeAt(i)) & 0x7fffffff;
  }
  return hash;
}

export class StellarService {
  private horizonUrl: string;
  private sorobanRpcUrl: string;
  private networkPassphrase: string;
  private contractId: string;
  private readOnlySimulationAccount: string;
  private sorobanServer: SorobanServer;

  constructor() {
    this.horizonUrl = config.stellar.horizonUrl;
    this.sorobanRpcUrl = config.stellar.sorobanRpcUrl;
    this.networkPassphrase = config.stellar.networkPassphrase;
    this.contractId = config.stellar.contractId;
    this.readOnlySimulationAccount = config.stellar.readOnlySimulationAccount;
    this.sorobanServer = new SorobanServer(this.sorobanRpcUrl);
  }

  async relayExecuteDelegated(
    delegatorAddress: string,
    nonce: string,
    deadline: string,
    callsXdr: string
  ): Promise<{
    delegateAddress: string;
    txXdr: string;
    txHash?: string;
    success: boolean;
    error?: string;
  }> {
    if (!config.stellar.relayerSecret) {
      throw new InternalServerError('RELAYER_SECRET is not configured');
    }

    const relayer = keypairFromSecret(config.stellar.relayerSecret);
    const delegateAddress = relayer.publicKey();

    const account = await this.getAccount(delegateAddress);
    const contract = new Contract(this.contractId);

    // `callsXdr` is an XDR-encoded ScVal representing Vec<Call> as defined by the contract.
    // This keeps the API generic and avoids having to replicate Soroban struct encoding here.
    const calls = xdr.ScVal.fromXDR(callsXdr, 'base64');

    const params = [
      new Address(delegatorAddress).toScVal(),
      new Address(delegateAddress).toScVal(),
      nativeToScVal(BigInt(nonce), { type: 'u64' }),
      nativeToScVal(BigInt(deadline), { type: 'u64' }),
      calls,
    ];

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call('execute_delegated', ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const preparedTx = await this.sorobanServer.prepareTransaction(tx);
    preparedTx.sign(relayer);

    const submit = await this.submitTransaction(preparedTx.toXDR());
    return {
      delegateAddress,
      txXdr: preparedTx.toXDR(),
      txHash: submit.transactionHash,
      success: submit.success,
      error: submit.success ? undefined : submit.error,
    };
  }

  async getAccount(address: string): Promise<Account> {
    try {
      const response = await axios.get(`${this.horizonUrl}/accounts/${address}`);
      const data = response.data as { id: string; sequence: string };
      return new Account(data.id, data.sequence);
    } catch (error) {
      logger.error('Failed to fetch account:', error);
      throw new InternalServerError('Failed to fetch account information');
    }
  }

  private async buildTransaction(
    operation: LendingOperation,
    userAddress: string,
    assetAddress: string | undefined,
    amount: string
  ): Promise<string> {
    const account = await this.getAccount(userAddress);
    const contract = new Contract(this.contractId);

    const params = [
      new Address(userAddress).toScVal(),
      assetAddress ? new Address(assetAddress).toScVal() : xdr.ScVal.scvVoid(),
      nativeToScVal(BigInt(amount), { type: 'i128' }),
    ];

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(CONTRACT_METHODS[operation], ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const preparedTx = await this.sorobanServer.prepareTransaction(tx);
    return preparedTx.toXDR();
  }

  async buildUnsignedTransaction(
    operation: LendingOperation,
    userAddress: string,
    assetAddress: string | undefined,
    amount: string
  ): Promise<string> {
    try {
      return await this.buildTransaction(operation, userAddress, assetAddress, amount);
    } catch (error) {
      logger.error(`Failed to build unsigned ${operation} transaction:`, error);
      throw new InternalServerError(`Failed to build ${operation} transaction`);
    }
  }

  async estimateGas(
    operation: LendingOperation,
    userAddress: string,
    assetAddress: string | undefined,
    amount: string
  ): Promise<{ cpuInstructions: string; memoryBytes: string; minResourceFee: string }> {
    const coalescingKey = requestCoalescingService.generateKey('estimateGas', {
      operation,
      userAddress,
      assetAddress,
      amount,
    });
    return requestCoalescingService.execute(coalescingKey, async () => {
      try {
        const account = await this.getAccount(userAddress);
        const contract = new Contract(this.contractId);

        const params = [
          new Address(userAddress).toScVal(),
          assetAddress ? new Address(assetAddress).toScVal() : xdr.ScVal.scvVoid(),
          nativeToScVal(BigInt(amount), { type: 'i128' }),
        ];

        const tx = new TransactionBuilder(account, {
          fee: BASE_FEE,
          networkPassphrase: this.networkPassphrase,
        })
          .addOperation(contract.call(CONTRACT_METHODS[operation], ...params))
          .setTimeout(TX_TIMEOUT_SECONDS)
          .build();

        const simulation = await (this.sorobanServer as any).simulateTransaction(tx);

        if (simulation.error) {
          throw new InternalServerError(`Simulation failed: ${simulation.error}`);
        }

        return {
          cpuInstructions: simulation.cost?.cpuInsns || '0',
          memoryBytes: simulation.cost?.memBytes || '0',
          minResourceFee: simulation.minResourceFee || '0',
        };
      } catch (error: any) {
        logger.error(`Failed to estimate gas for ${operation}:`, error);
        throw new InternalServerError(error.message || `Failed to estimate gas for ${operation}`);
      }
    });
  }

  private buildReadOnlyTransaction(methodName: string, ...params: any[]): any {
    const account = new Account(this.readOnlySimulationAccount, '0');
    const contract = new Contract(this.contractId);

    return new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(methodName, ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();
  }

  private async simulateContractCall(methodName: string, ...params: any[]): Promise<any> {
    const tx = this.buildReadOnlyTransaction(methodName, ...params);
    const simulation = await (this.sorobanServer as any).simulateTransaction(tx);
    return decodeSimulationResult(simulation);
  }

  /**
   * Get TWAP-based liquidation price for an asset.
   * Calls the contract's get_liquidation_price function which returns TWAP
   * with fallback to median spot price across sources on manipulation.
   */
  async getLiquidationPrice(asset: string): Promise<string> {
    try {
      const assetAddress = new Address(asset);
      const result = await this.simulateContractCall(
        'get_liquidation_price',
        assetAddress.toScVal()
      );
      return result?.toString() ?? '0';
    } catch (error) {
      logger.error('Failed to get liquidation price:', error);
      return '0';
    }
  }

  async getProtocolStats(): Promise<ProtocolStatsResponse> {
    const coalescingKey = requestCoalescingService.generateKey('getProtocolStats', {});

    return requestCoalescingService.execute(coalescingKey, async () => {
      const redisKey = redisCacheService.buildKey('protocol', 'stats');
      const redisCached = await redisCacheService.get<ProtocolStatsResponse>(redisKey);
      if (redisCached) return redisCached;

      const cachedResponse = protocolStatsCache.get(PROTOCOL_STATS_CACHE_KEY);
      if (cachedResponse) {
        return cachedResponse;
      }

      try {
        const report = await this.simulateContractCall('get_protocol_report');
        const metrics = report?.metrics ?? report ?? {};

        const response: ProtocolStatsResponse = {
          totalDeposits: toIntegerString(metrics.total_deposits ?? metrics.totalDeposits ?? 0),
          totalBorrows: toIntegerString(metrics.total_borrows ?? metrics.totalBorrows ?? 0),
          utilizationRate: formatBpsAsRatio(
            toIntegerString(metrics.utilization_rate ?? metrics.utilizationRate ?? 0)
          ),
          numberOfUsers: toSafeNumber(metrics.total_users ?? metrics.totalUsers ?? 0),
          tvl: toIntegerString(metrics.total_value_locked ?? metrics.totalValueLocked ?? 0),
        };

        protocolStatsCache.set(PROTOCOL_STATS_CACHE_KEY, response);
        await redisCacheService.set(
          redisKey,
          response,
          Math.floor(config.cache.protocolStatsTtlMs / 1000)
        );
        return response;
      } catch (error) {
        logger.error('Failed to fetch protocol stats:', error);
        throw new InternalServerError('Failed to fetch protocol stats');
      }
    });
  }

  async submitTransaction(txXdr: string): Promise<TransactionResponse> {
    const {
      request: { maxRetries, retryInitialDelayMs, retryMaxDelayMs, timeout },
    } = config;

    for (let attempt = 0; attempt <= maxRetries; attempt++) {
      try {
        const response = await axios.post(
          `${this.horizonUrl}/transactions`,
          { tx: txXdr },
          { timeout }
        );
        // Horizon and other RPCs can return slightly different shapes; the only
        // reliable indicator we validate here is `successful` when present.
        const data = response.data as any;
        const successful: unknown = data?.successful;
        const transactionHash: string | undefined =
          data?.hash ?? data?.transaction_hash ?? data?.transactionHash;
        const ledger: number | undefined = data?.ledger ?? data?.ledger_index ?? data?.ledgerIndex;

        if (successful === false) {
          return {
            success: false,
            transactionHash,
            status: 'failed',
            error: 'Transaction failed on-chain',
            message: 'Provider reported on-chain failure despite successful HTTP submission',
            ledger,
            details: data,
          };
        }

        return {
          success: true,
          transactionHash,
          status: 'success',
          ledger,
        };
      } catch (error: any) {
        const status = error?.response?.status as number | undefined;
        const isClientError = typeof status === 'number' && status >= 400 && status < 500;
        const isRetryable =
          // Network error (no response) is retryable
          !error?.response ||
          // 5xx server errors are retryable
          (typeof status === 'number' && status >= 500);

        // Immediately fail on non-retryable 4xx errors
        if (isClientError && status !== 429) {
          logger.error('Transaction submission failed (non-retryable):', error);
          return {
            success: false,
            status: 'failed',
            error: error.response?.data?.extras?.result_codes || error.message,
          };
        }

        // If we've exhausted retries or it's not retryable, return failure
        if (attempt === maxRetries || !isRetryable) {
          logger.error('Transaction submission failed (final):', error);
          return {
            success: false,
            status: 'failed',
            error: error.response?.data?.extras?.result_codes || error.message,
          };
        }

        // Exponential backoff with cap
        const backoff = Math.min(retryInitialDelayMs * Math.pow(2, attempt), retryMaxDelayMs);
        logger.warn(
          `Submit transaction attempt ${attempt + 1} failed${
            status ? ` (status ${status})` : ''
          }. Retrying in ${backoff} ms...`
        );
        await new Promise((resolve) => setTimeout(resolve, backoff));
      }
    }

    // Fallback — should be unreachable because loop returns
    return {
      success: false,
      status: 'failed',
      error: 'Unknown submission error',
    };
  }

  async monitorTransaction(
    txHash: string,
    timeoutMs = 30000,
    abortSignal?: AbortSignal
  ): Promise<TransactionResponse> {
    const startTime = Date.now();
    let delay = 500;
    const maxDelay = 5000;

    while (Date.now() - startTime < timeoutMs) {
      if (abortSignal?.aborted) {
        return {
          success: false,
          transactionHash: txHash,
          status: 'cancelled',
          message: 'Transaction monitoring cancelled',
        };
      }
      try {
        const response = await axios.get(`${this.horizonUrl}/transactions/${txHash}`);
        const data = response.data as { successful: boolean; ledger: number };
        if (data.successful) {
          return {
            success: true,
            transactionHash: txHash,
            status: 'success',
            ledger: data.ledger,
          };
        }
        return {
          success: false,
          transactionHash: txHash,
          status: 'failed',
          error: 'Transaction failed',
        };
      } catch (error: any) {
        if (error.response?.status === 404) {
          // Wait for delay or until aborted
          await new Promise((resolve) => {
            const timeout = setTimeout(resolve, delay);
            if (abortSignal) {
              abortSignal.addEventListener(
                'abort',
                () => {
                  clearTimeout(timeout);
                  resolve(undefined);
                },
                { once: true }
              );
            }
          });
          if (abortSignal?.aborted) {
            return {
              success: false,
              transactionHash: txHash,
              status: 'cancelled',
              message: 'Transaction monitoring cancelled',
            };
          }
          delay = Math.min(delay * 2, maxDelay);
          continue;
        }
        logger.error('Error monitoring transaction:', error);
        throw new InternalServerError('Failed to monitor transaction');
      }
    }

    return {
      success: false,
      transactionHash: txHash,
      status: 'pending',
      message: 'Transaction monitoring timeout',
    };
  }

  async healthCheck(): Promise<{ horizon: boolean; sorobanRpc: boolean }> {
    const results = { horizon: false, sorobanRpc: false };

    try {
      await axios.get(`${this.horizonUrl}/`);
      results.horizon = true;
    } catch (error) {
      logger.error('Horizon health check failed:', error);
    }

    try {
      await this.sorobanServer.getHealth();
      results.sorobanRpc = true;
    } catch (error) {
      logger.error('Soroban RPC health check failed:', error);
    }

    return results;
  }

  async getTransactionHistory(query: TransactionHistoryQuery): Promise<TransactionHistoryResponse> {
    const coalescingKey = requestCoalescingService.generateKey('getTransactionHistory', query);

    return requestCoalescingService.execute(coalescingKey, async () => {
      try {
        const { userAddress } = query;
        const { limit, cursor } = parsePaginationParams(query as any);
        const historyCacheKey = redisCacheService.buildKey(
          'position',
          `${userAddress}:${limit}:${cursor ?? 'first'}`
        );
        const cached = await redisCacheService.get<TransactionHistoryResponse>(historyCacheKey);
        if (cached) return cached;

        // Validate Stellar address format
        if (!this.isValidStellarAddress(userAddress)) {
          throw new InternalServerError('Invalid Stellar address format');
        }

        // Build Horizon API URL for transactions
        let url = `${this.horizonUrl}/accounts/${userAddress}/transactions?limit=${limit}&order=desc`;
        if (cursor) {
          url += `&cursor=${encodeURIComponent(cursor)}`;
        }

        const response = await axios.get(url);
        const transactions = response.data._embedded?.records || [];

        // Filter and map transactions related to lending contract
        const lendingTransactions = await this.filterLendingTransactions(transactions);

        // Extract pagination info from Horizon next link
        const nextCursor = response.data._links?.next
          ? new URL(response.data._links.next.href).searchParams.get('cursor')
          : null;
        const hasNextPage = !!response.data._links?.next;

        const result = {
          data: lendingTransactions,
          pagination: buildPaginationMeta(
            nextCursor,
            hasNextPage,
            limit ?? config.pagination.defaultLimit
          ),
        };
        await redisCacheService.set(
          historyCacheKey,
          result,
          Math.floor(config.cache.positionTtlMs / 1000)
        );
        return result;
      } catch (error) {
        logger.error('Failed to fetch transaction history:', error);
        throw new InternalServerError('Failed to fetch transaction history');
      }
    });
  }

  private async filterLendingTransactions(transactions: any[]): Promise<TransactionHistoryItem[]> {
    const lendingTransactions: TransactionHistoryItem[] = [];

    for (const tx of transactions) {
      // Check if transaction involves our lending contract
      if (this.isLendingTransaction(tx)) {
        const item = await this.mapToTransactionHistoryItem(tx);
        if (item) {
          lendingTransactions.push(item);
        }
      }
    }

    return lendingTransactions;
  }

  private isLendingTransaction(transaction: any): boolean {
    try {
      // Check if transaction has operations that interact with our contract
      if (!transaction.operations || !Array.isArray(transaction.operations)) {
        return false;
      }

      return transaction.operations.some(
        (op: any) => op.type === 'invoke_contract_function' && op.contract_id === this.contractId
      );
    } catch {
      return false;
    }
  }

  private async mapToTransactionHistoryItem(
    transaction: any
  ): Promise<TransactionHistoryItem | null> {
    try {
      // Extract operation details
      const lendingOp = transaction.operations.find(
        (op: any) => op.type === 'invoke_contract_function' && op.contract_id === this.contractId
      );

      if (!lendingOp) {
        return null;
      }

      // Map function name to operation type
      const operationType = this.mapFunctionToOperation(lendingOp.function_name);
      if (!operationType) {
        return null;
      }

      // Extract amount from function parameters
      const amount = this.extractAmountFromParams(lendingOp.function_parameters);

      return {
        transactionHash: transaction.hash,
        type: operationType,
        amount: amount || '0',
        assetAddress: this.extractAssetFromParams(lendingOp.function_parameters),
        timestamp: transaction.created_at,
        status: transaction.successful ? 'success' : 'failed',
        ledger: transaction.ledger,
        memo: transaction.memo || undefined,
      };
    } catch (error) {
      logger.error('Failed to map transaction to history item:', error);
      return null;
    }
  }

  private mapFunctionToOperation(functionName: string): LendingOperation | null {
    const functionToOperation: Record<string, LendingOperation> = {
      deposit_collateral: 'deposit',
      borrow_asset: 'borrow',
      repay_debt: 'repay',
      withdraw_collateral: 'withdraw',
    };

    return functionToOperation[functionName] || null;
  }

  private extractAmountFromParams(params: any[]): string {
    try {
      // Look for amount parameter (typically the third parameter)
      if (params && params.length >= 3) {
        const amountParam = params[2];
        if (amountParam && amountParam.value) {
          return amountParam.value.toString();
        }
      }
      return '0';
    } catch {
      return '0';
    }
  }

  private extractAssetFromParams(params: any[]): string | undefined {
    try {
      // Look for asset address parameter (typically the second parameter)
      if (params && params.length >= 2) {
        const assetParam = params[1];
        if (assetParam && assetParam.value) {
          return assetParam.value;
        }
      }
      return undefined;
    } catch {
      return undefined;
    }
  }

  /**
   * Async generator that pages through the full transaction history for a user
   * and yields items one at a time, keeping memory usage bounded.
   * Callers should check `signal.aborted` and stop consuming when the client disconnects.
   */
  async *streamTransactionHistory(
    userAddress: string,
    pageSize: number = config.pagination.defaultLimit,
    signal?: AbortSignal
  ): AsyncGenerator<TransactionHistoryItem> {
    if (!this.isValidStellarAddress(userAddress)) {
      throw new InternalServerError('Invalid Stellar address format');
    }

    let nextUrl: string | null =
      `${this.horizonUrl}/accounts/${userAddress}/transactions?limit=${pageSize}&order=desc`;

    while (nextUrl) {
      if (signal?.aborted) return;

      const response: AxiosResponse<any> = await axios.get(nextUrl);
      const transactions: any[] = response.data._embedded?.records ?? [];

      const lendingTxs = await this.filterLendingTransactions(transactions);
      for (const tx of lendingTxs) {
        if (signal?.aborted) return;
        yield tx;
      }

      nextUrl = response.data._links?.next?.href ?? null;
    }
  }

  async getUserPosition(userAddress: string): Promise<PositionResponse> {
    if (!this.isValidStellarAddress(userAddress)) {
      throw new InternalServerError('Invalid Stellar address format');
    }

    const coalescingKey = requestCoalescingService.generateKey('getUserPosition', { userAddress });

    return requestCoalescingService.execute(coalescingKey, async () => {
      const cacheKey = redisCacheService.buildKey('position', userAddress);
      const cached = await redisCacheService.get<PositionResponse>(cacheKey);
      if (cached) return cached;

      try {
        const userParam = new Address(userAddress).toScVal();
        const raw = await this.simulateContractCall('get_user_position', userParam);

        const collateral = toIntegerString(raw?.collateral ?? raw?.collateral_amount ?? 0);
        const debt = toIntegerString(raw?.debt ?? raw?.debt_amount ?? 0);
        const borrowInterest = toIntegerString(raw?.borrow_interest ?? raw?.interest ?? 0);
        const lastAccrualTime = toSafeNumber(raw?.last_accrual_time ?? raw?.lastAccrualTime ?? 0);

        const collateralBig = BigInt(collateral);
        const debtBig = BigInt(debt);
        const collateralRatio =
          debtBig > 0n ? ((collateralBig * 10000n) / debtBig).toString() : 'Infinity';

        const result: PositionResponse = {
          userAddress,
          collateral,
          debt,
          borrowInterest,
          lastAccrualTime,
          collateralRatio,
        };

        await redisCacheService.set(
          cacheKey,
          result,
          Math.floor(config.cache.positionTtlMs / 1000)
        );
        return result;
      } catch (error) {
        logger.error('Failed to fetch user position:', error);
        throw new InternalServerError('Failed to fetch user position');
      }
    });
  }

  private isValidStellarAddress(address: string): boolean {
    try {
      // Basic Stellar address validation (G followed by 56 alphanumeric characters)
      return /^G[A-Z0-9]{56}$/.test(address);
    } catch {
      return false;
    }
  }

  // ─── Recurring Operations ──────────────────────────────────────────────────

  async executeRecurringOperation(
    userAddress: string,
    action: RecurringAction,
    amount: string,
    assetAddress?: string
  ): Promise<TransactionResponse> {
    try {
      logger.info('Executing recurring operation', { userAddress, action, amount });
      const unsignedXdr = await this.buildUnsignedTransaction(
        action as LendingOperation,
        userAddress,
        assetAddress,
        amount
      );
      // Simulated: in production, this would be signed by a relayer key
      return {
        success: true,
        transactionHash: `tx_recurring_${Date.now()}`,
        status: 'success',
        message: `Recurring ${action} executed`,
      };
    } catch (error) {
      logger.error('Recurring operation failed', { userAddress, action, error });
      return {
        success: false,
        status: 'failed',
        error: (error as Error).message || 'execution_failed',
      };
    }
  }

  // ─── Analytics helpers (simulated contract reads) ──────────────────────────

  async getPoolRateAt(
    poolAddress: string,
    _timestamp: number
  ): Promise<{ depositApy: number; borrowApy: number; utilizationRate: number }> {
    // Simulated: returns mock historical rate data
    const baseRate = poolAddress ? 0.03 + (parseInt(poolAddress.slice(-4), 16) % 10) / 100 : 0.05;
    return {
      depositApy: baseRate,
      borrowApy: baseRate * 1.5,
      utilizationRate: 0.45 + Math.random() * 0.3,
    };
  }

  async getPoolStateAt(
    poolAddress: string,
    _timestamp: number
  ): Promise<{ utilizationRate: number; totalDeposits: string; totalBorrows: string }> {
    const simVal = poolAddress ? simpleHash(poolAddress) % 100 : 50;
    return {
      utilizationRate: simVal / 100,
      totalDeposits: (1000000n * BigInt(simVal + 50)).toString(),
      totalBorrows: (500000n * BigInt(simVal + 30)).toString(),
    };
  }

  async getProtocolRevenueAt(
    _timestamp: number
  ): Promise<{ cumulativeRevenue: string; periodRevenue: string }> {
    return {
      cumulativeRevenue: (BigInt(Math.trunc(1000000 * (1 + Math.random())))).toString(),
      periodRevenue: (BigInt(Math.trunc(10000 * (1 + Math.random())))).toString(),
    };
  }

  async getAllPools(): Promise<
    Array<{
      address: string;
      name?: string;
      depositApy: number;
      borrowApy: number;
      utilizationRate: number;
      tvl: string;
    }>
  > {
    // Simulated: returns mock pool data
    return [
      {
        address: 'pool_xlm_001',
        name: 'XLM Pool',
        depositApy: 0.035,
        borrowApy: 0.052,
        utilizationRate: 0.48,
        tvl: '5000000000',
      },
      {
        address: 'pool_usdc_001',
        name: 'USDC Pool',
        depositApy: 0.042,
        borrowApy: 0.063,
        utilizationRate: 0.62,
        tvl: '8000000000',
      },
      {
        address: 'pool_btc_001',
        name: 'BTC Pool',
        depositApy: 0.028,
        borrowApy: 0.045,
        utilizationRate: 0.35,
        tvl: '12000000000',
      },
    ];
  }
}

// ─── Type import for RecurringAction ───────────────────────────────────────────
type RecurringAction = 'deposit' | 'borrow' | 'repay';
