import { Request, Response } from 'express';
import { treasuryRebalancerService } from '../services/treasury-rebalancer.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class TreasuryRebalancerController {
  async setTargetAllocations(req: Request, res: Response): Promise<void> {
    try {
      const { allocations } = req.body;

      if (!allocations || !Array.isArray(allocations)) {
        throw new ValidationError('allocations array is required');
      }

      treasuryRebalancerService.setTargetAllocations(allocations);

      res.json({
        success: true,
        message: 'Target allocations updated',
        count: allocations.length,
      });
    } catch (error) {
      logger.error('Failed to set target allocations:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to set target allocations' });
      }
    }
  }

  async setAssetPrices(req: Request, res: Response): Promise<void> {
    try {
      const { prices } = req.body;

      if (!prices || typeof prices !== 'object') {
        throw new ValidationError('prices object is required');
      }

      treasuryRebalancerService.setAssetPrices(prices);

      res.json({
        success: true,
        message: 'Asset prices updated',
        count: Object.keys(prices).length,
      });
    } catch (error) {
      logger.error('Failed to set asset prices:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to set asset prices' });
      }
    }
  }

  async setCurrentAllocations(req: Request, res: Response): Promise<void> {
    try {
      const { allocations } = req.body;

      if (!allocations || !Array.isArray(allocations)) {
        throw new ValidationError('allocations array is required');
      }

      treasuryRebalancerService.setCurrentAllocations(allocations);

      res.json({
        success: true,
        message: 'Current allocations updated',
        count: allocations.length,
      });
    } catch (error) {
      logger.error('Failed to set current allocations:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to set current allocations' });
      }
    }
  }

  async getCurrentAllocation(req: Request, res: Response): Promise<void> {
    try {
      const allocation = treasuryRebalancerService.getCurrentAllocation();

      res.json({
        success: true,
        data: allocation,
      });
    } catch (error) {
      logger.error('Failed to fetch current allocation:', error);
      res.status(500).json({ error: 'Failed to fetch current allocation' });
    }
  }

  async checkRebalanceTrigger(req: Request, res: Response): Promise<void> {
    try {
      const { deviationThreshold = 5 } = req.query;
      const result = treasuryRebalancerService.checkRebalanceTrigger(parseFloat(deviationThreshold as string) || 5);

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to check rebalance trigger:', error);
      res.status(500).json({ error: 'Failed to check rebalance trigger' });
    }
  }

  async simulateRebalance(req: Request, res: Response): Promise<void> {
    try {
      const { slippagePercentage = 0.3 } = req.query;
      const simulation = treasuryRebalancerService.simulateRebalance(parseFloat(slippagePercentage as string) || 0.3);

      res.json({
        success: true,
        data: simulation,
      });
    } catch (error) {
      logger.error('Failed to simulate rebalance:', error);
      res.status(500).json({ error: 'Failed to simulate rebalance' });
    }
  }

  async executeRebalance(req: Request, res: Response): Promise<void> {
    try {
      const { slippagePercentage = 0.3 } = req.body;
      const simulation = treasuryRebalancerService.simulateRebalance(slippagePercentage);
      const report = await treasuryRebalancerService.executeRebalance(simulation);

      res.json({
        success: true,
        data: report,
      });
    } catch (error) {
      logger.error('Failed to execute rebalance:', error);
      if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute rebalance' });
      }
    }
  }

  async getRebalanceHistory(req: Request, res: Response): Promise<void> {
    try {
      const { limit = 50 } = req.query;
      const history = treasuryRebalancerService.getRebalanceHistory(parseInt(limit as string) || 50);

      res.json({
        success: true,
        data: history,
        count: history.length,
      });
    } catch (error) {
      logger.error('Failed to fetch rebalance history:', error);
      res.status(500).json({ error: 'Failed to fetch rebalance history' });
    }
  }

  async createGovernanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { allocations, proposedBy } = req.body;

      if (!allocations || !Array.isArray(allocations)) {
        throw new ValidationError('allocations array is required');
      }

      if (!proposedBy) {
        throw new ValidationError('proposedBy is required');
      }

      const proposal = treasuryRebalancerService.createGovernanceProposal(allocations, proposedBy);

      res.json({
        success: true,
        data: proposal,
      });
    } catch (error) {
      logger.error('Failed to create governance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to create governance proposal' });
      }
    }
  }

  async getGovernanceProposals(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const proposals = treasuryRebalancerService.getGovernanceProposals(status as any);

      res.json({
        success: true,
        data: proposals,
        count: proposals.length,
      });
    } catch (error) {
      logger.error('Failed to fetch governance proposals:', error);
      res.status(500).json({ error: 'Failed to fetch governance proposals' });
    }
  }

  async approveGovernanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      treasuryRebalancerService.approveGovernanceProposal(proposalId);

      res.json({
        success: true,
        message: `Proposal ${proposalId} approved`,
      });
    } catch (error) {
      logger.error('Failed to approve governance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to approve governance proposal' });
      }
    }
  }

  async rejectGovernanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      treasuryRebalancerService.rejectGovernanceProposal(proposalId);

      res.json({
        success: true,
        message: `Proposal ${proposalId} rejected`,
      });
    } catch (error) {
      logger.error('Failed to reject governance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to reject governance proposal' });
      }
    }
  }

  async pauseRebalancer(req: Request, res: Response): Promise<void> {
    try {
      treasuryRebalancerService.pauseRebalancer();

      res.json({
        success: true,
        message: 'Rebalancer paused',
      });
    } catch (error) {
      logger.error('Failed to pause rebalancer:', error);
      res.status(500).json({ error: 'Failed to pause rebalancer' });
    }
  }

  async resumeRebalancer(req: Request, res: Response): Promise<void> {
    try {
      treasuryRebalancerService.resumeRebalancer();

      res.json({
        success: true,
        message: 'Rebalancer resumed',
      });
    } catch (error) {
      logger.error('Failed to resume rebalancer:', error);
      res.status(500).json({ error: 'Failed to resume rebalancer' });
    }
  }

  async getRebalancerStatus(req: Request, res: Response): Promise<void> {
    try {
      const isPaused = treasuryRebalancerService.isRebalancerPaused();

      res.json({
        success: true,
        data: {
          paused: isPaused,
        },
      });
    } catch (error) {
      logger.error('Failed to fetch rebalancer status:', error);
      res.status(500).json({ error: 'Failed to fetch rebalancer status' });
    }
  }
}

export const treasuryRebalancerController = new TreasuryRebalancerController();
