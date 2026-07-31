import { Router } from 'express';
import * as configController from '../controllers/config.controller';
import { requireRole } from '../middleware/rbac';

const router: Router = Router();

/**
 * @openapi
 * /config:
 *   get:
 *     summary: Get current configuration
 *     description: Returns the full application configuration with secrets masked.
 *     tags:
 *       - Config
 *     responses:
 *       200:
 *         description: Current configuration
 */
router.get('/', requireRole('operator'), configController.getConfig);

/**
 * @openapi
 * /config/validate:
 *   get:
 *     summary: Validate current configuration
 *     description: Runs all config validation rules and returns any errors.
 *     tags:
 *       - Config
 */
router.get('/validate', requireRole('operator'), configController.validateCurrentConfig);

/**
 * @openapi
 * /config/reload:
 *   post:
 *     summary: Reload configuration from environment
 *     description: Re-reads env vars and reloads configuration without restarting the server.
 *     tags:
 *       - Config
 */
router.post('/reload', requireRole('admin'), configController.reloadConfiguration);

/**
 * @openapi
 * /config/update:
 *   post:
 *     summary: Update a configuration section at runtime
 *     description: Updates a specific config section with new values. Validates before applying.
 *     tags:
 *       - Config
 */
router.post('/update', requireRole('admin'), configController.updateConfigSection);

/**
 * @openapi
 * /config/audit-log:
 *   get:
 *     summary: Get configuration change audit log
 *     description: Returns the history of configuration changes, reloads, and validations.
 *     tags:
 *       - Config
 */
router.get('/audit-log', requireRole('operator'), configController.getConfigAuditLog);

export default router;
