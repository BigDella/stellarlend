import { createServer } from 'http';
import app from './app';
import { config } from './config';
import logger from './utils/logger';
import { createPriceWebSocket } from './ws/priceWebSocket';
import { createHealthWebSocket } from './ws/healthWebSocket';
import { createCollateralRatioWebSocket } from './ws/collateralRatioWebSocket';
import { SubscriptionService } from './services/subscription.service';
import { startRiskEngineScheduler } from './services/risk-engine';
import { startPoolSnapshotCron } from './jobs/poolSnapshot.job';

const PORT = config.server.port;

const server = createServer(app);

// Attach WebSocket price, health, and collateral ratio servers to the same HTTP server
createPriceWebSocket(server);
createHealthWebSocket(server);
createCollateralRatioWebSocket(server);

// Start subscription keeper for recurring operations
const subscriptionService = new SubscriptionService();
subscriptionService.startKeeper();

// Start risk engine hourly recalculation scheduler
startRiskEngineScheduler();
startPoolSnapshotCron();

server.listen(PORT, () => {
  logger.info(`StellarLend API server running on port ${PORT}`);
  logger.info(`Environment: ${config.server.env}`);
  logger.info(`Network: ${config.stellar.network}`);
  logger.info(`WebSocket price feed: ws://localhost:${PORT}/api/ws/prices`);
});

process.on('unhandledRejection', (reason, promise) => {
  logger.error('Unhandled Rejection at:', promise, 'reason:', reason);
  process.exit(1);
});

process.on('uncaughtException', (error) => {
  logger.error('Uncaught Exception:', error);
  process.exit(1);
});

const shutdown = (signal: string) => {
  logger.info(`${signal} received, shutting down gracefully`);
  server.close(() => {
    logger.info('Server closed, all in-flight requests completed');
    process.exit(0);
  });
  setTimeout(() => {
    logger.error('Graceful shutdown timed out, forcing exit');
    process.exit(1);
  }, 10000);
};

process.on('SIGTERM', () => shutdown('SIGTERM'));
process.on('SIGINT', () => shutdown('SIGINT'));