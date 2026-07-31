/**
 * Risk Engine — barrel export and hourly scheduler
 *
 * Starts an hourly cron that recalculates:
 *   - Correlation matrices (all windows)
 *   - Realized volatilities (all assets)
 *   - Dynamic LTV adjustments (all assets)
 *   - Concentration metrics (all assets)
 *   - Risk-adjusted collateral ratios (all assets)
 *
 * Call `startRiskEngineScheduler()` once from the server entry-point.
 */

export { correlationMatrixService } from './correlationMatrix.service';
export { volatilityOracleService } from './volatilityOracle.service';
export { dynamicLiquidationService } from './dynamicLiquidation.service';
export { concentrationMonitorService } from './concentrationMonitor.service';
export { riskAdjustedRatioService } from './riskAdjustedRatio.service';

import { correlationMatrixService } from './correlationMatrix.service';
import { volatilityOracleService } from './volatilityOracle.service';
import { dynamicLiquidationService } from './dynamicLiquidation.service';
import { concentrationMonitorService } from './concentrationMonitor.service';
import { riskAdjustedRatioService } from './riskAdjustedRatio.service';
import logger from '../../utils/logger';

const SUPPORTED_ASSETS = ['XLM', 'USDC', 'BTC', 'ETH', 'AQUA', 'yXLM'];
const HOURLY_MS = 60 * 60 * 1000;

let schedulerHandle: ReturnType<typeof setInterval> | null = null;

async function runRecalculation(): Promise<void> {
  logger.info('Risk engine: starting hourly recalculation');
  try {
    await Promise.allSettled([
      correlationMatrixService.recalculateAll(),
      volatilityOracleService.recalculateAll(),
      dynamicLiquidationService.recalculateAll(SUPPORTED_ASSETS),
      concentrationMonitorService.recalculateAll(),
      riskAdjustedRatioService.recalculateAll(),
    ]);
    logger.info('Risk engine: hourly recalculation complete');
  } catch (err) {
    logger.error('Risk engine: recalculation failed', { error: err });
  }
}

/**
 * Start the hourly risk engine scheduler.
 * Safe to call multiple times — subsequent calls are no-ops.
 */
export function startRiskEngineScheduler(): void {
  if (schedulerHandle) return;
  logger.info('Risk engine scheduler started (interval: 1h)');
  // Run immediately on startup, then every hour
  void runRecalculation();
  schedulerHandle = setInterval(() => void runRecalculation(), HOURLY_MS);
}

/**
 * Stop the scheduler (useful for clean test teardown).
 */
export function stopRiskEngineScheduler(): void {
  if (schedulerHandle) {
    clearInterval(schedulerHandle);
    schedulerHandle = null;
    logger.info('Risk engine scheduler stopped');
  }
}
