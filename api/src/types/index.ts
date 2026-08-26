export interface DepositRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface BorrowRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface RepayRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface WithdrawRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export type LendingOperation = 'deposit' | 'borrow' | 'repay' | 'withdraw';

export interface PrepareRequest {
  userAddress: string;
  assetAddress?: string;
  amount: string;
}

export interface PrepareResponse {
  unsignedXdr: string;
  operation: LendingOperation;
  expiresAt: string;
}

export interface SubmitRequest {
  signedXdr: string;
  operation?: LendingOperation;
  userAddress?: string;
  amount?: string;
  assetAddress?: string;
}

export interface TransactionResponse {
  success: boolean;
  transactionHash?: string;
  status: 'pending' | 'success' | 'failed' | 'cancelled';
  message?: string;
  error?: string;
  ledger?: number;
  /**
   * Optional raw provider payload for debugging (e.g. Horizon or RPC result details).
   * Kept generic since provider response shapes differ.
   */
  details?: unknown;
}

export interface PositionResponse {
  userAddress: string;
  collateral: string;
  debt: string;
  borrowInterest: string;
  lastAccrualTime: number;
  collateralRatio?: string;
}

export interface HealthCheckResponse {
  status: 'healthy' | 'unhealthy';
  timestamp: string;
  services: {
    horizon: boolean;
    sorobanRpc: boolean;
  };
}

export interface ProtocolStatsResponse {
  totalDeposits: string;
  totalBorrows: string;
  utilizationRate: string;
  numberOfUsers: number;
  tvl: string;
  stablecoinStats?: StablecoinAssetStats[];
}

export interface StablecoinAssetStats {
  asset: string;
  price: string;
  targetPrice: string;
  deviationBps: number;
  stabilityFeeBps: number;
  isDepegged: boolean;
}

export enum TransactionStatus {
  PENDING = 'pending',
  SUCCESS = 'success',
  FAILED = 'failed',
  NOT_FOUND = 'not_found',
}

// ─── WebSocket Types ───────────────────────────────────────────────────────────

export interface PriceData {
  asset: string;
  price: number;
  timestamp: number;
}

export interface WsSubscribeMessage {
  type: 'subscribe';
  /** Asset symbols to subscribe to, e.g. ["XLM","BTC"] or ["*"] for all */
  assets: string[];
}

export interface WsUnsubscribeMessage {
  type: 'unsubscribe';
  assets: string[];
}

export interface WsPingMessage {
  type: 'ping';
}

export type ClientMessage = WsSubscribeMessage | WsUnsubscribeMessage | WsPingMessage;

export type ServerMessage =
  | { type: 'price_update'; asset: string; price: number; timestamp: number }
  | { type: 'subscribed'; assets: string[] }
  | { type: 'unsubscribed'; assets: string[] }
  | { type: 'pong' }
  | { type: 'error'; message: string };

// ─── Transaction History Types ──────────────────────────────────────────────────

export interface TransactionHistoryItem {
  transactionHash: string;
  type: LendingOperation;
  amount: string;
  assetAddress?: string;
  timestamp: string;
  status: 'success' | 'failed' | 'pending';
  ledger?: number;
  memo?: string;
}

export * from './subscriptions';
import { PaginatedResponse, PaginationParams } from './pagination';

export type TransactionHistoryResponse = PaginatedResponse<TransactionHistoryItem>;

export interface TransactionHistoryQuery extends PaginationParams {
  userAddress: string;
}
