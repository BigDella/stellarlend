import { Request, Response } from 'express';
import {
  riskSimulationService,
  PositionInput,
  SimulationScenario,
  CorrelationInput,
} from '../services/risk-simulation.service';

export class RiskSimulationController {
  async simulate(req: Request, res: Response): Promise<void> {
    try {
      const { position, scenario } = req.body as {
        position: PositionInput;
        scenario: SimulationScenario;
      };

      if (!position || !scenario) {
        res.status(400).json({ error: 'Position and scenario are required' });
        return;
      }

      const result = riskSimulationService.simulatePosition(position, scenario);
      res.json(result);
    } catch (error) {
      res.status(500).json({ error: 'Simulation failed', details: String(error) });
    }
  }

  async simulateBatch(req: Request, res: Response): Promise<void> {
    try {
      const { position, scenarios } = req.body as {
        position: PositionInput;
        scenarios: SimulationScenario[];
      };

      if (!position || !scenarios || !Array.isArray(scenarios)) {
        res.status(400).json({ error: 'Position and scenarios array are required' });
        return;
      }

      const results = riskSimulationService.simulateMultipleScenarios(position, scenarios);
      res.json({ results, count: results.length });
    } catch (error) {
      res.status(500).json({ error: 'Batch simulation failed', details: String(error) });
    }
  }

  async getScenarios(_req: Request, res: Response): Promise<void> {
    try {
      const scenarios = riskSimulationService.getPredefinedScenarios();
      res.json({ scenarios });
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch scenarios' });
    }
  }

  async getCorrelation(req: Request, res: Response): Promise<void> {
    try {
      const input = req.body as CorrelationInput;

      if (!input?.assets || !input?.prices) {
        res.status(400).json({ error: 'Assets and prices are required' });
        return;
      }

      const matrix = riskSimulationService.calculateCorrelationMatrix(input);
      res.json(matrix);
    } catch (error) {
      res.status(500).json({ error: 'Correlation analysis failed', details: String(error) });
    }
  }

  async getRecommendations(req: Request, res: Response): Promise<void> {
    try {
      const { position, targetHealthFactor } = req.body as {
        position: PositionInput;
        targetHealthFactor?: number;
      };

      if (!position) {
        res.status(400).json({ error: 'Position is required' });
        return;
      }

      const recommendations = riskSimulationService.generateRecommendations(
        position,
        targetHealthFactor
      );
      res.json({ recommendations });
    } catch (error) {
      res.status(500).json({ error: 'Failed to generate recommendations', details: String(error) });
    }
  }
}

export const riskSimulationController = new RiskSimulationController();
