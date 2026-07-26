import { Request, Response } from 'express';

// Mock DB for demonstration
const auditFindings: any[] = [];

export const getAuditFindings = async (req: Request, res: Response) => {
  res.json({
    success: true,
    data: auditFindings
  });
};

export const createAuditFinding = async (req: Request, res: Response) => {
  const finding = {
    id: `AF-${Date.now()}`,
    ...req.body,
    createdAt: new Date(),
    status: 'open'
  };
  auditFindings.push(finding);
  res.status(201).json({ success: true, data: finding });
};

export const getAuditFindingById = async (req: Request, res: Response) => {
  const { id } = req.params;
  const finding = auditFindings.find(f => f.id === id);
  if (!finding) {
    return res.status(404).json({ success: false, message: 'Finding not found' });
  }
  res.json({ success: true, data: finding });
};

export const updateAuditFinding = async (req: Request, res: Response) => {
  const { id } = req.params;
  const index = auditFindings.findIndex(f => f.id === id);
  if (index === -1) {
    return res.status(404).json({ success: false, message: 'Finding not found' });
  }
  
  auditFindings[index] = { ...auditFindings[index], ...req.body, updatedAt: new Date() };
  res.json({ success: true, data: auditFindings[index] });
};

export const getAuditMetrics = async (req: Request, res: Response) => {
  const openCount = auditFindings.filter(f => f.status === 'open').length;
  const fixedCount = auditFindings.filter(f => f.status === 'fixed' || f.status === 'verified').length;
  
  res.json({
    success: true,
    data: {
      total: auditFindings.length,
      open: openCount,
      fixed: fixedCount,
      fixRate: auditFindings.length > 0 ? fixedCount / auditFindings.length : 0
    }
  });
};

export const getAuditReport = async (req: Request, res: Response) => {
  res.json({
    success: true,
    data: {
      lastAuditDate: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000).toISOString(),
      nextScheduled: new Date(Date.now() + 60 * 24 * 60 * 60 * 1000).toISOString(),
      agingReport: {
        over30Days: auditFindings.filter(f => f.status === 'open' && (new Date().getTime() - new Date(f.createdAt).getTime()) > 30 * 24 * 60 * 60 * 1000).length,
        over60Days: auditFindings.filter(f => f.status === 'open' && (new Date().getTime() - new Date(f.createdAt).getTime()) > 60 * 24 * 60 * 60 * 1000).length,
        over90Days: auditFindings.filter(f => f.status === 'open' && (new Date().getTime() - new Date(f.createdAt).getTime()) > 90 * 24 * 60 * 60 * 1000).length,
      }
    }
  });
};
