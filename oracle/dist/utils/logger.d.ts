/**
 * Logger Utility
 *
 * Centralized logging using Winston with configurable levels
 * and structured output for the Oracle Service.
 */
import winston from 'winston';
/**
 * Create a configured logger instance
 */
export declare function createLogger(level?: string, useJson?: boolean): winston.Logger;
/**
 * Default logger instance (can be reconfigured at runtime)
 */
export declare let logger: winston.Logger;
/**
 * Configure the global logger with new settings
 */
export declare function configureLogger(level: string, useJson?: boolean): void;
/**
 * Log with additional context for price operations
 */
export declare function logPriceUpdate(asset: string, price: bigint, source: string, success: boolean, details?: Record<string, unknown>): void;
/**
 * Log provider health status
 */
export declare function logProviderHealth(provider: string, healthy: boolean, latencyMs?: number, error?: string): void;
/**
 * Log Oracle price staleness alert
 */
export declare function logStalenessAlert(ageSeconds: number, thresholdSeconds: number, lastUpdateTime?: number): void;
//# sourceMappingURL=logger.d.ts.map