import { Position, StressVaRScenario } from './types';
import { computeParametricVaR } from './parametric-var';

const BPS_DIVISOR = 10000;

export const PREDEFINED_STRESS_SCENARIOS: StressVaRScenario[] = [
  {
    name: '2008 Financial Crisis',
    description: 'Lehman Brothers collapse, credit freeze, equity crash',
    volatilityMultiplier: 4.0,
    correlationShift: 0.5,
    priceDropPercent: 50,
  },
  {
    name: '2020 COVID Crash',
    description: 'Global pandemic, circuit breakers, liquidity crisis',
    volatilityMultiplier: 3.5,
    correlationShift: 0.4,
    priceDropPercent: 35,
  },
  {
    name: '2018 Crypto Winter',
    description: 'Bitcoin drops from $20k to $3k, altcoin mass liquidation',
    volatilityMultiplier: 5.0,
    correlationShift: 0.7,
    priceDropPercent: 80,
  },
  {
    name: 'Luna/UST Collapse',
    description: 'Algorithmic stablecoin death spiral, $40B wiped out',
    volatilityMultiplier: 6.0,
    correlationShift: 0.9,
    priceDropPercent: 95,
  },
  {
    name: 'FTX Collapse',
    description: 'Exchange insolvency, contagion across crypto lending',
    volatilityMultiplier: 4.5,
    correlationShift: 0.6,
    priceDropPercent: 70,
  },
  {
    name: 'Interest Rate Shock',
    description: 'Aggressive rate hikes, risk asset repricing',
    volatilityMultiplier: 2.5,
    correlationShift: 0.3,
    priceDropPercent: 25,
  },
];

export function computeStressVaR(
  positions: Position[],
  scenario: StressVaRScenario,
  portfolioValue: number
): {
  scenario: StressVaRScenario;
  var: number;
  cvar: number;
  stressedVolatility: number;
  collateralValueAfterStress: number;
} {
  const stressedPositions: Position[] = positions.map((p) => ({
    ...p,
    volatility: p.volatility * scenario.volatilityMultiplier,
    collateralValue: p.collateralValue * (1 - scenario.priceDropPercent / 100),
  }));

  const input = {
    positions: stressedPositions,
    confidenceLevel: 99,
    timeHorizon: 10,
    portfolioValue: portfolioValue * (1 - scenario.priceDropPercent / 100),
  };

  const result = computeParametricVaR(input);

  const avgVol = stressedPositions.reduce((sum, p) => sum + p.volatility, 0) / Math.max(1, stressedPositions.length);

  return {
    scenario,
    var: result.var,
    cvar: result.cvar,
    stressedVolatility: avgVol,
    collateralValueAfterStress: input.portfolioValue,
  };
}
