import { Position, MarginalVaR } from './types';

const BPS_DIVISOR = 10000;

export function computeMarginalVaR(
  positions: Position[],
  portfolioVaR: number,
  portfolioValue: number
): MarginalVaR[] {
  const totalWeight = positions.reduce((sum, p) => sum + p.weight, 0);

  return positions.map((p) => {
    const normalizedWeight = totalWeight > 0 ? p.weight / totalWeight : 0;
    const ivar = portfolioVaR * normalizedWeight;
    const cvar = ivar * normalizedWeight * (p.volatility > 0 ? p.volatility / BPS_DIVISOR : 1);

    return {
      asset: p.asset,
      marginalVaR: ivar,
      componentVaR: cvar,
      percentContribution: totalWeight > 0 && portfolioVaR > 0
        ? (ivar / portfolioVaR) * 100
        : 0,
    };
  });
}

export function computeIncrementalVaR(
  positions: Position[],
  newPosition: Position,
  currentVaR: number,
  portfolioValue: number
): { incrementalVaR: number; newPortfolioVaR: number } {
  const newTotalValue = portfolioValue + newPosition.collateralValue;
  const marginalVaRs = computeMarginalVaR(positions, currentVaR, portfolioValue);

  const newAssetVaR = (newPosition.volatility / BPS_DIVISOR) *
    newPosition.collateralValue *
    (currentVaR / portfolioValue);

  const diversificationBenefit = marginalVaRs.reduce((sum, m) => {
    return sum + m.marginalVaR * (newPosition.collateralValue / newTotalValue);
  }, 0);

  const incrementalVaR = newAssetVaR - diversificationBenefit;
  const newPortfolioVaR = currentVaR + incrementalVaR;

  return { incrementalVaR, newPortfolioVaR };
}
