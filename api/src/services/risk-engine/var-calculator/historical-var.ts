import { VaRInput, VaRResult, HistoricalPrice } from './types';
import { computeParametricVaR } from './parametric-var';

const WINDOW_DAYS = 90;

export function computeHistoricalVaR(
  input: VaRInput,
  historicalPrices: HistoricalPrice[]
): VaRResult {
  const { positions, confidenceLevel, timeHorizon, portfolioValue } = input;
  const alpha = 1 - confidenceLevel / 100;

  const returns = computePortfolioReturns(positions, historicalPrices);

  const scaledReturns = returns.map((r) => r * Math.sqrt(timeHorizon));

  const sortedReturns = [...scaledReturns].sort((a, b) => a - b);

  const varIndex = Math.floor(alpha * sortedReturns.length);
  const var_ = -portfolioValue * Math.abs(sortedReturns[Math.max(0, varIndex)]);

  const tailReturns = sortedReturns.slice(0, varIndex + 1);
  const cvar_ = tailReturns.length > 0
    ? -portfolioValue * Math.abs(tailReturns.reduce((a, b) => a + b, 0) / tailReturns.length)
    : var_;

  const paramVaR = computeParametricVaR(input);

  return {
    var: var_,
    cvar: cvar_,
    confidenceLevel,
    timeHorizon,
    method: 'historical',
    marginalContributions: paramVaR.marginalContributions,
    componentVaR: paramVaR.componentVaR,
    timestamp: Date.now(),
  };
}

function computePortfolioReturns(
  positions: import('./types').Position[],
  historicalPrices: HistoricalPrice[]
): number[] {
  const returns: number[] = [];

  const windowPrices = historicalPrices.slice(-WINDOW_DAYS);

  for (let i = 1; i < windowPrices.length; i++) {
    let portfolioReturn = 0;
    for (const position of positions) {
      const prev = windowPrices[i - 1].prices[position.asset] ?? 0;
      const curr = windowPrices[i].prices[position.asset] ?? 0;
      if (prev > 0) {
        const assetReturn = (curr - prev) / prev;
        portfolioReturn += assetReturn * position.weight;
      }
    }
    returns.push(portfolioReturn);
  }

  return returns;
}

export { WINDOW_DAYS };
