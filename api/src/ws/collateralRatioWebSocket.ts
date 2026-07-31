/**
 * WebSocket Server for Real-time Collateral Ratio Monitoring
 * 
 * Broadcasts real-time collateral ratio updates, risk level changes,
 * and alerts to connected clients.
 */

import { WebSocket, WebSocketServer } from 'ws';
import { IncomingMessage } from 'http';
import { Server } from 'http';
import { collateralRatioMonitorService, CollateralRatioSnapshot, PositionRiskData, RiskAlert } from '../services/collateralRatioMonitor.service';
import logger from '../utils/logger';

const COLLATERAL_WS_PATH = '/api/ws/collateral-ratios';

interface CollateralWsClientState {
  assetFilter?: string;
  minHealthFactor?: number;
  maxHealthFactor?: number;
  riskLevelFilter?: 'safe' | 'warning' | 'danger' | 'critical';
  subscribeToAlerts: boolean;
  subscribeToPositions: boolean;
}

export class CollateralRatioWebSocketServer {
  private wss: WebSocketServer;
  private clientStates: Map<WebSocket, CollateralWsClientState> = new Map();

  constructor(server: Server) {
    this.wss = new WebSocketServer({ server, path: COLLATERAL_WS_PATH });
    this.setupConnectionHandler();
    this.setupEventListeners();

    logger.info(`Collateral Ratio WebSocket server initialised at ${COLLATERAL_WS_PATH}`);
  }

  private setupConnectionHandler(): void {
    this.wss.on('connection', (ws: WebSocket, req: IncomingMessage) => {
      const urlParams = new URL(req.url || '', `http://${req.headers.host}`);
      const assetFilter = urlParams.searchParams.get('asset') || undefined;
      const minHf = urlParams.searchParams.get('minHf')
        ? parseFloat(urlParams.searchParams.get('minHf')!)
        : undefined;
      const maxHf = urlParams.searchParams.get('maxHf')
        ? parseFloat(urlParams.searchParams.get('maxHf')!)
        : undefined;
      const riskLevelFilter = urlParams.searchParams.get('riskLevel') as 
        'safe' | 'warning' | 'danger' | 'critical' | undefined;
      const subscribeToAlerts = urlParams.searchParams.get('alerts') === 'true';
      const subscribeToPositions = urlParams.searchParams.get('positions') === 'true';

      const state: CollateralWsClientState = {
        assetFilter,
        minHealthFactor: minHf,
        maxHealthFactor: maxHf,
        riskLevelFilter,
        subscribeToAlerts,
        subscribeToPositions,
      };
      this.clientStates.set(ws, state);

      // Send initial snapshot
      const snapshots = this.filterSnapshots(collateralRatioMonitorService.getCurrentSnapshots(), state);
      this.send(ws, { type: 'initial_snapshot', snapshots, timestamp: Date.now() });

      ws.on('message', (data) => {
        try {
          const msg = JSON.parse(data.toString());
          this.handleClientMessage(ws, msg, state);
        } catch {
          this.send(ws, { type: 'error', message: 'Invalid JSON' });
        }
      });

      ws.on('close', () => this.clientStates.delete(ws));
      ws.on('error', () => this.clientStates.delete(ws));
    });
  }

  private setupEventListeners(): void {
    collateralRatioMonitorService.on('ratio_update', (snapshot: CollateralRatioSnapshot) => {
      this.broadcastRatioUpdate(snapshot);
    });

    collateralRatioMonitorService.on('position_update', (positions: PositionRiskData[]) => {
      this.broadcastPositionUpdate(positions);
    });

    collateralRatioMonitorService.on('alert', (alert: RiskAlert) => {
      this.broadcastAlert(alert);
    });
  }

  private handleClientMessage(ws: WebSocket, msg: any, state: CollateralWsClientState): void {
    switch (msg.type) {
      case 'filter':
        state.assetFilter = msg.asset || undefined;
        state.minHealthFactor = msg.minHf;
        state.maxHealthFactor = msg.maxHf;
        state.riskLevelFilter = msg.riskLevel;
        break;
      case 'subscribe_alerts':
        state.subscribeToAlerts = msg.subscribe;
        break;
      case 'subscribe_positions':
        state.subscribeToPositions = msg.subscribe;
        break;
      case 'refresh':
        const snapshots = this.filterSnapshots(collateralRatioMonitorService.getCurrentSnapshots(), state);
        this.send(ws, { type: 'snapshot', snapshots, timestamp: Date.now() });
        break;
      default:
        this.send(ws, { type: 'error', message: 'Unknown message type' });
    }
  }

  private filterSnapshots(snapshots: CollateralRatioSnapshot[], state: CollateralWsClientState): CollateralRatioSnapshot[] {
    let filtered = snapshots;
    
    if (state.assetFilter) {
      filtered = filtered.filter((s) => s.asset === state.assetFilter);
    }
    
    if (state.minHealthFactor !== undefined) {
      filtered = filtered.filter((s) => s.healthFactor >= state.minHealthFactor!);
    }
    
    if (state.maxHealthFactor !== undefined) {
      filtered = filtered.filter((s) => s.healthFactor <= state.maxHealthFactor!);
    }
    
    if (state.riskLevelFilter) {
      filtered = filtered.filter((s) => s.riskLevel === state.riskLevelFilter);
    }
    
    return filtered;
  }

  private broadcastRatioUpdate(snapshot: CollateralRatioSnapshot): void {
    this.clientStates.forEach((state, ws) => {
      const filtered = this.filterSnapshots([snapshot], state);
      if (filtered.length > 0) {
        this.send(ws, {
          type: 'ratio_update',
          snapshot: filtered[0],
          timestamp: Date.now(),
        });
      }
    });
  }

  private broadcastPositionUpdate(positions: PositionRiskData[]): void {
    this.clientStates.forEach((state, ws) => {
      if (state.subscribeToPositions) {
        this.send(ws, {
          type: 'position_update',
          positions,
          timestamp: Date.now(),
        });
      }
    });
  }

  private broadcastAlert(alert: RiskAlert): void {
    this.clientStates.forEach((state, ws) => {
      if (state.subscribeToAlerts) {
        this.send(ws, {
          type: 'alert',
          alert,
          timestamp: Date.now(),
        });
      }
    });
  }

  private send(ws: WebSocket, msg: unknown): void {
    if (ws.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(msg));
    }
  }

  close(): void {
    this.wss.close();
  }

  get clientCount(): number {
    return this.wss.clients.size;
  }
}

export function createCollateralRatioWebSocket(server: Server): CollateralRatioWebSocketServer {
  return new CollateralRatioWebSocketServer(server);
}
