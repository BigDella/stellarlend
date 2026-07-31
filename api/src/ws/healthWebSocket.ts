import { WebSocket, WebSocketServer } from 'ws';
import { IncomingMessage } from 'http';
import { Server } from 'http';
import { liquidationMonitorService, PositionHealth } from '../services/liquidation-monitor/liquidationMonitor.service';
import logger from '../utils/logger';

const HEALTH_WS_PATH = '/api/ws/health-updates';

interface HealthWsClientState {
  addressFilter?: string;
  minHf?: number;
  maxHf?: number;
}

export class HealthWebSocketServer {
  private wss: WebSocketServer;
  private clientStates: Map<WebSocket, HealthWsClientState> = new Map();
  private broadcastIntervalId?: ReturnType<typeof setInterval>;

  constructor(server: Server) {
    this.wss = new WebSocketServer({ server, path: HEALTH_WS_PATH });
    this.setupConnectionHandler();
    this.startBroadcastLoop();

    liquidationMonitorService.onUpdate((positions) => {
      this.broadcastPositions(positions);
    });

    logger.info(`Health WebSocket server initialised at ${HEALTH_WS_PATH}`);
  }

  private setupConnectionHandler(): void {
    this.wss.on('connection', (ws: WebSocket, req: IncomingMessage) => {
      const urlParams = new URL(req.url || '', `http://${req.headers.host}`);
      const addressFilter = urlParams.searchParams.get('address') || undefined;
      const minHf = urlParams.searchParams.get('minHf')
        ? parseFloat(urlParams.searchParams.get('minHf')!)
        : undefined;
      const maxHf = urlParams.searchParams.get('maxHf')
        ? parseFloat(urlParams.searchParams.get('maxHf')!)
        : undefined;

      const state: HealthWsClientState = { addressFilter, minHf, maxHf };
      this.clientStates.set(ws, state);

      const initial = this.filterPositions(
        liquidationMonitorService.getAllPositions(),
        state
      );
      this.send(ws, { type: 'health_snapshot', positions: initial, timestamp: Date.now() });

      ws.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString());
          if (msg.type === 'filter') {
            state.addressFilter = msg.address || undefined;
            state.minHf = msg.minHf;
            state.maxHf = msg.maxHf;
          }
        } catch {
          this.send(ws, { type: 'error', message: 'Invalid JSON' });
        }
      });

      ws.on('close', () => this.clientStates.delete(ws));
      ws.on('error', () => this.clientStates.delete(ws));
    });
  }

  private startBroadcastLoop(): void {
    this.broadcastIntervalId = setInterval(() => {
      const positions = liquidationMonitorService.getAllPositions();
      this.broadcastPositions(positions);
    }, 3000);
  }

  private broadcastPositions(positions: PositionHealth[]): void {
    this.clientStates.forEach((state, ws) => {
      const filtered = this.filterPositions(positions, state);
      this.send(ws, {
        type: 'health_update',
        positions: filtered,
        timestamp: Date.now(),
      });
    });
  }

  private filterPositions(
    positions: PositionHealth[],
    state: HealthWsClientState
  ): PositionHealth[] {
    let filtered = positions;
    if (state.addressFilter) {
      filtered = filtered.filter((p) =>
        p.address.toLowerCase().includes(state.addressFilter!.toLowerCase())
      );
    }
    if (state.minHf !== undefined) {
      filtered = filtered.filter((p) => p.healthFactor >= state.minHf!);
    }
    if (state.maxHf !== undefined) {
      filtered = filtered.filter((p) => p.healthFactor <= state.maxHf!);
    }
    return filtered;
  }

  private send(ws: WebSocket, msg: unknown): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    }
  }

  close(): void {
    if (this.broadcastIntervalId) clearInterval(this.broadcastIntervalId);
    this.wss.close();
  }
}

export function createHealthWebSocket(server: Server): HealthWebSocketServer {
  return new HealthWebSocketServer(server);
}
