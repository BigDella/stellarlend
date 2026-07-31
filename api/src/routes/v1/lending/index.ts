/**
 * Lending Domain Routes (v1)
 *
 * Aggregates all lending-related routes under /v1/lending:
 * - Core lending operations (prepare, submit, transactions)
 * - Gas estimation
 * - Debt token operations
 * - Cross-asset operations
 */

import { Router } from 'express';
import lendingRoutes from '../../lending.routes';
import gasRoutes from '../../gas.routes';
import debtTokenRoutes from '../../debtToken.routes';
import crossAssetRoutes from '../../crossAsset.routes';

const router = Router();

// Core lending: /v1/lending/*
router.use('/', lendingRoutes);

// Gas estimation: /v1/lending/gas/*
router.use('/gas', gasRoutes);

// Debt tokens: /v1/lending/debt-token/*
router.use('/debt-token', debtTokenRoutes);

// Cross-asset: /v1/lending/cross-asset/*
router.use('/cross-asset', crossAssetRoutes);

export default router;
