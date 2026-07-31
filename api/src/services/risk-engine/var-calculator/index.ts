import { VaRInput, VaRResult, VaRHistoryEntry, HistoricalPrice, Position } from './types';
import { computeParametricVaR } from './parametric-var';
import { computeHistoricalVaR } from './historical-var';
import { computeMonteCarloVaR } from './monte-carlo-var';
import { computeStressVaR, PREDEFINED_STRESS_SCENARIOS } from './stress-var';

export class VaRCalculator {
  computeParametricVaR(input: VaRInput): VaRResult {
    return computeParametricVaR(input);
  }

  computeHistoricalVaR(input: VaRInput, historicalPrices: HistoricalPrice[]): VaRResult {
    return computeHistoricalVaR(input, historicalPrices);
  }

  computeMonteCarloVaR(input: VaRInput): VaRResult {
    return computeMonteCarloVaR(input);
  }

  computeAllMethods(input: VaRInput, historicalPrices: HistoricalPrice[]): {
    parametric: VaRResult;
    historical: VaRResult;
    monteCarlo: VaRResult;
  } {
    return {
      parametric: this.computeParametricVaR(input),
      historical: this.computeHistoricalVaR(input, historicalPrices),
      monteCarlo: this.computeMonteCarloVaR(input),
    };
  }

  computeStressVaR(
    positions: Position[],
    portfolioValue: number
  ): { scenario: string; var: number; cvar: number }[] {
    return PREDEFINED_STRESS_SCENARIOS.map((scenario) => {
      const result = computeStressVaR(positions, scenario, portfolioValue);
      return {
        scenario: result.scenario.name,
        var: result.var,
        cvar: result.cvar,
      };
    });
  }

  generateHistoricalVaRData(
    positions: Position[],
    historicalPrices: HistoricalPrice[],
    portfolioValue: number,
    days: number = 30
  ): VaRHistoryEntry[] {
    const entries: VaRHistoryEntry[] = [];
    const today = new Date();

    for (let i = days; i >= 0; i--) {
      const date = new Date(today);
      date.setDate(date.getDate() - i);

      const input: VaRInput = {
        positions,
        confidenceLevel: 95,
        timeHorizon: 1,
        portfolioValue,
      };

      const paramResult = this.computeParametricVaR(input);
      const histPrices = historicalPrices.slice(0, Math.max(1, historicalPrices.length - i));
      const histResult = computeHistoricalVaR(input, histPrices);

      const input99: VaRInput = { ...input, confidenceLevel: 99 };
      const paramResult99 = this.computeParametricVaR(input99);
      const histResult99 = computeHistoricalVaR(input99, histPrices);

      entries.push({
        date: date.toISOString().split('T')[0],
        var95: paramResult.var,
        var99: paramResult99.var,
        cvar95: paramResult.cvar,
        cvar99: paramResult99.cvar,
        portfolioValue,
        method: 'parametric',
      });
    }

    return entries;
  }
}

export const varCalculator = new VaRCalculator();
export { PREDEFINED_STRESS_SCENARIOS };
