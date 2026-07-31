import { Request, Response } from 'express';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

interface ProposalRequest {
  amount: string;
  recipient: string;
  category: 'operations' | 'marketing' | 'development' | 'emergency';
}

interface ApprovalRequest {
  proposalId: string;
  signer: string;
}

interface SpendingLimitUpdate {
  category: 'operations' | 'marketing' | 'development' | 'emergency';
  dailyLimit: string;
  monthlyLimit: string;
}

const categoryMap: Record<string, number> = {
  operations: 0,
  marketing: 1,
  development: 2,
  emergency: 3,
};

export class TreasuryController {
  async createProposal(req: Request, res: Response): Promise<void> {
    try {
      const proposal: ProposalRequest = {
        amount: req.body.amount,
        recipient: req.body.recipient,
        category: req.body.category,
      };

      if (!proposal.amount || !proposal.recipient || !proposal.category) {
        throw new ValidationError('amount, recipient, and category are required');
      }

      if (!categoryMap[proposal.category]) {
        throw new ValidationError('Invalid category');
      }

      const proposalId = Math.random().toString(36).substr(2, 9);

      res.json({
        success: true,
        proposalId,
        proposal: {
          amount: proposal.amount,
          recipient: proposal.recipient,
          category: proposal.category,
          status: 'pending',
          approvals: 0,
        },
      });
    } catch (error) {
      logger.error('Proposal creation failed:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to create proposal' });
      }
    }
  }

  async approveProposal(req: Request, res: Response): Promise<void> {
    try {
      const approval: ApprovalRequest = {
        proposalId: req.body.proposalId,
        signer: req.body.signer,
      };

      if (!approval.proposalId || !approval.signer) {
        throw new ValidationError('proposalId and signer are required');
      }

      res.json({
        success: true,
        message: `Proposal ${approval.proposalId} approved by ${approval.signer}`,
        approvals: 1,
      });
    } catch (error) {
      logger.error('Approval failed:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to approve proposal' });
      }
    }
  }

  async executeProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      res.json({
        success: true,
        message: `Proposal ${proposalId} executed successfully`,
        transactionHash: `0x${Math.random().toString(16).substr(2)}`,
      });
    } catch (error) {
      logger.error('Execution failed:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute proposal' });
      }
    }
  }

  async getProposal(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId } = req.params;

      if (!proposalId) {
        throw new ValidationError('proposalId is required');
      }

      res.json({
        id: proposalId,
        amount: '1000000000',
        recipient: '0x0000000000000000000000000000000000000000',
        category: 'operations',
        status: 'pending',
        approvals: 0,
        requiredApprovals: 2,
        createdAt: new Date().toISOString(),
        requiresTimelock: false,
      });
    } catch (error) {
      logger.error('Failed to fetch proposal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to fetch proposal' });
      }
    }
  }

  async listProposals(req: Request, res: Response): Promise<void> {
    try {
      const { status, category } = req.query;

      res.json({
        total: 5,
        proposals: [
          {
            id: '1',
            amount: '1000000000',
            recipient: '0x0000000000000000000000000000000000000000',
            category: category || 'operations',
            status: status || 'pending',
            approvals: 1,
            requiredApprovals: 2,
            createdAt: new Date().toISOString(),
          },
        ],
      });
    } catch (error) {
      logger.error('Failed to list proposals:', error);
      res.status(500).json({ error: 'Failed to list proposals' });
    }
  }

  async updateSpendingLimit(req: Request, res: Response): Promise<void> {
    try {
      const update: SpendingLimitUpdate = {
        category: req.body.category,
        dailyLimit: req.body.dailyLimit,
        monthlyLimit: req.body.monthlyLimit,
      };

      if (!update.category || !update.dailyLimit || !update.monthlyLimit) {
        throw new ValidationError('category, dailyLimit, and monthlyLimit are required');
      }

      if (!categoryMap[update.category]) {
        throw new ValidationError('Invalid category');
      }

      res.json({
        success: true,
        message: `Spending limit updated for ${update.category}`,
        limits: {
          category: update.category,
          dailyLimit: update.dailyLimit,
          monthlyLimit: update.monthlyLimit,
        },
      });
    } catch (error) {
      logger.error('Failed to update spending limit:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to update spending limit' });
      }
    }
  }

  async getSpendingLimits(req: Request, res: Response): Promise<void> {
    try {
      const { category } = req.query;

      const limits = {
        operations: {
          daily: '1000000000',
          monthly: '10000000000',
          dailySpent: '500000000',
          monthlySpent: '2000000000',
        },
        marketing: {
          daily: '500000000',
          monthly: '5000000000',
          dailySpent: '100000000',
          monthlySpent: '1000000000',
        },
        development: {
          daily: '2000000000',
          monthly: '20000000000',
          dailySpent: '1000000000',
          monthlySpent: '5000000000',
        },
        emergency: {
          daily: '5000000000',
          monthly: '50000000000',
          dailySpent: '0',
          monthlySpent: '0',
        },
      };

      if (category && category in limits) {
        res.json({
          category,
          ...limits[category as keyof typeof limits],
        });
      } else {
        res.json(limits);
      }
    } catch (error) {
      logger.error('Failed to fetch spending limits:', error);
      res.status(500).json({ error: 'Failed to fetch spending limits' });
    }
  }

  async getAuditTrail(req: Request, res: Response): Promise<void> {
    try {
      const { proposalId, limit = 50, offset = 0 } = req.query;

      res.json({
        total: 10,
        events: [
          {
            timestamp: new Date().toISOString(),
            proposalId: proposalId || '1',
            action: 'approved',
            actor: '0x0000000000000000000000000000000000000000',
            details: {
              previousApprovals: 0,
              newApprovals: 1,
            },
          },
        ],
      });
    } catch (error) {
      logger.error('Failed to fetch audit trail:', error);
      res.status(500).json({ error: 'Failed to fetch audit trail' });
    }
  }
}

export const treasuryController = new TreasuryController();
