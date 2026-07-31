import dotenv from 'dotenv';
dotenv.config();

export interface BotConfig {
  stellarNetwork: string;
  stellarRpcUrl: string;
  stellarHorizonUrl: string;
  botSecretKey: string;
  botPublicKey: string;
  minProfitThresholdXlm: number;
  maxGasPriceStroops: number;
  liquidationTargetStrategy: 'highest_profit' | 'highest_risk' | 'lowest_hf';
  pollIntervalMs: number;
  dryRun: boolean;
  maxConcurrentLiquidations: number;
  wsUrl: string;
  apiBaseUrl: string;
  logLevel: string;
}

export function loadConfig(): BotConfig {
  return {
    stellarNetwork: process.env.STELLAR_NETWORK || 'testnet',
    stellarRpcUrl: process.env.STELLAR_RPC_URL || 'https://soroban-testnet.stellar.org',
    stellarHorizonUrl: process.env.STELLAR_HORIZON_URL || 'https://horizon-testnet.stellar.org',
    botSecretKey: process.env.BOT_SECRET_KEY || '',
    botPublicKey: process.env.BOT_PUBLIC_KEY || '',
    minProfitThresholdXlm: parseFloat(process.env.MIN_PROFIT_THRESHOLD_XLM || '1.0'),
    maxGasPriceStroops: parseInt(process.env.MAX_GAS_PRICE_STROOPS || '100000', 10),
    liquidationTargetStrategy: (process.env.LIQUIDATION_TARGET_STRATEGY || 'highest_profit') as BotConfig['liquidationTargetStrategy'],
    pollIntervalMs: parseInt(process.env.POLL_INTERVAL_MS || '5000', 10),
    dryRun: process.env.DRY_RUN !== 'false',
    maxConcurrentLiquidations: parseInt(process.env.MAX_CONCURRENT_LIQUIDATIONS || '3', 10),
    wsUrl: process.env.WS_URL || 'ws://localhost:3000/api/ws/health-updates',
    apiBaseUrl: process.env.API_BASE_URL || 'http://localhost:3000/api',
    logLevel: process.env.LOG_LEVEL || 'info',
  };
}

export const botConfig = loadConfig();
