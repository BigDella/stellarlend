import { Router, Request, Response, NextFunction } from 'express';
import { emergencyPauseService } from '../services/emergencyPause.service';
import logger from '../utils/logger';

const router: Router = Router();

const emergencyController = {
  async getStatus(_req: Request, res: Response, next: NextFunction) {
    try {
      const state = emergencyPauseService.isPaused();
      const queue = emergencyPauseService.getWithdrawalQueue();
      const history = emergencyPauseService.getEmergencyHistory();
      res.json({
        success: true,
        data: {
          ...state,
          queueLength: queue.length,
          queue,
          history,
        },
      });
    } catch (error) {
      next(error);
    }
  },

  async pause(req: Request, res: Response, next: NextFunction) {
    try {
      const { reason } = req.body;
      const pauseReason = reason || 'manual';
      emergencyPauseService.pause(pauseReason);
      logger.info(`Emergency pause triggered: ${pauseReason}`);
      res.json({ success: true, data: { paused: true, reason: pauseReason } });
    } catch (error) {
      next(error);
    }
  },

  async resume(_req: Request, res: Response, next: NextFunction) {
    try {
      emergencyPauseService.resume();
      logger.info('Emergency pause resumed');
      res.json({ success: true, data: { paused: false } });
    } catch (error) {
      next(error);
    }
  },

  async queueWithdrawal(req: Request, res: Response, next: NextFunction) {
    try {
      const { userAddress, assetAddress, amount } = req.body;
      if (!userAddress || !amount) {
        return res.status(400).json({ success: false, error: 'userAddress and amount required' });
      }
      emergencyPauseService.queueWithdrawal({ userAddress, assetAddress, amount });
      const queue = emergencyPauseService.getWithdrawalQueue();
      res.json({ success: true, data: { queued: true, queueLength: queue.length } });
    } catch (error) {
      next(error);
    }
  },

  async drainQueue(_req: Request, res: Response, next: NextFunction) {
    try {
      const drained = emergencyPauseService.drainWithdrawalQueue();
      logger.info(`Emergency withdrawal queue drained: ${drained.length} entries`);
      res.json({ success: true, data: { drained, count: drained.length } });
    } catch (error) {
      next(error);
    }
  },

  async getQueue(_req: Request, res: Response, next: NextFunction) {
    try {
      const queue = emergencyPauseService.getWithdrawalQueue();
      res.json({ success: true, data: { queue, length: queue.length } });
    } catch (error) {
      next(error);
    }
  },

  async triggerFailure(_req: Request, res: Response, next: NextFunction) {
    try {
      emergencyPauseService.recordFailure();
      const state = emergencyPauseService.isPaused();
      res.json({ success: true, data: state });
    } catch (error) {
      next(error);
    }
  },

  async triggerSuccess(_req: Request, res: Response, next: NextFunction) {
    try {
      emergencyPauseService.recordSuccess();
      res.json({ success: true, data: { consecutiveFailures: 0 } });
    } catch (error) {
      next(error);
    }
  },

  async getNotifications(_req: Request, res: Response, next: NextFunction) {
    try {
      const notifications = emergencyPauseService.getNotifications();
      res.json({ success: true, data: notifications });
    } catch (error) {
      next(error);
    }
  },

  async getHistory(_req: Request, res: Response, next: NextFunction) {
    try {
      const history = emergencyPauseService.getEmergencyHistory();
      res.json({ success: true, data: history });
    } catch (error) {
      next(error);
    }
  },
};

router.get('/status', emergencyController.getStatus);
router.post('/pause', emergencyController.pause);
router.post('/resume', emergencyController.resume);
router.post('/queue-withdrawal', emergencyController.queueWithdrawal);
router.post('/drain-queue', emergencyController.drainQueue);
router.get('/queue', emergencyController.getQueue);
router.post('/trigger-failure', emergencyController.triggerFailure);
router.post('/trigger-success', emergencyController.triggerSuccess);
router.get('/notifications', emergencyController.getNotifications);
router.get('/history', emergencyController.getHistory);

export default router;
