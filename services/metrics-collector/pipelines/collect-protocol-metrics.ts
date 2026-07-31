/**
 * Pipeline entrypoints (CronJob / scheduled worker).
 * Implementation lives in src/pipelines for typechecking; this file is the
 * issue-scoped path services/metrics-collector/pipelines/.
 */
export {
  runProtocolMetricsPipeline,
  runGapBackfillPipeline,
} from '../src/pipelines/collect-protocol-metrics.js';
