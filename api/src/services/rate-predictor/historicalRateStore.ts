export interface HistoricalRateSnapshot {
  timestamp: number;
  asset: string;
  utilizationBps: number;
  borrowRateBps: number;
  supplyRateBps: number;
  totalValueLocked: number;
  volatility: number;
}

export class HistoricalRateStore {
  private store: Map<string, HistoricalRateSnapshot[]> = new Map();

  constructor() {
    this.seedMockData('XLM');
    this.seedMockData('USDC');
    this.seedMockData('USDT');
  }

  /**
   * Seeds historical 90-day time-series data for testing and initial model training.
   */
  private seedMockData(asset: string): void {
    const snapshots: HistoricalRateSnapshot[] = [];
    const now = Date.now();
    const dayMs = 86400 * 1000;

    const baseUtil = asset === 'USDC' ? 7500 : asset === 'USDT' ? 7000 : 5500;
    const baseTvl = asset === 'XLM' ? 50000000 : 100000000;

    for (let i = 90; i >= 0; i--) {
      const ts = now - i * dayMs;
      // Add pseudo-random walk noise
      const utilNoise = Math.sin(i / 5) * 500 + (Math.random() - 0.5) * 200;
      const util = Math.max(1000, Math.min(9500, Math.round(baseUtil + utilNoise)));
      
      const borrowRate = Math.round(200 + (util <= 8000 ? (util * 1000) / 8000 : 1200 + ((util - 8000) * 6000) / 2000));
      const supplyRate = Math.round((borrowRate * (util / 10000) * 0.9));
      const volatility = 0.02 + Math.abs(Math.sin(i / 3)) * 0.05 + (Math.random() - 0.5) * 0.01;
      const tvl = baseTvl * (1 + Math.sin(i / 10) * 0.1);

      snapshots.push({
        timestamp: ts,
        asset,
        utilizationBps: util,
        borrowRateBps: borrowRate,
        supplyRateBps: supplyRate,
        totalValueLocked: Math.round(tvl),
        volatility: Math.round(volatility * 10000) / 10000,
      });
    }

    this.store.set(asset.toUpperCase(), snapshots);
  }

  public getHistory(asset: string, limitDays: number = 90): HistoricalRateSnapshot[] {
    const key = asset.toUpperCase();
    const data = this.store.get(key) || [];
    return data.slice(-limitDays);
  }

  public addSnapshot(snapshot: HistoricalRateSnapshot): void {
    const key = snapshot.asset.toUpperCase();
    const existing = this.store.get(key) || [];
    existing.push(snapshot);
    if (existing.length > 365) {
      existing.shift(); // keep max 1 year of daily snapshots
    }
    this.store.set(key, existing);
  }
}

export const historicalRateStore = new HistoricalRateStore();
