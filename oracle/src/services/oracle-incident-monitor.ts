/**
 * Oracle Incident Monitor Service
 *
 * Detects and tracks oracle anomalies:
 * - Price deviation between sources (>2% alert, >10% pause)
 * - Price staleness (>1 hour)
 * - Price volatility (>20% in 10 minutes)
 * - Automatic recovery tracking
 */

import type { AggregatedPrice, PriceData } from '../types/index.js';
import { logger } from '../utils/logger.js';

/**
 * Incident severity levels
 */
export enum IncidentSeverity {
    INFO = 'INFO',
    ALERT = 'ALERT',
    CRITICAL = 'CRITICAL',
}

/**
 * Incident types
 */
export enum IncidentType {
    PRICE_DEVIATION_BETWEEN_SOURCES = 'PRICE_DEVIATION_BETWEEN_SOURCES',
    PRICE_STALENESS = 'PRICE_STALENESS',
    PRICE_VOLATILITY = 'PRICE_VOLATILITY',
    ORACLE_FAILURE = 'ORACLE_FAILURE',
    RECOVERY = 'RECOVERY',
}

/**
 * Oracle incident report
 */
export interface OracleIncident {
    id: string;
    type: IncidentType;
    severity: IncidentSeverity;
    asset: string;
    timestamp: number;
    message: string;
    details?: Record<string, unknown>;
    resolvedAt?: number;
    recoveryDetails?: Record<string, unknown>;
}

/**
 * Price volatility tracking
 */
interface VolatilityWindow {
    asset: string;
    prices: Array<{ price: number; timestamp: number }>;
    maxAge: number; // milliseconds
}

/**
 * Oracle Incident Monitor
 */
export class OracleIncidentMonitor {
    private incidents: Map<string, OracleIncident> = new Map();
    private volatilityWindows: Map<string, VolatilityWindow> = new Map();
    private pausedAssets: Set<string> = new Set();
    private lastAlertTime: Map<string, number> = new Map();

    // Configuration thresholds
    private readonly DEVIATION_ALERT_THRESHOLD = 0.02; // 2%
    private readonly DEVIATION_PAUSE_THRESHOLD = 0.10; // 10%
    private readonly STALENESS_THRESHOLD = 3600; // 1 hour in seconds
    private readonly VOLATILITY_THRESHOLD = 0.20; // 20%
    private readonly VOLATILITY_WINDOW = 600_000; // 10 minutes in milliseconds
    private readonly ALERT_COOLDOWN = 300; // 5 minutes cooldown between alerts

    constructor() {
        logger.info('Oracle Incident Monitor initialized');
    }

    /**
     * Check price deviation between sources
     */
    checkSourceDeviation(prices: PriceData[], asset: string): void {
        if (prices.length < 2) {
            return;
        }

        // Calculate average price
        const avgPrice =
            prices.reduce((sum, p) => sum + Number(p.price), 0) / prices.length;

        if (avgPrice === 0) {
            return;
        }

        // Find max deviation
        let maxDeviation = 0;
        let deviatingSource = '';

        for (const price of prices) {
            const deviation = Math.abs(Number(price.price) - avgPrice) / avgPrice;
            if (deviation > maxDeviation) {
                maxDeviation = deviation;
                deviatingSource = price.source;
            }
        }

        // Alert on significant deviation
        if (maxDeviation > this.DEVIATION_ALERT_THRESHOLD) {
            const severity =
                maxDeviation > this.DEVIATION_PAUSE_THRESHOLD
                    ? IncidentSeverity.CRITICAL
                    : IncidentSeverity.ALERT;

            this.recordIncident({
                type: IncidentType.PRICE_DEVIATION_BETWEEN_SOURCES,
                severity,
                asset,
                message: `Price deviation ${(maxDeviation * 100).toFixed(2)}% detected from source ${deviatingSource}`,
                details: {
                    maxDeviation: maxDeviation * 100,
                    deviatingSource,
                    avgPrice,
                    deviation: deviatingSource,
                    prices: prices.map((p) => ({
                        source: p.source,
                        price: Number(p.price),
                    })),
                },
            });

            // Trigger pause if critical
            if (severity === IncidentSeverity.CRITICAL) {
                this.pauseAsset(asset);
            }
        }
    }

    /**
     * Check for price staleness
     */
    checkStaleness(price: AggregatedPrice): void {
        const now = Math.floor(Date.now() / 1000);
        const age = now - price.timestamp;

        if (age > this.STALENESS_THRESHOLD) {
            this.recordIncident({
                type: IncidentType.PRICE_STALENESS,
                severity: IncidentSeverity.CRITICAL,
                asset: price.asset,
                message: `Price is stale: ${age} seconds old (max: ${this.STALENESS_THRESHOLD}s)`,
                details: {
                    age,
                    maxAge: this.STALENESS_THRESHOLD,
                    lastUpdate: price.timestamp,
                },
            });

            this.pauseAsset(price.asset);
        }
    }

    /**
     * Check price volatility within time window
     */
    checkVolatility(price: AggregatedPrice): void {
        const asset = price.asset.toUpperCase();
        const now = Date.now();

        // Get or create volatility window
        let window = this.volatilityWindows.get(asset);
        if (!window) {
            window = {
                asset,
                prices: [],
                maxAge: this.VOLATILITY_WINDOW,
            };
            this.volatilityWindows.set(asset, window);
        }

        // Add price and remove old entries
        window.prices.push({
            price: Number(price.price),
            timestamp: now,
        });

        window.prices = window.prices.filter((p) => now - p.timestamp <= window!.maxAge);

        if (window.prices.length < 2) {
            return;
        }

        // Calculate volatility
        const oldestPrice = window.prices[0]!.price;
        const newestPrice = window.prices[window.prices.length - 1]!.price;

        if (oldestPrice === 0) {
            return;
        }

        const changePercent = Math.abs(newestPrice - oldestPrice) / oldestPrice;

        if (changePercent > this.VOLATILITY_THRESHOLD) {
            this.recordIncident({
                type: IncidentType.PRICE_VOLATILITY,
                severity: IncidentSeverity.ALERT,
                asset,
                message: `High price volatility: ${(changePercent * 100).toFixed(2)}% change in ${this.VOLATILITY_WINDOW / 1000}s`,
                details: {
                    changePercent: changePercent * 100,
                    oldPrice: oldestPrice,
                    newPrice: newestPrice,
                    windowSeconds: this.VOLATILITY_WINDOW / 1000,
                    dataPoints: window.prices.length,
                },
            });
        }
    }

    /**
     * Record an incident
     */
    private recordIncident(data: {
        type: IncidentType;
        severity: IncidentSeverity;
        asset: string;
        message: string;
        details?: Record<string, unknown>;
    }): void {
        const asset = data.asset.toUpperCase();
        const incidentKey = `${data.type}_${asset}`;

        // Check cooldown to avoid alert spam
        const lastAlert = this.lastAlertTime.get(incidentKey);
        if (lastAlert && Date.now() - lastAlert < this.ALERT_COOLDOWN * 1000) {
            return;
        }

        const incident: OracleIncident = {
            id: `${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
            type: data.type,
            severity: data.severity,
            asset,
            timestamp: Math.floor(Date.now() / 1000),
            message: data.message,
            details: data.details,
        };

        this.incidents.set(incident.id, incident);
        this.lastAlertTime.set(incidentKey, Date.now());

        logger.warn('Oracle incident recorded', {
            incidentId: incident.id,
            type: incident.type,
            severity: incident.severity,
            asset,
            message: incident.message,
        });
    }

    /**
     * Pause an asset due to critical incident
     */
    private pauseAsset(asset: string): void {
        const upper = asset.toUpperCase();
        if (!this.pausedAssets.has(upper)) {
            this.pausedAssets.add(upper);
            logger.error('Asset paused due to oracle incident', { asset: upper });
        }
    }

    /**
     * Check if asset is paused
     */
    isPaused(asset: string): boolean {
        return this.pausedAssets.has(asset.toUpperCase());
    }

    /**
     * Attempt recovery for paused asset
     */
    attemptRecovery(asset: string, currentPrice: AggregatedPrice): void {
        const upper = asset.toUpperCase();

        if (!this.pausedAssets.has(upper)) {
            return;
        }

        // Check if price is stable (low volatility in recent window)
        const window = this.volatilityWindows.get(upper);
        if (!window || window.prices.length < 2) {
            return;
        }

        // Calculate recent volatility
        const recentPrices = window.prices.slice(-5); // Last 5 data points
        if (recentPrices.length < 2) {
            return;
        }

        const minPrice = Math.min(...recentPrices.map((p) => p.price));
        const maxPrice = Math.max(...recentPrices.map((p) => p.price));
        const stability = (maxPrice - minPrice) / minPrice;

        // If volatility is low and staleness is resolved, attempt recovery
        if (stability < this.VOLATILITY_THRESHOLD / 2) {
            const now = Math.floor(Date.now() / 1000);
            const age = now - currentPrice.timestamp;

            if (age < this.STALENESS_THRESHOLD) {
                this.pausedAssets.delete(upper);
                const incident: OracleIncident = {
                    id: `${Date.now()}_${Math.random().toString(36).substr(2, 9)}`,
                    type: IncidentType.RECOVERY,
                    severity: IncidentSeverity.INFO,
                    asset: upper,
                    timestamp: now,
                    message: `Asset recovered after price stabilization`,
                    resolvedAt: now,
                    recoveryDetails: {
                        stability: stability * 100,
                        priceAge: age,
                        recentDataPoints: recentPrices.length,
                    },
                };

                this.incidents.set(incident.id, incident);
                logger.info('Asset recovered from oracle pause', {
                    asset: upper,
                    stability: (stability * 100).toFixed(2),
                });
            }
        }
    }

    /**
     * Get active incidents for an asset
     */
    getIncidents(asset?: string): OracleIncident[] {
        const incidents = Array.from(this.incidents.values());

        if (!asset) {
            return incidents;
        }

        return incidents.filter(
            (i) => i.asset === asset.toUpperCase() && !i.resolvedAt
        );
    }

    /**
     * Generate incident report
     */
    generateReport(timeWindowSeconds = 3600): {
        totalIncidents: number;
        byType: Record<string, number>;
        bySeverity: Record<string, number>;
        pausedAssets: string[];
        incidents: OracleIncident[];
    } {
        const now = Math.floor(Date.now() / 1000);
        const startTime = now - timeWindowSeconds;

        const relevantIncidents = Array.from(this.incidents.values()).filter(
            (i) => i.timestamp >= startTime
        );

        const byType: Record<string, number> = {};
        const bySeverity: Record<string, number> = {};

        for (const incident of relevantIncidents) {
            byType[incident.type] = (byType[incident.type] || 0) + 1;
            bySeverity[incident.severity] = (bySeverity[incident.severity] || 0) + 1;
        }

        return {
            totalIncidents: relevantIncidents.length,
            byType,
            bySeverity,
            pausedAssets: Array.from(this.pausedAssets),
            incidents: relevantIncidents.slice(-100), // Last 100 incidents
        };
    }

    /**
     * Clear old incidents
     */
    cleanup(maxAgeSeconds = 86400): void {
        const now = Math.floor(Date.now() / 1000);
        const maxAge = maxAgeSeconds;

        for (const [id, incident] of this.incidents.entries()) {
            if (now - incident.timestamp > maxAge) {
                this.incidents.delete(id);
            }
        }
    }
}

/**
 * Create oracle incident monitor instance
 */
export function createOracleIncidentMonitor(): OracleIncidentMonitor {
    return new OracleIncidentMonitor();
}
