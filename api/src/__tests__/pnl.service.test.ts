import * as pnlService from '../services/pnl.service';
import { ValidationError } from '../utils/errors';

describe('pnl.service', () => {
  beforeEach(() => {
    pnlService.resetForTests();
  });

  describe('recordRevenue / recordExpense', () => {
    it('rejects an invalid revenue source', () => {
      expect(() => pnlService.recordRevenue('bogus' as any, 100, 'USDC')).toThrow(ValidationError);
    });

    it('rejects an invalid expense category', () => {
      expect(() => pnlService.recordExpense('bogus' as any, 100, 'desc')).toThrow(ValidationError);
    });

    it('rejects non-positive amounts', () => {
      expect(() => pnlService.recordRevenue('interest_spread', 0, 'USDC')).toThrow(ValidationError);
    });
  });

  describe('getSummary and getBreakdown', () => {
    it('computes total revenue, expenses, and net income', () => {
      pnlService.recordRevenue('interest_spread', 1000, 'USDC');
      pnlService.recordRevenue('liquidation_penalties', 500, 'XLM');
      pnlService.recordExpense('gas_costs', 200, 'network fees');

      const summary = pnlService.getSummary();
      expect(summary.totalRevenue).toBe(1500);
      expect(summary.totalExpenses).toBe(200);
      expect(summary.netIncome).toBe(1300);

      const breakdown = pnlService.getBreakdown();
      expect(breakdown.revenueBySource.interest_spread).toBe(1000);
      expect(breakdown.revenueBySource.liquidation_penalties).toBe(500);
      expect(breakdown.revenueBySource.flash_loan_fees).toBe(0);
      expect(breakdown.expenseByCategory.gas_costs).toBe(200);
    });
  });

  describe('getRevenueByPool', () => {
    it('breaks revenue down per asset pool', () => {
      pnlService.recordRevenue('interest_spread', 300, 'USDC', 'pool_usdc_001');
      pnlService.recordRevenue('interest_spread', 700, 'XLM', 'pool_xlm_001');
      pnlService.recordRevenue('flash_loan_fees', 100, 'XLM', 'pool_xlm_001');

      const pools = pnlService.getRevenueByPool();
      expect(pools[0]!.poolAddress).toBe('pool_xlm_001');
      expect(pools[0]!.totalRevenue).toBe(800);
      expect(pools[1]!.totalRevenue).toBe(300);
    });
  });

  describe('getCumulativeChart', () => {
    it('produces a running cumulative total in chronological order', () => {
      pnlService.recordRevenue('interest_spread', 100, 'USDC');
      pnlService.recordExpense('gas_costs', 40, 'fees');
      pnlService.recordRevenue('flash_loan_fees', 50, 'USDC');

      const points = pnlService.getCumulativeChart();
      expect(points).toHaveLength(3);
      expect(points[points.length - 1]!.cumulativeRevenue).toBe(150);
      expect(points[points.length - 1]!.cumulativeExpense).toBe(40);
    });
  });

  describe('getYieldVsBenchmark', () => {
    it('annualizes protocol yield and compares against benchmarks', () => {
      pnlService.recordRevenue('interest_spread', 50_000, 'USDC');
      const result = pnlService.getYieldVsBenchmark(1_000_000, 365);

      expect(result.protocolAnnualizedYieldPercent).toBeCloseTo(5);
      expect(result.benchmarks.length).toBeGreaterThan(0);
      const treasuryBills = result.benchmarks.find((b) => b.name === 'treasury_bills')!;
      expect(treasuryBills.annualRatePercent).toBe(5);
      expect(treasuryBills.deltaPercent).toBeCloseTo(0);
    });

    it('returns zero yield when TVL is zero', () => {
      const result = pnlService.getYieldVsBenchmark(0);
      expect(result.protocolAnnualizedYieldPercent).toBe(0);
    });
  });

  describe('exportToAccountingFormat', () => {
    it('produces QuickBooks-formatted rows with a header', () => {
      pnlService.recordRevenue('interest_spread', 100, 'USDC');
      pnlService.recordExpense('gas_costs', 10, 'network fees');

      const csv = pnlService.exportToAccountingFormat('quickbooks');
      const lines = csv.split('\n');
      expect(lines[0]).toBe('Date,Type,Category,Amount,Memo');
      expect(lines.some((l) => l.includes('Income') && l.includes('interest_spread'))).toBe(true);
      expect(lines.some((l) => l.includes('Expense') && l.includes('gas_costs'))).toBe(true);
    });

    it('produces Xero-formatted rows with a different header', () => {
      pnlService.recordRevenue('interest_spread', 100, 'USDC');
      const csv = pnlService.exportToAccountingFormat('xero');
      expect(csv.split('\n')[0]).toBe('Date,AccountType,Category,Amount,Description');
    });
  });
});
