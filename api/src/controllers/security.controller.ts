import { Request, Response } from 'express';
import { vulnerabilityDisclosureService } from '../services/vulnerability-disclosure';

export const submitVulnerabilityReport = async (req: Request, res: Response) => {
  try {
    const report = await vulnerabilityDisclosureService.submitReport(req.body);
    res.status(201).json({ success: true, data: report });
  } catch (error: any) {
    res.status(400).json({ success: false, message: error.message });
  }
};

export const getVulnerabilityReport = async (req: Request, res: Response) => {
  try {
    const report = await vulnerabilityDisclosureService.getReportById(req.params.id);
    if (!report) {
      return res.status(404).json({ success: false, message: 'Report not found' });
    }
    res.json({ success: true, data: report });
  } catch (error: any) {
    res.status(400).json({ success: false, message: error.message });
  }
};

export const getTriageQueue = async (req: Request, res: Response) => {
  try {
    const queue = await vulnerabilityDisclosureService.getTriageQueue();
    res.json({ success: true, data: queue });
  } catch (error: any) {
    res.status(400).json({ success: false, message: error.message });
  }
};
