import { Router, Request, Response, NextFunction } from 'express';
import { snsService } from '../services/sns.service';
import logger from '../utils/logger';

const router: Router = Router();

const snsController = {
  async registerName(req: Request, res: Response, next: NextFunction) {
    try {
      const { name, address, owner } = req.body;
      if (!name || !address || !owner) {
        return res.status(400).json({ success: false, error: 'name, address, and owner are required' });
      }
      const record = snsService.registerName(name, address, owner);
      res.json({ success: true, data: record });
    } catch (error) {
      next(error);
    }
  },

  async resolveName(req: Request, res: Response, next: NextFunction) {
    try {
      const { name } = req.query;
      if (!name || typeof name !== 'string') {
        return res.status(400).json({ success: false, error: 'name query param required' });
      }
      const address = snsService.resolveName(name);
      res.json({ success: true, data: { name, address } });
    } catch (error) {
      next(error);
    }
  },

  async resolveBatch(req: Request, res: Response, next: NextFunction) {
    try {
      const { names } = req.body;
      if (!names || !Array.isArray(names)) {
        return res.status(400).json({ success: false, error: 'names array is required' });
      }
      const results = snsService.resolveNameBatch(names);
      const data: Record<string, string | null> = {};
      results.forEach((val, key) => { data[key] = val; });
      res.json({ success: true, data });
    } catch (error) {
      next(error);
    }
  },

  async validateName(req: Request, res: Response, next: NextFunction) {
    try {
      const { name } = req.query;
      if (!name || typeof name !== 'string') {
        return res.status(400).json({ success: false, error: 'name query param required' });
      }
      const valid = snsService.validateName(name);
      res.json({ success: true, data: { name, valid } });
    } catch (error) {
      next(error);
    }
  },

  async renewName(req: Request, res: Response, next: NextFunction) {
    try {
      const { name, owner } = req.body;
      if (!name || !owner) {
        return res.status(400).json({ success: false, error: 'name and owner are required' });
      }
      const record = snsService.renewName(name, owner);
      res.json({ success: true, data: record });
    } catch (error) {
      next(error);
    }
  },

  async isExpired(req: Request, res: Response, next: NextFunction) {
    try {
      const { name } = req.query;
      if (!name || typeof name !== 'string') {
        return res.status(400).json({ success: false, error: 'name query param required' });
      }
      const expired = snsService.isNameExpired(name);
      res.json({ success: true, data: { name, expired } });
    } catch (error) {
      next(error);
    }
  },

  async getAnalytics(_req: Request, res: Response, next: NextFunction) {
    try {
      const analytics = snsService.getAnalytics();
      res.json({ success: true, data: analytics });
    } catch (error) {
      next(error);
    }
  },

  async getRecord(req: Request, res: Response, next: NextFunction) {
    try {
      const { name } = req.query;
      if (!name || typeof name !== 'string') {
        return res.status(400).json({ success: false, error: 'name query param required' });
      }
      const record = snsService.getRecordByName(name);
      if (!record) {
        return res.status(404).json({ success: false, error: 'Name not found' });
      }
      res.json({ success: true, data: record });
    } catch (error) {
      next(error);
    }
  },

  async getAllNames(_req: Request, res: Response, next: NextFunction) {
    try {
      const names = snsService.getAllNames();
      res.json({ success: true, data: { names, count: names.length } });
    } catch (error) {
      next(error);
    }
  },

  async invalidateCache(req: Request, res: Response, next: NextFunction) {
    try {
      const { name } = req.body;
      if (!name) {
        return res.status(400).json({ success: false, error: 'name is required' });
      }
      snsService.invalidateCache(name);
      res.json({ success: true, data: { invalidated: name } });
    } catch (error) {
      next(error);
    }
  },
};

router.post('/register', snsController.registerName);
router.get('/resolve', snsController.resolveName);
router.post('/resolve-batch', snsController.resolveBatch);
router.get('/validate', snsController.validateName);
router.post('/renew', snsController.renewName);
router.get('/expired', snsController.isExpired);
router.get('/analytics', snsController.getAnalytics);
router.get('/record', snsController.getRecord);
router.get('/names', snsController.getAllNames);
router.post('/invalidate-cache', snsController.invalidateCache);

export default router;
