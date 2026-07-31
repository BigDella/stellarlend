import { VaRInput, VaRResult, MarginalVaR, Position } from './types';

const Z_SCORES: Record<number, number> = {
  90: 1.28155,
  95: 1.64485,
  97.5: 1.95996,
  99: 2.32635,
  99.9: 3.09023,
};

const BPS_DIVISOR = 10000;

export function computeParametricVaR(input: VaRInput): VaRResult {
  const { positions, confidenceLevel, timeHorizon, portfolioValue } = input;
  const zScore = Z_SCORES[confidenceLevel] ?? 1.64485;

  const weightedVol = positions.reduce((sum, p) => {
    return sum + (p.volatility / BPS_DIVISOR) * p.weight;
  }, 0);

  const scaledVol = weightedVol * Math.sqrt(timeHorizon);

  const var_ = portfolioValue * scaledVol * zScore;
  const cvar_ = portfolioValue * scaledVol * this.cvarAdjustment(zScore, confidenceLevel);

  const marginalContributions = computeMarginalContributions(positions, scaledVol, portfolioValue, zScore);

  const componentVaR = marginalContributions.map((m) => m.componentVaR);

  return {
    var: var_,
    cvar: cvar_,
    confidenceLevel,
    timeHorizon,
    method: 'parametric',
    marginalContributions,
    componentVaR,
    timestamp: Date.now(),
  };
}

function cvarAdjustment(zScore: number, confidenceLevel: number): number {
  const alpha = 1 - confidenceLevel / 100;
  const pdf = Math.exp(-0.5 * zScore * zScore) / Math.sqrt(2 * Math.PI);
  return pdf / alpha;
}

function computeMarginalContributions(
  positions: Position[],
  scaledVol: number,
  portfolioValue: number,
  zScore: number
): MarginalVaR[] {
  return positions.map((p) => {
    const ivar = (p.volatility / BPS_DIVISOR) * portfolioValue * scaledVol * zScore * p.weight;
    const cvar = ivar * p.weight * portfolioValue;
    return {
      asset: p.asset,
      marginalVaR: ivar,
      componentVaR: cvar,
      percentContribution: positions.length > 0 ? (ivar / Math.max(1, ivar)) * 100 * p.weight : 0,
    };
  });
}

export { Z_SCORES };
