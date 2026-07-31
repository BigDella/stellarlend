import { Contract, Address, TransactionBuilder, scValToNative } from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import { config } from '../config';
import logger from '../utils/logger';

const AUTO_COMPOUND_CONTRACT_ID = process.env.AUTO_COMPOUND_CONTRACT_ID ?? '';

export interface VaultConfig {
  performanceFeeBps: number;
  managementFeeBps: number;
  harvestIntervalSecs: number;
  slippageToleranceBps: number;
  depositPaused: boolean;
  withdrawPaused: boolean;
  active: boolean;
}

export interface VaultSnapshot {
  totalAssets: string;
  totalShares: string;
  sharePrice: string;
  lastHarvestedAt: number;
  accruedManagementFees: string;
  accruedPerformanceFees: string;
}

export interface UserVaultPosition {
  address: string;
  depositedAmount: string;
  sharesOwned: string;
  lastDepositAt: number;
}

class AutoCompoundVaultService {
  private server: SorobanServer;

  private mockConfig: VaultConfig = {
    performanceFeeBps: 500,
    managementFeeBps: 100,
    harvestIntervalSecs: 86400,
    slippageToleranceBps: 100,
    depositPaused: false,
    withdrawPaused: false,
    active: true,
  };

  private mockSnapshot: VaultSnapshot = {
    totalAssets: '5000000',
    totalShares: '4750000',
    sharePrice: '1052631',
    lastHarvestedAt: Math.floor(Date.now() / 1000) - 43200,
    accruedManagementFees: '25000',
    accruedPerformanceFees: '50000',
  };

  private mockPositions: Map<string, UserVaultPosition> = new Map();

  constructor() {
    this.server = new SorobanServer(config.stellar.sorobanRpcUrl);
    this.seedMockPositions();
  }

  private seedMockPositions(): void {
    const now = Math.floor(Date.now() / 1000);
    this.mockPositions.set('GABCDE12345', {
      address: 'GABCDE12345',
      depositedAmount: '100000',
      sharesOwned: '95000',
      lastDepositAt: now - 604800,
    });
    this.mockPositions.set('GFGHIJ67890', {
      address: 'GFGHIJ67890',
      depositedAmount: '250000',
      sharesOwned: '237500',
      lastDepositAt: now - 2592000,
    });
    this.mockPositions.set('GKLMNO11121', {
      address: 'GKLMNO11121',
      depositedAmount: '50000',
      sharesOwned: '47500',
      lastDepositAt: now - 86400,
    });
  }

  async getConfig(): Promise<VaultConfig> {
    if (!AUTO_COMPOUND_CONTRACT_ID) return this.mockConfig;
    try {
      const contract = new Contract(AUTO_COMPOUND_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_config'))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return this.mockConfig;

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      return {
        performanceFeeBps: Number(raw['performance_fee_bps'] ?? 500),
        managementFeeBps: Number(raw['management_fee_bps'] ?? 100),
        harvestIntervalSecs: Number(raw['harvest_interval_secs'] ?? 86400),
        slippageToleranceBps: Number(raw['slippage_tolerance_bps'] ?? 100),
        depositPaused: Boolean(raw['deposit_paused'] ?? false),
        withdrawPaused: Boolean(raw['withdraw_paused'] ?? false),
        active: Boolean(raw['active'] ?? true),
      };
    } catch (err) {
      logger.warn('Auto-compound config fetch failed', { err: String(err) });
      return this.mockConfig;
    }
  }

  async getSnapshot(): Promise<VaultSnapshot> {
    if (!AUTO_COMPOUND_CONTRACT_ID) return this.mockSnapshot;
    try {
      const contract = new Contract(AUTO_COMPOUND_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_vault_snapshot'))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return this.mockSnapshot;

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      return {
        totalAssets: String(raw['total_assets'] ?? '0'),
        totalShares: String(raw['total_shares'] ?? '0'),
        sharePrice: String(raw['share_price'] ?? '0'),
        lastHarvestedAt: Number(raw['last_harvested_at'] ?? 0),
        accruedManagementFees: String(raw['accrued_management_fees'] ?? '0'),
        accruedPerformanceFees: String(raw['accrued_performance_fees'] ?? '0'),
      };
    } catch (err) {
      logger.warn('Snapshot fetch failed', { err: String(err) });
      return this.mockSnapshot;
    }
  }

  async getUserPosition(address: string): Promise<UserVaultPosition | null> {
    if (!AUTO_COMPOUND_CONTRACT_ID) {
      return this.mockPositions.get(address) ?? null;
    }
    try {
      const contract = new Contract(AUTO_COMPOUND_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('preview_deposit', new Address(address).toScVal()))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return null;

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      return {
        address,
        depositedAmount: String(raw['deposited_amount'] ?? '0'),
        sharesOwned: String(raw['shares_owned'] ?? '0'),
        lastDepositAt: Number(raw['last_deposit_at'] ?? 0),
      };
    } catch (err) {
      logger.warn('User position fetch failed', { address, err: String(err) });
      return this.mockPositions.get(address) ?? null;
    }
  }

  async computeApyBoost(compoundingInterval: string): Promise<{ manualApy: number; autoApy: number; boostBps: number }> {
    const baseApy = 8.5;
    const intervalMultipliers: Record<string, number> = {
      hourly: 1.35,
      daily: 1.28,
      weekly: 1.15,
    };
    const multiplier = intervalMultipliers[compoundingInterval] ?? 1.28;
    const autoApy = baseApy * multiplier;
    return {
      manualApy: baseApy,
      autoApy: Math.round(autoApy * 100) / 100,
      boostBps: Math.round((autoApy - baseApy) * 100),
    };
  }
}

export const autoCompoundVaultService = new AutoCompoundVaultService();