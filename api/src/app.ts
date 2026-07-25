import express, { Application, Request, Response, NextFunction } from 'express';
import helmet from 'helmet';
import cors from 'cors';
import rateLimit, { MemoryStore } from 'express-rate-limit';
import { config } from './config';
import { bodySizeLimitMiddleware } from './middleware/bodySizeLimit';
// Versioned domain route imports (v1)
import v1Routes from './routes/v1';

// Legacy route imports for backward compatibility
import lendingRoutes from './routes/lending.routes';
import healthRoutes from './routes/health.routes';
import protocolRoutes from './routes/protocol.routes';
import subscriptionRoutes from './routes/subscription.routes';
import portfolioRoutes from './routes/portfolio.routes';
import gasRoutes from './routes/gas.routes';
import stakingRoutes from './routes/staking.routes';
import transactionRoutes from './routes/transaction.routes';
import merkleRoutes from './routes/merkle.routes';
import zkProofRoutes from './routes/zkProof.routes';
import verificationRoutes from './routes/verification.routes';
import configRoutes from './routes/config.routes';
import analyticsRoutes from './routes/analytics.routes';
import poolPerformanceRoutes from './routes/poolPerformance.routes';
import migrationRoutes from './routes/migration.routes';
import ratesRoutes from './routes/rates.routes';
import crossProtocolRoutes from './routes/crossProtocol.routes';
import developerRoutes from './routes/developer.routes';
import mevRoutes from './routes/mev.routes';
import reputationRoutes from './routes/reputation.routes';
import socialRoutes from './routes/social.routes';
import notificationRoutes from './routes/notification.routes';
import disputeRoutes from './routes/dispute.routes';
import creditRoutes from './routes/credit.routes';
import nonceRoutes from './routes/nonce.routes';
import riskEngineRoutes from './routes/riskEngine.routes';
import yieldCurveRoutes from './routes/yieldCurve.routes';
import rateForecastRoutes from './routes/rateForecast.routes';
import liquidationDashboardRoutes from './routes/liquidationDashboard.routes';
import opportunityExplorerRoutes from './routes/opportunityExplorer.routes';
import { treasuryRoutes } from './routes/treasury.routes';
import metricsRoutes from './routes/metrics.routes';
import referralRoutes from './routes/referral.routes';
import snsRoutes from './routes/sns.routes';
import simulatorRoutes from './routes/simulator.routes';
import emergencyRoutes from './routes/emergency.routes';

import compression from 'compression';
import { errorHandler } from './middleware/errorHandler';
import { idempotencyMiddleware } from './middleware/idempotency';
import { resetSensitiveRateLimits, sensitiveOperationRateLimiter } from './middleware/rate-limit';
import { swaggerSpec, versionListHandler, v1Spec } from './config/swagger';
import { versionMiddleware, legacyCompatibilityMiddleware } from './middleware/versioning';
import logger from './utils/logger';
import { requestIdMiddleware } from './middleware/requestId';
import { requestLogger } from './middleware/requestLogger';
import { sanitizeInput } from './middleware/sanitizeInput';
import { fieldSelectionMiddleware } from './middleware/fieldSelection';
import { redisCacheService } from './services/redisCache.service';

const app: Application = express();
app.use(requestIdMiddleware);
app.use(requestLogger);

const ipRateLimitStore = new MemoryStore();
const userRateLimitStore = new MemoryStore();

app.use(
  helmet({
    hsts: {
      maxAge: 31536000,
      includeSubDomains: true,
      preload: true,
    },
  })
);

// Compress responses (gzip/deflate/br) when the client supports it.
// `threshold` skips compressing tiny responses where the gzip overhead
// would outweigh the bandwidth savings.
app.use(
  compression({
    threshold: 1024, // don't bother compressing responses under 1KB
  })
);

// Enforce HTTPS in production
if (config.server.env === 'production') {
  app.use((req, res, next) => {
    if (req.header('x-forwarded-proto') !== 'https' && !req.secure) {
      return res.redirect(`https://${req.header('host')}${req.url}`);
    }
    next();
  });
}

const corsOptions: cors.CorsOptions = {
  origin: (origin, callback) => {
    const allowed = config.cors.allowedOrigins;
    // Allow server-to-server (no Origin header) and wildcard in non-production
    if (!origin || allowed.includes('*') || allowed.includes(origin)) {
      callback(null, true);
    } else {
      callback(new Error(`CORS: origin '${origin}' not allowed`));
    }
  },
  credentials: true,
  methods: ['GET', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'],
  allowedHeaders: [
    'Authorization',
    'Content-Type',
    'Idempotency-Key',
    'X-API-Key',
    'X-Developer-Id',
    'X-User-Address',
  ],
};
app.use(cors(corsOptions));
app.use(express.json({ limit: config.bodySizeLimit.limit }));
app.use(express.urlencoded({ extended: true, limit: config.bodySizeLimit.limit }));
app.use(sanitizeInput);
app.use(bodySizeLimitMiddleware);
app.use(fieldSelectionMiddleware);

const limiter = rateLimit({
  windowMs: config.rateLimit.windowMs,
  max: config.rateLimit.maxRequests,
  message: 'Too many requests from this IP, please try again later.',
  store: ipRateLimitStore,
});

app.use('/api/', limiter);

// Per-user rate limiter for lending endpoints
const userRateLimiter = rateLimit({
  windowMs: 60 * 1000, // 1 minute window
  max: 10, // 10 requests per minute per user
  store: userRateLimitStore,
  keyGenerator: (req) => {
    // Try to get userAddress from request body first, then query params, then fall back to IP
    const userAddress = req.body?.userAddress || req.query?.userAddress || req.ip;
    return userAddress;
  },
  message: { success: false, error: 'Too many requests for this account' },
  standardHeaders: true,
  legacyHeaders: false,
});

// Lazy-load Swagger UI so the module is only imported when /api/docs is hit
let swaggerUiLoaded = false;
app.use('/api/docs', (req: Request, res: Response, next: NextFunction) => {
  if (swaggerUiLoaded) return next();
  import('swagger-ui-express')
    .then((swaggerUi) => {
      app.use('/api/docs', swaggerUi.serve, swaggerUi.setup(swaggerSpec));
      swaggerUiLoaded = true;
      next();
    })
    .catch(next);
});

// ─── API Version listing ──────────────────────────────────────────────────
app.get('/api/versions', versionListHandler);

// ─── OpenAPI specs per version ────────────────────────────────────────────
app.get('/api/v1/openapi.json', (_req, res) => {
  res.json(v1Spec);
});

app.get('/api/openapi.json', (_req, res) => {
  // Legacy: return the v1 spec with deprecation notice
  res.setHeader('X-API-Deprecated', 'true');
  res.setHeader('X-API-Migrate-To', '/api/v1/openapi.json');
  res.json(swaggerSpec);
});

// ─── Versioned v1 domain routes ──────────────────────────────────────────
// All v1 routes are mounted under /api/v1 with version headers
app.use('/api/v1', versionMiddleware({ version: 'v1' }), v1Routes);

// ─── Legacy route compatibility (deprecated) ─────────────────────────────
// These routes are preserved for backward compatibility.
// Clients receive deprecation headers and should migrate to /api/v1/* paths.

const legacyLendingCompat = legacyCompatibilityMiddleware('/api/v1/lending');
const legacyProtocolCompat = legacyCompatibilityMiddleware('/api/v1/protocol');
const legacyGovernanceCompat = legacyCompatibilityMiddleware('/api/v1/governance');
const legacyAccountCompat = legacyCompatibilityMiddleware('/api/v1/account');
const legacySystemCompat = legacyCompatibilityMiddleware('/api/v1/system');
const legacySecurityCompat = legacyCompatibilityMiddleware('/api/v1/security');

app.use('/api/developer', legacySystemCompat, developerRoutes);
app.use('/api/health', legacySystemCompat, healthRoutes);
app.use('/api/protocol', legacyProtocolCompat, protocolRoutes);
app.use(
  '/api/lending',
  legacyLendingCompat,
  idempotencyMiddleware,
  userRateLimiter,
  sensitiveOperationRateLimiter,
  lendingRoutes
);
app.use('/api/subscriptions', legacyAccountCompat, subscriptionRoutes);
app.use('/api/portfolio', legacyAccountCompat, portfolioRoutes);
app.use('/api/gas', legacyLendingCompat, userRateLimiter, gasRoutes);
app.use('/api/staking', legacyGovernanceCompat, stakingRoutes);
app.use('/api/transactions', legacyAccountCompat, transactionRoutes);
app.use('/api/merkle', legacySecurityCompat, merkleRoutes);
app.use('/api/zk', legacySecurityCompat, zkProofRoutes);
app.use('/api/verification', legacySecurityCompat, verificationRoutes);
app.use('/api/config', legacySystemCompat, configRoutes);
app.use('/api/analytics', legacySystemCompat, analyticsRoutes);
app.use('/api/pool-performance', legacySystemCompat, poolPerformanceRoutes);
app.use('/api/migration', legacySystemCompat, migrationRoutes);
app.use('/api/rates', legacySystemCompat, ratesRoutes);
app.use('/api/cross-protocol', legacySystemCompat, crossProtocolRoutes);
app.use('/api/mev', legacySecurityCompat, mevRoutes);
app.use('/api/reputation', reputationRoutes);
app.use('/api/social', socialRoutes);
app.use('/api/notifications', notificationRoutes);
app.use('/api/disputes', disputeRoutes);
app.use('/api/credit', creditRoutes);
app.use('/api/nonce', nonceRoutes);
app.use('/api/risk', riskEngineRoutes);
app.use('/api/yield-curve', yieldCurveRoutes);
app.use('/api/rates', rateForecastRoutes);
app.use('/api/liquidations', liquidationDashboardRoutes);
app.use('/api/liquidations', opportunityExplorerRoutes);
app.use('/api/treasury', treasuryRoutes);
app.use('/api/metrics', legacySystemCompat, metricsRoutes);
app.use('/api/referral', referralRoutes);
app.use('/api/sns', snsRoutes);
app.use('/api/simulator', simulatorRoutes);
app.use('/api/emergency', emergencyRoutes);

app.use(errorHandler);

void redisCacheService.warmup(async () => {
  const { StellarService } = await import('./services/stellar.service.js');
  const svc = new StellarService();
  await svc.getProtocolStats();
});

export async function resetRateLimiters(): Promise<void> {
  resetSensitiveRateLimits();
  await Promise.all([ipRateLimitStore.resetAll(), userRateLimitStore.resetAll()]);
}

export default app;
