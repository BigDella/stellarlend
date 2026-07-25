import {
  clearMemorySeries,
  isValidInterval,
  isValidMetric,
  queryTimeSeries,
  seedMemoryPoint,
} from '../services/metricsTimeseries.service';
import { getTimeSeries } from '../controllers/metrics.controller';
import type { Request, Response, NextFunction } from 'express';

describe('metricsTimeseries.service', () => {
  beforeEach(() => {
    clearMemorySeries();
  });

  it('validates metric and interval names', () => {
    expect(isValidMetric('tvl')).toBe(true);
    expect(isValidMetric('nope')).toBe(false);
    expect(isValidInterval('1h')).toBe(true);
    expect(isValidInterval('2h')).toBe(false);
  });

  it('aggregates memory series by interval', async () => {
    seedMemoryPoint(new Date('2026-01-01T00:00:00Z'), { tvl: 100 });
    seedMemoryPoint(new Date('2026-01-01T00:30:00Z'), { tvl: 200 });
    seedMemoryPoint(new Date('2026-01-01T01:00:00Z'), { tvl: 300 });

    const result = await queryTimeSeries({
      metric: 'tvl',
      from: new Date('2026-01-01T00:00:00Z'),
      to: new Date('2026-01-01T02:00:00Z'),
      interval: '1h',
    });

    expect(result.metric).toBe('tvl');
    expect(result.points.length).toBe(2);
    expect(result.points[0]!.value).toBe(150);
    expect(result.points[1]!.value).toBe(300);
  });
});

describe('metrics.controller getTimeSeries', () => {
  function mockRes() {
    const res = {
      statusCode: 200,
      body: null as unknown,
      status(code: number) {
        this.statusCode = code;
        return this;
      },
      json(payload: unknown) {
        this.body = payload;
        return this;
      },
    };
    return res;
  }

  it('returns 400 for invalid metric', async () => {
    const req = { query: { metric: 'bad' } } as unknown as Request;
    const res = mockRes();
    const next: NextFunction = jest.fn();
    await getTimeSeries(req, res as unknown as Response, next);
    expect(res.statusCode).toBe(400);
  });

  it('returns series for valid query', async () => {
    clearMemorySeries();
    seedMemoryPoint(new Date('2026-01-01T00:00:00Z'), { tvl: 50 });
    const req = {
      query: {
        metric: 'tvl',
        from: '2026-01-01T00:00:00.000Z',
        to: '2026-01-01T01:00:00.000Z',
        interval: '1h',
      },
    } as unknown as Request;
    const res = mockRes();
    await getTimeSeries(req, res as unknown as Response, jest.fn());
    expect(res.statusCode).toBe(200);
    expect((res.body as { points: unknown[] }).points.length).toBeGreaterThanOrEqual(1);
  });
});
