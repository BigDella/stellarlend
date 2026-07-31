import logger from '../utils/logger';

interface AssetBalance {
  asset: string;
  balance: number;
  usdValue: number;
  percentage: number;
}

interface RevenueItem {
  source: 'fees' | 'liquidation_penalties' | 'yield';
  amount: number;
  timestamp: number;
  asset: string;
}

interface ExpenseItem {
  category: 'development_grants' | 'operational_costs' | 'marketing';
  amount: number;
  timestamp: number;
  description: string;
}

interface CashFlowForecast {
  period: '30d' | '60d' | '90d';
  projectedInflow: number;
  projectedOutflow: number;
  projectedBalance: number;
  confidence: number;
  trend: 'increasing' | 'decreasing' | 'stable';
}

interface ScenarioAnalysis {
  name: string;
  revenueMultiplier: number;
  expenseMultiplier: number;
  projectedBalance: number;
  projectedBurnRate: number;
  runwayMonths: number;
}

interface TreasuryOverview {
  totalBalance: number;
  totalUsdValue: number;
  balanceByAsset: AssetBalance[];
  lastUpdated: number;
}

interface CashFlowReport {
  period: {
    start: number;
    end: number;
  };
  totalRevenue: number;
  totalExpenses: number;
  netCashFlow: number;
  revenueBySource: Record<string, number>;
  expenseByCategory: Record<string, number>;
}

const treasuryStore = {
  balances: new Map<string, number>(),
  revenue: [] as RevenueItem[],
  expenses: [] as ExpenseItem[],
  usdPrices: new Map<string, number>(),
};

export const treasuryDashboardService = {
  setAssetBalance(asset: string, balance: number, usdPrice: number): void {
    treasuryStore.balances.set(asset, balance);
    treasuryStore.usdPrices.set(asset, usdPrice);
    logger.info(`Updated balance for ${asset}: ${balance}`);
  },

  getOverview(): TreasuryOverview {
    const balanceByAsset: AssetBalance[] = [];
    let totalUsdValue = 0;

    treasuryStore.balances.forEach((balance, asset) => {
      const price = treasuryStore.usdPrices.get(asset) || 1;
      const usdValue = balance * price;
      totalUsdValue += usdValue;

      balanceByAsset.push({
        asset,
        balance,
        usdValue,
        percentage: 0,
      });
    });

    balanceByAsset.forEach((item) => {
      item.percentage = totalUsdValue > 0 ? (item.usdValue / totalUsdValue) * 100 : 0;
    });

    return {
      totalBalance: treasuryStore.balances.size,
      totalUsdValue,
      balanceByAsset,
      lastUpdated: Date.now(),
    };
  },

  recordRevenue(source: 'fees' | 'liquidation_penalties' | 'yield', amount: number, asset: string): void {
    treasuryStore.revenue.push({
      source,
      amount,
      asset,
      timestamp: Date.now(),
    });
    logger.info(`Recorded revenue: ${source} - ${amount} ${asset}`);
  },

  recordExpense(category: 'development_grants' | 'operational_costs' | 'marketing', amount: number, description: string): void {
    treasuryStore.expenses.push({
      category,
      amount,
      description,
      timestamp: Date.now(),
    });
    logger.info(`Recorded expense: ${category} - ${amount}`);
  },

  getRevenueTracking(days: number = 30): { source: string; total: number; count: number }[] {
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    const filtered = treasuryStore.revenue.filter((r) => r.timestamp >= cutoff);

    const grouped = new Map<string, { total: number; count: number }>();

    filtered.forEach((item) => {
      const key = item.source;
      const current = grouped.get(key) || { total: 0, count: 0 };
      current.total += item.amount;
      current.count += 1;
      grouped.set(key, current);
    });

    return Array.from(grouped.entries()).map(([source, data]) => ({
      source,
      ...data,
    }));
  },

  getExpenseTracking(days: number = 30): { category: string; total: number; count: number }[] {
    const cutoff = Date.now() - days * 24 * 60 * 60 * 1000;
    const filtered = treasuryStore.expenses.filter((e) => e.timestamp >= cutoff);

    const grouped = new Map<string, { total: number; count: number }>();

    filtered.forEach((item) => {
      const key = item.category;
      const current = grouped.get(key) || { total: 0, count: 0 };
      current.total += item.amount;
      current.count += 1;
      grouped.set(key, current);
    });

    return Array.from(grouped.entries()).map(([category, data]) => ({
      category,
      ...data,
    }));
  },

  getCashFlowForecasts(): CashFlowForecast[] {
    const dailyRevenue = treasuryStore.revenue.reduce((sum, r) => sum + r.amount, 0) / 30 || 0;
    const dailyExpense = treasuryStore.expenses.reduce((sum, e) => sum + e.amount, 0) / 30 || 0;

    return [
      {
        period: '30d',
        projectedInflow: dailyRevenue * 30,
        projectedOutflow: dailyExpense * 30,
        projectedBalance: treasuryStore.balances.size * (dailyRevenue - dailyExpense),
        confidence: 0.75,
        trend: dailyRevenue > dailyExpense ? 'increasing' : 'decreasing',
      },
      {
        period: '60d',
        projectedInflow: dailyRevenue * 60,
        projectedOutflow: dailyExpense * 60,
        projectedBalance: treasuryStore.balances.size * (dailyRevenue - dailyExpense) * 2,
        confidence: 0.65,
        trend: dailyRevenue > dailyExpense ? 'increasing' : 'decreasing',
      },
      {
        period: '90d',
        projectedInflow: dailyRevenue * 90,
        projectedOutflow: dailyExpense * 90,
        projectedBalance: treasuryStore.balances.size * (dailyRevenue - dailyExpense) * 3,
        confidence: 0.55,
        trend: dailyRevenue > dailyExpense ? 'increasing' : 'decreasing',
      },
    ];
  },

  getScenarioAnalysis(): ScenarioAnalysis[] {
    const totalUsdValue = Array.from(treasuryStore.balances.entries()).reduce((sum, [asset, balance]) => {
      const price = treasuryStore.usdPrices.get(asset) || 1;
      return sum + balance * price;
    }, 0);

    const monthlyRevenue = treasuryStore.revenue.reduce((sum, r) => sum + r.amount, 0) / 12 || 0;
    const monthlyExpense = treasuryStore.expenses.reduce((sum, e) => sum + e.amount, 0) / 12 || 0;

    return [
      {
        name: 'Conservative (0.5x revenue, 1.5x expense)',
        revenueMultiplier: 0.5,
        expenseMultiplier: 1.5,
        projectedBalance: totalUsdValue + monthlyRevenue * 0.5 * 12 - monthlyExpense * 1.5 * 12,
        projectedBurnRate: monthlyExpense * 1.5 - monthlyRevenue * 0.5,
        runwayMonths: totalUsdValue / (monthlyExpense * 1.5 - monthlyRevenue * 0.5) || 0,
      },
      {
        name: 'Base case (1x revenue, 1x expense)',
        revenueMultiplier: 1,
        expenseMultiplier: 1,
        projectedBalance: totalUsdValue,
        projectedBurnRate: monthlyExpense - monthlyRevenue,
        runwayMonths: monthlyExpense > monthlyRevenue ? totalUsdValue / (monthlyExpense - monthlyRevenue) : Infinity,
      },
      {
        name: 'Optimistic (2x revenue, 0.8x expense)',
        revenueMultiplier: 2,
        expenseMultiplier: 0.8,
        projectedBalance: totalUsdValue + monthlyRevenue * 2 * 12 - monthlyExpense * 0.8 * 12,
        projectedBurnRate: monthlyExpense * 0.8 - monthlyRevenue * 2,
        runwayMonths: Infinity,
      },
    ];
  },

  getBurnRateAndRunway(): { burnRate: number; burnRateMonthly: number; runwayMonths: number } {
    const totalUsdValue = Array.from(treasuryStore.balances.entries()).reduce((sum, [asset, balance]) => {
      const price = treasuryStore.usdPrices.get(asset) || 1;
      return sum + balance * price;
    }, 0);

    const monthlyExpense = treasuryStore.expenses.reduce((sum, e) => sum + e.amount, 0) / 12 || 1;
    const monthlyRevenue = treasuryStore.revenue.reduce((sum, r) => sum + r.amount, 0) / 12 || 0;

    const burnRate = monthlyExpense - monthlyRevenue;
    const runwayMonths = burnRate > 0 ? totalUsdValue / burnRate : Infinity;

    return {
      burnRate: burnRate / 30,
      burnRateMonthly: burnRate,
      runwayMonths,
    };
  },

  getCashFlowHistory(months: number = 12): CashFlowReport[] {
    const reports: CashFlowReport[] = [];
    const now = Date.now();

    for (let i = 0; i < months; i++) {
      const periodEnd = now - i * 30 * 24 * 60 * 60 * 1000;
      const periodStart = periodEnd - 30 * 24 * 60 * 60 * 1000;

      const periodRevenue = treasuryStore.revenue.filter((r) => r.timestamp >= periodStart && r.timestamp <= periodEnd);
      const periodExpense = treasuryStore.expenses.filter((e) => e.timestamp >= periodStart && e.timestamp <= periodEnd);

      const revenueBySource: Record<string, number> = {};
      const expenseByCategory: Record<string, number> = {};

      let totalRevenue = 0;
      let totalExpense = 0;

      periodRevenue.forEach((r) => {
        revenueBySource[r.source] = (revenueBySource[r.source] || 0) + r.amount;
        totalRevenue += r.amount;
      });

      periodExpense.forEach((e) => {
        expenseByCategory[e.category] = (expenseByCategory[e.category] || 0) + e.amount;
        totalExpense += e.amount;
      });

      reports.push({
        period: { start: periodStart, end: periodEnd },
        totalRevenue,
        totalExpenses: totalExpense,
        netCashFlow: totalRevenue - totalExpense,
        revenueBySource,
        expenseByCategory,
      });
    }

    return reports;
  },
};
