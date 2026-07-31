import WebSocket from 'ws';
import axios from 'axios';
import { botConfig, BotConfig } from './config';
import { Logger } from './logger';

interface Auction {
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

interface BidResult {
  auctionId: number;
  success: boolean;
  txHash?: string;
  collateralReceived?: number;
  debtRepaid?: number;
  premiumBps?: number;
  error?: string;
  timestamp: number;
}

interface AuctionAnalytics {
  totalAuctions: number;
  settledAuctions: number;
  avgPremiumBps: number;
  avgTimeToFillSecs: number;
  totalCollateralLiquidated: number;
}

type BidStrategy = 'earliest_discount' | 'highest_premium' | 'fastest_fill';

export class AuctionBidderService {
  private config: BotConfig;
  private logger: Logger;
  private running = false;
  private activeBids = 0;
  private results: BidResult[] = [];
  private ws: WebSocket | null = null;
  private watchedAuctions: Map<number, Auction> = new Map();

  constructor() {
    this.config = botConfig;
    this.logger = new Logger(this.config.logLevel);
  }

  public async start(): Promise<void> {
    this.running = true;
    this.logger.info('Auction bidder starting', {
      dryRun: this.config.dryRun,
      minProfit: this.config.minProfitThresholdXlm,
    });

    this.connectWebSocket();
    this.startAuctionPolling();
  }

  public stop(): void {
    this.running = false;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.watchedAuctions.clear();
    this.logger.info('Auction bidder stopped');
  }

  private connectWebSocket(): void {
    try {
      this.ws = new WebSocket(this.config.wsUrl);

      this.ws.on('open', () => {
        this.logger.info('Auction bidder WebSocket connected');
      });

      this.ws.on('message', (data: string) => {
        try {
          const msg = JSON.parse(data);
          if (msg.type === 'auction_update' || msg.type === 'auction_created') {
            this.handleAuctionUpdate(msg.auctions || []);
          }
        } catch (err) {
          this.logger.error('Failed to parse auction WS message', { error: err });
        }
      });

      this.ws.on('close', () => {
        this.logger.warn('Auction WebSocket disconnected, reconnecting');
        if (this.running) {
          setTimeout(() => this.connectWebSocket(), 5000);
        }
      });

      this.ws.on('error', (err) => {
        this.logger.error('Auction WebSocket error', { error: err.message });
      });
    } catch (err) {
      this.logger.error('Failed to connect auction WebSocket', { error: err });
    }
  }

  private startAuctionPolling(): void {
    const poll = async () => {
      if (!this.running) return;
      try {
        const res = await axios.get(`${this.config.apiBaseUrl}/auctions/active`, {
          timeout: 10000,
        });
        if (res.data?.auctions) {
          this.handleAuctionUpdate(res.data.auctions);
        }
      } catch (err: any) {
        this.logger.warn('Auction poll failed', { error: err.message });
      }
      if (this.running) {
        setTimeout(poll, this.config.pollIntervalMs);
      }
    };
    setTimeout(poll, this.config.pollIntervalMs);
  }

  private handleAuctionUpdate(auctions: Auction[]): void {
    for (const auction of auctions) {
      if (auction.status !== 'Active') {
        this.watchedAuctions.delete(auction.id);
        continue;
      }

      this.watchedAuctions.set(auction.id, auction);

      const discountBps = this.calculateDiscountBps(auction);
      const profitPotential = this.estimateProfit(auction);

      if (
        discountBps >= this.config.minDiscountBps &&
        profitPotential >= this.config.minProfitThresholdXlm * 10_000_000
      ) {
        this.logger.info('Profitable auction detected', {
          auctionId: auction.id,
          discountBps,
          currentPrice: auction.currentPrice,
          profitPotential,
        });
        this.placeBid(auction);
      }
    }
  }

  private calculateDiscountBps(auction: Auction): number {
    if (auction.oraclePrice <= 0) return 0;
    return Math.round(((auction.oraclePrice - auction.currentPrice) * 10_000) / auction.oraclePrice);
  }

  private estimateProfit(auction: Auction): number {
    const collateralValueAtOracle = (auction.collateralAmount * auction.oraclePrice) / 10_000;
    const collateralValueAtCurrent = (auction.collateralAmount * auction.currentPrice) / 10_000;
    const debtRepay = auction.debtAmount;
    return collateralValueAtCurrent - debtRepay;
  }

  private async placeBid(auction: Auction): Promise<void> {
    if (this.activeBids >= this.config.maxConcurrentLiquidations) {
      return;
    }

    this.activeBids++;
    const maxRepay = Math.floor(auction.debtAmount * 0.5);

    this.logger.info('Placing auction bid', {
      auctionId: auction.id,
      maxRepay,
      currentPrice: auction.currentPrice,
    });

    if (this.config.dryRun) {
      this.logger.info('DRY RUN - Would bid on auction', {
        auctionId: auction.id,
        estimatedCollateral: Math.floor(
          (maxRepay * auction.currentPrice) / auction.oraclePrice
        ),
      });

      this.results.push({
        auctionId: auction.id,
        success: true,
        collateralReceived: Math.floor(
          (maxRepay * auction.currentPrice) / auction.oraclePrice
        ),
        debtRepaid: maxRepay,
        premiumBps: this.calculateDiscountBps(auction),
        timestamp: Date.now(),
      });
      this.activeBids--;
      return;
    }

    try {
      const res = await axios.post(
        `${this.config.apiBaseUrl}/auctions/bid`,
        {
          auctionId: auction.id,
          debtRepayAmount: maxRepay,
        },
        { timeout: 30000 }
      );

      this.results.push({
        auctionId: auction.id,
        success: true,
        txHash: res.data?.txHash,
        collateralReceived: res.data?.collateralReceived,
        debtRepaid: maxRepay,
        premiumBps: this.calculateDiscountBps(auction),
        timestamp: Date.now(),
      });

      this.logger.info('Auction bid successful', {
        auctionId: auction.id,
        txHash: res.data?.txHash,
      });
    } catch (err: any) {
      this.results.push({
        auctionId: auction.id,
        success: false,
        error: err.message || 'Bid failed',
        timestamp: Date.now(),
      });
      this.logger.error('Auction bid failed', {
        auctionId: auction.id,
        error: err.message,
      });
    } finally {
      this.activeBids--;
    }
  }

  public getStats(): {
    totalBidAttempted: number;
    totalBidSuccessful: number;
    avgPremiumBps: number;
    totalCollateralReceived: number;
  } {
    const successful = this.results.filter((r) => r.success);
    return {
      totalBidAttempted: this.results.length,
      totalBidSuccessful: successful.length,
      avgPremiumBps:
        successful.length > 0
          ? Math.round(
              successful.reduce((s, r) => s + (r.premiumBps || 0), 0) / successful.length
            )
          : 0,
      totalCollateralReceived: successful.reduce(
        (s, r) => s + (r.collateralReceived || 0),
        0
      ),
    };
  }

  public getResults(): BidResult[] {
    return [...this.results];
  }

  public getWatchedAuctions(): Auction[] {
    return Array.from(this.watchedAuctions.values());
  }
}
