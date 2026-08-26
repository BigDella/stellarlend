/**
 * Hourly pool snapshot job (Issue #611).
 *
 * Production: attach `processHourlySnapshots` to a BullMQ Worker on queue
 * `pool-snapshots` with `{ repeat: { pattern: '0 * * * *' } }`.
 * Local/dev uses an in-process interval so tests do not require Redis.
 */

import logger from '../utils/logger';
import * as poolPerformanceService from '../services/poolPerformance.service';

const HOURLY_MS = 60 * 60 * 1000;
const QUEUE_NAME = 'pool-snapshots';

let schedulerHandle: ReturnType<typeof setInterval> | null = null;

export const POOL_SNAPSHOT_QUEUE = QUEUE_NAME;

export async function processHourlySnapshots(): Promise<number> {
  const snapshots = await poolPerformanceService.captureAllPoolSnapshots();
  logger.info('Captured hourly pool performance snapshots', {
    queue: QUEUE_NAME,
    count: snapshots.length,
  });
  return snapshots.length;
}

export function startPoolSnapshotCron(): void {
  if (schedulerHandle) return;
  logger.info('Pool snapshot cron started (hourly / BullMQ-compatible processor)');
  void processHourlySnapshots();
  schedulerHandle = setInterval(() => void processHourlySnapshots(), HOURLY_MS);
}

export function stopPoolSnapshotCron(): void {
  if (!schedulerHandle) return;
  clearInterval(schedulerHandle);
  schedulerHandle = null;
}
