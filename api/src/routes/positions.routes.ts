import { Router } from 'express';
import * as positionsController from '../controllers/positions.controller';
import { validatePositionImport, validatePositionExport } from '../middleware/validation';

const router: Router = Router();

/**
 * @openapi
 * /positions/import/validate:
 *   post:
 *     summary: Validate position import data
 *     description: Validates CSV or JSON import data without executing the import
 *     tags:
 *       - Positions
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             $ref: '#/components/schemas/PositionImportData'
 *     responses:
 *       200:
 *         description: Validation result with errors and warnings
 *       400:
 *         description: Invalid import format
 */
router.post('/import/validate', validatePositionImport, positionsController.validateImport);

/**
 * @openapi
 * /positions/import:
 *   post:
 *     summary: Import positions in bulk
 *     description: Import multiple positions via CSV or JSON with transaction batching
 *     tags:
 *       - Positions
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             $ref: '#/components/schemas/PositionImportData'
 *     responses:
 *       200:
 *         description: Import completed with results
 *       400:
 *         description: Validation failed or import error
 */
router.post('/import', validatePositionImport, positionsController.importPositions);

/**
 * @openapi
 * /positions/export:
 *   post:
 *     summary: Export positions in CSV or JSON format
 *     description: Export positions with optional filtering by user addresses or asset
 *     tags:
 *       - Positions
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             $ref: '#/components/schemas/PositionExportOptions'
 *     responses:
 *       200:
 *         description: Exported positions data
 */
router.post('/export', validatePositionExport, positionsController.exportPositions);

/**
 * @openapi
 * /positions/import/template:
 *   get:
 *     summary: Generate import template
 *     description: Returns a template CSV or JSON structure for position imports
 *     tags:
 *       - Positions
 *     parameters:
 *       - in: query
 *         name: format
 *         schema:
 *           type: string
 *           enum: [csv, json]
 *         description: Template format
 *     responses:
 *       200:
 *         description: Import template with instructions
 */
router.get('/import/template', positionsController.getImportTemplate);

/**
 * @openapi
 * /positions/import/history:
 *   get:
 *     summary: Get import history
 *     description: Retrieve past position import operations with rollback capability
 *     tags:
 *       - Positions
 *     parameters:
 *       - in: query
 *         name: limit
 *         schema:
 *           type: integer
 *         description: Number of history entries to return
 *     responses:
 *       200:
 *         description: List of import history entries
 */
router.get('/import/history', positionsController.getImportHistory);

/**
 * @openapi
 * /positions/import/{importId}/rollback:
 *   post:
 *     summary: Rollback a position import
 *     description: Attempts to revert changes from a previous import operation
 *     tags:
 *       - Positions
 *     parameters:
 *       - in: path
 *         name: importId
 *         required: true
 *         schema:
 *           type: string
 *     responses:
 *       200:
 *         description: Rollback completed
 *       400:
 *         description: Import cannot be rolled back
 *       404:
 *         description: Import not found
 */
router.post('/import/:importId/rollback', positionsController.rollbackImport);

export default router;
