import { Router } from 'express';
import * as lendingController from '../controllers/lending.controller';
import {
  prepareValidation,
  submitValidation,
  relayDelegatedValidation,
  paginationValidation,
} from '../middleware/validation';

const router: Router = Router();

/**
 * @openapi
 * /lending/prepare/{operation}:
 *   get:
 *     summary: Prepare an unsigned lending transaction
 *     description: Builds an unsigned Soroban transaction XDR for the given lending operation. The client must sign the XDR and submit it via the submit endpoint.
 *     tags:
 *       - Lending
 *     parameters:
 *       - in: path
 *         name: operation
 *         required: true
 *         schema:
 *           type: string
 *           enum: [deposit, borrow, repay, withdraw]
 *         description: The lending operation to prepare
 *       - in: query
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *         description: Stellar public key (Ed25519)
 *       - in: query
 *         name: amount
 *         required: true
 *         schema:
 *           type: string
 *         description: Amount as a positive integer string (stroops)
 *       - in: query
 *         name: assetAddress
 *         required: false
 *         schema:
 *           type: string
 *         description: Optional asset contract address
 *     responses:
 *       200:
 *         description: Unsigned transaction XDR ready for signing
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/PrepareResponse'
 *       400:
 *         description: Validation error
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 *       500:
 *         description: Internal server error
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 */
// v2: client-side signing flow
router.get('/prepare/:operation', prepareValidation, lendingController.prepare);

/**
 * @openapi
 * /lending/submit:
 *   post:
 *     summary: Submit a signed transaction
 *     description: Submits a client-signed transaction XDR to the Stellar network and monitors it until completion.
 *     tags:
 *       - Lending
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required:
 *               - signedXdr
 *             properties:
 *               signedXdr:
 *                 type: string
 *                 description: Signed transaction XDR
 *               operation:
 *                 type: string
 *                 enum: [deposit, borrow, repay, withdraw]
 *                 description: Optional lending operation for audit logging
 *               userAddress:
 *                 type: string
 *                 description: Optional user address for audit logging
 *               amount:
 *                 type: string
 *                 description: Optional amount for audit logging
 *               assetAddress:
 *                 type: string
 *                 description: Optional asset address for audit logging
 *     responses:
 *       200:
 *         description: Transaction submitted and monitored successfully
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/TransactionResponse'
 *       400:
 *         description: Validation error or transaction failed
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 *       500:
 *         description: Internal server error
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 */
router.post('/submit', submitValidation, lendingController.submit);

router.post(
  '/relay-delegated',
  relayDelegatedValidation,
  lendingController.relayDelegated
);

/**
 * @openapi
 * /lending/transactions/{userAddress}:
 *   get:
 *     summary: Get transaction history for a user
 *     description: Retrieves a paginated list of past lending transactions for a specific user address, filtered by the lending contract operations.
 *     tags:
 *       - Lending
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *         description: Stellar public key (Ed25519) of the user
 *       - in: query
 *         name: limit
 *         required: false
 *         schema:
 *           type: integer
 *           minimum: 1
 *           maximum: 100
 *           default: 10
 *         description: Maximum number of transactions to return
 *       - in: query
 *         name: cursor
 *         required: false
 *         schema:
 *           type: string
 *         description: Pagination cursor for retrieving the next page
 *     responses:
 *       200:
 *         description: Transaction history retrieved successfully
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/PaginatedResponseTransactionHistory'
 *       400:
 *         description: Validation error or invalid address format
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 *       500:
 *         description: Internal server error
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/ErrorResponse'
 */
router.get(
  '/transactions/:userAddress',
  paginationValidation,
  lendingController.getTransactionHistory
);

/**
 * @openapi
 * /lending/transactions/{userAddress}/stream:
 *   get:
 *     summary: Stream full transaction history as NDJSON
 *     description: >
 *       Streams all lending transactions for the given user as newline-delimited JSON
 *       (one object per line, content-type application/x-ndjson). Uses chunked transfer
 *       encoding so clients receive items progressively without buffering the full dataset.
 *     tags:
 *       - Lending
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *         description: Stellar public key (Ed25519) of the user
 *       - in: query
 *         name: pageSize
 *         required: false
 *         schema:
 *           type: integer
 *           minimum: 1
 *           maximum: 200
 *           default: 10
 *         description: Horizon page size (internal batch size per upstream fetch)
 *     responses:
 *       200:
 *         description: NDJSON stream of TransactionHistoryItem objects
 *         content:
 *           application/x-ndjson:
 *             schema:
 *               $ref: '#/components/schemas/TransactionHistoryItem'
 */
router.get('/transactions/:userAddress/stream', lendingController.streamTransactionHistory);

/**
 * @openapi
 * /lending/liquidation-price/{asset}:
 *   get:
 *     summary: Get TWAP-based liquidation price
 *     description: Returns the manipulation-resistant TWAP liquidation price for an asset with fallback to median across sources.
 *     tags:
 *       - Lending
 *     parameters:
 *       - in: path
 *         name: asset
 *         required: true
 *         schema:
 *           type: string
 *         description: Asset contract address
 *     responses:
 *       200:
 *         description: TWAP-based liquidation price
 */
router.get('/liquidation-price/:asset', lendingController.getLiquidationPrice);

export default router;
