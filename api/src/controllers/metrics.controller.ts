import { Request, Response, NextFunction } from 'express';
import {
  isValidInterval,
  isValidMetric,
  queryTimeSeries,
  type MetricInterval,
  type MetricName,
} from '../services/metricsTimeseries.service';

/**
 * GET /api/metrics/timeseries?metric=&from=&to=&interval=
 */
export const getTimeSeries = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const metric = String(req.query.metric ?? '');
    const interval = String(req.query.interval ?? '1h');
    const fromRaw = String(req.query.from ?? '');
    const toRaw = String(req.query.to ?? '');

    if (!isValidMetric(metric)) {
      res.status(400).json({
        error: 'Invalid metric',
        allowed: [
          'tvl',
          'totalBorrows',
          'utilizationRate',
          'liquidations',
          'totalDeposits',
          'activeUsers',
        ],
      });
      return;
    }

    if (!isValidInterval(interval)) {
      res.status(400).json({
        error: 'Invalid interval',
        allowed: ['1m', '5m', '1h', '1d'],
      });
      return;
    }

    const from = fromRaw ? new Date(fromRaw) : new Date(Date.now() - 24 * 60 * 60 * 1000);
    const to = toRaw ? new Date(toRaw) : new Date();

    if (Number.isNaN(from.getTime()) || Number.isNaN(to.getTime())) {
      res.status(400).json({ error: 'from and to must be valid ISO timestamps' });
      return;
    }

    if (from > to) {
      res.status(400).json({ error: 'from must be <= to' });
      return;
    }

    const result = await queryTimeSeries({
      metric: metric as MetricName,
      from,
      to,
      interval: interval as MetricInterval,
    });

    res.status(200).json(result);
  } catch (error) {
    next(error);
  }
};
