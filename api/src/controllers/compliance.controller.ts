import { Request, Response } from 'express';
import { complianceService } from '../services/compliance.service';

export class ComplianceController {
  // ─── Sanctions ──────────────────────────────────────────────────────────

  addSanction(req: Request, res: Response) {
    try {
      const { address, source, reason, expiresAt } = req.body;
      if (!address || !source || !reason) {
        return res.status(400).json({ error: 'address, source, reason required' });
      }
      const entry = complianceService.addSanction(address, source, reason, expiresAt);
      return res.json({ success: true, entry });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  removeSanction(req: Request, res: Response) {
    try {
      const { address } = req.body;
      complianceService.removeSanction(address);
      return res.json({ success: true });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  checkSanctioned(req: Request, res: Response) {
    try {
      const { address } = req.query;
      if (!address) return res.status(400).json({ error: 'address required' });
      const sanctioned = complianceService.checkSanctioned(address as string);
      const ofac = complianceService.screenAgainstOFAC(address as string);
      return res.json({ address, sanctioned, ofacMatch: ofac.match, ofacConfidence: ofac.confidence });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── KYC ────────────────────────────────────────────────────────────────

  setKyc(req: Request, res: Response) {
    try {
      const { address, tier, jurisdiction, kycProvider, validityDays } = req.body;
      if (!address || !jurisdiction || !kycProvider) {
        return res.status(400).json({ error: 'address, jurisdiction, kycProvider required' });
      }
      const kyc = complianceService.setKycVerification({
        address, tier: tier ?? 1, jurisdiction, kycProvider, validityDays,
      });
      return res.json({ success: true, kyc });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  revokeKyc(req: Request, res: Response) {
    try {
      const { address } = req.body;
      complianceService.revokeKyc(address);
      return res.json({ success: true });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  checkKyc(req: Request, res: Response) {
    try {
      const { address } = req.query;
      if (!address) return res.status(400).json({ error: 'address required' });
      const valid = complianceService.checkKyc(address as string);
      const kyc = complianceService.getKyc(address as string);
      return res.json({ address, valid, kyc });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  listKycVerifications(req: Request, res: Response) {
    try {
      const verifications = complianceService.listKycVerifications();
      return res.json(verifications);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── AML ────────────────────────────────────────────────────────────────

  assessAmlRisk(req: Request, res: Response) {
    try {
      const { address } = req.params;
      if (!address) return res.status(400).json({ error: 'address required' });
      const assessment = complianceService.assessAmlRisk(address);
      return res.json(assessment);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  getAmlRisk(req: Request, res: Response) {
    try {
      const { address } = req.params;
      if (!address) return res.status(400).json({ error: 'address required' });
      const assessment = complianceService.getAmlRisk(address);
      if (!assessment) return res.status(404).json({ error: 'No AML assessment found for address' });
      return res.json(assessment);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  listAmlAssessments(req: Request, res: Response) {
    try {
      const { riskLevel } = req.query;
      const assessments = complianceService.listAmlAssessments(riskLevel as string);
      return res.json(assessments);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── Transaction Compliance ─────────────────────────────────────────────

  checkTransaction(req: Request, res: Response) {
    try {
      const { from, to, amount, asset, jurisdiction } = req.body;
      if (!from || !to || !amount || !asset) {
        return res.status(400).json({ error: 'from, to, amount, asset required' });
      }
      const result = complianceService.checkTransaction({ from, to, amount, asset, jurisdiction });
      return res.json(result);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── SAR ────────────────────────────────────────────────────────────────

  fileSar(req: Request, res: Response) {
    try {
      const { address, reason, amount, assetAddress, filedBy } = req.body;
      if (!address || !reason || !amount || !assetAddress || !filedBy) {
        return res.status(400).json({ error: 'address, reason, amount, assetAddress, filedBy required' });
      }
      const sar = complianceService.fileSar({ address, reason, amount, assetAddress, filedBy });
      return res.json({ success: true, sar });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  getSar(req: Request, res: Response) {
    try {
      const { sarId } = req.params!;
      const sar = complianceService.getSar(parseInt(sarId!));
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
      return res.json(sars);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  updateSarStatus(req: Request, res: Response) {
    try {
      const { sarId } = req.params!;
      const { status, notes } = req.body;
      if (!status) return res.status(400).json({ error: 'status required' });
      const sar = complianceService.updateSarStatus(parseInt(sarId!), status, notes);
      if (!sar) return res.status(404).json({ error: 'SAR not found' });
      return res.json({ success: true, sar });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── Compliance Reporting ───────────────────────────────────────────────

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

  // ─── Audit Trail ────────────────────────────────────────────────────────

  getAuditTrail(req: Request, res: Response) {
    try {
      const { address, limit, eventType } = req.query;
      const trail = complianceService.getAuditTrail(
        address as string,
        limit ? parseInt(limit as string) : 100,
        eventType as string
      );
      return res.json(trail);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  verifyAuditTrailIntegrity(req: Request, res: Response) {
    try {
      const result = complianceService.verifyAuditTrailIntegrity();
      return res.json(result);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── Compliance Configuration ───────────────────────────────────────────

  getConfig(req: Request, res: Response) {
    try {
      const config = complianceService.getConfig();
      return res.json(config);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  updateConfig(req: Request, res: Response) {
    try {
      const updates = req.body;
      if (!updates || Object.keys(updates).length === 0) {
        return res.status(400).json({ error: 'No configuration updates provided' });
      }
      const config = complianceService.updateConfig(updates);
      return res.json({ success: true, config });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  setJurisdictionLimits(req: Request, res: Response) {
    try {
      const { jurisdiction, limits } = req.body;
      if (!jurisdiction || !limits) {
        return res.status(400).json({ error: 'jurisdiction, limits required' });
      }
      complianceService.setJurisdictionLimits(jurisdiction, limits);
      return res.json({ success: true });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  addRestrictedJurisdiction(req: Request, res: Response) {
    try {
      const { jurisdiction, addedBy } = req.body;
      if (!jurisdiction || !addedBy) {
        return res.status(400).json({ error: 'jurisdiction, addedBy required' });
      }
      complianceService.addRestrictedJurisdiction(jurisdiction, addedBy);
      return res.json({ success: true });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  removeRestrictedJurisdiction(req: Request, res: Response) {
    try {
      const { jurisdiction, removedBy } = req.body;
      if (!jurisdiction || !removedBy) {
        return res.status(400).json({ error: 'jurisdiction, removedBy required' });
      }
      complianceService.removeRestrictedJurisdiction(jurisdiction, removedBy);
      return res.json({ success: true });
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── Compliance Dashboard ───────────────────────────────────────────────

  getDashboard(req: Request, res: Response) {
    try {
      const dashboard = complianceService.getDashboard();
      return res.json(dashboard);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }

  // ─── Regulatory Limits ──────────────────────────────────────────────────

  checkRegulatoryLimits(req: Request, res: Response) {
    try {
      const { address, amount, jurisdiction } = req.body;
      if (!address || !amount) {
        return res.status(400).json({ error: 'address, amount required' });
      }
      const result = complianceService.checkRegulatoryLimits(address, amount, jurisdiction);
      return res.json(result);
    } catch (err: any) {
      return res.status(500).json({ error: err.message });
    }
  }
}

export const complianceController = new ComplianceController();
