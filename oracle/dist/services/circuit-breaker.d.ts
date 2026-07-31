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
/**
 * Circuit breaker states
 */
export declare enum CircuitState {
    CLOSED = "CLOSED",
    OPEN = "OPEN",
    HALF_OPEN = "HALF_OPEN"
}
/**
 * Configuration for a circuit breaker instance
 */
export interface CircuitBreakerConfig {
    /** Number of consecutive failures before opening the circuit */
    failureThreshold: number;
    /** Milliseconds to wait in OPEN state before moving to HALF_OPEN */
    backoffMs: number;
    /** Provider name – used for logging */
    providerName: string;
}
/**
 * Snapshot of circuit breaker metrics
 */
export interface CircuitBreakerMetrics {
    providerName: string;
    state: CircuitState;
    consecutiveFailures: number;
    totalFailures: number;
    totalSuccesses: number;
    lastFailureTime: number | null;
    lastStateChangeTime: number;
}
/**
 * Per-provider circuit breaker
 */
export declare class CircuitBreaker {
    private state;
    private consecutiveFailures;
    private totalFailures;
    private totalSuccesses;
    private lastFailureTime;
    private lastStateChangeTime;
    private readonly config;
    constructor(config: Partial<CircuitBreakerConfig> & {
        providerName: string;
    });
    /**
     * Current circuit state
     */
    get currentState(): CircuitState;
    /**
     * Returns true when the circuit allows a request to proceed.
     *
     * CLOSED    → always allow
     * OPEN      → allow only after backoff has elapsed (transitions to HALF_OPEN)
     * HALF_OPEN → allow exactly one probe request
     */
    isAllowed(): boolean;
    /**
     * Record a successful request.
     * Resets failure count and closes the circuit.
     */
    recordSuccess(): void;
    /**
     * Record a failed request.
     * Opens the circuit once the failure threshold is reached.
     */
    recordFailure(): void;
    /**
     * Snapshot of current metrics
     */
    getMetrics(): CircuitBreakerMetrics;
    private transitionTo;
}
/**
 * Factory – creates one circuit breaker per provider name
 */
export declare function createCircuitBreaker(config: Partial<CircuitBreakerConfig> & {
    providerName: string;
}): CircuitBreaker;
//# sourceMappingURL=circuit-breaker.d.ts.map