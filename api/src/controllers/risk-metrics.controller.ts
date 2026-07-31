import { Request, Response } from 'express';
import { varCalculator } from '../services/risk-engine/var-calculator';
import { VaRInput, Position, HistoricalPrice } from '../services/risk-engine/var-calculator/types';

export class RiskMetricsController {
  async getVaR(req: Request, res: Response): Promise<void> {
    try {
      const {
        confidenceLevel,
        timeHorizon,
        method,
        positions: positionsJson,
        portfolioValue,
      } = req.query;

      const confidence = confidenceLevel ? parseInt(confidenceLevel as string) : 95;
      const horizon = timeHorizon ? parseFloat(timeHorizon as string) : 1;
      const methodStr = (method as string) || 'all';
      const portfolioVal = portfolioValue ? parseFloat(portfolioValue as string) : 0;

      let positions: Position[];
      try {
        positions = positionsJson
          ? JSON.parse(positionsJson as string)
          : getDefaultPositions();
      } catch {
        positions = getDefaultPositions();
      }

      const input: VaRInput = {
        positions,
        confidenceLevel: confidence,
        timeHorizon: horizon,
        portfolioValue: portfolioVal > 0 ? portfolioVal : positions.reduce(
          (sum, p) => sum + p.collateralValue,
          0
        ),
      };

      let result: any;
      switch (methodStr) {
        case 'parametric':
          result = varCalculator.computeParametricVaR(input);
          break;
        case 'historical':
          result = varCalculator.computeHistoricalVaR(input, getMockHistoricalPrices());
          break;
        case 'monte-carlo':
          result = varCalculator.computeMonteCarloVaR(input);
          break;
        default:
          result = varCalculator.computeAllMethods(input, getMockHistoricalPrices());
      }

      res.json(result);
    } catch (error) {
      res.status(500).json({ error: 'VaR computation failed', details: String(error) });
    }
  }

  async getVaRHistory(req: Request, res: Response): Promise<void> {
    try {
      const { days, positions: positionsJson, portfolioValue } = req.query;
      const historyDays = days ? parseInt(days as string) : 30;

      let positions: Position[];
      try {
        positions = positionsJson
          ? JSON.parse(positionsJson as string)
          : getDefaultPositions();
      } catch {
        positions = getDefaultPositions();
      }

      const portfolioVal = portfolioValue
        ? parseFloat(portfolioValue as string)
        : positions.reduce((sum, p) => sum + p.collateralValue, 0);

      const history = varCalculator.generateHistoricalVaRData(
        positions,
        getMockHistoricalPrices(),
        portfolioVal,
        historyDays
      );

      res.json({ history, count: history.length });
    } catch (error) {
      res.status(500).json({ error: 'VaR history computation failed', details: String(error) });
    }
  }

  async getStressVaR(_req: Request, res: Response): Promise<void> {
    try {
      const positions = getDefaultPositions();
      const portfolioValue = positions.reduce((sum, p) => sum + p.collateralValue, 0);

      const stressResults = varCalculator.computeStressVaR(positions, portfolioValue);
      res.json({ scenarios: stressResults });
    } catch (error) {
      res.status(500).json({ error: 'Stress VaR computation failed', details: String(error) });
    }
  }
}

function getDefaultPositions(): Position[] {
  return [
    {
      asset: 'XLM',
      collateralValue: 150000,
      debtValue: 80000,
      volatility: 6500,
      weight: 0.4,
    },
    {
      asset: 'USDC',
      collateralValue: 200000,
      debtValue: 120000,
      volatility: 50,
      weight: 0.35,
    },
    {
      asset: 'BTC',
      collateralValue: 100000,
      debtValue: 50000,
      volatility: 8000,
      weight: 0.25,
    },
  ];
}

function getMockHistoricalPrices(): HistoricalPrice[] {
  const prices: HistoricalPrice[] = [];
  const today = Date.now();
  const basePrices = { XLM: 0.12, USDC: 1.0, BTC: 65000 };

  for (let i = 365; i >= 0; i--) {
    const timestamp = today - i * 24 * 60 * 60 * 1000;
    const dailyPrices: Record<string, number> = {};
    const volatility = 0.03;

    for (const [asset, base] of Object.entries(basePrices)) {
      const shock = (Math.random() - 0.5) * 2 * volatility;
      dailyPrices[asset] = base * (1 + shock);
    }

    prices.push({ timestamp, prices: dailyPrices });
  }

  return prices;
}

export const riskMetricsController = new RiskMetricsController();
