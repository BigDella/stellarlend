import { Request, Response } from 'express';
import { stressTestService } from '../services/risk-engine/stress-tester/stress-test.service';
import { scenarioRegistry } from '../services/risk-engine/stress-tester/scenario-registry';

export class StressTestController {
  async runStressTest(req: Request, res: Response): Promise<void> {
    try {
      const { scenarioId, scenario, positions: providedPositions, protocolLiquidity } = req.body;

      let selectedScenario = scenarioId
        ? scenarioRegistry.getScenario(scenarioId)
        : null;

      if (!selectedScenario && scenario) {
        selectedScenario = scenarioRegistry.buildCustomScenario(scenario);
      }

      if (!selectedScenario) {
        res.status(400).json({
          error: 'No valid scenario provided. Specify scenarioId or scenario config.',
        });
        return;
      }

      const positions = providedPositions || stressTestService.getDefaultPositions();

      const input = {
        scenario: selectedScenario,
        positions,
        totalCollateralValue: positions.reduce(
          (s: number, p: any) => s + p.collateral.reduce((cs: number, c: any) => cs + c.amount * c.price, 0),
          0
        ),
        totalDebtValue: positions.reduce(
          (s: number, p: any) => s + p.borrow.reduce((bs: number, b: any) => bs + b.amount * b.price, 0),
          0
        ),
        protocolLiquidity: protocolLiquidity || 1000000,
      };

      const result = stressTestService.runStressTest(input);
      res.json(result);
    } catch (error) {
      res.status(500).json({ error: 'Stress test execution failed', details: String(error) });
    }
  }

  async runAllScenarios(req: Request, res: Response): Promise<void> {
    try {
      const inputs = stressTestService.getDefaultInput();
      const results = stressTestService.runAllPredefined(inputs);

      const summary = Array.from(results.entries()).map(([id, result]) => ({
        scenarioId: id,
        scenarioName: result.scenarioName,
        riskLevel: result.summary.riskLevel,
        passed: result.passed,
        totalShortfall: result.summary.totalShortfall,
        positionsAffected: result.summary.totalPositionsAffected,
        durationMs: result.durationMs,
        recommendations: result.recommendations,
      }));

      res.json({
        totalScenarios: summary.length,
        passed: summary.filter((s) => s.passed).length,
        failed: summary.filter((s) => !s.passed).length,
        results: summary,
      });
    } catch (error) {
      res.status(500).json({ error: 'Batch stress test failed', details: String(error) });
    }
  }

  async runByCategory(req: Request, res: Response): Promise<void> {
    try {
      const { category } = req.params;
      const inputs = stressTestService.getDefaultInput();
      const results = stressTestService.runByCategory(inputs, category!);

      const summary = Array.from(results.entries()).map(([id, result]) => ({
        scenarioId: id,
        scenarioName: result.scenarioName,
        riskLevel: result.summary.riskLevel,
        passed: result.passed,
        totalShortfall: result.summary.totalShortfall,
      }));

      res.json({ category, count: summary.length, results: summary });
    } catch (error) {
      res.status(500).json({ error: 'Category stress test failed', details: String(error) });
    }
  }

  async getScenarios(_req: Request, res: Response): Promise<void> {
    try {
      const scenarios = scenarioRegistry.getAllScenarios();
      res.json({
        count: scenarios.length,
        scenarios: scenarios.map((s) => ({
          id: s.id,
          name: s.name,
          description: s.description,
          category: s.category,
          version: s.version,
          priceChanges: s.priceChanges,
          tags: s.tags,
          cascadingLiquidation: s.cascadingLiquidation,
        })),
      });
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch scenarios' });
    }
  }

  async getScenario(req: Request, res: Response): Promise<void> {
    try {
      const { id } = req.params;
      const scenario = scenarioRegistry.getScenario(id!);
      if (!scenario) {
        res.status(404).json({ error: 'Scenario not found' });
        return;
      }
      res.json(scenario);
    } catch (error) {
      res.status(500).json({ error: 'Failed to fetch scenario' });
    }
  }

  async createCustomScenario(req: Request, res: Response): Promise<void> {
    try {
      const { scenario } = req.body;
      if (!scenario) {
        res.status(400).json({ error: 'Scenario data required' });
        return;
      }
      const newScenario = scenarioRegistry.addCustomScenario(scenario);
      res.status(201).json(newScenario);
    } catch (error) {
      res.status(500).json({ error: 'Failed to create scenario', details: String(error) });
    }
  }

  async buildCustomScenario(req: Request, res: Response): Promise<void> {
    try {
      const config = req.body;
      if (!config?.name || !config?.priceChanges) {
        res.status(400).json({ error: 'Name and priceChanges are required' });
        return;
      }
      const scenario = scenarioRegistry.buildCustomScenario(config);
      const registered = scenarioRegistry.addCustomScenario(scenario);
      res.status(201).json(registered);
    } catch (error) {
      res.status(500).json({ error: 'Failed to build scenario', details: String(error) });
    }
  }

  async deleteScenario(req: Request, res: Response): Promise<void> {
    try {
      const { id } = req.params;
      const removed = scenarioRegistry.removeCustomScenario(id!);
      if (!removed) {
        res.status(404).json({ error: 'Custom scenario not found or cannot be removed' });
        return;
      }
      res.json({ success: true });
    } catch (error) {
      res.status(500).json({ error: 'Failed to delete scenario' });
    }
  }

  async exportScenario(req: Request, res: Response): Promise<void> {
    try {
      const { id } = req.params;
      const json = scenarioRegistry.exportScenario(id!);
      if (!json) {
        res.status(404).json({ error: 'Scenario not found' });
        return;
      }
      res.json({ id, json: JSON.parse(json) });
    } catch (error) {
      res.status(500).json({ error: 'Failed to export scenario' });
    }
  }

  async importScenario(req: Request, res: Response): Promise<void> {
    try {
      const { scenarioJson } = req.body;
      if (!scenarioJson) {
        res.status(400).json({ error: 'scenarioJson required' });
        return;
      }
      const json = typeof scenarioJson === 'string' ? scenarioJson : JSON.stringify(scenarioJson);
      const scenario = scenarioRegistry.importScenario(json);
      if (!scenario) {
        res.status(400).json({ error: 'Invalid scenario format' });
        return;
      }
      res.status(201).json(scenario);
    } catch (error) {
      res.status(500).json({ error: 'Failed to import scenario' });
    }
  }
}

export const stressTestController = new StressTestController();
