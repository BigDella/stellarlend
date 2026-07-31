import { Router } from 'express';
import * as controller from '../controllers/migration.controller';

const router = Router();

router.post('/preview', controller.getMigrationPreview);
router.get('/history', controller.getMigrationHistory);
router.post('/full', controller.executeFullMigration);
router.post('/partial', controller.executePartialMigration);
router.post('/rollback/:migrationId', controller.rollbackMigration);
router.post('/bulk-preview', controller.getBulkMigrationPreview);

export default router;
