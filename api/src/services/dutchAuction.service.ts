import { Contract, Address, TransactionBuilder, scValToNative } from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import { config } from '../config';
import logger from '../utils/logger';

const DUTCH_AUCTION_CONTRACT_ID = process.env.DUTCH_AUCTION_CONTRACT_ID ?? '';

export interface Auction {
  id: number;
  pool: string;
  collateralAsset: string;
  debtAsset: string;
  collateralAmount: number;
  debtAmount: number;
  oraclePrice: number;
  startPrice: number;
  currentPrice: number;
  startTime: number;
  endTime: number;
  status: 'Active' | 'Settled' | 'Expired';
  borrower: string;
  highestBidder: string | null;
  highestBidAmount: number | null;
}

export interface AuctionAnalytics {
  totalAuctions: number;
  settledAuctions: number;
  avgPremiumBps: number;
  avgTimeToFillSecs: number;
  totalCollateralLiquidated: number;
}

export interface AuctionBidRequest {
  auctionId: number;
  bidder: string;
  debtRepayAmount: string;
}

class DutchAuctionService {
  private server: SorobanServer;
  private mockAuctions: Auction[] = [];
  private mockAnalytics: AuctionAnalytics = {
    totalAuctions: 0,
    settledAuctions: 0,
    avgPremiumBps: 0,
    avgTimeToFillSecs: 0,
    totalCollateralLiquidated: 0,
  };
  private nextId = 1;

  constructor() {
    this.server = new SorobanServer(config.stellar.sorobanRpcUrl);
    this.seedMockData();
  }

  private seedMockData(): void {
    const now = Math.floor(Date.now() / 1000);
    this.mockAuctions = [
      {
        id: this.nextId++,
        pool: 'CAQJ7R3X7R7X7R7X7R7X7R7X7R7X7R7X7R7X7R7X',
        collateralAsset: 'XLM',
        debtAsset: 'USDC',
        collateralAmount: 100000,
        debtAmount: 50000,
        oraclePrice: 20000,
        startPrice: 20000,
        currentPrice: 18500,
        startTime: now - 1200,
        endTime: now + 2400,
        status: 'Active',
        borrower: 'GABCDE12345',
        highestBidder: null,
        highestBidAmount: null,
      },
      {
        id: this.nextId++,
        pool: 'CBQJ7R3X7R7X7R7X7R7X7R7X7R7X7R7X7R7X7R7Y',
        collateralAsset: 'USDC',
        debtAsset: 'XLM',
        collateralAmount: 50000,
        debtAmount: 25000,
        oraclePrice: 10000,
        startPrice: 10000,
        currentPrice: 8500,
        startTime: now - 3600,
        endTime: now + 1800,
        status: 'Active',
        borrower: 'GFGHIJ67890',
        highestBidder: null,
        highestBidAmount: null,
      },
      {
        id: this.nextId++,
        pool: 'CCQJ7R3X7R7X7R7X7R7X7R7X7R7X7R7X7R7X7R7Z',
        collateralAsset: 'XLM',
        debtAsset: 'USDT',
        collateralAmount: 250000,
        debtAmount: 120000,
        oraclePrice: 30000,
        startPrice: 30000,
        currentPrice: 21000,
        startTime: now - 7200,
        endTime: now - 3600,
        status: 'Settled',
        borrower: 'GKLMNO11121',
        highestBidder: 'GPQRST31415',
        highestBidAmount: 120000,
      },
    ];

    this.mockAnalytics = {
      totalAuctions: 5,
      settledAuctions: 2,
      avgPremiumBps: 1500,
      avgTimeToFillSecs: 5400,
      totalCollateralLiquidated: 350000,
    };
  }

  async getAuctions(filter?: string, sortBy?: string, sortDir?: string): Promise<Auction[]> {
    if (!DUTCH_AUCTION_CONTRACT_ID) {
      let auctions = [...this.mockAuctions];
      if (filter && filter !== 'all') {
        auctions = auctions.filter((a) => a.status.toLowerCase() === filter.toLowerCase());
      }
      if (sortBy) {
        auctions.sort((a, b) => {
          const aVal = (a as any)[sortBy] ?? 0;
          const bVal = (b as any)[sortBy] ?? 0;
          return sortDir === 'desc' ? (aVal > bVal ? -1 : 1) : (aVal < bVal ? -1 : 1);
        });
      }
      return auctions;
    }

    try {
      const contract = new Contract(DUTCH_AUCTION_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_active_auctions'))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return this.mockAuctions;

      const raw = scValToNative(sim.result.retval) as any[];
      return raw.map((a: any) => this.mapContractAuction(a));
    } catch (err) {
      logger.warn('Dutch auction contract simulation failed', { err: String(err) });
      return this.mockAuctions;
    }
  }

  async getAnalytics(): Promise<AuctionAnalytics> {
    if (!DUTCH_AUCTION_CONTRACT_ID) return this.mockAnalytics;

    try {
      const contract = new Contract(DUTCH_AUCTION_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_analytics'))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return this.mockAnalytics;

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      return {
        totalAuctions: Number(raw['total_auctions'] ?? 0),
        settledAuctions: Number(raw['settled_auctions'] ?? 0),
        avgPremiumBps: Number(raw['avg_premium_bps'] ?? 0),
        avgTimeToFillSecs: Number(raw['avg_time_to_fill_secs'] ?? 0),
        totalCollateralLiquidated: Number(raw['total_collateral_liquidated'] ?? 0),
      };
    } catch (err) {
      logger.warn('Analytics fetch failed', { err: String(err) });
      return this.mockAnalytics;
    }
  }

  async placeBid(auctionId: number, bidder: string, amount: string): Promise<void> {
    if (!DUTCH_AUCTION_CONTRACT_ID) {
      const idx = this.mockAuctions.findIndex((a) => a.id === auctionId);
      if (idx >= 0 && this.mockAuctions[idx].status === 'Active') {
        this.mockAuctions[idx].status = 'Settled';
        this.mockAuctions[idx].highestBidder = bidder;
        this.mockAuctions[idx].highestBidAmount = parseInt(amount);
      }
      return;
    }

    try {
      const contract = new Contract(DUTCH_AUCTION_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(
          contract.call('place_bid', ...[
            new Address(bidder).toScVal(),
            (auctionId as any),
            (amount as any),
          ] as any[])
        )
        .setTimeout(30)
        .build();

      await this.server.simulateTransaction(tx);
    } catch (err) {
      logger.warn('Bid placement failed', { auctionId, bidder, err: String(err) });
      throw err;
    }
  }

  private mapContractAuction(raw: any): Auction {
    return {
      id: Number(raw.id ?? 0),
      pool: (raw.config?.pool?.toString() ?? ''),
      collateralAsset: (raw.config?.collateral_asset?.toString() ?? ''),
      debtAsset: (raw.config?.debt_asset?.toString() ?? ''),
      collateralAmount: Number(raw.config?.collateral_amount ?? 0),
      debtAmount: Number(raw.config?.debt_amount ?? 0),
      oraclePrice: Number(raw.config?.oracle_price ?? 0),
      startPrice: Number(raw.start_price ?? 0),
      currentPrice: Number(raw.current_price ?? 0),
      startTime: Number(raw.start_time ?? 0),
      endTime: Number(raw.end_time ?? 0),
      status: String(raw.status ?? 'Active') as Auction['status'],
      borrower: (raw.borrower?.toString() ?? ''),
      highestBidder: raw.highest_bidder?.toString() ?? null,
      highestBidAmount: raw.highest_bid_amount != null ? Number(raw.highest_bid_amount) : null,
    };
  }
}

export const dutchAuctionService = new DutchAuctionService();