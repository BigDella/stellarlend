import { Request, Response } from 'express';
import { complianceService } from '../services/compliance.service';

export class ComplianceController {
  addSanction(req: Request, res: Response) {
    try {
      const { address, source, reason, expiresAt } = req.body;
      if (!address || !source || !reason) {
        return res.status(400).json({ error: 'address, source, reason required' });
      }
      const entry = complianceService.addSanction(address, source, reason, expiresAt);
      res.json({ success: true, entry });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  removeSanction(req: Request, res: Response) {
    try {
      const { address } = req.body;
      complianceService.removeSanction(address);
      res.json({ success: true });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  checkSanctioned(req: Request, res: Response) {
    try {
      const { address } = req.query;
      if (!address) return res.status(400).json({ error: 'address required' });
      const sanctioned = complianceService.checkSanctioned(address as string);
      res.json({ address, sanctioned });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  setKyc(req: Request, res: Response) {
    try {
      const { address, tier, jurisdiction, kycProvider, validityDays } = req.body;
      if (!address || !jurisdiction || !kycProvider) {
        return res.status(400).json({ error: 'address, jurisdiction, kycProvider required' });
      }
      const kyc = complianceService.setKycVerification({
        address, tier: tier ?? 1, jurisdiction, kycProvider, validityDays,
      });
      res.json({ success: true, kyc });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  revokeKyc(req: Request, res: Response) {
    try {
      const { address } = req.body;
      complianceService.revokeKyc(address);
      res.json({ success: true });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  checkKyc(req: Request, res: Response) {
    try {
      const { address } = req.query;
      if (!address) return res.status(400).json({ error: 'address required' });
      const valid = complianceService.checkKyc(address as string);
      const kyc = complianceService.getKyc(address as string);
      res.json({ address, valid, kyc });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  checkTransaction(req: Request, res: Response) {
    try {
      const { from, to, amount, asset } = req.body;
      if (!from || !to || !amount || !asset) {
        return res.status(400).json({ error: 'from, to, amount, asset required' });
      }
      const result = complianceService.checkTransaction({ from, to, amount, asset });
      res.json(result);
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  fileSar(req: Request, res: Response) {
    try {
      const { address, reason, amount, assetAddress, filedBy } = req.body;
      if (!address || !reason || !amount || !assetAddress || !filedBy) {
        return res.status(400).json({ error: 'address, reason, amount, assetAddress, filedBy required' });
      }
      const sar = complianceService.fileSar({ address, reason, amount, assetAddress, filedBy });
      res.json({ success: true, sar });
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  getSar(req: Request, res: Response) {
    try {
      const { sarId } = req.params!;
      const sar = complianceService.getSar(parseInt(sarId));
      if (!sar) return res.status(404).json({ error: 'SAR not found' });
      return res.json(sar);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  listSars(req: Request, res: Response) {
    try {
      const { status } = req.query;
      const sars = complianceService.listSars(status as string);
      res.json(sars);
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }

  getReport(req: Request, res: Response) {
    try {
      const { from, to } = req.query;
      if (!from || !to) return res.status(400).json({ error: 'from, to required' });
      const report = complianceService.getComplianceReport(from as string, to as string);
      return res.json(report);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  getAuditTrail(req: Request, res: Response) {
    try {
      const { address, limit } = req.query;
      const trail = complianceService.getAuditTrail(
        address as string,
        limit ? parseInt(limit as string) : 100
      );
      res.json(trail);
    } catch (err: any) {
      res.status(500).json({ error: err.message });
    }
  }
}

export const complianceController = new ComplianceController();
