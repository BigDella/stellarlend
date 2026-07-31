/**
 * System Domain Routes (v1)
 *
 * Aggregates all infrastructure routes under /v1/system:
 * - Health checks (liveness, readiness, coalescing, cache metrics)
 * - Configuration management
 * - Developer portal (API keys, GraphQL playground, usage, webhooks, SDK)
 * - Analytics
 */

import { Router } from 'express';
import healthRoutes from '../../health.routes';
import configRoutes from '../../config.routes';
import developerRoutes from '../../developer.routes';
import analyticsRoutes from '../../analytics.routes';

const router = Router();

// Health: /v1/system/health/*
router.use('/health', healthRoutes);

// Config: /v1/system/config/*
router.use('/config', configRoutes);

// Developer portal: /v1/system/developer/*
router.use('/developer', developerRoutes);

// Analytics: /v1/system/analytics/*
router.use('/analytics', analyticsRoutes);

export default router;
