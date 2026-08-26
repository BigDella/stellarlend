import { Request, Response, NextFunction } from 'express';
import * as govSim from '../services/governanceSimulation.service';

export async function simulate(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { proposalId, kind, wouldSucceed, current, proposed } = req.body ?? {};
    if (!proposalId || !kind) {
      res.status(400).json({ success: false, error: 'proposalId and kind required' });
      return;
    }
    const result = govSim.simulateProposal({
      proposalId: String(proposalId),
      kind,
      wouldSucceed,
      current,
      proposed,
    });
    res.status(200).json({
      success: true,
      data: result,
      shareUrl: `/api/governance/simulate/share/${result.shareId}`,
    });
  } catch (error) {
    next(error);
  }
}

export async function getShared(req: Request, res: Response, next: NextFunction): Promise<void> {
  try {
    const { shareId } = req.params;
    const result = govSim.getSharedSimulation(shareId);
    if (!result) {
      res.status(404).json({ success: false, error: 'simulation not found' });
      return;
    }
    res.status(200).json({ success: true, data: result });
  } catch (error) {
    next(error);
  }
}
