import { Router, Request, Response, NextFunction } from 'express';
import { positionSimulator } from '../services/position-simulator';
import logger from '../utils/logger';

const router: Router = Router();

const simulatorController = {
  async simulatePriceDrop(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, priceDropPercent } = req.body;
      if (!position || priceDropPercent === undefined) {
        return res.status(400).json({ success: false, error: 'position and priceDropPercent required' });
      }
      const result = positionSimulator.simulatePriceDrop(position, priceDropPercent);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async simulateRateIncrease(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, rateIncreasePercent } = req.body;
      if (!position || rateIncreasePercent === undefined) {
        return res.status(400).json({ success: false, error: 'position and rateIncreasePercent required' });
      }
      const result = positionSimulator.simulateRateIncrease(position, rateIncreasePercent);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async simulateDeposit(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, depositAmount } = req.body;
      if (!position || depositAmount === undefined) {
        return res.status(400).json({ success: false, error: 'position and depositAmount required' });
      }
      const result = positionSimulator.simulateAdditionalDeposit(position, depositAmount);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async simulateRepay(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, repaymentAmount } = req.body;
      if (!position || repaymentAmount === undefined) {
        return res.status(400).json({ success: false, error: 'position and repaymentAmount required' });
      }
      const result = positionSimulator.simulatePartialRepayment(position, repaymentAmount);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async simulateComplex(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, priceDropPercent, rateIncreasePercent } = req.body;
      if (!position || priceDropPercent === undefined || rateIncreasePercent === undefined) {
        return res.status(400).json({
          success: false,
          error: 'position, priceDropPercent, and rateIncreasePercent required',
        });
      }
      const result = positionSimulator.simulateComplexScenario(position, priceDropPercent, rateIncreasePercent);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async simulateRealTime(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, changeType, amount } = req.body;
      if (!position || !changeType || amount === undefined) {
        return res.status(400).json({ success: false, error: 'position, changeType, and amount required' });
      }
      const result = positionSimulator.simulateRealTimeChange(position, changeType, amount);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async compareScenarios(req: Request, res: Response, next: NextFunction) {
    try {
      const { scenarioA, scenarioB } = req.body;
      if (!scenarioA || !scenarioB) {
        return res.status(400).json({ success: false, error: 'scenarioA and scenarioB required' });
      }
      const result = positionSimulator.compareScenarios(scenarioA, scenarioB);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async replayHistorical(req: Request, res: Response, next: NextFunction) {
    try {
      const { position, historicalDate, pricesOnDate } = req.body;
      if (!position || !historicalDate || !pricesOnDate) {
        return res.status(400).json({
          success: false,
          error: 'position, historicalDate, and pricesOnDate required',
        });
      }
      const pricesMap = new Map(Object.entries(pricesOnDate) as [string, number][]);
      const result = positionSimulator.replayHistoricalScenario(
        position,
        new Date(historicalDate),
        pricesMap,
      );
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async getAccuracy(req: Request, res: Response, next: NextFunction) {
    try {
      const { simulated, actual } = req.body;
      if (!simulated || !actual) {
        return res.status(400).json({ success: false, error: 'simulated and actual results required' });
      }
      const result = positionSimulator.getSimulationAccuracy(simulated, actual);
      res.json({ success: true, data: result });
    } catch (error) {
      next(error);
    }
  },

  async exportResult(req: Request, res: Response, next: NextFunction) {
    try {
      const { result } = req.body;
      if (!result) {
        return res.status(400).json({ success: false, error: 'result is required' });
      }
      const exported = positionSimulator.exportScenarioResult(result);
      res.json({ success: true, data: { json: exported } });
    } catch (error) {
      next(error);
    }
  },

  async saveSimulation(req: Request, res: Response, next: NextFunction) {
    try {
      const { userId, result } = req.body;
      if (!userId || !result) {
        return res.status(400).json({ success: false, error: 'userId and result required' });
      }
      positionSimulator.saveSimulation(userId, result);
      res.json({ success: true, data: { saved: true } });
    } catch (error) {
      next(error);
    }
  },

  async getUserSimulations(req: Request, res: Response, next: NextFunction) {
    try {
      const { userId } = req.query;
      if (!userId || typeof userId !== 'string') {
        return res.status(400).json({ success: false, error: 'userId query param required' });
      }
      const simulations = positionSimulator.getUserSimulations(userId);
      res.json({ success: true, data: simulations });
    } catch (error) {
      next(error);
    }
  },
};

router.post('/price-drop', simulatorController.simulatePriceDrop);
router.post('/rate-increase', simulatorController.simulateRateIncrease);
router.post('/deposit', simulatorController.simulateDeposit);
router.post('/repay', simulatorController.simulateRepay);
router.post('/complex', simulatorController.simulateComplex);
router.post('/realtime', simulatorController.simulateRealTime);
router.post('/compare', simulatorController.compareScenarios);
router.post('/historical', simulatorController.replayHistorical);
router.post('/accuracy', simulatorController.getAccuracy);
router.post('/export', simulatorController.exportResult);
router.post('/save', simulatorController.saveSimulation);
router.get('/history', simulatorController.getUserSimulations);

export default router;
