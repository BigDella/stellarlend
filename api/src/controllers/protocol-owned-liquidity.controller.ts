import { Request, Response } from 'express';
import { protocolOwnedLiquidityService } from '../services/protocol-owned-liquidity.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class ProtocolOwnedLiquidityController {
  async initializePool(req: Request, res: Response): Promise<void> {
    try {
      const { pool, totalLiquidity, utilizationRate, currentAPY } = req.body;

      if (!pool || totalLiquidity === undefined || utilizationRate === undefined || currentAPY === undefined) {
        throw new ValidationError('pool, totalLiquidity, utilizationRate, and currentAPY are required');
      }

      protocolOwnedLiquidityService.initializePool(pool, totalLiquidity, utilizationRate, currentAPY);

      res.json({
        success: true,
        message: `Pool ${pool} initialized`,
      });
    } catch (error) {
      logger.error('Failed to initialize pool:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to initialize pool' });
      }
    }
  }

  async createDeploymentProposal(req: Request, res: Response): Promise<void> {
    try {
      const { poolId, amount, proposedBy, timelockDuration } = req.body;

      if (!poolId || !amount || !proposedBy) {
        throw new ValidationError('poolId, amount, and proposedBy are required');
      }

      const proposal = protocolOwnedLiquidityService.createDeploymentProposal(
        poolId,
        amount,
        proposedBy,
        timelockDuration,
      );

      res.json({
        success: true,
        data: proposal,
      });
    } catch (error) {
      logger.error('Failed to create deployment proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to create deployment proposal' });
      }
    }
  }

  async approveDeploymentProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.approveDeploymentProposal(proposalId);

      res.json({
        success: true,
        message: `Deployment proposal ${proposalId} approved`,
      });
    } catch (error) {
      logger.error('Failed to approve deployment proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to approve deployment proposal' });
      }
    }
  }

  async executeDeploymentProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      const position = protocolOwnedLiquidityService.executeDeploymentProposal(proposalId);

      res.json({
        success: true,
        data: position,
      });
    } catch (error) {
      logger.error('Failed to execute deployment proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute deployment proposal' });
      }
    }
  }

  async createWithdrawalProposal(req: Request, res: Response): Promise<void> {
    try {
      const { positionId, proposedBy, timelockDuration } = req.body;

      if (!positionId || !proposedBy) {
        throw new ValidationError('positionId and proposedBy are required');
      }

      const proposal = protocolOwnedLiquidityService.createWithdrawalProposal(positionId, proposedBy, timelockDuration);

      res.json({
        success: true,
        data: proposal,
      });
    } catch (error) {
      logger.error('Failed to create withdrawal proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to create withdrawal proposal' });
      }
    }
  }

  async approveWithdrawalProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.approveWithdrawalProposal(proposalId);

      res.json({
        success: true,
        message: `Withdrawal proposal ${proposalId} approved`,
      });
    } catch (error) {
      logger.error('Failed to approve withdrawal proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to approve withdrawal proposal' });
      }
    }
  }

  async executeWithdrawalProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      const result = protocolOwnedLiquidityService.executeWithdrawalProposal(proposalId);

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to execute withdrawal proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute withdrawal proposal' });
      }
    }
  }

  async getPositions(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const positions = protocolOwnedLiquidityService.getPositions(status as any);

      res.json({
        success: true,
        data: positions,
        count: positions.length,
      });
    } catch (error) {
      logger.error('Failed to fetch positions:', error);
      res.status(500).json({ error: 'Failed to fetch positions' });
    }
  }

  async harvestYield(req: Request, res: Response): Promise<void> {
    try {
      const result = protocolOwnedLiquidityService.harvestYield();

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to harvest yield:', error);
      res.status(500).json({ error: 'Failed to harvest yield' });
    }
  }

  async createRebalanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { movements, proposedBy } = req.body;

      if (!movements || !Array.isArray(movements) || !proposedBy) {
        throw new ValidationError('movements array and proposedBy are required');
      }

      const proposal = protocolOwnedLiquidityService.createRebalanceProposal(movements, proposedBy);

      res.json({
        success: true,
        data: proposal,
      });
    } catch (error) {
      logger.error('Failed to create rebalance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to create rebalance proposal' });
      }
    }
  }

  async approveRebalanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.approveRebalanceProposal(proposalId);

      res.json({
        success: true,
        message: `Rebalance proposal ${proposalId} approved`,
      });
    } catch (error) {
      logger.error('Failed to approve rebalance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to approve rebalance proposal' });
      }
    }
  }

  async executeRebalanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.executeRebalanceProposal(proposalId);

      res.json({
        success: true,
        message: `Rebalance proposal ${proposalId} executed`,
      });
    } catch (error) {
      logger.error('Failed to execute rebalance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute rebalance proposal' });
      }
    }
  }

  async getDashboard(req: Request, res: Response): Promise<void> {
    try {
      const dashboard = protocolOwnedLiquidityService.getDashboard();

      res.json({
        success: true,
        data: dashboard,
      });
    } catch (error) {
      logger.error('Failed to fetch POL dashboard:', error);
      res.status(500).json({ error: 'Failed to fetch POL dashboard' });
    }
  }

  async getHistory(req: Request, res: Response): Promise<void> {
    try {
      const { limit = 100 } = req.query;
      const history = protocolOwnedLiquidityService.getHistory(parseInt(limit as string) || 100);

      res.json({
        success: true,
        data: history,
        count: history.length,
      });
    } catch (error) {
      logger.error('Failed to fetch POL history:', error);
      res.status(500).json({ error: 'Failed to fetch POL history' });
    }
  }

  async getDeploymentProposals(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const proposals = protocolOwnedLiquidityService.getDeploymentProposals(status as any);

      res.json({
        success: true,
        data: proposals,
        count: proposals.length,
      });
    } catch (error) {
      logger.error('Failed to fetch deployment proposals:', error);
      res.status(500).json({ error: 'Failed to fetch deployment proposals' });
    }
  }

  async getWithdrawalProposals(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const proposals = protocolOwnedLiquidityService.getWithdrawalProposals(status as any);

      res.json({
        success: true,
        data: proposals,
        count: proposals.length,
      });
    } catch (error) {
      logger.error('Failed to fetch withdrawal proposals:', error);
      res.status(500).json({ error: 'Failed to fetch withdrawal proposals' });
    }
  }

  async getRebalanceProposals(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const proposals = protocolOwnedLiquidityService.getRebalanceProposals(status as any);

      res.json({
        success: true,
        data: proposals,
        count: proposals.length,
      });
    } catch (error) {
      logger.error('Failed to fetch rebalance proposals:', error);
      res.status(500).json({ error: 'Failed to fetch rebalance proposals' });
    }
  }

  async rejectDeploymentProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.rejectDeploymentProposal(proposalId);

      res.json({
        success: true,
        message: `Deployment proposal ${proposalId} rejected`,
      });
    } catch (error) {
      logger.error('Failed to reject deployment proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to reject deployment proposal' });
      }
    }
  }

  async rejectWithdrawalProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.rejectWithdrawalProposal(proposalId);

      res.json({
        success: true,
        message: `Withdrawal proposal ${proposalId} rejected`,
      });
    } catch (error) {
      logger.error('Failed to reject withdrawal proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to reject withdrawal proposal' });
      }
    }
  }

  async rejectRebalanceProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      protocolOwnedLiquidityService.rejectRebalanceProposal(proposalId);

      res.json({
        success: true,
        message: `Rebalance proposal ${proposalId} rejected`,
      });
    } catch (error) {
      logger.error('Failed to reject rebalance proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to reject rebalance proposal' });
      }
    }
  }
}

export const protocolOwnedLiquidityController = new ProtocolOwnedLiquidityController();
