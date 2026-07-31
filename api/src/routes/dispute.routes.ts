import { Router } from 'express';
import * as disputeController from '../controllers/dispute.controller';

const router: Router = Router();

/**
 * @openapi
 * /disputes/file:
 *   post:
 *     summary: File a dispute against a liquidation
 *     tags:
 *       - Disputes
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [liquidationTxHash, collateralAmount, evidence, disputeFee]
 *             properties:
 *               liquidationTxHash:
 *                 type: string
 *               collateralAmount:
 *                 type: string
 *               evidence:
 *                 type: string
 *               disputeFee:
 *                 type: string
 *     responses:
 *       201:
 *         description: Dispute created
 */
router.post('/file', disputeController.fileDispute);

/**
 * @openapi
 * /disputes/{id}:
 *   get:
 *     summary: Get dispute details
 *     tags:
 *       - Disputes
 *     parameters:
 *       - in: path
 *         name: id
 *         required: true
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Dispute details
 */
router.get('/:id', disputeController.getDispute);

/**
 * @openapi
 * /disputes/{id}/evidence:
 *   post:
 *     summary: Submit evidence for a dispute
 *     tags:
 *       - Disputes
 */
router.post('/:id/evidence', disputeController.submitEvidence);

/**
 * @openapi
 * /disputes/{id}/vote:
 *   post:
 *     summary: Cast vote as juror
 *     tags:
 *       - Disputes
 */
router.post('/:id/vote', disputeController.vote);

/**
 * @openapi
 * /disputes/{id}/appeal:
 *   post:
 *     summary: Appeal a resolved dispute
 *     tags:
 *       - Disputes
 */
router.post('/:id/appeal', disputeController.appeal);

router.get('/my/list', disputeController.getMyDisputes);

/**
 * @openapi
 * /disputes/juror/register:
 *   post:
 *     summary: Register as a juror
 *     tags:
 *       - Disputes
 */
router.post('/juror/register', disputeController.registerJuror);

export default router;
