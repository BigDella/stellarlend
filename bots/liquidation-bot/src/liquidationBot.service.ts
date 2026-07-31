import WebSocket from 'ws';
import axios from 'axios';
import { botConfig, BotConfig } from './config';
import { Logger } from './logger';

interface PositionHealth {
  address: string;
  collateralAsset: string;
  debtAsset: string;
  collateralValue: number;
  debtValue: number;
  healthFactor: number;
  estimatedProfit: number;
  gasCostStroops: number;
  netProfit: number;
}

interface LiquidationResult {
  positionAddress: string;
  success: boolean;
  txHash?: string;
  profitStroops?: number;
  error?: string;
  timestamp: number;
}

type TargetStrategy = 'highest_profit' | 'highest_risk' | 'lowest_hf';

export class LiquidationBotService {
  private config: BotConfig;
  private logger: Logger;
  private running = false;
  private activeLiquidations = 0;
  private results: LiquidationResult[] = [];
  private ws: WebSocket | null = null;

  constructor() {
    this.config = botConfig;
    this.logger = new Logger(this.config.logLevel);
  }

  public async start(): Promise<void> {
    this.running = true;
    this.logger.info('Liquidation bot starting', {
      strategy: this.config.liquidationTargetStrategy,
      dryRun: this.config.dryRun,
      minProfit: this.config.minProfitThresholdXlm,
    });

    this.connectWebSocket();
    this.startPolling();
  }

  public stop(): void {
    this.running = false;
    if (this.ws) {
      this.ws.close();
      this.ws = null;
    }
    this.logger.info('Liquidation bot stopped');
  }

  private connectWebSocket(): void {
    try {
      this.ws = new WebSocket(this.config.wsUrl);

      this.ws.on('open', () => {
        this.logger.info('WebSocket connected to health feed');
      });

      this.ws.on('message', (data: string) => {
        try {
          const msg = JSON.parse(data);
          if (msg.type === 'health_update' || msg.type === 'health_snapshot') {
            this.evaluatePositions(msg.positions || []);
          }
        } catch (err) {
          this.logger.error('Failed to parse WS message', { error: err });
        }
      });

      this.ws.on('close', () => {
        this.logger.warn('WebSocket disconnected, will reconnect');
        if (this.running) {
          setTimeout(() => this.connectWebSocket(), 5000);
        }
      });

      this.ws.on('error', (err) => {
        this.logger.error('WebSocket error', { error: err.message });
      });
    } catch (err) {
      this.logger.error('Failed to connect WebSocket', { error: err });
    }
  }

  private startPolling(): void {
    const poll = async () => {
      if (!this.running) return;
      try {
        const res = await axios.get(`${this.config.apiBaseUrl}/liquidations/opportunities`, {
          params: {
            maxHf: 1.2,
            sortBy: 'netProfitStroops',
            sortDir: 'desc',
          },
          timeout: 10000,
        });
        if (res.data?.opportunities) {
          this.evaluatePositions(res.data.opportunities);
        }
      } catch (err: any) {
        this.logger.warn('Poll failed', { error: err.message });
      }
      if (this.running) {
        setTimeout(poll, this.config.pollIntervalMs);
      }
    };
    setTimeout(poll, this.config.pollIntervalMs);
  }

  private evaluatePositions(positions: PositionHealth[]): void {
    const liquidatable = positions.filter((p) => {
      const profitXlm = p.netProfit / 10000000;
      return p.healthFactor < 1.2 && profitXlm >= this.config.minProfitThresholdXlm;
    });

    if (liquidatable.length === 0) return;

    const selected = this.selectTargets(liquidatable);
    for (const target of selected) {
      if (this.activeLiquidations < this.config.maxConcurrentLiquidations) {
        this.executeLiquidation(target);
      }
    }
  }

  private selectTargets(positions: PositionHealth[]): PositionHealth[] {
    const strategy: TargetStrategy = this.config.liquidationTargetStrategy;
    const sorted = [...positions];

    switch (strategy) {
      case 'highest_profit':
        sorted.sort((a, b) => b.netProfit - a.netProfit);
        break;
      case 'highest_risk':
        sorted.sort((a, b) => a.healthFactor - b.healthFactor);
        break;
      case 'lowest_hf':
        sorted.sort((a, b) => a.healthFactor - b.healthFactor);
        break;
    }

    return sorted.slice(0, this.config.maxConcurrentLiquidations);
  }

  private async executeLiquidation(position: PositionHealth): Promise<void> {
    this.activeLiquidations++;
    const profitXlm = position.netProfit / 10000000;

    this.logger.info('Attempting liquidation', {
      position: position.address,
      profitXlm: profitXlm.toFixed(2),
      hf: position.healthFactor,
    });

    if (this.config.dryRun) {
      this.logger.info('DRY RUN - Would liquidate', {
        position: position.address,
        estimatedProfitStroops: position.estimatedProfit,
        gasCostStroops: position.gasCostStroops,
      });

      const result: LiquidationResult = {
        positionAddress: position.address,
        success: true,
        profitStroops: position.estimatedProfit,
        timestamp: Date.now(),
      };
      this.results.push(result);
      this.activeLiquidations--;
      return;
    }

    try {
      const res = await axios.post(
        `${this.config.apiBaseUrl}/lending/liquidate`,
        {
          positionAddress: position.address,
          maxRepayAmount: Math.floor(position.debtValue * 0.5).toString(),
        },
        { timeout: 30000 }
      );

      const result: LiquidationResult = {
        positionAddress: position.address,
        success: true,
        txHash: res.data?.txHash,
        profitStroops: position.estimatedProfit,
        timestamp: Date.now(),
      };

      this.results.push(result);
      this.logger.info('Liquidation successful', {
        position: position.address,
        txHash: result.txHash,
      });
    } catch (err: any) {
      const result: LiquidationResult = {
        positionAddress: position.address,
        success: false,
        error: err.message || 'Liquidation failed',
        timestamp: Date.now(),
      };
      this.results.push(result);
      this.logger.error('Liquidation failed', {
        position: position.address,
        error: err.message,
      });
    } finally {
      this.activeLiquidations--;
    }
  }

  public getStats(): { totalAttempted: number; totalSuccessful: number; totalProfit: number } {
    const successful = this.results.filter((r) => r.success);
    return {
      totalAttempted: this.results.length,
      totalSuccessful: successful.length,
      totalProfit: successful.reduce((sum, r) => sum + (r.profitStroops || 0), 0),
    };
  }

  public getResults(): LiquidationResult[] {
    return [...this.results];
  }
}
