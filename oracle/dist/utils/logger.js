/**
 * Logger Utility
 *
 * Centralized logging using Winston with configurable levels
 * and structured output for the Oracle Service.
 */
import winston from 'winston';
const { combine, timestamp, printf, colorize, errors } = winston.format;
/**
 * Custom log format for console output
 */
const consoleFormat = printf(({ level, message, timestamp, ...meta }) => {
    const metaStr = Object.keys(meta).length ? ` ${JSON.stringify(meta)}` : '';
    return `${timestamp} [${level}]: ${message}${metaStr}`;
});
/**
 * Custom log format for JSON output (production)
 */
const jsonFormat = printf(({ level, message, timestamp, ...meta }) => {
    return JSON.stringify({
        timestamp,
        level,
        message,
        ...meta,
    });
});
/**
 * Create a configured logger instance
 */
export function createLogger(level = 'info', useJson = false) {
    return winston.createLogger({
        level,
        format: combine(errors({ stack: true }), timestamp({ format: 'YYYY-MM-DD HH:mm:ss.SSS' })),
        transports: [
            new winston.transports.Console({
                format: combine(useJson ? jsonFormat : combine(colorize(), consoleFormat)),
            }),
        ],
    });
}
/**
 * Default logger instance (can be reconfigured at runtime)
 */
export let logger = createLogger('info');
/**
 * Configure the global logger with new settings
 */
export function configureLogger(level, useJson = false) {
    logger = createLogger(level, useJson);
}
/**
 * Log with additional context for price operations
 */
export function logPriceUpdate(asset, price, source, success, details) {
    const logData = {
        asset,
        price: price.toString(),
        source,
        success,
        ...details,
    };
    if (success) {
        logger.info('Price update', logData);
    }
    else {
        logger.error('Price update failed', logData);
    }
}
/**
 * Log provider health status
 */
export function logProviderHealth(provider, healthy, latencyMs, error) {
    const logData = {
        provider,
        healthy,
        latencyMs,
        error,
    };
    if (healthy) {
        logger.debug('Provider health check', logData);
    }
    else {
        logger.warn('Provider unhealthy', logData);
    }
}
/**
 * Log Oracle price staleness alert
 */
export function logStalenessAlert(ageSeconds, thresholdSeconds, lastUpdateTime) {
    logger.warn('Oracle price staleness detected', {
        ageSeconds,
        thresholdSeconds,
        lastUpdateTime: lastUpdateTime ? new Date(lastUpdateTime).toISOString() : 'never',
        alertType: 'staleness_monitor',
    });
}
//# sourceMappingURL=logger.js.map