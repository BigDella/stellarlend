import { Router } from 'express';
import * as creditController from '../controllers/credit.controller';

const router: Router = Router();

/**
 * @openapi
 * /credit/create:
 *   post:
 *     summary: Create a credit line
 *     tags:
 *       - Credit Delegation
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [delegateAddress, maxAmount, interestRate, maturityDate]
 *             properties:
 *               delegateAddress:
 *                 type: string
 *               maxAmount:
 *                 type: string
 *               interestRate:
 *                 type: string
 *               maturityDate:
 *                 type: string
 *               collateral:
 *                 type: string
 *     responses:
 *       201:
 *         description: Credit line created
 */
router.post('/create', creditController.createCreditLine);

/**
 * @openapi
 * /credit/{id}:
 *   get:
 *     summary: Get credit line details
 *     tags:
 *       - Credit Delegation
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Credit line details
 */
router.get('/:id', creditController.getCreditLine);

/**
 * @openapi
 * /credit/{id}/draw:
 *   post:
 *     summary: Draw from credit line
 *     tags:
 *       - Credit Delegation
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [amount]
 *             properties:
 *               amount:
 *                 type: string
 *     responses:
 *       200:
 *         description: Amount drawn
 */
router.post('/:id/draw', creditController.draw);

/**
 * @openapi
 * /credit/{id}/repay:
 *   post:
 *     summary: Repay credit line
 *     tags:
 *       - Credit Delegation
 */
router.post('/:id/repay', creditController.repay);

router.post('/:id/default', creditController.claimDefault);
router.put('/:id/limit', creditController.adjustLimit);
router.post('/:id/transfer', creditController.transfer);

router.get('/my/list', creditController.getMyCreditLines);

export default router;
