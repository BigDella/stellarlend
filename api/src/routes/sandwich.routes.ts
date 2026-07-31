import { Router } from 'express';

const router = Router();

const sandwichController = {
  getConfig: async (req: any, res: any) => {
    try {
      res.json({
        min_delay_secs: 1,
        max_delay_secs: 10,
        commit_expiry_secs: 60,
        large_tx_threshold: 100000,
        premium_fee_bps: 50,
      });
    } catch (error) {
      res.status(500).json({ error: 'Failed to get sandwich config' });
    }
  },

  setProtection: async (req: any, res: any) => {
    try {
      const { level } = req.body;
      if (!['None', 'Basic', 'Max'].includes(level)) {
        return res.status(400).json({ error: 'Invalid protection level' });
      }
      res.json({ success: true, level });
    } catch (error) {
      res.status(500).json({ error: 'Failed to set protection level' });
    }
  },

  getDetections: async (req: any, res: any) => {
    try {
      res.json([]);
    } catch (error) {
      res.status(500).json({ error: 'Failed to get detection log' });
    }
  },

  getExecutionOrder: async (req: any, res: any) => {
    try {
      res.json({ order: [], count: 0 });
    } catch (error) {
      res.status(500).json({ error: 'Failed to get execution order' });
    }
  },
};

router.get('/config', sandwichController.getConfig);
router.post('/protection', sandwichController.setProtection);
router.get('/detections', sandwichController.getDetections);
router.get('/execution-order', sandwichController.getExecutionOrder);

export default router;
