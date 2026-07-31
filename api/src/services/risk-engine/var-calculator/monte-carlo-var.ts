import { VaRInput, VaRResult } from './types';
import { computeParametricVaR } from './parametric-var';

const NUM_SIMULATIONS = 10000;

function boxMullerTransform(): [number, number] {
  const u1 = Math.random();
  const u2 = Math.random();
  const r = Math.sqrt(-2 * Math.log(Math.max(u1, 1e-10)));
  const theta = 2 * Math.PI * u2;
  return [r * Math.cos(theta), r * Math.sin(theta)];
}

export function computeMonteCarloVaR(input: VaRInput): VaRResult {
  const { positions, confidenceLevel, timeHorizon, portfolioValue } = input;
  const alpha = 1 - confidenceLevel / 100;

  const weightedVol = positions.reduce((sum, p) => {
    return sum + (p.volatility / 10000) * p.weight;
  }, 0);

  const drift = 0;

  const simulatedReturns: number[] = [];
  for (let i = 0; i < NUM_SIMULATIONS; i++) {
    const [z1] = boxMullerTransform();

    const return_ = drift * timeHorizon + weightedVol * z1 * Math.sqrt(timeHorizon);
    simulatedReturns.push(return_);
  }

  const sortedReturns = [...simulatedReturns].sort((a, b) => a - b);

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
    method: 'monte-carlo',
    marginalContributions: paramVaR.marginalContributions,
    componentVaR: paramVaR.componentVaR,
    timestamp: Date.now(),
  };
}

export { NUM_SIMULATIONS };
