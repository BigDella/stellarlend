import { ValidationError } from '../utils/errors';

export type RevenueSource = 'interest_spread' | 'liquidation_penalties' | 'flash_loan_fees';
export type ExpenseCategory = 'gas_costs' | 'oracle_operations' | 'development_costs';
export type PnlPeriod = 'daily' | 'monthly' | 'annual';

const REVENUE_SOURCES: RevenueSource[] = [
  'interest_spread',
  'liquidation_penalties',
  'flash_loan_fees',
];
const EXPENSE_CATEGORIES: ExpenseCategory[] = ['gas_costs', 'oracle_operations', 'development_costs'];

const PERIOD_MS: Record<PnlPeriod, number> = {
  daily: 24 * 60 * 60 * 1000,
  monthly: 30 * 24 * 60 * 60 * 1000,
  annual: 365 * 24 * 60 * 60 * 1000,
};

/** Simulated annual benchmark yields, used purely as a comparison baseline. */
const BENCHMARK_ANNUAL_RATES: Record<string, number> = {
  treasury_bills: 0.05,
  stablecoin_yield: 0.04,
};

interface RevenueEntry {
  source: RevenueSource;
  amount: number;
  asset: string;
  poolAddress?: string;
  timestamp: number;
}

interface ExpenseEntry {
  category: ExpenseCategory;
  amount: number;
  description: string;
  timestamp: number;
}

export interface PnlSummary {
  totalRevenue: number;
  totalExpenses: number;
  netIncome: number;
  generatedAt: string;
}

export interface PnlBreakdown {
  revenueBySource: Record<RevenueSource, number>;
  expenseByCategory: Record<ExpenseCategory, number>;
}

export interface PnlStatement {
  period: { start: number; end: number };
  totalRevenue: number;
  totalExpenses: number;
  netIncome: number;
  revenueBySource: Record<RevenueSource, number>;
  expenseByCategory: Record<ExpenseCategory, number>;
}

export interface PoolRevenueBreakdown {
  poolAddress: string;
  totalRevenue: number;
}

export interface CumulativePoint {
  timestamp: number;
  cumulativeRevenue: number;
  cumulativeExpense: number;
}

export interface YieldVsBenchmark {
  protocolAnnualizedYieldPercent: number;
  benchmarks: Array<{ name: string; annualRatePercent: number; deltaPercent: number }>;
}

export interface RevenueGrowth {
  currentPeriodRevenue: number;
  previousPeriodRevenue: number;
  growthRatePercent: number;
  trend: 'up' | 'down' | 'flat';
}

const store = {
  revenue: [] as RevenueEntry[],
  expenses: [] as ExpenseEntry[],
};

export function resetForTests(): void {
  store.revenue = [];
  store.expenses = [];
}

export function recordRevenue(
  source: RevenueSource,
  amount: number,
  asset: string,
  poolAddress?: string
): void {
  if (!REVENUE_SOURCES.includes(source)) {
    throw new ValidationError(`source must be one of: ${REVENUE_SOURCES.join(', ')}`);
  }
  if (!(amount > 0) || !asset) {
    throw new ValidationError('amount must be positive and asset is required');
  }
  store.revenue.push({ source, amount, asset, poolAddress, timestamp: Date.now() });
}

export function recordExpense(
  category: ExpenseCategory,
  amount: number,
  description: string
): void {
  if (!EXPENSE_CATEGORIES.includes(category)) {
    throw new ValidationError(`category must be one of: ${EXPENSE_CATEGORIES.join(', ')}`);
  }
  if (!(amount > 0) || !description) {
    throw new ValidationError('amount must be positive and description is required');
  }
  store.expenses.push({ category, amount, description, timestamp: Date.now() });
}

function emptyRevenueBySource(): Record<RevenueSource, number> {
  return { interest_spread: 0, liquidation_penalties: 0, flash_loan_fees: 0 };
}

function emptyExpenseByCategory(): Record<ExpenseCategory, number> {
  return { gas_costs: 0, oracle_operations: 0, development_costs: 0 };
}

function revenueInRange(start: number, end: number): RevenueEntry[] {
  return store.revenue.filter((r) => r.timestamp >= start && r.timestamp < end);
}

function expensesInRange(start: number, end: number): ExpenseEntry[] {
  return store.expenses.filter((e) => e.timestamp >= start && e.timestamp < end);
}

export function getSummary(): PnlSummary {
  const totalRevenue = store.revenue.reduce((sum, r) => sum + r.amount, 0);
  const totalExpenses = store.expenses.reduce((sum, e) => sum + e.amount, 0);
  return {
    totalRevenue,
    totalExpenses,
    netIncome: totalRevenue - totalExpenses,
    generatedAt: new Date().toISOString(),
  };
}

export function getBreakdown(): PnlBreakdown {
  const revenueBySource = emptyRevenueBySource();
  const expenseByCategory = emptyExpenseByCategory();

  for (const r of store.revenue) revenueBySource[r.source] += r.amount;
  for (const e of store.expenses) expenseByCategory[e.category] += e.amount;

  return { revenueBySource, expenseByCategory };
}

export function getStatements(period: PnlPeriod, count = 12): PnlStatement[] {
  const periodMs = PERIOD_MS[period];
  const now = Date.now();
  const statements: PnlStatement[] = [];

  for (let i = 0; i < count; i++) {
    const end = now - i * periodMs;
    const start = end - periodMs;

    const revenue = revenueInRange(start, end);
    const expenses = expensesInRange(start, end);

    const revenueBySource = emptyRevenueBySource();
    const expenseByCategory = emptyExpenseByCategory();
    revenue.forEach((r) => (revenueBySource[r.source] += r.amount));
    expenses.forEach((e) => (expenseByCategory[e.category] += e.amount));

    const totalRevenue = revenue.reduce((sum, r) => sum + r.amount, 0);
    const totalExpenses = expenses.reduce((sum, e) => sum + e.amount, 0);

    statements.push({
      period: { start, end },
      totalRevenue,
      totalExpenses,
      netIncome: totalRevenue - totalExpenses,
      revenueBySource,
      expenseByCategory,
    });
  }

  return statements;
}

export function getRevenueByPool(): PoolRevenueBreakdown[] {
  const totals = new Map<string, number>();
  for (const r of store.revenue) {
    const key = r.poolAddress ?? 'unassigned';
    totals.set(key, (totals.get(key) ?? 0) + r.amount);
  }
  return Array.from(totals.entries())
    .map(([poolAddress, totalRevenue]) => ({ poolAddress, totalRevenue }))
    .sort((a, b) => b.totalRevenue - a.totalRevenue);
}

export function getCumulativeChart(): CumulativePoint[] {
  const events: Array<{ timestamp: number; revenue: number; expense: number }> = [
    ...store.revenue.map((r) => ({ timestamp: r.timestamp, revenue: r.amount, expense: 0 })),
    ...store.expenses.map((e) => ({ timestamp: e.timestamp, revenue: 0, expense: e.amount })),
  ].sort((a, b) => a.timestamp - b.timestamp);

  let cumulativeRevenue = 0;
  let cumulativeExpense = 0;

  return events.map((event) => {
    cumulativeRevenue += event.revenue;
    cumulativeExpense += event.expense;
    return { timestamp: event.timestamp, cumulativeRevenue, cumulativeExpense };
  });
}

export function getYieldVsBenchmark(tvlUsd: number, lookbackDays = 365): YieldVsBenchmark {
  const cutoff = Date.now() - lookbackDays * 24 * 60 * 60 * 1000;
  const periodRevenue = store.revenue
    .filter((r) => r.timestamp >= cutoff)
    .reduce((sum, r) => sum + r.amount, 0);

  const annualizedYield = tvlUsd > 0 ? (periodRevenue / tvlUsd) * (365 / lookbackDays) : 0;
  const protocolAnnualizedYieldPercent = annualizedYield * 100;

  const benchmarks = Object.entries(BENCHMARK_ANNUAL_RATES).map(([name, rate]) => ({
    name,
    annualRatePercent: rate * 100,
    deltaPercent: protocolAnnualizedYieldPercent - rate * 100,
  }));

  return { protocolAnnualizedYieldPercent, benchmarks };
}

export function getRevenueGrowth(period: PnlPeriod = 'monthly'): RevenueGrowth {
  const periodMs = PERIOD_MS[period];
  const now = Date.now();

  const currentPeriodRevenue = revenueInRange(now - periodMs, now).reduce(
    (sum, r) => sum + r.amount,
    0
  );
  const previousPeriodRevenue = revenueInRange(now - 2 * periodMs, now - periodMs).reduce(
    (sum, r) => sum + r.amount,
    0
  );

  const growthRatePercent =
    previousPeriodRevenue > 0
      ? ((currentPeriodRevenue - previousPeriodRevenue) / previousPeriodRevenue) * 100
      : 0;

  const trend: RevenueGrowth['trend'] =
    growthRatePercent > 1 ? 'up' : growthRatePercent < -1 ? 'down' : 'flat';

  return { currentPeriodRevenue, previousPeriodRevenue, growthRatePercent, trend };
}

export function exportToAccountingFormat(format: 'quickbooks' | 'xero'): string {
  const dateFormatter = (timestamp: number): string => new Date(timestamp).toISOString().slice(0, 10);

  const revenueRows = store.revenue.map((r) =>
    format === 'quickbooks'
      ? `${dateFormatter(r.timestamp)},Income,${r.source},${r.amount},${r.asset}`
      : `${dateFormatter(r.timestamp)},Sales,${r.source},${r.amount},${r.asset}`
  );

  const expenseRows = store.expenses.map((e) =>
    format === 'quickbooks'
      ? `${dateFormatter(e.timestamp)},Expense,${e.category},${e.amount},"${e.description}"`
      : `${dateFormatter(e.timestamp)},Purchases,${e.category},${e.amount},"${e.description}"`
  );

  const header =
    format === 'quickbooks'
      ? 'Date,Type,Category,Amount,Memo'
      : 'Date,AccountType,Category,Amount,Description';

  return [header, ...revenueRows, ...expenseRows].join('\n');
}
