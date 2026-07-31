import { Router } from 'express';
import * as notificationController from '../controllers/notification.controller';

const router: Router = Router();

/**
 * @openapi
 * /notifications/subscribe:
 *   post:
 *     summary: Subscribe to notification channel
 *     tags:
 *       - Notifications
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [channel, recipient, alertTypes]
 *             properties:
 *               channel:
 *                 type: string
 *                 enum: [email, telegram, discord, push]
 *               recipient:
 *                 type: string
 *               alertTypes:
 *                 type: array
 *                 items:
 *                   type: string
 *     responses:
 *       201:
 *         description: Subscription created
 */
router.post('/subscribe', notificationController.subscribe);

/**
 * @openapi
 * /notifications/preferences:
 *   get:
 *     summary: Get notification preferences
 *     tags:
 *       - Notifications
 *     responses:
 *       200:
 *         description: User preferences
 */
router.get('/preferences', notificationController.getPreferences);

/**
 * @openapi
 * /notifications/preferences:
 *   put:
 *     summary: Update notification preference
 *     tags:
 *       - Notifications
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [channel, alertType, enabled]
 *             properties:
 *               channel:
 *                 type: string
 *               alertType:
 *                 type: string
 *               enabled:
 *                 type: boolean
 *               threshold:
 *                 type: number
 *     responses:
 *       200:
 *         description: Preference updated
 */
router.put('/preferences', notificationController.updatePreference);

/**
 * @openapi
 * /notifications/history:
 *   get:
 *     summary: Get notification history
 *     tags:
 *       - Notifications
 *     parameters:
 *       - in: query
 *         name: alertType
 *         schema:
 *           type: string
 *       - in: query
 *         name: channel
 *         schema:
 *           type: string
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *       - in: query
 *         name: cursor
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Notification history
 */
router.get('/history', notificationController.getHistory);

router.post('/:messageId/delivered', notificationController.markDelivered);
router.post('/:messageId/read', notificationController.markRead);

export default router;
