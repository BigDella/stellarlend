/**
 * MEV Protection Service
 *
 * Provides:
 *  - Commit-reveal scheme helpers (build unsigned commit / reveal transactions)
 *  - Batch auction management (place bid, settle, query)
 *  - MEV extraction monitoring dashboard data
 *  - Gas price bidding analysis
 *  - Private mempool routing hints
 */

import {
  Contract,
  Address,
  nativeToScVal,
  xdr,
  scValToNative,
  TransactionBuilder,
  BASE_FEE,
  Account,
} from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import { config } from '../config';
import logger from '../utils/logger';
import { InternalServerError, ValidationError } from '../utils/errors';
import { redisCacheService } from './redisCache.service';

// ─── Types ────────────────────────────────────────────────────────────────────

export type SensitiveOperation = 'Borrow' | 'Withdraw' | 'Liquidate';
export type TxOrderingHint = 'Default' | 'PrivateMempool' | 'BatchAuction' | 'DelayedReveal';

export interface CommitRequest {
  userAddress: string;
  operation: SensitiveOperation;
  assetAddress?: string;
  secondaryAssetAddress?: string;
  borrowerAddress?: string;
  amount: string;
  maxFeeBps: number;
  hint: TxOrderingHint;
  maxSlippageBps?: number;
  deadline?: number;
}

export interface CommitResponse {
  commitId: string;
  revealAfter: number;
  expiresAt: number;
  hint: TxOrderingHint;
  unsignedXdr: string;
}

export interface RevealRequest {
  userAddress: string;
  commitId: string;
  operation: SensitiveOperation;
}

export interface AuctionBidRequest {
  bidderAddress: string;
  borrowerAddress: string;
  debtAmount: string;
  minCollateralOut: string;
  maxFeeBps: number;
  deadline?: number;
}

export interface AuctionBidResponse {
  slotId: string;
  unsignedXdr: string;
}

export interface AuctionResult {
  slotId: string;
  bidCount: number;
  clearingFeeBps: number;
  totalDebtLiquidated: string;
  settledAt: number;
}

export interface MevDashboardData {
  orderingStats: {
    suspiciousSequences: number;
    sandwichAlerts: number;
    lastAlertTimestamp: number;
    lastEffectiveFeeBps: number;
    totalCommits: number;
    totalReveals: number;
    totalAuctionBids: number;
    totalAuctionsSettled: number;
    cumulativeFeeBpsCollected: number;
  };
  protectionConfig: {
    commitDelaySecs: number;
    commitExpirySecs: number;
    suspiciousWindowSecs: number;
    baseProtectionFeeBps: number;
    surgeProtectionFeeBps: number;
    privateMempoolEnabled: boolean;
    batchingEnabled: boolean;
    batchWindowSecs: number;
    defaultMaxSlippageBps: number;
  };
  currentAuctionSlot: string;
  timestamp: string;
}

export interface GasBidAnalysis {
  smoothedBaseFeeBps: number;
  currentSurgeFeeBps: number;
  recommendedBidBps: number;
  highCongestionBidBps: number;
  inSuspiciousWindow: boolean;
  recentSandwichAlerts: number;
  operation: SensitiveOperation;
  assetAddress?: string;
  timestamp: string;
}

export interface PrivateMempoolRoute {
  hint: TxOrderingHint;
  guidance: string;
  commitDelaySecs: number;
  recommendedFeeBps: number;
  privateMempoolEnabled: boolean;
  batchingEnabled: boolean;
}

const TX_TIMEOUT_SECONDS = 300;
const DASHBOARD_CACHE_TTL_SECS = 15;
const GAS_ANALYSIS_CACHE_TTL_SECS = 10;

// ─── Helpers ──────────────────────────────────────────────────────────────────

function operationToScVal(env: { operation: SensitiveOperation }): xdr.ScVal {
  // Soroban enum variant encoding: { tag: "Borrow" } etc.
  return xdr.ScVal.scvVec([
    xdr.ScVal.scvSymbol(env.operation),
  ]);
}

function hintToScVal(hint: TxOrderingHint): xdr.ScVal {
  return xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(hint)]);
}

function optionalAddress(addr?: string): xdr.ScVal {
  if (!addr) return xdr.ScVal.scvVoid();
  return new Address(addr).toScVal();
}

function decodeSimResult(simulation: any): any {
  const raw =
    simulation?.result?.retval ??
    simulation?.retval ??
    simulation?.results?.[0]?.xdr;
  if (!raw) throw new InternalServerError('Missing Soroban simulation result');
  if (typeof raw === 'string') return scValToNative(xdr.ScVal.fromXDR(raw, 'base64'));
  return scValToNative(raw);
}

// ─── Service Class ────────────────────────────────────────────────────────────

export class MevService {
  private readonly server: SorobanServer;
  private readonly contractId: string;
  private readonly networkPassphrase: string;
  private readonly horizonUrl: string;
  private readonly readOnlyAccount: string;

  constructor() {
    this.server = new SorobanServer(config.stellar.sorobanRpcUrl);
    this.contractId = config.stellar.contractId;
    this.networkPassphrase = config.stellar.networkPassphrase;
    this.horizonUrl = config.stellar.horizonUrl;
    this.readOnlyAccount = config.stellar.readOnlySimulationAccount;
  }

  // ── Commit-Reveal ──────────────────────────────────────────────────────────

  /**
   * Build an unsigned commit transaction for a sensitive operation.
   * The client signs and submits it; the returned `commitId` is used at reveal time.
   */
  async buildCommitTransaction(req: CommitRequest): Promise<CommitResponse> {
    const contract = new Contract(this.contractId);
    const account = await this.getAccount(req.userAddress);

    const methodMap: Record<SensitiveOperation, string> = {
      Borrow: req.maxSlippageBps || req.deadline
        ? 'commit_borrow_with_slippage'
        : 'commit_borrow_protected',
      Withdraw: req.maxSlippageBps || req.deadline
        ? 'commit_withdraw_with_slippage'
        : 'commit_withdraw_protected',
      Liquidate: req.maxSlippageBps || req.deadline
        ? 'commit_liquidation_with_slippage'
        : 'commit_liquidation_protected',
    };

    const method = methodMap[req.operation];
    const params = this.buildCommitParams(req);

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(method, ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const prepared = await this.server.prepareTransaction(tx);
    const cfg = await this.getProtectionConfig();

    const now = Math.floor(Date.now() / 1000);
    return {
      commitId: 'pending', // actual ID returned after on-chain execution
      revealAfter: now + cfg.commitDelaySecs,
      expiresAt: now + cfg.commitExpirySecs,
      hint: req.hint,
      unsignedXdr: prepared.toXDR(),
    };
  }

  /**
   * Build an unsigned reveal transaction for a previously committed operation.
   */
  async buildRevealTransaction(req: RevealRequest): Promise<string> {
    const contract = new Contract(this.contractId);
    const account = await this.getAccount(req.userAddress);

    const methodMap: Record<SensitiveOperation, string> = {
      Borrow: 'reveal_borrow_protected',
      Withdraw: 'reveal_withdraw_protected',
      Liquidate: 'reveal_liquidation_protected',
    };

    const params = [
      new Address(req.userAddress).toScVal(),
      nativeToScVal(BigInt(req.commitId), { type: 'u64' }),
    ];

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(methodMap[req.operation], ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const prepared = await this.server.prepareTransaction(tx);
    return prepared.toXDR();
  }

  // ── Batch Auction ──────────────────────────────────────────────────────────

  /**
   * Build an unsigned transaction to place a bid in the current batch auction.
   */
  async buildPlaceAuctionBidTransaction(req: AuctionBidRequest): Promise<AuctionBidResponse> {
    const contract = new Contract(this.contractId);
    const account = await this.getAccount(req.bidderAddress);

    const params = [
      new Address(req.bidderAddress).toScVal(),
      new Address(req.borrowerAddress).toScVal(),
      nativeToScVal(BigInt(req.debtAmount), { type: 'i128' }),
      nativeToScVal(BigInt(req.minCollateralOut), { type: 'i128' }),
      nativeToScVal(BigInt(req.maxFeeBps), { type: 'i128' }),
      nativeToScVal(BigInt(req.deadline ?? 0), { type: 'u64' }),
    ];

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call('place_auction_bid', ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const prepared = await this.server.prepareTransaction(tx);
    const slotId = await this.getCurrentAuctionSlot();

    return { slotId: slotId.toString(), unsignedXdr: prepared.toXDR() };
  }

  /**
   * Build an unsigned transaction to settle a closed auction slot.
   */
  async buildSettleAuctionTransaction(callerAddress: string, slotId: string): Promise<string> {
    const contract = new Contract(this.contractId);
    const account = await this.getAccount(callerAddress);

    const params = [
      new Address(callerAddress).toScVal(),
      nativeToScVal(BigInt(slotId), { type: 'u64' }),
    ];

    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call('settle_batch_auction', ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const prepared = await this.server.prepareTransaction(tx);
    return prepared.toXDR();
  }

  /**
   * Query the settled result for a given auction slot (read-only simulation).
   */
  async getAuctionResult(slotId: string): Promise<AuctionResult | null> {
    try {
      const raw = await this.simulateCall('get_auction_result', [
        nativeToScVal(BigInt(slotId), { type: 'u64' }),
      ]);
      if (!raw) return null;
      return {
        slotId: raw.slot_id?.toString() ?? slotId,
        bidCount: Number(raw.bid_count ?? 0),
        clearingFeeBps: Number(raw.clearing_fee_bps ?? 0),
        totalDebtLiquidated: raw.total_debt_liquidated?.toString() ?? '0',
        settledAt: Number(raw.settled_at ?? 0),
      };
    } catch {
      return null;
    }
  }

  /**
   * Return the current open auction slot ID.
   */
  async getCurrentAuctionSlot(): Promise<number> {
    try {
      const raw = await this.simulateCall('get_current_auction_slot', []);
      return Number(raw ?? 0);
    } catch {
      return 0;
    }
  }

  // ── Monitoring Dashboard ───────────────────────────────────────────────────

  /**
   * Return the full MEV extraction monitoring dashboard snapshot.
   * Results are cached for `DASHBOARD_CACHE_TTL_SECS` seconds.
   */
  async getDashboard(): Promise<MevDashboardData> {
    const cacheKey = redisCacheService.buildKey('mev', 'dashboard');
    const cached = await redisCacheService.get<MevDashboardData>(cacheKey);
    if (cached) return cached;

    const [statsRaw, cfgRaw, slotRaw] = await Promise.all([
      this.simulateCall('get_mev_ordering_stats', []).catch(() => null),
      this.simulateCall('get_mev_protection_config', []).catch(() => null),
      this.simulateCall('get_current_auction_slot', []).catch(() => 0),
    ]);

    const data: MevDashboardData = {
      orderingStats: {
        suspiciousSequences: Number(statsRaw?.suspicious_sequences ?? 0),
        sandwichAlerts: Number(statsRaw?.sandwich_alerts ?? 0),
        lastAlertTimestamp: Number(statsRaw?.last_alert_timestamp ?? 0),
        lastEffectiveFeeBps: Number(statsRaw?.last_effective_fee_bps ?? 0),
        totalCommits: Number(statsRaw?.total_commits ?? 0),
        totalReveals: Number(statsRaw?.total_reveals ?? 0),
        totalAuctionBids: Number(statsRaw?.total_auction_bids ?? 0),
        totalAuctionsSettled: Number(statsRaw?.total_auctions_settled ?? 0),
        cumulativeFeeBpsCollected: Number(statsRaw?.cumulative_fee_bps_collected ?? 0),
      },
      protectionConfig: {
        commitDelaySecs: Number(cfgRaw?.commit_delay_secs ?? 30),
        commitExpirySecs: Number(cfgRaw?.commit_expiry_secs ?? 300),
        suspiciousWindowSecs: Number(cfgRaw?.suspicious_window_secs ?? 45),
        baseProtectionFeeBps: Number(cfgRaw?.base_protection_fee_bps ?? 10),
        surgeProtectionFeeBps: Number(cfgRaw?.surge_protection_fee_bps ?? 60),
        privateMempoolEnabled: Boolean(cfgRaw?.private_mempool_enabled ?? true),
        batchingEnabled: Boolean(cfgRaw?.batching_enabled ?? true),
        batchWindowSecs: Number(cfgRaw?.batch_window_secs ?? 60),
        defaultMaxSlippageBps: Number(cfgRaw?.default_max_slippage_bps ?? 100),
      },
      currentAuctionSlot: String(slotRaw ?? 0),
      timestamp: new Date().toISOString(),
    };

    await redisCacheService.set(cacheKey, data, DASHBOARD_CACHE_TTL_SECS);
    return data;
  }

  // ── Gas Bidding Analysis ───────────────────────────────────────────────────

  /**
   * Return a gas bidding analysis snapshot for the given operation.
   * Helps callers decide how much to bid to get included without overpaying.
   */
  async getGasBidAnalysis(
    operation: SensitiveOperation,
    assetAddress?: string,
    amount = '0',
  ): Promise<GasBidAnalysis> {
    const cacheKey = redisCacheService.buildKey(
      'mev',
      `gas:${operation}:${assetAddress ?? 'native'}:${amount}`,
    );
    const cached = await redisCacheService.get<GasBidAnalysis>(cacheKey);
    if (cached) return cached;

    try {
      const params = [
        xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(operation)]),
        assetAddress ? new Address(assetAddress).toScVal() : xdr.ScVal.scvVoid(),
        nativeToScVal(BigInt(amount), { type: 'i128' }),
      ];
      const raw = await this.simulateCall('get_gas_bid_analysis', params);

      const result: GasBidAnalysis = {
        smoothedBaseFeeBps: Number(raw?.smoothed_base_fee_bps ?? 10),
        currentSurgeFeeBps: Number(raw?.current_surge_fee_bps ?? 0),
        recommendedBidBps: Number(raw?.recommended_bid_bps ?? 10),
        highCongestionBidBps: Number(raw?.high_congestion_bid_bps ?? 30),
        inSuspiciousWindow: Boolean(raw?.in_suspicious_window ?? false),
        recentSandwichAlerts: Number(raw?.recent_sandwich_alerts ?? 0),
        operation,
        assetAddress,
        timestamp: new Date().toISOString(),
      };

      await redisCacheService.set(cacheKey, result, GAS_ANALYSIS_CACHE_TTL_SECS);
      return result;
    } catch (err) {
      logger.warn('Gas bid analysis simulation failed, returning defaults', { err });
      return {
        smoothedBaseFeeBps: 10,
        currentSurgeFeeBps: 0,
        recommendedBidBps: 10,
        highCongestionBidBps: 30,
        inSuspiciousWindow: false,
        recentSandwichAlerts: 0,
        operation,
        assetAddress,
        timestamp: new Date().toISOString(),
      };
    }
  }

  // ── Private Mempool Routing ────────────────────────────────────────────────

  /**
   * Return routing guidance for the given operation.
   * Combines on-chain hint with human-readable guidance text.
   */
  async getPrivateMempoolRoute(
    operation: SensitiveOperation,
    requestedHint: TxOrderingHint = 'Default',
  ): Promise<PrivateMempoolRoute> {
    const [hintRaw, guidanceRaw, cfgRaw, gasRaw] = await Promise.all([
      this.simulateCall('get_mev_ordering_hint', [
        xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(requestedHint)]),
      ]).catch(() => null),
      this.simulateCall('get_mev_user_guidance', [
        xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(operation)]),
      ]).catch(() => null),
      this.simulateCall('get_mev_protection_config', []).catch(() => null),
      this.getGasBidAnalysis(operation).catch(() => null),
    ]);

    const hint: TxOrderingHint =
      (typeof hintRaw === 'string' ? hintRaw : hintRaw?.toString?.()) as TxOrderingHint
      ?? requestedHint;

    return {
      hint,
      guidance: typeof guidanceRaw === 'string'
        ? guidanceRaw
        : 'Use commit/reveal to protect your transaction from MEV extraction.',
      commitDelaySecs: Number(cfgRaw?.commit_delay_secs ?? 30),
      recommendedFeeBps: gasRaw?.recommendedBidBps ?? 10,
      privateMempoolEnabled: Boolean(cfgRaw?.private_mempool_enabled ?? true),
      batchingEnabled: Boolean(cfgRaw?.batching_enabled ?? true),
    };
  }

  // ── Fee Preview ────────────────────────────────────────────────────────────

  /**
   * Preview the effective MEV protection fee without committing.
   */
  async previewFeeBps(
    operation: SensitiveOperation,
    assetAddress?: string,
    amount = '0',
  ): Promise<number> {
    try {
      const params = [
        xdr.ScVal.scvVec([xdr.ScVal.scvSymbol(operation)]),
        assetAddress ? new Address(assetAddress).toScVal() : xdr.ScVal.scvVoid(),
        nativeToScVal(BigInt(amount), { type: 'i128' }),
      ];
      const raw = await this.simulateCall('preview_mev_fee_bps', params);
      return Number(raw ?? 10);
    } catch {
      return 10;
    }
  }

  // ── Internal Helpers ───────────────────────────────────────────────────────

  private async getProtectionConfig(): Promise<{
    commitDelaySecs: number;
    commitExpirySecs: number;
  }> {
    try {
      const raw = await this.simulateCall('get_mev_protection_config', []);
      return {
        commitDelaySecs: Number(raw?.commit_delay_secs ?? 30),
        commitExpirySecs: Number(raw?.commit_expiry_secs ?? 300),
      };
    } catch {
      return { commitDelaySecs: 30, commitExpirySecs: 300 };
    }
  }

  private async getAccount(address: string): Promise<Account> {
    const axios = (await import('axios')).default;
    try {
      const response = await axios.get(`${this.horizonUrl}/accounts/${address}`);
      const data = response.data as { id: string; sequence: string };
      return new Account(data.id, data.sequence);
    } catch (err) {
      logger.error('Failed to fetch account for MEV tx', { address, err });
      throw new InternalServerError('Failed to fetch account information');
    }
  }

  private async simulateCall(method: string, params: xdr.ScVal[]): Promise<any> {
    const account = new Account(this.readOnlyAccount, '0');
    const contract = new Contract(this.contractId);
    const tx = new TransactionBuilder(account, {
      fee: BASE_FEE,
      networkPassphrase: this.networkPassphrase,
    })
      .addOperation(contract.call(method, ...params))
      .setTimeout(TX_TIMEOUT_SECONDS)
      .build();

    const simulation = await (this.server as any).simulateTransaction(tx);
    return decodeSimResult(simulation);
  }

  private buildCommitParams(req: CommitRequest): xdr.ScVal[] {
    const base = [
      new Address(req.userAddress).toScVal(),
      optionalAddress(req.assetAddress),
      nativeToScVal(BigInt(req.amount), { type: 'i128' }),
      nativeToScVal(BigInt(req.maxFeeBps), { type: 'i128' }),
      hintToScVal(req.hint),
    ];

    if (req.operation === 'Liquidate') {
      // liquidation commits include borrower + secondary asset before amount
      const liquidationBase = [
        new Address(req.userAddress).toScVal(),
        new Address(req.borrowerAddress ?? req.userAddress).toScVal(),
        optionalAddress(req.assetAddress),
        optionalAddress(req.secondaryAssetAddress),
        nativeToScVal(BigInt(req.amount), { type: 'i128' }),
        nativeToScVal(BigInt(req.maxFeeBps), { type: 'i128' }),
        hintToScVal(req.hint),
      ];
      if (req.maxSlippageBps !== undefined || req.deadline !== undefined) {
        liquidationBase.push(
          nativeToScVal(BigInt(req.maxSlippageBps ?? 0), { type: 'i128' }),
          nativeToScVal(BigInt(req.deadline ?? 0), { type: 'u64' }),
        );
      }
      return liquidationBase;
    }

    if (req.maxSlippageBps !== undefined || req.deadline !== undefined) {
      base.push(
        nativeToScVal(BigInt(req.maxSlippageBps ?? 0), { type: 'i128' }),
        nativeToScVal(BigInt(req.deadline ?? 0), { type: 'u64' }),
      );
    }
    return base;
  }
}

export const mevService = new MevService();
