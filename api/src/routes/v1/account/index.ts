/**
 * Account Domain Routes (v1)
 *
 * Aggregates all user account routes under /v1/account:
 * - Portfolio analytics
 * - Transaction history
 * - Subscriptions
 */

import { Router } from 'express';
import portfolioRoutes from '../../portfolio.routes';
import transactionRoutes from '../../transaction.routes';
import subscriptionRoutes from '../../subscription.routes';

const router = Router();

// Portfolio: /v1/account/portfolio/*
router.use('/portfolio', portfolioRoutes);

// Transactions: /v1/account/transactions/*
router.use('/transactions', transactionRoutes);

// Subscriptions: /v1/account/subscriptions/*
router.use('/subscriptions', subscriptionRoutes);

export default router;
