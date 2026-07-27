import { StellarService } from './stellar.service';
import { redisCacheService } from './redisCache.service';
import logger from '../utils/logger';

const MIGRATION_CACHE_TTL_S = 60;

export interface MigrationPreview {
  estimatedGas: number;
  estimatedSlippageBps: number;
  interestImpact: number;
  expectedOutput: number;
  sourcePositionValue: number;
  destinationPoolApy: number;
  netBenefitBps: number;
}

export interface MigrationRecord {
  id: number;
  user: string;
  sourcePool: string;
  destinationPool: string;
  asset: string;
  amount: number;
  percentage?: number;
  interestAtMigration: number;
  isPartial: boolean;
  status: 'pending' | 'completed' | 'failed' | 'rolled_back';
  estimatedGas?: number;
  actualGas?: number;
  estimatedSlippageBps?: number;
  actualSlippageBps?: number;
  rollbackReason?: string;
  createdAt: string;
  completedAt?: string;
}

export interface MigrationHistory {
  migrations: MigrationRecord[];
  totalCount: number;
  successRate: number;
  totalMigratedValue: number;
}

export interface BulkMigrationRequest {
  sourcePool: string;
  destinationPool: string;
  asset: string;
  percentage?: number;
}

export interface BulkMigrationResult {
  affectedUsers: number;
  estimatedTotalValue: number;
  estimatedGas: number;
  preview: MigrationPreview[];
}

export async function getMigrationPreview(
  user: string,
  sourcePool: string,
  destinationPool: string,
  asset: string,
  amount: number,
  percentage: number = 100
): Promise<MigrationPreview> {
  const cacheKey = redisCacheService.buildKey(
    'migration',
    `preview:${user}:${sourcePool}:${destinationPool}:${amount}:${percentage}`
  );

  const cached = await redisCacheService.get<MigrationPreview>(cacheKey);
  if (cached) return cached;

  const stellarService = new StellarService();

  const [sourcePoolData, destPoolData] = await Promise.all([
    stellarService.getPoolStateAt(sourcePool, Math.floor(Date.now() / 1000)),
    stellarService.getPoolStateAt(destinationPool, Math.floor(Date.now() / 1000)),
  ]);

  const migrateAmount = (amount * percentage) / 100;
  const estimatedGas = 75_000 + Math.floor(migrateAmount / 1_000_000) * 5_000;
  const estimatedSlippageBps = 15 + Math.floor(Math.log10(migrateAmount + 1) * 5);
  const interestImpact = Math.floor(migrateAmount * 0.001);
  const expectedOutput = migrateAmount - interestImpact;
  const sourcePositionValue = migrateAmount;
  const destinationPoolApy = destPoolData.depositApy || 0;
  const netBenefitBps = Math.round(
    ((destPoolData.depositApy - sourcePoolData.depositApy) * 10_000) || 25
  );

  const preview: MigrationPreview = {
    estimatedGas,
    estimatedSlippageBps,
    interestImpact,
    expectedOutput,
    sourcePositionValue,
    destinationPoolApy,
    netBenefitBps,
  };

  await redisCacheService.set(cacheKey, preview, MIGRATION_CACHE_TTL_S);
  return preview;
}

export async function getMigrationHistory(
  user: string,
  page: number = 1,
  limit: number = 20
): Promise<MigrationHistory> {
  const cacheKey = redisCacheService.buildKey(
    'migration',
    `history:${user}:${page}:${limit}`
  );

  const cached = await redisCacheService.get<MigrationHistory>(cacheKey);
  if (cached) return cached;

  const history: MigrationHistory = {
    migrations: [],
    totalCount: 0,
    successRate: 0,
    totalMigratedValue: 0,
  };

  await redisCacheService.set(cacheKey, history, MIGRATION_CACHE_TTL_S);
  return history;
}

export async function getBulkMigrationPreview(
  request: BulkMigrationRequest
): Promise<BulkMigrationResult> {
  const cacheKey = redisCacheService.buildKey(
    'migration',
    `bulk-preview:${request.sourcePool}:${request.destinationPool}:${request.asset}`
  );

  const cached = await redisCacheService.get<BulkMigrationResult>(cacheKey);
  if (cached) return cached;

  const result: BulkMigrationResult = {
    affectedUsers: 0,
    estimatedTotalValue: 0,
    estimatedGas: 0,
    preview: [],
  };

  await redisCacheService.set(cacheKey, result, MIGRATION_CACHE_TTL_S);
  return result;
}

export async function executeMigration(
  user: string,
  sourcePool: string,
  destinationPool: string,
  asset: string,
  amount: number,
  percentage: number = 100
): Promise<MigrationRecord> {
  logger.info('Executing migration', {
    user,
    sourcePool,
    destinationPool,
    amount,
    percentage,
  });

  const record: MigrationRecord = {
    id: Date.now(),
    user,
    sourcePool,
    destinationPool,
    asset,
    amount,
    percentage,
    interestAtMigration: 0,
    isPartial: percentage < 100,
    status: 'pending',
    createdAt: new Date().toISOString(),
  };

  return record;
}

export async function rollbackMigration(
  migrationId: number,
  reason: string
): Promise<MigrationRecord> {
  logger.info('Rolling back migration', { migrationId, reason });

  const record: MigrationRecord = {
    id: migrationId,
    user: '',
    sourcePool: '',
    destinationPool: '',
    asset: '',
    amount: 0,
    interestAtMigration: 0,
    isPartial: false,
    status: 'rolled_back',
    rollbackReason: reason,
    createdAt: new Date().toISOString(),
    completedAt: new Date().toISOString(),
  };

  return record;
}
