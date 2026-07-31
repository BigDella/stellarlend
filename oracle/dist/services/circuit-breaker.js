/**
 * Circuit Breaker Service
 *
 * Implements the circuit breaker pattern for per-provider fault tolerance.
 * Prevents wasting resources on providers that are consistently failing
 * by backing off during outages and testing recovery automatically.
 *
 * States:
 *   CLOSED    – Normal operation. Requests pass through.
 *   OPEN      – Provider is down. Requests are skipped for a backoff period.
 *   HALF_OPEN – Backoff expired. A single probe request is allowed to test recovery.
 */
import { logger } from '../utils/logger.js';
/**
 * Circuit breaker states
 */
export var CircuitState;
(function (CircuitState) {
    CircuitState["CLOSED"] = "CLOSED";
    CircuitState["OPEN"] = "OPEN";
    CircuitState["HALF_OPEN"] = "HALF_OPEN";
})(CircuitState || (CircuitState = {}));
const DEFAULT_CONFIG = {
    failureThreshold: 3,
    backoffMs: 30_000,
};
/**
 * Per-provider circuit breaker
 */
export class CircuitBreaker {
    state = CircuitState.CLOSED;
    consecutiveFailures = 0;
    totalFailures = 0;
    totalSuccesses = 0;
    lastFailureTime = null;
    lastStateChangeTime = Date.now();
    config;
    constructor(config) {
        this.config = { ...DEFAULT_CONFIG, ...config };
    }
    /**
     * Current circuit state
     */
    get currentState() {
        return this.state;
    }
    /**
     * Returns true when the circuit allows a request to proceed.
     *
     * CLOSED    → always allow
     * OPEN      → allow only after backoff has elapsed (transitions to HALF_OPEN)
     * HALF_OPEN → allow exactly one probe request
     */
    isAllowed() {
        if (this.state === CircuitState.CLOSED) {
            return true;
        }
        if (this.state === CircuitState.OPEN) {
            const elapsed = Date.now() - (this.lastFailureTime ?? 0);
            if (elapsed >= this.config.backoffMs) {
                this.transitionTo(CircuitState.HALF_OPEN);
                return true; // probe request
            }
            return false;
        }
        // HALF_OPEN – allow the single probe that triggered the transition
        return true;
    }
    /**
     * Record a successful request.
     * Resets failure count and closes the circuit.
     */
    recordSuccess() {
        this.totalSuccesses++;
        this.consecutiveFailures = 0;
        if (this.state !== CircuitState.CLOSED) {
            logger.info(`Circuit breaker CLOSED for provider "${this.config.providerName}" – recovery confirmed`);
            this.transitionTo(CircuitState.CLOSED);
        }
    }
    /**
     * Record a failed request.
     * Opens the circuit once the failure threshold is reached.
     */
    recordFailure() {
        this.totalFailures++;
        this.consecutiveFailures++;
        this.lastFailureTime = Date.now();
        if (this.state === CircuitState.HALF_OPEN ||
            (this.state === CircuitState.CLOSED &&
                this.consecutiveFailures >= this.config.failureThreshold)) {
            logger.warn(`Circuit breaker OPEN for provider "${this.config.providerName}" – ` +
                `${this.consecutiveFailures} consecutive failures. ` +
                `Backing off for ${this.config.backoffMs}ms.`);
            this.transitionTo(CircuitState.OPEN);
        }
    }
    /**
     * Snapshot of current metrics
     */
    getMetrics() {
        return {
            providerName: this.config.providerName,
            state: this.state,
            consecutiveFailures: this.consecutiveFailures,
            totalFailures: this.totalFailures,
            totalSuccesses: this.totalSuccesses,
            lastFailureTime: this.lastFailureTime,
            lastStateChangeTime: this.lastStateChangeTime,
        };
    }
    // ── private ──────────────────────────────────────────────────────────────
    transitionTo(next) {
        const prev = this.state;
        if (prev === next)
            return;
        this.state = next;
        this.lastStateChangeTime = Date.now();
        logger.info(`Circuit breaker state transition for "${this.config.providerName}"`, {
            from: prev,
            to: next,
            consecutiveFailures: this.consecutiveFailures,
            totalFailures: this.totalFailures,
            totalSuccesses: this.totalSuccesses,
        });
    }
}
/**
 * Factory – creates one circuit breaker per provider name
 */
export function createCircuitBreaker(config) {
    return new CircuitBreaker(config);
}
//# sourceMappingURL=circuit-breaker.js.map