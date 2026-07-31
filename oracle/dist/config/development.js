export const developmentOverrides = {
    logLevel: 'debug',
    dryRun: true,
    cacheTtlSeconds: 60,
    updateIntervalMs: 120000,
    maxPriceDeviationPercent: 15,
    priceStaleThresholdSeconds: 600,
    circuitBreaker: {
        failureThreshold: 5,
        backoffMs: 60000,
    },
};
//# sourceMappingURL=development.js.map