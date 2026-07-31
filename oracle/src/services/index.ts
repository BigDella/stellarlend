/**
 * Services Index
 *
 * Exports all service implementations.
 */

export { PriceValidator, createValidator } from './price-validator.js';
export type { ValidatorConfig } from './price-validator.js';

export { Cache, PriceCache, createCache, createPriceCache } from './cache.js';
export type { CacheConfig } from './cache.js';

export { PriceAggregator, createAggregator } from './price-aggregator.js';
export type { AggregatorConfig } from './price-aggregator.js';

export { ContractUpdater, createContractUpdater } from './contract-updater.js';
export type { ContractUpdaterConfig } from './contract-updater.js';

export { PriceHistoryService, createPriceHistoryService } from './price-history.js';
export type { PriceHistoryConfig, PriceHistoryEntry, TWAPResult } from './price-history.js';

export { CircuitBreaker, CircuitState, createCircuitBreaker } from './circuit-breaker.js';
export type { CircuitBreakerConfig, CircuitBreakerMetrics } from './circuit-breaker.js';

export { MetricsService, createMetricsService } from './metrics-service.js';
export type { MetricsResponse, ProviderHealth, AssetPriceInfo } from './metrics-service.js';

export {
    OracleIncidentMonitor,
    IncidentSeverity,
    IncidentType,
    createOracleIncidentMonitor,
} from './oracle-incident-monitor.js';
export type { OracleIncident } from './oracle-incident-monitor.js';

export { TWAPService, createTWAPService } from './twap.service.js';
export type { TWAPConfig, TWAPStatus } from './twap.service.js';

export {
    ManipulationDetector,
    AlertSeverity,
    AlertType,
    createManipulationDetector,
} from './manipulation-detector.js';
export type { ManipulationAlert, ManipulationDetectorConfig } from './manipulation-detector.js';
