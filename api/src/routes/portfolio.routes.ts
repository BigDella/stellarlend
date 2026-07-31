import { Router } from 'express';
import * as portfolioController from '../controllers/portfolio.controller';
import { paginationValidation } from '../middleware/validation';

const router: Router = Router();

/**
 * @openapi
 * /portfolio/{userAddress}:
 *   get:
 *     summary: Full portfolio analytics
 *     description: >
 *       Returns a comprehensive analytics snapshot for the given user address including
 *       portfolio value, risk metrics (VaR, health factor, liquidation probability),
 *       optimisation suggestions, and historical performance summary.
 *       Results are cached for the configured position TTL.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *         description: Stellar public key (Ed25519)
 *     responses:
 *       200:
 *         description: Portfolio analytics snapshot
 *         content:
 *           application/json:
 *             schema:
 *               $ref: '#/components/schemas/PortfolioAnalyticsResponse'
 *       400:
 *         description: Invalid address
 *       500:
 *         description: Internal server error
 */
router.get('/:userAddress', portfolioController.getPortfolioAnalytics);

/**
 * @openapi
 * /portfolio/{userAddress}/risk:
 *   get:
 *     summary: Risk metrics and optimisation suggestions
 *     description: >
 *       Returns only the risk metrics (health factor, VaR, liquidation probability,
 *       drawdown) and optimisation suggestions for the user's current position.
 *       Lighter than the full analytics endpoint — skips transaction history fetch.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 */
router.get('/:userAddress/risk', portfolioController.getPortfolioRisk);

/**
 * @openapi
 * /portfolio/{userAddress}/performance:
 *   get:
 *     summary: Historical performance summary
 *     description: >
 *       Returns performance metrics derived from the user's transaction history:
 *       total deposited, withdrawn, borrowed, repaid, net flow, and operation breakdown.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *       - in: query
 *         name: limit
 *         required: false
 *         schema:
 *           type: integer
 *           minimum: 1
 *           maximum: 200
 *           default: 200
 */
router.get(
  '/:userAddress/performance',
  paginationValidation,
  portfolioController.getPortfolioPerformance
);

/**
 * @openapi
 * /portfolio/{userAddress}/export:
 *   get:
 *     summary: Export portfolio data for tax / accounting
 *     description: >
 *       Downloads the portfolio analytics and full transaction history as a JSON or CSV
 *       attachment. Use ?format=csv for a spreadsheet-ready transaction list,
 *       or omit / use ?format=json for the complete analytics bundle.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *       - in: query
 *         name: format
 *         required: false
 *         schema:
 *           type: string
 *           enum: [json, csv]
 *           default: json
 */
router.get('/:userAddress/export', portfolioController.exportPortfolio);

/**
 * @openapi
 * /portfolio/{userAddress}/interest-projection:
 *   get:
 *     summary: Interest accrual projection
 *     description: Projects interest accrual forward for 7, 30, and 90 day periods.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *       - in: query
 *         name: borrowApy
 *         schema:
 *           type: number
 *           default: 0.05
 */
router.get(
  '/:userAddress/interest-projection',
  portfolioController.getInterestProjection
);

/**
 * @openapi
 * /portfolio/{userAddress}/liquidation-price:
 *   get:
 *     summary: Liquidation price calculator
 *     description: Calculates the price threshold at which position becomes liquidatable.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 *       - in: query
 *         name: price
 *         schema:
 *           type: number
 *           default: 1
 */
router.get(
  '/:userAddress/liquidation-price',
  portfolioController.getLiquidationPrice
);

/**
 * @openapi
 * /portfolio/{userAddress}/health-monitor:
 *   get:
 *     summary: Health factor monitor with history
 *     description: Returns current health factor, historical trend, and risk assessment.
 *     tags:
 *       - Portfolio
 *     parameters:
 *       - in: path
 *         name: userAddress
 *         required: true
 *         schema:
 *           type: string
 */
router.get(
  '/:userAddress/health-monitor',
  portfolioController.getHealthFactorMonitor
);

export default router;
