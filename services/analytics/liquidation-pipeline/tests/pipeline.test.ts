import { describe, expect, it } from 'vitest';
import { promises as fs } from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { computeLiquidationMetrics, extractLiquidations } from '../src/extract/extract.js';
import {
  clusterByDayOfWeek,
  clusterByHour,
  collateralFrequency,
  profitabilityDistribution,
} from '../src/metrics/analytics.js';
import { detectAnomalies } from '../src/anomaly/detect.js';
import { buildReport, writeReportFiles } from '../src/reports/generate.js';
import { toDashboardCharts } from '../src/reports/dashboard.js';
import type { LiquidationEvent } from '../src/types.js';

function liq(partial: Partial<LiquidationEvent> & Pick<LiquidationEvent, 'txHash' | 'timestamp'>): LiquidationEvent {
  return {
    ledger: 1,
    liquidator: 'GLIQ',
    borrower: 'GBOR',
    debtAsset: 'USDC',
    collateralAsset: 'XLM',
    debtLiquidated: 100,
    collateralSeized: 110,
    incentiveAmount: 5,
    debtAssetPrice: 1,
    collateralAssetPrice: 1,
    gasCost: 1,
    ...partial,
  };
}

describe('extract + metrics', () => {
  it('extracts liquidation events only', () => {
    const events = extractLiquidations([
      {
        topic: 'deposit_event',
        txHash: 'd1',
        ledger: 1,
        timestamp: new Date(),
        payload: {},
      },
      {
        topic: 'liquidation_event',
        txHash: 'l1',
        ledger: 2,
        timestamp: new Date('2026-07-01T15:00:00Z'),
        payload: {
          liquidator: 'GLIQ',
          borrower: 'GBOR',
          debt_asset: 'USDC',
          collateral_asset: 'XLM',
          debt_liquidated: '100',
          collateral_seized: '120',
          incentive_amount: '5',
          gas_cost: '2',
        },
      },
    ]);
    expect(events).toHaveLength(1);
    const metrics = computeLiquidationMetrics(events[0]!);
    expect(metrics.gasCost).toBe(2);
    expect(metrics.hourOfDay).toBe(15);
    expect(metrics.profit).toBeGreaterThan(0);
  });
});

describe('analytics aggregates', () => {
  const metrics = [
    computeLiquidationMetrics(liq({ txHash: 'a', timestamp: new Date('2026-07-06T01:00:00Z') })), // Mon
    computeLiquidationMetrics(
      liq({
        txHash: 'b',
        timestamp: new Date('2026-07-06T01:00:00Z'),
        collateralSeized: 200,
        debtLiquidated: 100,
      })
    ),
    computeLiquidationMetrics(
      liq({
        txHash: 'c',
        timestamp: new Date('2026-07-07T13:00:00Z'),
        collateralAsset: 'BTC',
      })
    ),
  ];

  it('computes profitability distribution', () => {
    const dist = profitabilityDistribution(metrics);
    expect(dist.count).toBe(3);
    expect(dist.profitableShare).toBeGreaterThan(0);
  });

  it('clusters by hour and weekday', () => {
    expect(clusterByHour(metrics).find((b) => b.key === 'hour-1')?.count).toBe(2);
    expect(clusterByDayOfWeek(metrics).find((b) => b.key === 'Mon')?.count).toBe(2);
  });

  it('ranks collateral frequency', () => {
    const freq = collateralFrequency(metrics);
    expect(freq[0]!.asset).toBe('XLM');
  });
});

describe('anomaly detection', () => {
  it('flags extreme profits', () => {
    const base = Array.from({ length: 8 }, (_, i) =>
      computeLiquidationMetrics(
        liq({
          txHash: `n${i}`,
          timestamp: new Date('2026-07-01T00:00:00Z'),
          debtLiquidated: 100,
          collateralSeized: 105,
          incentiveAmount: 0,
          gasCost: 0,
        })
      )
    );
    const outlier = computeLiquidationMetrics(
      liq({
        txHash: 'outlier',
        timestamp: new Date('2026-07-01T00:00:00Z'),
        debtLiquidated: 100,
        collateralSeized: 10_000,
        incentiveAmount: 0,
        gasCost: 0,
      })
    );
    const anomalies = detectAnomalies([...base, outlier], 2.5);
    expect(anomalies.some((a) => a.txHash === 'outlier')).toBe(true);
  });
});

describe('reports + dashboard', () => {
  it('writes automated report files', async () => {
    const dir = await fs.mkdtemp(path.join(os.tmpdir(), 'liq-'));
    const events = [
      liq({ txHash: 'a', timestamp: new Date('2026-07-01T02:00:00Z') }),
      liq({ txHash: 'b', timestamp: new Date('2026-07-01T14:00:00Z'), collateralAsset: 'ETH' }),
    ];
    const report = buildReport(
      events,
      'daily',
      new Date('2026-07-01T00:00:00Z'),
      new Date('2026-07-02T00:00:00Z')
    );
    const paths = await writeReportFiles(report, dir);
    const json = JSON.parse(await fs.readFile(paths.jsonPath, 'utf8'));
    expect(json.totalLiquidations).toBe(2);
    const md = await fs.readFile(paths.mdPath, 'utf8');
    expect(md).toContain('Liquidation daily report');

    const dash = toDashboardCharts(report);
    expect(dash.charts.hourOfDay.length).toBe(24);
    expect(dash.summary.totalLiquidations).toBe(2);
  });
});
