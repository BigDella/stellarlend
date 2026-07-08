import logger from '../utils/logger';

export interface DatabaseConfig {
  primary: {
    host: string;
    port: number;
    database: string;
    user: string;
    password: string;
    maxConnections: number;
  };
  replicas: Array<{
    host: string;
    port: number;
    database: string;
    user: string;
    password: string;
    maxConnections: number;
    priority: number; // Higher priority = preferred replica
  }>;
  replication: {
    maxLagMs: number; // Maximum acceptable replication lag
    healthCheckIntervalMs: number;
    failoverEnabled: boolean;
  };
}

interface ReplicaHealth {
  host: string;
  healthy: boolean;
  lagMs: number;
  lastCheck: number;
}

export class DatabaseConnectionManager {
  private config: DatabaseConfig;
  private replicaHealth: Map<string, ReplicaHealth>;
  private healthCheckTimer?: NodeJS.Timeout;
  private currentTransactionConnection?: 'primary' | string;

  constructor(config: DatabaseConfig) {
    this.config = config;
    this.replicaHealth = new Map();

    // Initialize replica health
    this.config.replicas.forEach((replica) => {
      this.replicaHealth.set(replica.host, {
        host: replica.host,
        healthy: true,
        lagMs: 0,
        lastCheck: Date.now(),
      });
    });

    this.startHealthCheck();
    logger.info('Database connection manager initialized', {
      primary: config.primary.host,
      replicas: config.replicas.length,
    });
  }

  /**
   * Get connection for read operations
   * Routes to healthy replica with lowest lag
   */
  getReadConnection(): string {
    // If in transaction, use transaction connection
    if (this.currentTransactionConnection) {
      return this.currentTransactionConnection;
    }

    // Find healthy replicas
    const healthyReplicas = this.config.replicas.filter((replica) => {
      const health = this.replicaHealth.get(replica.host);
      return health?.healthy && health.lagMs <= this.config.replication.maxLagMs;
    });

    if (healthyReplicas.length === 0) {
      logger.warn('No healthy replicas available, falling back to primary');
      return 'primary';
    }

    // Sort by priority (descending) and lag (ascending)
    healthyReplicas.sort((a, b) => {
      const healthA = this.replicaHealth.get(a.host)!;
      const healthB = this.replicaHealth.get(b.host)!;

      if (a.priority !== b.priority) {
        return b.priority - a.priority;
      }

      return healthA.lagMs - healthB.lagMs;
    });

    return healthyReplicas[0]!.host;
  }

  /**
   * Get connection for write operations
   * Always routes to primary
   */
  getWriteConnection(): string {
    return 'primary';
  }

  /**
   * Begin transaction - all subsequent operations use primary
   */
  beginTransaction(): void {
    this.currentTransactionConnection = 'primary';
    logger.debug('Transaction started, routing to primary');
  }

  /**
   * Commit transaction
   */
  commitTransaction(): void {
    this.currentTransactionConnection = undefined;
    logger.debug('Transaction committed');
  }

  /**
   * Rollback transaction
   */
  rollbackTransaction(): void {
    this.currentTransactionConnection = undefined;
    logger.debug('Transaction rolled back');
  }

  /**
   * Check if currently in transaction
   */
  isInTransaction(): boolean {
    return this.currentTransactionConnection !== undefined;
  }

  /**
   * Get connection pool configuration for a specific host
   */
  getPoolConfig(host: string): any {
    if (host === 'primary') {
      return {
        host: this.config.primary.host,
        port: this.config.primary.port,
        database: this.config.primary.database,
        user: this.config.primary.user,
        password: this.config.primary.password,
        max: this.config.primary.maxConnections,
        idleTimeoutMillis: 30000,
        connectionTimeoutMillis: 5000,
      };
    }

    const replica = this.config.replicas.find((r) => r.host === host);
    if (!replica) {
      throw new Error(`Unknown database host: ${host}`);
    }

    return {
      host: replica.host,
      port: replica.port,
      database: replica.database,
      user: replica.user,
      password: replica.password,
      max: replica.maxConnections,
      idleTimeoutMillis: 30000,
      connectionTimeoutMillis: 5000,
    };
  }

  private startHealthCheck(): void {
    this.healthCheckTimer = setInterval(() => {
      this.performHealthCheck();
    }, this.config.replication.healthCheckIntervalMs);
  }

  private async performHealthCheck(): Promise<void> {
    for (const replica of this.config.replicas) {
      try {
        // Simulate health check (in production, query replication lag)
        const lagMs = await this.checkReplicationLag(replica.host);

        this.replicaHealth.set(replica.host, {
          host: replica.host,
          healthy: lagMs <= this.config.replication.maxLagMs * 2, // Allow 2x lag for health
          lagMs,
          lastCheck: Date.now(),
        });

        if (lagMs > this.config.replication.maxLagMs) {
          logger.warn('Replica lag exceeds threshold', {
            host: replica.host,
            lagMs,
            threshold: this.config.replication.maxLagMs,
          });
        }
      } catch (error) {
        logger.error('Replica health check failed', {
          host: replica.host,
          error,
        });

        this.replicaHealth.set(replica.host, {
          host: replica.host,
          healthy: false,
          lagMs: Infinity,
          lastCheck: Date.now(),
        });

        if (this.config.replication.failoverEnabled) {
          this.handleReplicaFailure(replica.host);
        }
      }
    }
  }

  private async checkReplicationLag(host: string): Promise<number> {
    // Simulated lag check
    // In production, query: SELECT EXTRACT(EPOCH FROM (now() - pg_last_xact_replay_timestamp())) * 1000
    return Math.random() * 1000; // 0-1000ms simulated lag
  }

  private handleReplicaFailure(host: string): void {
    logger.error('Replica failed, initiating failover', { host });

    // Mark as unhealthy
    const health = this.replicaHealth.get(host);
    if (health) {
      health.healthy = false;
    }

    // In production, implement failover logic:
    // 1. Remove from load balancer
    // 2. Alert operations team
    // 3. Attempt automatic recovery
  }

  getHealthStatus(): Record<string, ReplicaHealth> {
    const status: Record<string, ReplicaHealth> = {};
    this.replicaHealth.forEach((health, host) => {
      status[host] = { ...health };
    });
    return status;
  }

  destroy(): void {
    if (this.healthCheckTimer) {
      clearInterval(this.healthCheckTimer);
    }
    logger.info('Database connection manager destroyed');
  }
}

// Default configuration from environment
export const databaseConfig: DatabaseConfig = {
  primary: {
    host: process.env.DB_PRIMARY_HOST || 'localhost',
    port: parseInt(process.env.DB_PRIMARY_PORT || '5432', 10),
    database: process.env.DB_NAME || 'stellarlend',
    user: process.env.DB_USER || 'postgres',
    password: process.env.DB_PASSWORD || '',
    maxConnections: parseInt(process.env.DB_PRIMARY_MAX_CONN || '20', 10),
  },
  replicas: [
    {
      host: process.env.DB_REPLICA1_HOST || 'localhost',
      port: parseInt(process.env.DB_REPLICA1_PORT || '5433', 10),
      database: process.env.DB_NAME || 'stellarlend',
      user: process.env.DB_USER || 'postgres',
      password: process.env.DB_PASSWORD || '',
      maxConnections: parseInt(process.env.DB_REPLICA1_MAX_CONN || '30', 10),
      priority: 1,
    },
    {
      host: process.env.DB_REPLICA2_HOST || 'localhost',
      port: parseInt(process.env.DB_REPLICA2_PORT || '5434', 10),
      database: process.env.DB_NAME || 'stellarlend',
      user: process.env.DB_USER || 'postgres',
      password: process.env.DB_PASSWORD || '',
      maxConnections: parseInt(process.env.DB_REPLICA2_MAX_CONN || '30', 10),
      priority: 2,
    },
  ],
  replication: {
    maxLagMs: parseInt(process.env.DB_MAX_LAG_MS || '5000', 10),
    healthCheckIntervalMs: parseInt(process.env.DB_HEALTH_CHECK_MS || '10000', 10),
    failoverEnabled: process.env.DB_FAILOVER_ENABLED !== 'false',
  },
};

export const dbConnectionManager = new DatabaseConnectionManager(databaseConfig);
