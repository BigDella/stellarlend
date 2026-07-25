import { Request, Response } from 'express';
import { treasuryYieldHarvesterService } from '../services/treasury-yield-harvester.service';
import { ValidationError } from '../utils/errors';
import logger from '../utils/logger';

export class TreasuryYieldHarvesterController {
  async registerProtocol(req: Request, res: Response): Promise<void> {
    try {
      const { protocolId, name, tvl, auditStatus, ageMonths, supportedStrategies, maxAllocationPercentage } = req.body;

      if (!protocolId || !name || tvl === undefined) {
        throw new ValidationError('protocolId, name, and tvl are required');
      }

      treasuryYieldHarvesterService.registerProtocol({
        protocolId,
        name,
        tvl,
        auditStatus: auditStatus || 'unaudited',
        ageMonths: ageMonths || 0,
        supportedStrategies: supportedStrategies || [],
        maxAllocationPercentage: maxAllocationPercentage || 20,
      });

      res.json({
        success: true,
        message: `Protocol ${name} registered`,
      });
    } catch (error) {
      logger.error('Failed to register protocol:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to register protocol' });
      }
    }
  }

  async updateWhitelist(req: Request, res: Response): Promise<void> {
    try {
      const { protocolIds } = req.body;

      if (!protocolIds || !Array.isArray(protocolIds)) {
        throw new ValidationError('protocolIds array is required');
      }

      treasuryYieldHarvesterService.updateProtocolWhitelist(protocolIds);

      res.json({
        success: true,
        message: 'Protocol whitelist updated',
        count: protocolIds.length,
      });
    } catch (error) {
      logger.error('Failed to update whitelist:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to update whitelist' });
      }
    }
  }

  async getProtocols(req: Request, res: Response): Promise<void> {
    try {
      const protocols = treasuryYieldHarvesterService.getProtocols();

      res.json({
        success: true,
        data: protocols,
        count: protocols.length,
      });
    } catch (error) {
      logger.error('Failed to fetch protocols:', error);
      res.status(500).json({ error: 'Failed to fetch protocols' });
    }
  }

  async getWhitelistedProtocols(req: Request, res: Response): Promise<void> {
    try {
      const protocols = treasuryYieldHarvesterService.getWhitelistedProtocols();

      res.json({
        success: true,
        data: protocols,
        count: protocols.length,
      });
    } catch (error) {
      logger.error('Failed to fetch whitelisted protocols:', error);
      res.status(500).json({ error: 'Failed to fetch whitelisted protocols' });
    }
  }

  async calculateRiskScore(req: Request, res: Response): Promise<void> {
    try {
      const { protocolId } = req.params;

      if (!protocolId) {
        throw new ValidationError('protocolId is required');
      }

      const riskScore = treasuryYieldHarvesterService.calculateRiskScore(protocolId);

      res.json({
        success: true,
        data: riskScore,
      });
    } catch (error) {
      logger.error('Failed to calculate risk score:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to calculate risk score' });
      }
    }
  }

  async deployToProtocol(req: Request, res: Response): Promise<void> {
    try {
      const { protocolId, asset, amount, strategy } = req.body;

      if (!protocolId || !asset || !amount || !strategy) {
        throw new ValidationError('protocolId, asset, amount, and strategy are required');
      }

      const position = treasuryYieldHarvesterService.deployToProtocol(protocolId, asset, amount, strategy);

      res.json({
        success: true,
        data: position,
      });
    } catch (error) {
      logger.error('Failed to deploy to protocol:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to deploy to protocol' });
      }
    }
  }

  async getPositions(req: Request, res: Response): Promise<void> {
    try {
      const { status } = req.query;
      const positions = treasuryYieldHarvesterService.getPositions(status as any);

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
      const result = treasuryYieldHarvesterService.harvestYield();

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to harvest yield:', error);
      res.status(500).json({ error: 'Failed to harvest yield' });
    }
  }

  async simulateWithdrawal(req: Request, res: Response): Promise<void> {
    try {
      const { positionId } = req.params;

      if (!positionId) {
        throw new ValidationError('positionId is required');
      }

      const simulation = treasuryYieldHarvesterService.simulateWithdrawal(positionId);

      res.json({
        success: true,
        data: simulation,
      });
    } catch (error) {
      logger.error('Failed to simulate withdrawal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to simulate withdrawal' });
      }
    }
  }

  async withdrawFromProtocol(req: Request, res: Response): Promise<void> {
    try {
      const { positionId } = req.params;

      if (!positionId) {
        throw new ValidationError('positionId is required');
      }

      const result = treasuryYieldHarvesterService.withdrawFromProtocol(positionId);

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to withdraw from protocol:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to withdraw from protocol' });
      }
    }
  }

  async emergencyWithdraw(req: Request, res: Response): Promise<void> {
    try {
      const { positionId } = req.params;
      const { reason } = req.body;

      if (!positionId || !reason) {
        throw new ValidationError('positionId and reason are required');
      }

      const result = treasuryYieldHarvesterService.emergencyWithdraw(positionId, reason);

      res.json({
        success: true,
        data: result,
      });
    } catch (error) {
      logger.error('Failed to execute emergency withdrawal:', error);
      if (error instanceof ValidationError) {
        res.status(400).json({ error: error.message });
      } else if (error instanceof Error) {
        res.status(400).json({ error: error.message });
      } else {
        res.status(500).json({ error: 'Failed to execute emergency withdrawal' });
      }
    }
  }

  async getYieldReport(req: Request, res: Response): Promise<void> {
    try {
      const { days = 30 } = req.query;
      const report = treasuryYieldHarvesterService.getYieldReport(parseInt(days as string) || 30);

      res.json({
        success: true,
        data: report,
      });
    } catch (error) {
      logger.error('Failed to fetch yield report:', error);
      res.status(500).json({ error: 'Failed to fetch yield report' });
    }
  }

  async getEmergencyWithdrawals(req: Request, res: Response): Promise<void> {
    try {
      const { limit = 50 } = req.query;
      const withdrawals = treasuryYieldHarvesterService.getEmergencyWithdrawals(parseInt(limit as string) || 50);

      res.json({
        success: true,
        data: withdrawals,
        count: withdrawals.length,
      });
    } catch (error) {
      logger.error('Failed to fetch emergency withdrawals:', error);
      res.status(500).json({ error: 'Failed to fetch emergency withdrawals' });
    }
  }
}

export const treasuryYieldHarvesterController = new TreasuryYieldHarvesterController();
