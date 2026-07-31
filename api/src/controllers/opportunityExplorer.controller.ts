import { Request, Response } from 'express';
import { opportunityExplorerService } from '../services/opportunity-explorer/opportunityExplorer.service';

export class OpportunityExplorerController {
  public getOpportunities = (req: Request, res: Response): void => {
    try {
      const filters = {
        minProfit: req.query.minProfit ? parseInt(req.query.minProfit as string, 10) : undefined,
        maxHf: req.query.maxHf ? parseFloat(req.query.maxHf as string) : undefined,
        minHf: req.query.minHf ? parseFloat(req.query.minHf as string) : undefined,
        asset: req.query.asset as string | undefined,
        minCollateral: req.query.minCollateral
          ? parseInt(req.query.minCollateral as string, 10)
          : undefined,
        sortBy: (req.query.sortBy as string) || 'healthFactor',
        sortDir: (req.query.sortDir as 'asc' | 'desc') || 'asc',
      };
      const opportunities = opportunityExplorerService.getOpportunities(filters);
      res.status(200).json({ opportunities, total: opportunities.length });
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch opportunities' });
    }
  };

  public getHistoricalLiquidations = (_req: Request, res: Response): void => {
    try {
      const history = opportunityExplorerService.getHistoricalLiquidations();
      res.status(200).json({ history });
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to fetch history' });
    }
  };

  public getGasEstimate = (_req: Request, res: Response): void => {
    try {
      const estimate = opportunityExplorerService.getGasEstimate();
      res.status(200).json(estimate);
    } catch (error: any) {
      res.status(500).json({ error: error.message || 'Failed to get gas estimate' });
    }
  };
}

export const opportunityExplorerController = new OpportunityExplorerController();
