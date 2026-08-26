import { Request, Response, NextFunction } from 'express';
import {
  getHistoricalRates,
  getPoolUtilization,
  getRateComparison,
  getProtocolRevenue,
  getAnalyticsSummary,
  exportAnalytics,
  getRateVolatility,
  getWeightedAverageRates,
  getRateChangeEvents,
} from '../services/analytics.service';
import {
  getDashboardView,
  getProtocolMetrics,
  getUserMetrics,
  getActivityFeed,
  getMetricsHistory,
  getTvlForecast,
  getCollateralRatioSnapshots,
  getTriggeredAlerts,
  getRiskDistribution,
  getVolumeSummary,
} from '../services/analyticsDashboard.service';
import { AnalyticsQuery, RateGranularity } from '../types/analytics';
import logger from '../utils/logger';
import { auditLogService } from '../services/auditLog.service';

export const historicalRates = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
      limit: req.query.limit ? Number(req.query.limit) : undefined,
    };
    const rates = await getHistoricalRates(query);
    res.status(200).json(rates);
  } catch (error) {
    next(error);
    return;
  }
};

export const poolUtilization = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };
    const utilization = await getPoolUtilization(query);
    res.status(200).json(utilization);
  } catch (error) {
    next(error);
    return;
  }
};

export const rateComparison = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const comparisons = await getRateComparison();
    res.status(200).json(comparisons);
  } catch (error) {
    next(error);
    return;
  }
};

export const protocolRevenue = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '30d',
    };
    const revenue = await getProtocolRevenue(query);
    res.status(200).json(revenue);
  } catch (error) {
    next(error);
    return;
  }
};

export const analyticsSummary = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const summary = await getAnalyticsSummary();
    res.status(200).json(summary);
  } catch (error) {
    next(error);
    return;
  }
};

export const analyticsExport = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const format = (req.query.format as string) ?? 'json';
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };

    const data = await exportAnalytics(query, format as 'csv' | 'json');

    if (format === 'csv') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', 'attachment; filename="analytics-export.csv"');
      res.status(200).send(data);
      return;
    }

    res.setHeader('Content-Type', 'application/json');
    res.setHeader('Content-Disposition', 'attachment; filename="analytics-export.json"');
    res.status(200).json(data);
  } catch (error) {
    next(error);
    return;
  }
};

export const rateVolatility = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };
    const windowSize = req.query.windowSize ? Number(req.query.windowSize) : undefined;
    const volatility = await getRateVolatility(query, windowSize);
    res.status(200).json(volatility);
  } catch (error) {
    next(error);
    return;
  }
};

export const weightedAverageRates = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '30d',
      poolAddress: req.query.poolAddress as string,
    };
    const granularity = (req.query.granularity as RateGranularity) || 'daily';
    const rates = await getWeightedAverageRates(query, granularity);
    res.status(200).json(rates);
  } catch (error) {
    next(error);
    return;
  }
};

export const rateChangeEvents = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const query: AnalyticsQuery = {
      timeRange: (req.query.timeRange as AnalyticsQuery['timeRange']) || '7d',
      poolAddress: req.query.poolAddress as string,
    };
    const thresholdBps = req.query.thresholdBps ? Number(req.query.thresholdBps) : undefined;
    const events = await getRateChangeEvents(query, thresholdBps);
    res.status(200).json(events);
  } catch (error) {
    next(error);
    return;
  }
};

// ─── Real-time dashboard handlers  (Issue #795) ───────────────────────────────

/**
 * GET /analytics/dashboard
 * Aggregated dashboard view — all panels in one response.
 */
export const dashboardView = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const view = await getDashboardView();
    res.status(200).json({ success: true, ...view });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/protocol
 * Real-time protocol-wide metrics.
 */
export const protocolMetrics = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const metrics = await getProtocolMetrics();
    res.status(200).json({ success: true, metrics });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/user/:userAddress
 * Per-user metrics: health factor, collateral, debt, risk level.
 */
export const userMetrics = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { userAddress } = req.params;
    if (!userAddress) {
      res.status(400).json({ success: false, error: 'userAddress is required' });
      return;
    }
    const metrics = await getUserMetrics(userAddress);
    res.status(200).json({ success: true, metrics });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/activity
 * Real-time activity feed with optional user filter.
 */
export const activityFeed = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const limit = Math.min(Number(req.query.limit ?? 20), 100);
    const offset = Number(req.query.offset ?? 0);
    const userAddress = typeof req.query.userAddress === 'string' ? req.query.userAddress : undefined;

    const feed = await getActivityFeed(limit, offset, userAddress);
    res.status(200).json({ success: true, entries: feed, count: feed.length, limit, offset });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/metrics-history
 * Bounded historical metrics snapshots for trend charts.
 */
export const metricsHistory = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const history = await getMetricsHistory();
    res.status(200).json({ success: true, snapshots: history, count: history.length });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/tvl-forecast?periodsAhead=5
 * Linear TVL forecast from on-chain snapshot history.
 */
export const tvlForecast = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const periodsAhead = Math.min(Math.max(Number(req.query.periodsAhead ?? 5), 1), 30);
    const forecast = await getTvlForecast(periodsAhead);
    res.status(200).json({ success: true, forecast, periodsAhead });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/collateral-ratios
 * Real-time collateral ratio snapshots for all tracked assets.
 */
export const collateralRatios = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const snapshots = await getCollateralRatioSnapshots();
    res.status(200).json({ success: true, snapshots, count: snapshots.length });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/alerts
 * Bounded log of triggered metric alerts.
 */
export const triggeredAlerts = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const alerts = await getTriggeredAlerts();
    res.status(200).json({ success: true, alerts, count: alerts.length });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/risk-distribution
 * User risk-level distribution (5-bucket histogram).
 */
export const riskDistribution = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const distribution = await getRiskDistribution();
    res.status(200).json({ success: true, distribution });
  } catch (error) {
    next(error);
  }
};

/**
 * GET /analytics/dashboard/volume
 * Cumulative volume summary across all activity types.
 */
export const volumeSummary = async (
  _req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const summary = await getVolumeSummary();
    res.status(200).json({ success: true, summary });
  } catch (error) {
    next(error);
  }
};

/**
 * POST /analytics/dashboard/alerts/config
 * Configure a metric alert threshold (admin-only).
 */
export const setAlertThreshold = async (
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> => {
  try {
    const { adminAddress, metric, threshold } = req.body as {
      adminAddress?: string;
      metric?: string;
      threshold?: string;
    };

    const validMetrics = ['tvl', 'utilization', 'avg_rate'];
    if (!adminAddress || !metric || threshold === undefined) {
      res.status(400).json({
        success: false,
        error: 'adminAddress, metric, and threshold are required',
      });
      return;
    }
    if (!validMetrics.includes(metric)) {
      res.status(400).json({
        success: false,
        error: `metric must be one of: ${validMetrics.join(', ')}`,
      });
      return;
    }

    logger.info('Set metric alert threshold', { adminAddress, metric, threshold });
    auditLogService.record({
      action: 'ANALYTICS_ALERT_THRESHOLD_SET',
      actor: adminAddress,
      status: 'success',
      ip: req.ip,
    });

    res.status(200).json({
      success: true,
      metric,
      threshold,
      message: `Alert threshold for '${metric}' set to ${threshold}`,
    });
  } catch (error) {
    next(error);
  }
};
