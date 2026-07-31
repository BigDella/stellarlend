import { Router } from 'express';
import * as socialController from '../controllers/social.controller';

const router: Router = Router();

/**
 * @openapi
 * /social/leaderboard:
 *   get:
 *     summary: Get top lenders leaderboard
 *     tags:
 *       - Social Trading
 *     parameters:
 *       - in: query
 *         name: sortBy
 *         schema:
 *           type: string
 *           enum: [apy, totalReturns, riskAdjustedReturns, followers]
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *       - in: query
 *         name: offset
 *         schema:
 *           type: integer
 *       - in: query
 *         name: riskLevel
 *         schema:
 *           type: string
 *           enum: [low, medium, high]
 *     responses:
 *       200:
 *         description: Leaderboard entries
 */
router.get('/leaderboard', socialController.getLeaderboard);

/**
 * @openapi
 * /social/leader/{address}:
 *   get:
 *     summary: Get leader profile and strategy details
 *     tags:
 *       - Social Trading
 *     parameters:
 *       - in: path
 *         name: address
 *         required: true
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Leader profile
 */
router.get('/leader/:address', socialController.getLeaderProfile);

/**
 * @openapi
 * /social/follow:
 *   post:
 *     summary: Follow a leader to copy their strategy
 *     tags:
 *       - Social Trading
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [leaderAddress, amount, acknowledgeRisk]
 *             properties:
 *               leaderAddress:
 *                 type: string
 *               amount:
 *                 type: string
 *               acknowledgeRisk:
 *                 type: boolean
 *     responses:
 *       201:
 *         description: Follow relation created
 */
router.post('/follow', socialController.follow);

/**
 * @openapi
 * /social/unfollow:
 *   post:
 *     summary: Unfollow a leader (positions remain)
 *     tags:
 *       - Social Trading
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [leaderAddress]
 *             properties:
 *               leaderAddress:
 *                 type: string
 *     responses:
 *       200:
 *         description: Unfollowed successfully
 */
router.post('/unfollow', socialController.unfollow);

router.get('/my-following', socialController.getMyFollowing);

/**
 * @openapi
 * /social/privacy:
 *   put:
 *     summary: Set privacy opt-out for copying
 *     tags:
 *       - Social Trading
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               optOutCopying:
 *                 type: boolean
 *     responses:
 *       200:
 *         description: Privacy settings updated
 */
router.put('/privacy', socialController.setPrivacy);

export default router;
