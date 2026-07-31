/**
 * Issue-scoped ETL entry: services/data-lake/etl/
 */
export { DailyEtlJob, StaticEventSource } from '../src/etl/daily-job.js';
export { buildPartition, groupByPartition } from '../src/etl/partition.js';
