import { PositionResponse, TransactionHistoryItem } from '../types';
import {
  PortfolioAnalyticsResponse,
  PortfolioPosition,
  PortfolioValue,
  RiskMetrics,
  RiskLevel,
  OptimizationSuggestion,
  PerformanceSummary,
  InterestAccrualProjection,
  LiquidationPrice,
  HealthFactorHistoryPoint,
  HealthFactorMonitor,
} from '../types/portfolio';

// ─── Constants ────────────────────────────────────────────────────────────────

/** Minimum collateral-to-debt ratio before liquidation (120 %). */
const LIQUIDATION_THRESHOLD = 1.2;

/**
 * Assumed annualised asset volatility used for VaR and drawdown estimates.
 * Conservative 40 % is typical for large-cap crypto collateral.
 */
const ANNUAL_VOLATILITY = 0.4;
const DAILY_VOLATILITY = ANNUAL_VOLATILITY / Math.sqrt(252);

const Z_95 = 1.6449; // 95 % one-tailed z-score
const Z_99 = 2.3263; // 99 % one-tailed z-score

const STROOP_SCALE = 10_000_000n; // 1 XLM = 10,000,000 stroops

// ─── BigInt helpers ────────────────────────────────────────────────────────────

function safeBigInt(value: string): bigint {
  try {
    const cleaned = value.trim();
    return cleaned === '' || cleaned === 'Infinity' ? 0n : BigInt(cleaned);
  } catch {
    return 0n;
  }
}

/** Scale bigint by a float factor; returns a bigint (truncated). */
function scaleByFloat(amount: bigint, factor: number): bigint {
  const scaled = Number(amount) * factor;
  return BigInt(Math.trunc(scaled));
}

/** Format a bigint ratio with 4 decimal places as a string. */
function formatRatio(numerator: bigint, denominator: bigint): string {
  if (denominator === 0n) return 'Infinity';
  const whole = numerator / denominator;
  const remainder = ((numerator % denominator) * 10000n) / denominator;
  return `${whole}.${remainder.toString().padStart(4, '0')}`;
}

// ─── Portfolio value ───────────────────────────────────────────────────────────

function buildPortfolioValue(position: PositionResponse): PortfolioValue {
  const collateral = safeBigInt(position.collateral);
  const debt = safeBigInt(position.debt);
  const interest = safeBigInt(position.borrowInterest);
  const totalDebt = debt + interest;
  const netValue = collateral - totalDebt;

  const portfolioPosition: PortfolioPosition = {
    assetAddress: undefined,
    collateral: collateral.toString(),
    debt: totalDebt.toString(),
    borrowInterest: interest.toString(),
    netValue: netValue.toString(),
    collateralRatio: totalDebt > 0n ? formatRatio(collateral, totalDebt) : 'Infinity',
    lastAccrualTime: position.lastAccrualTime,
  };

  return {
    totalCollateral: collateral.toString(),
    totalDebt: totalDebt.toString(),
    netValue: netValue.toString(),
    utilizationRate: collateral > 0n ? formatRatio(totalDebt, collateral) : '0.0000',
    positions: [portfolioPosition],
    snapshotTimestamp: new Date().toISOString(),
  };
}

// ─── Risk metrics ─────────────────────────────────────────────────────────────

function computeHealthFactor(collateral: bigint, totalDebt: bigint): number {
  if (totalDebt === 0n) return Infinity;
  return Number(collateral) / (Number(totalDebt) * LIQUIDATION_THRESHOLD);
}

function liquidationProbability(healthFactor: number): number {
  if (!isFinite(healthFactor)) return 0;
  if (healthFactor < 1.0) return 95;
  if (healthFactor < 1.2) return 60;
  if (healthFactor < 1.5) return 25;
  if (healthFactor < 2.0) return 10;
  if (healthFactor < 3.0) return 3;
  return 1;
}

function riskLevel(healthFactor: number): RiskLevel {
  if (!isFinite(healthFactor)) return 'low';
  if (healthFactor < 1.0) return 'critical';
  if (healthFactor < 1.5) return 'high';
  if (healthFactor < 2.0) return 'moderate';
  return 'low';
}

function buildRiskMetrics(collateral: bigint, totalDebt: bigint): RiskMetrics {
  const hf = computeHealthFactor(collateral, totalDebt);
  const hfDisplay = isFinite(hf) ? hf.toFixed(4) : 'Infinity';

  // Distance = how far HF is above 1.0, as a percentage of current HF
  const distancePct = isFinite(hf) ? Math.max(0, ((hf - 1.0) / hf) * 100) : 100;

  // VaR applied to net debt exposure (the amount at risk if prices move)
  const exposure = Number(totalDebt);
  const var95 = Math.trunc(exposure * DAILY_VOLATILITY * Z_95);
  const var99 = Math.trunc(exposure * DAILY_VOLATILITY * Z_99);

  // Estimated max drawdown: leverage = debt / collateral; drawdown ≈ leverage * annual_vol
  const leverage = Number(collateral) > 0 ? Number(totalDebt) / Number(collateral) : 0;
  const maxDrawdownPct = Math.min(100, leverage * ANNUAL_VOLATILITY * 100);

  return {
    healthFactor: hfDisplay,
    liquidationThreshold: LIQUIDATION_THRESHOLD.toFixed(2),
    liquidationDistancePct: distancePct.toFixed(2),
    liquidationProbabilityPct: liquidationProbability(hf),
    valueAtRisk95: var95.toString(),
    valueAtRisk99: var99.toString(),
    estimatedMaxDrawdownPct: parseFloat(maxDrawdownPct.toFixed(2)),
    riskLevel: riskLevel(hf),
  };
}

// ─── Optimization suggestions ─────────────────────────────────────────────────

function buildSuggestions(
  healthFactor: number,
  utilizationRate: number,
  collateral: bigint,
  totalDebt: bigint
): OptimizationSuggestion[] {
  const suggestions: OptimizationSuggestion[] = [];

  if (!isFinite(healthFactor) || collateral === 0n) {
    suggestions.push({
      type: 'maintain',
      priority: 'optional',
      description: 'No active position. Deposit collateral to begin earning and borrowing.',
    });
    return suggestions;
  }

  if (healthFactor < 1.0) {
    suggestions.push({
      type: 'add_collateral',
      priority: 'urgent',
      description:
        'Position is under the liquidation threshold. Add collateral immediately to avoid liquidation.',
      estimatedImpact: 'Prevents forced liquidation and associated penalty fees.',
    });
  } else if (healthFactor < 1.5) {
    suggestions.push({
      type: 'add_collateral',
      priority: 'urgent',
      description:
        'Health factor is dangerously low. Add collateral or repay debt to increase your safety buffer.',
      estimatedImpact: `Raising health factor to 2.0 would reduce liquidation risk from ${liquidationProbability(healthFactor)}% to ~10%.`,
    });
  } else if (healthFactor < 2.0) {
    suggestions.push({
      type: 'reduce_debt',
      priority: 'recommended',
      description:
        'Consider partially repaying your debt to improve your health factor and reduce risk.',
      estimatedImpact: 'Each 10% debt reduction improves the health factor proportionally.',
    });
  }

  if (utilizationRate < 0.3 && collateral > 0n) {
    suggestions.push({
      type: 'borrow_more',
      priority: 'optional',
      description:
        'Your collateral utilization is low. You can safely borrow more against your collateral.',
      estimatedImpact: `Current utilization: ${(utilizationRate * 100).toFixed(1)}%. Borrowing up to 50% utilization remains low-risk.`,
    });
  }

  if (utilizationRate > 0.7 && healthFactor >= 2.0) {
    suggestions.push({
      type: 'rebalance',
      priority: 'recommended',
      description:
        'High utilization detected. Consider adding more collateral to maintain a comfortable buffer.',
    });
  }

  if (healthFactor >= 2.0 && utilizationRate >= 0.3 && utilizationRate <= 0.7) {
    suggestions.push({
      type: 'maintain',
      priority: 'optional',
      description:
        'Portfolio is well-balanced. Health factor and utilization are in the optimal range.',
    });
  }

  return suggestions;
}

// ─── Historical performance ────────────────────────────────────────────────────

function buildPerformanceSummary(history: TransactionHistoryItem[]): PerformanceSummary {
  let totalDeposited = 0n;
  let totalWithdrawn = 0n;
  let totalBorrowed = 0n;
  let totalRepaid = 0n;
  const breakdown: Record<string, number> = {};

  for (const tx of history) {
    if (tx.status !== 'success') continue;
    const amount = safeBigInt(tx.amount);
    breakdown[tx.type] = (breakdown[tx.type] ?? 0) + 1;

    switch (tx.type) {
      case 'deposit':
        totalDeposited += amount;
        break;
      case 'withdraw':
        totalWithdrawn += amount;
        break;
      case 'borrow':
        totalBorrowed += amount;
        break;
      case 'repay':
        totalRepaid += amount;
        break;
    }
  }

  const netFlow = totalDeposited + totalRepaid - totalWithdrawn - totalBorrowed;

  const sorted = [...history].sort(
    (a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime()
  );

  return {
    totalDeposited: totalDeposited.toString(),
    totalWithdrawn: totalWithdrawn.toString(),
    totalBorrowed: totalBorrowed.toString(),
    totalRepaid: totalRepaid.toString(),
    netFlow: netFlow.toString(),
    transactionCount: history.length,
    operationBreakdown: breakdown,
    firstTransactionAt: sorted[0]?.timestamp,
    lastTransactionAt: sorted[sorted.length - 1]?.timestamp,
  };
}

// ─── CSV export ───────────────────────────────────────────────────────────────

export function toCSV(history: TransactionHistoryItem[]): string {
  const header = 'date,type,amount,assetAddress,txHash,ledger,status';
  const rows = history.map((tx) =>
    [
      tx.timestamp,
      tx.type,
      tx.amount,
      tx.assetAddress ?? '',
      tx.transactionHash,
      tx.ledger ?? '',
      tx.status,
    ].join(',')
  );
  return [header, ...rows].join('\n');
}

// ─── Interest Accrual Projections ──────────────────────────────────────────────

export function computeInterestAccrualProjection(
  position: PositionResponse,
  borrowApy: number
): InterestAccrualProjection {
  const debt = safeBigInt(position.debt);
  const interest = safeBigInt(position.borrowInterest);
  const currentTotalDebt = debt + interest;

  const apr = borrowApy * 100;
  const dailyRate = apr / 36500;

  const oneDayMultiplier = 1 + dailyRate;

  function projectDebt(days: number): string {
    const projected = Number(currentTotalDebt) * Math.pow(oneDayMultiplier, days);
    return BigInt(Math.trunc(projected)).toString();
  }

  function projectInterest(days: number): string {
    const projected = Number(interest) * Math.pow(oneDayMultiplier, days);
    return BigInt(Math.trunc(projected)).toString();
  }

  return {
    currentDebt: debt.toString(),
    currentInterest: interest.toString(),
    dailyAccrualRate: dailyRate,
    projectedDebt7d: projectDebt(7),
    projectedDebt30d: projectDebt(30),
    projectedDebt90d: projectDebt(90),
    projectedInterest7d: projectInterest(7),
    projectedInterest30d: projectInterest(30),
    projectedInterest90d: projectInterest(90),
    annualPercentageRate: apr,
  };
}

// ─── Liquidation Price Calculator ──────────────────────────────────────────────

export function computeLiquidationPrice(
  position: PositionResponse,
  currentPrice: number,
  collateralDecimals: number = 7
): LiquidationPrice {
  const collateral = Number(safeBigInt(position.collateral)) / Math.pow(10, collateralDecimals);
  const totalDebt = Number(safeBigInt(position.debt) + safeBigInt(position.borrowInterest)) / Math.pow(10, collateralDecimals);

  if (totalDebt === 0 || collateral === 0) {
    return {
      currentPrice,
      liquidationPrice: 0,
      priceDistancePct: 100,
      estimatedPriceAtCurrentHealth: 0,
      scenarios: [],
    };
  }

  const healthFactor = collateral * currentPrice / (totalDebt * currentPrice * LIQUIDATION_THRESHOLD);
  const liquidationPriceVal = (totalDebt * LIQUIDATION_THRESHOLD * currentPrice) / collateral;
  const priceDistancePct = ((currentPrice - liquidationPriceVal) / currentPrice) * 100;

  const scenarioDrops = [5, 10, 15, 20, 25, 30, 40, 50];
  const scenarios = scenarioDrops.map((dropPct) => {
    const newPrice = currentPrice * (1 - dropPct / 100);
    const newHf = collateral * newPrice / (totalDebt * newPrice * LIQUIDATION_THRESHOLD);
    return {
      priceDropPct: dropPct,
      newHealthFactor: isFinite(newHf) ? newHf.toFixed(4) : 'Infinity',
      isLiquidated: newHf < 1.0,
    };
  });

  return {
    currentPrice,
    liquidationPrice: liquidationPriceVal,
    priceDistancePct,
    estimatedPriceAtCurrentHealth: currentPrice * healthFactor,
    scenarios,
  };
}

// ─── Health Factor History & Monitoring ────────────────────────────────────────

export function getHealthFactorHistory(
  position: PositionResponse,
  points: number = 24
): HealthFactorHistoryPoint[] {
  const collateral = safeBigInt(position.collateral);
  const totalDebt = safeBigInt(position.debt) + safeBigInt(position.borrowInterest);
  const now = Date.now();
  const interval = 3600000; // 1 hour intervals

  const history: HealthFactorHistoryPoint[] = [];
  for (let i = points; i >= 0; i--) {
    const timestamp = now - interval * i;
    const decay = 1 - i * 0.005;
    const historicalCollateral = collateral > 0n
      ? scaleByFloat(collateral, Math.max(0.5, decay))
      : 0n;
    const historicalDebt = totalDebt > 0n
      ? scaleByFloat(totalDebt, Math.max(0.5, decay))
      : 0n;
    const hf = computeHealthFactor(historicalCollateral, historicalDebt);
    history.push({
      timestamp: new Date(timestamp).toISOString(),
      healthFactor: isFinite(hf) ? hf.toFixed(4) : 'Infinity',
      riskLevel: riskLevel(hf),
    });
  }

  return history;
}

export function getHealthFactorMonitor(
  position: PositionResponse
): HealthFactorMonitor {
  const collateral = safeBigInt(position.collateral);
  const totalDebt = safeBigInt(position.debt) + safeBigInt(position.borrowInterest);
  const hf = computeHealthFactor(collateral, totalDebt);

  const riskMetrics = buildRiskMetrics(collateral, totalDebt);
  const history = getHealthFactorHistory(position);

  const recentFactors = history.slice(-5).map((h) => parseFloat(h.healthFactor));
  const trend: HealthFactorMonitor['trend'] =
    recentFactors.length < 2
      ? 'stable'
      : recentFactors[recentFactors.length - 1]! > recentFactors[0]!
        ? 'improving'
        : recentFactors[recentFactors.length - 1]! < recentFactors[0]!
          ? 'deteriorating'
          : 'stable';

  return {
    current: riskMetrics,
    history,
    trend,
  };
}

// ─── Main analytics entry point ────────────────────────────────────────────────

export function analyzePortfolio(
  userAddress: string,
  position: PositionResponse,
  history: TransactionHistoryItem[]
): PortfolioAnalyticsResponse {
  const portfolioValue = buildPortfolioValue(position);

  const collateral = safeBigInt(position.collateral);
  const totalDebt = safeBigInt(position.debt) + safeBigInt(position.borrowInterest);

  const riskMetrics = buildRiskMetrics(collateral, totalDebt);
  const hf = computeHealthFactor(collateral, totalDebt);
  const utilizationRate = collateral > 0n ? Number(totalDebt) / Number(collateral) : 0;

  const suggestions = buildSuggestions(hf, utilizationRate, collateral, totalDebt);
  const performance = buildPerformanceSummary(history);

  return {
    userAddress,
    portfolioValue,
    riskMetrics,
    suggestions,
    performance,
    generatedAt: new Date().toISOString(),
  };
}
