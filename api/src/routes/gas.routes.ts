/**
 * Gas Cost Estimation Routes
 * 
 * API endpoints for gas cost estimation and optimization
 */

import { Router } from 'express';
import { gasController } from '../controllers/gas.controller';

const router = Router();

/**
 * @route   POST /api/gas/estimate
 * @desc    Estimate gas cost for a specific operation
 * @body    { operation, userAddress, assetAddress?, amount, includeOptimizations?, includeHistorical? }
 * @access  Public
 */
router.post('/estimate', (req, res) => gasController.estimateGas(req, res));

/**
 * @route   GET /api/gas/historical/:operation
 * @desc    Get historical gas data for an operation
 * @query   period (optional): '24h', '7d', '30d' (default: '30d')
 * @access  Public
 */
router.get('/historical/:operation', (req, res) => gasController.getHistoricalData(req, res));

/**
 * @route   GET /api/gas/chart/:operation
 * @desc    Get historical gas chart data for visualization
 * @query   period (optional): '24h', '7d', '30d' (default: '7d')
 * @access  Public
 */
router.get('/chart/:operation', (req, res) => gasController.getChartData(req, res));

/**
 * @route   GET /api/gas/compare
 * @desc    Compare gas costs across all operations
 * @access  Public
 */
router.get('/compare', (req, res) => gasController.compareOperations(req, res));

/**
 * @route   POST /api/gas/alerts
 * @desc    Configure gas cost alert
 * @body    { userAddress?, operation, threshold, enabled }
 * @access  Public
 */
router.post('/alerts', (req, res) => gasController.configureAlert(req, res));

/**
 * @route   GET /api/gas/alerts
 * @desc    Get all alerts for a user
 * @query   userAddress (optional)
 * @access  Public
 */
router.get('/alerts', (req, res) => gasController.getAlerts(req, res));

/**
 * @route   POST /api/gas/accuracy
 * @desc    Record actual gas cost for accuracy tracking
 * @body    { operation, estimatedCost, actualCost, txHash }
 * @access  Public
 */
router.post('/accuracy', (req, res) => gasController.recordActualCost(req, res));

/**
 * @route   GET /api/gas/accuracy
 * @desc    Get accuracy report
 * @query   period (optional): '24h', '7d', '30d' (default: '7d')
 * @access  Public
 */
router.get('/accuracy', (req, res) => gasController.getAccuracyReport(req, res));

/**
 * @route   POST /api/gas/batch-estimate
 * @desc    Estimate gas cost for batch operations
 * @body    { operations: GasEstimateRequest[] }
 * @access  Public
 */
router.post('/batch-estimate', (req, res) => gasController.estimateBatchCost(req, res));

/**
 * @route   GET /api/gas/timing/:operation
 * @desc    Get timing recommendation for optimal execution
 * @access  Public
 */
router.get('/timing/:operation', (req, res) => gasController.getTimingRecommendation(req, res));

export default router;
