import { WebSocket, WebSocketServer } from 'ws';
import { IncomingMessage } from 'http';
import { Server } from 'http';
import axios from 'axios';
import logger from '../utils/logger';
import { config } from '../config/index';
import {
  PriceData,
  ClientMessage,
  ServerMessage,
  WsSubscribeMessage,
  WsUnsubscribeMessage,
} from '../types/index';
import { WsAnalyticsMessage } from '../types/analytics';

const SUPPORTED_ASSETS = ['XLM', 'USDC', 'BTC', 'ETH', 'SOL'];
const ANALYTICS_CHANNELS = ['apy', 'utilization', 'revenue'] as const;
const MARKET_DATA_CHANNELS = ['pools', 'liquidations', 'rates'] as const;
const MAX_SUBSCRIPTIONS_PER_CONNECTION = 20;
const THROTTLE_INTERVAL_MS = 1000;
const RECONNECT_STATE_TTL_MS = 300000;

type MarketDataChannel = typeof MARKET_DATA_CHANNELS[number];

interface ClientState {
  subscriptions: Set<string>;
  analyticsSubscriptions: Set<string>;
  marketDataSubscriptions: Set<MarketDataChannel>;
  lastUpdateSent: Map<string, number>;
  reconnectToken?: string;
  authenticated: boolean;
  userId?: string;
}

interface ReconnectState {
  subscriptions: Set<string>;
  analyticsSubscriptions: Set<string>;
  marketDataSubscriptions: Set<MarketDataChannel>;
  timestamp: number;
}

interface PoolUpdate {
  poolAddress: string;
  depositApy: number;
  borrowApy: number;
  utilizationRate: number;
  totalDeposits: string;
  totalBorrows: string;
  timestamp: number;
}

interface LiquidationEvent {
  poolAddress: string;
  userAddress: string;
  collateralSeized: string;
  debtRepaid: string;
  timestamp: number;
}

interface RateUpdate {
  assetAddress: string;
  borrowRate: number;
  supplyRate: number;
  timestamp: number;
}

const COINGECKO_IDS: Record<string, string> = {
  XLM: 'stellar',
  USDC: 'usd-coin',
  BTC: 'bitcoin',
  ETH: 'ethereum',
  SOL: 'solana',
};

export class PriceWebSocketServer {
  private wss: WebSocketServer;
  private clientStates: Map<WebSocket, ClientState> = new Map();
  private reconnectStates: Map<string, ReconnectState> = new Map();
  private lastPrices: Map<string, PriceData> = new Map();
  private lastPoolUpdates: Map<string, PoolUpdate> = new Map();
  private lastRateUpdates: Map<string, RateUpdate> = new Map();
  private recentLiquidations: LiquidationEvent[] = [];
  private pollIntervalId?: ReturnType<typeof setInterval>;
  private heartbeatIntervalId?: ReturnType<typeof setInterval>;
  private cleanupIntervalId?: ReturnType<typeof setInterval>;
  private marketDataIntervalId?: ReturnType<typeof setInterval>;

  constructor(server: Server) {
    this.wss = new WebSocketServer({ server, path: '/api/ws/prices' });
    this.setupConnectionHandler();
    this.startPricePolling();
    this.startHeartbeat();
    this.startCleanup();
    this.startMarketDataPolling();
    logger.info('WebSocket price server initialised at /api/ws/prices');
  }

  private setupConnectionHandler(): void {
    this.wss.on('connection', (ws: WebSocket, req: IncomingMessage) => {
      const urlParams = new URL(req.url || '', `http://${req.headers.host}`);
      const reconnectToken = urlParams.searchParams.get('reconnectToken') || undefined;
      const authToken = urlParams.searchParams.get('authToken') || undefined;

      logger.info('WebSocket client connected', { 
        ip: req.socket.remoteAddress,
        reconnectToken: reconnectToken ? 'present' : 'none',
      });

      const clientState: ClientState = {
        subscriptions: new Set(),
        analyticsSubscriptions: new Set(),
        marketDataSubscriptions: new Set(),
        lastUpdateSent: new Map(),
        reconnectToken: reconnectToken || this.generateReconnectToken(),
        authenticated: false,
      };

      if (reconnectToken && this.reconnectStates.has(reconnectToken)) {
        const savedState = this.reconnectStates.get(reconnectToken)!;
        if (Date.now() - savedState.timestamp < RECONNECT_STATE_TTL_MS) {
          clientState.subscriptions = new Set(savedState.subscriptions);
          clientState.analyticsSubscriptions = new Set(savedState.analyticsSubscriptions);
          clientState.marketDataSubscriptions = new Set(savedState.marketDataSubscriptions);
          logger.info('Client reconnected with previous state', { reconnectToken });
        }
        this.reconnectStates.delete(reconnectToken);
      }

      if (authToken) {
        clientState.authenticated = this.validateAuthToken(authToken);
        if (clientState.authenticated) {
          clientState.userId = this.extractUserIdFromToken(authToken);
        }
      }

      this.clientStates.set(ws, clientState);

      ws.on('message', (data) => {
        try {
          const msg: ClientMessage = JSON.parse(data.toString());
          this.handleClientMessage(ws, msg);
        } catch {
          this.send(ws, { type: 'error', message: 'Invalid JSON message' });
        }
      });

      ws.on('close', () => {
        const state = this.clientStates.get(ws);
        if (state?.reconnectToken) {
          this.reconnectStates.set(state.reconnectToken, {
            subscriptions: new Set(state.subscriptions),
            analyticsSubscriptions: new Set(state.analyticsSubscriptions),
            marketDataSubscriptions: new Set(state.marketDataSubscriptions),
            timestamp: Date.now(),
          });
        }
        this.clientStates.delete(ws);
        logger.info('WebSocket client disconnected');
      });

      ws.on('error', (err) => {
        logger.error('WebSocket client error', { error: err.message });
        this.clientStates.delete(ws);
      });
    });
  }

  private generateReconnectToken(): string {
    return `rc_${Date.now()}_${Math.random().toString(36).slice(2, 10)}`;
  }

  private validateAuthToken(token: string): boolean {
    return token.length > 10;
  }

  private extractUserIdFromToken(token: string): string | undefined {
    return 'user_' + token.slice(0, 8);
  }

  private canSendThrottled(ws: WebSocket, channel: string): boolean {
    const state = this.clientStates.get(ws);
    if (!state) return false;

    const now = Date.now();
    const lastSent = state.lastUpdateSent.get(channel) || 0;
    
    if (now - lastSent >= THROTTLE_INTERVAL_MS) {
      state.lastUpdateSent.set(channel, now);
      return true;
    }
    return false;
  }

  private handleClientMessage(ws: WebSocket, msg: ClientMessage): void {
    const state = this.clientStates.get(ws);
    if (!state) return;

    switch (msg.type) {
      case 'subscribe': {
        const requested = (msg as WsSubscribeMessage).assets;
        const toSubscribe = requested.includes('*')
          ? SUPPORTED_ASSETS
          : requested.map((a) => a.toUpperCase()).filter((a) => SUPPORTED_ASSETS.includes(a));

        const availableSlots = MAX_SUBSCRIPTIONS_PER_CONNECTION - state.subscriptions.size;
        const finalSubscribe = toSubscribe.slice(0, availableSlots);

        finalSubscribe.forEach((a) => state.subscriptions.add(a));
        this.send(ws, { type: 'subscribed', assets: finalSubscribe });

        finalSubscribe.forEach((asset) => {
          const cached = this.lastPrices.get(asset);
          if (cached && this.canSendThrottled(ws, asset)) {
            this.send(ws, {
              type: 'price_update',
              asset: cached.asset,
              price: cached.price,
              timestamp: cached.timestamp,
            });
          }
        });
        break;
      }

      case 'unsubscribe': {
        const assets = (msg as WsUnsubscribeMessage).assets.map((a) => a.toUpperCase());
        assets.forEach((a) => {
          state.subscriptions.delete(a);
          state.lastUpdateSent.delete(a);
        });
        this.send(ws, { type: 'unsubscribed', assets });
        break;
      }

      case 'subscribe_analytics': {
        const channels = (msg as any).channels as string[];
        const valid = channels.filter((c: string) =>
          ANALYTICS_CHANNELS.includes(c as typeof ANALYTICS_CHANNELS[number])
        );
        const availableSlots = MAX_SUBSCRIPTIONS_PER_CONNECTION - state.analyticsSubscriptions.size;
        const finalSubscribe = valid.slice(0, availableSlots);
        
        finalSubscribe.forEach((c: string) => state.analyticsSubscriptions.add(c));
        this.send(ws, { type: 'subscribed_analytics', channels: finalSubscribe } as any);
        break;
      }

      case 'unsubscribe_analytics': {
        const channels = (msg as any).channels as string[];
        channels.forEach((c: string) => state.analyticsSubscriptions.delete(c));
        this.send(ws, { type: 'unsubscribed_analytics', channels } as any);
        break;
      }

      case 'subscribe_market_data': {
        const channels = (msg as any).channels as MarketDataChannel[];
        const valid = channels.filter((c) =>
          MARKET_DATA_CHANNELS.includes(c)
        );
        const availableSlots = MAX_SUBSCRIPTIONS_PER_CONNECTION - state.marketDataSubscriptions.size;
        const finalSubscribe = valid.slice(0, availableSlots);
        
        finalSubscribe.forEach((c) => state.marketDataSubscriptions.add(c));
        this.send(ws, { type: 'subscribed_market_data', channels: finalSubscribe } as any);

        finalSubscribe.forEach((channel) => {
          if (channel === 'pools') {
            this.sendPoolsState(ws);
          } else if (channel === 'rates') {
            this.sendRatesState(ws);
          } else if (channel === 'liquidations') {
            this.sendLiquidationsState(ws);
          }
        });
        break;
      }

      case 'unsubscribe_market_data': {
        const channels = (msg as any).channels as MarketDataChannel[];
        channels.forEach((c) => state.marketDataSubscriptions.delete(c));
        this.send(ws, { type: 'unsubscribed_market_data', channels } as any);
        break;
      }

      case 'get_reconnect_token': {
        const token = state.reconnectToken || this.generateReconnectToken();
        state.reconnectToken = token;
        this.send(ws, { type: 'reconnect_token', token } as any);
        break;
      }

      case 'ping':
        this.send(ws, { type: 'pong' });
        break;

      default:
        this.send(ws, { type: 'error', message: 'Unknown message type' });
    }
  }

  private sendPoolsState(ws: WebSocket): void {
    const state = this.clientStates.get(ws);
    if (!state || !state.marketDataSubscriptions.has('pools')) return;

    const pools = Array.from(this.lastPoolUpdates.values());
    this.send(ws, { 
      type: 'market_data_snapshot', 
      channel: 'pools', 
      data: pools,
      timestamp: Date.now(),
    } as any);
  }

  private sendRatesState(ws: WebSocket): void {
    const state = this.clientStates.get(ws);
    if (!state || !state.marketDataSubscriptions.has('rates')) return;

    const rates = Array.from(this.lastRateUpdates.values());
    this.send(ws, { 
      type: 'market_data_snapshot', 
      channel: 'rates', 
      data: rates,
      timestamp: Date.now(),
    } as any);
  }

  private sendLiquidationsState(ws: WebSocket): void {
    const state = this.clientStates.get(ws);
    if (!state || !state.marketDataSubscriptions.has('liquidations')) return;

    const recent = this.recentLiquidations.slice(-50);
    this.send(ws, { 
      type: 'market_data_snapshot', 
      channel: 'liquidations', 
      data: recent,
      timestamp: Date.now(),
    } as any);
  }

  private send(ws: WebSocket, msg: ServerMessage): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    }
  }

  async fetchPrices(): Promise<Map<string, number>> {
    const prices = new Map<string, number>();

    // Prefer oracle API URL if configured
    const oracleUrl = config.ws.oracleApiUrl;
    if (oracleUrl) {
      try {
        const response = await axios.get<Record<string, number>>(`${oracleUrl}/prices`, {
          timeout: 5000,
        });
        Object.entries(response.data).forEach(([asset, price]) =>
          prices.set(asset.toUpperCase(), price)
        );
        if (prices.size > 0) return prices;
      } catch (err) {
        logger.warn('Oracle API price fetch failed, falling back to CoinGecko', { err });
      }
    }

    // Fallback: CoinGecko public API
    try {
      const ids = SUPPORTED_ASSETS.map((a) => COINGECKO_IDS[a]).join(',');
      const response = await axios.get<Record<string, Record<string, number>>>(
        `https://api.coingecko.com/api/v3/simple/price?ids=${ids}&vs_currencies=usd`,
        { timeout: 8000 }
      );

      SUPPORTED_ASSETS.forEach((asset) => {
        const id = COINGECKO_IDS[asset];
        if (!id) return;
        const price = response.data[id]?.usd;
        if (price !== undefined) {
          prices.set(asset, price);
        }
      });
    } catch (err) {
      logger.error('CoinGecko price fetch failed', { err });
    }

    return prices;
  }

  async pollAndBroadcast(): Promise<void> {
    const prices = await this.fetchPrices();
    const now = Math.floor(Date.now() / 1000);

    prices.forEach((price, asset) => {
      const last = this.lastPrices.get(asset);
      const changed = !last || last.price !== price;

      const update: PriceData = { asset, price, timestamp: now };
      this.lastPrices.set(asset, update);

      if (changed) {
        this.broadcastPriceUpdate(asset, update);
      }
    });
  }

  broadcastAnalyticsUpdate(channel: string, data: unknown): void {
    const msg: WsAnalyticsMessage = {
      type: 'analytics_update',
      channel: channel as WsAnalyticsMessage['channel'],
      data: data as any,
      timestamp: Date.now(),
    };
    this.clientStates.forEach((state, ws) => {
      if (state.analyticsSubscriptions.has(channel)) {
        this.send(ws, msg as any);
      }
    });
  }

  private broadcastPriceUpdate(asset: string, data: PriceData): void {
    const msg: ServerMessage = {
      type: 'price_update',
      asset: data.asset,
      price: data.price,
      timestamp: data.timestamp,
    };

    this.clientStates.forEach((state, ws) => {
      if (state.subscriptions.has(asset) && this.canSendThrottled(ws, asset)) {
        this.send(ws, msg);
      }
    });
  }

  private broadcastMarketDataUpdate(channel: MarketDataChannel, data: unknown): void {
    this.clientStates.forEach((state, ws) => {
      if (state.marketDataSubscriptions.has(channel) && this.canSendThrottled(ws, channel)) {
        this.send(ws, {
          type: 'market_data_update',
          channel,
          data,
          timestamp: Date.now(),
        } as any);
      }
    });
  }

  public broadcastLiquidation(event: LiquidationEvent): void {
    this.recentLiquidations.push(event);
    if (this.recentLiquidations.length > 1000) {
      this.recentLiquidations = this.recentLiquidations.slice(-500);
    }
    this.broadcastMarketDataUpdate('liquidations', event);
  }

  public updatePoolData(poolUpdate: PoolUpdate): void {
    this.lastPoolUpdates.set(poolUpdate.poolAddress, poolUpdate);
    this.broadcastMarketDataUpdate('pools', poolUpdate);
  }

  public updateRateData(rateUpdate: RateUpdate): void {
    this.lastRateUpdates.set(rateUpdate.assetAddress, rateUpdate);
    this.broadcastMarketDataUpdate('rates', rateUpdate);
  }

  private startMarketDataPolling(): void {
    const pollMarketData = async () => {
      try {
        const oracleUrl = config.ws.oracleApiUrl;
        if (!oracleUrl) return;

        const response = await axios.get(`${oracleUrl}/market-data`, {
          timeout: 5000,
        });

        if (response.data?.pools) {
          response.data.pools.forEach((pool: any) => {
            this.updatePoolData({
              poolAddress: pool.address,
              depositApy: pool.depositApy,
              borrowApy: pool.borrowApy,
              utilizationRate: pool.utilizationRate,
              totalDeposits: pool.totalDeposits,
              totalBorrows: pool.totalBorrows,
              timestamp: Date.now(),
            });
          });
        }

        if (response.data?.rates) {
          response.data.rates.forEach((rate: any) => {
            this.updateRateData({
              assetAddress: rate.asset,
              borrowRate: rate.borrowRate,
              supplyRate: rate.supplyRate,
              timestamp: Date.now(),
            });
          });
        }
      } catch (err) {
        logger.warn('Market data poll failed, using fallback polling', { err });
      }
    };

    pollMarketData().catch(() => {});
    this.marketDataIntervalId = setInterval(pollMarketData, 5000);
  }

  private startCleanup(): void {
    this.cleanupIntervalId = setInterval(() => {
      const now = Date.now();
      const expiredTokens: string[] = [];

      this.reconnectStates.forEach((state, token) => {
        if (now - state.timestamp > RECONNECT_STATE_TTL_MS) {
          expiredTokens.push(token);
        }
      });

      expiredTokens.forEach((token) => this.reconnectStates.delete(token));

      if (expiredTokens.length > 0) {
        logger.debug('Cleaned up expired reconnect tokens', { count: expiredTokens.length });
      }
    }, 60000);
  }

  private startPricePolling(): void {
    this.pollAndBroadcast().catch((err) => logger.error('Initial price poll failed', { err }));

    this.pollIntervalId = setInterval(() => {
      this.pollAndBroadcast().catch((err) => logger.error('Price poll cycle failed', { err }));
    }, config.ws.priceUpdateIntervalMs);
  }

  private startHeartbeat(): void {
    this.heartbeatIntervalId = setInterval(() => {
      this.wss.clients.forEach((ws) => {
        if (ws.readyState === WebSocket.OPEN) {
          ws.ping();
        } else {
          this.clientStates.delete(ws);
        }
      });
    }, config.ws.heartbeatIntervalMs);
  }

  close(): void {
    if (this.pollIntervalId) clearInterval(this.pollIntervalId);
    if (this.heartbeatIntervalId) clearInterval(this.heartbeatIntervalId);
    if (this.cleanupIntervalId) clearInterval(this.cleanupIntervalId);
    if (this.marketDataIntervalId) clearInterval(this.marketDataIntervalId);
    this.wss.close();
  }

  get clientCount(): number {
    return this.wss.clients.size;
  }

  get supportedAssets(): string[] {
    return [...SUPPORTED_ASSETS];
  }

  get marketDataChannels(): string[] {
    return [...MARKET_DATA_CHANNELS];
  }

  get reconnectTokensActive(): number {
    return this.reconnectStates.size;
  }
}

export function createPriceWebSocket(server: Server): PriceWebSocketServer {
  return new PriceWebSocketServer(server);
}
