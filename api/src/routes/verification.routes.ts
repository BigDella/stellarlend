import { Router } from 'express';
import * as verificationController from '../controllers/verification.controller';

const router: Router = Router();

/**
 * @openapi
 * /verification:
 *   get:
 *     summary: Verify contract against source code
 *     tags:
 *       - Verification
 *     parameters:
 *       - name: contractId
 *         in: query
 *         required: true
 *         schema:
 *           type: string
 *         description: Contract ID to verify
 *       - name: network
 *         in: query
 *         schema:
 *           type: string
 *           default: testnet
 *         description: Network to verify on
 *     responses:
 *       200:
 *         description: Verification result
 *       400:
 *         description: Bad request
 *       500:
 *         description: Verification error
 */
router.get('/', verificationController.verifyContract);

/**
 * @openapi
 * /verification/kyc:
 *   post:
 *     summary: Submit KYC/AML verification and create an attestation
 *     tags:
 *       - Verification
 */
router.post('/kyc', verificationController.submitKycVerification);

/**
 * @openapi
 * /verification/kyc/{userAddress}:
 *   get:
 *     summary: Get KYC/AML verification status for a user
 *     tags:
 *       - Verification
 */
router.get('/kyc/:userAddress', verificationController.getKycStatus);

/**
 * @openapi
 * /verification/kyc/{userAddress}/proof:
 *   get:
 *     summary: Return a privacy-preserving proof summary without PII
 *     tags:
 *       - Verification
 */
router.get('/kyc/:userAddress/proof', verificationController.getPrivacyProof);

/**
 * @openapi
 * /verification/kyc/{userAddress}/revoke:
 *   post:
 *     summary: Revoke an existing verification attestation
 *     tags:
 *       - Verification
 */
router.post('/kyc/:userAddress/revoke', verificationController.revokeKycAttestation);

export default router;
