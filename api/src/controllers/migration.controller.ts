import { Request, Response, NextFunction } from 'express';
import * as migrationService from '../services/migration.service';

export async function getMigrationPreview(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const { sourcePool, destinationPool, amount, percentage } = req.body;
    const user = (req as any).user?.address || 'anonymous';

    if (!sourcePool || !destinationPool || !amount) {
      res.status(400).json({ error: 'sourcePool, destinationPool, and amount are required' });
      return;
    }

    const preview = await migrationService.getMigrationPreview(
      user,
      sourcePool,
      destinationPool,
      req.body.asset || '',
      Number(amount),
      Number(percentage) || 100
    );

    res.json(preview);
  } catch (error) {
    next(error);
  }
}

export async function getMigrationHistory(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const user = (req as any).user?.address || 'anonymous';
    const page = Number(req.query.page) || 1;
    const limit = Number(req.query.limit) || 20;

    const history = await migrationService.getMigrationHistory(user, page, limit);
    res.json(history);
  } catch (error) {
    next(error);
  }
}

export async function executeFullMigration(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const { sourcePool, destinationPool, asset, amount } = req.body;
    const user = (req as any).user?.address || 'anonymous';

    if (!sourcePool || !destinationPool || !amount) {
      res.status(400).json({ error: 'sourcePool, destinationPool, and amount are required' });
      return;
    }

    const record = await migrationService.executeMigration(
      user,
      sourcePool,
      destinationPool,
      asset || '',
      Number(amount),
      100
    );

    res.json(record);
  } catch (error) {
    next(error);
  }
}

export async function executePartialMigration(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const { sourcePool, destinationPool, asset, amount, percentage } = req.body;
    const user = (req as any).user?.address || 'anonymous';

    if (!sourcePool || !destinationPool || !amount || !percentage) {
      res.status(400).json({
        error: 'sourcePool, destinationPool, amount, and percentage are required',
      });
      return;
    }

    const record = await migrationService.executeMigration(
      user,
      sourcePool,
      destinationPool,
      asset || '',
      Number(amount),
      Number(percentage)
    );

    res.json(record);
  } catch (error) {
    next(error);
  }
}

export async function rollbackMigration(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const { migrationId } = req.params;
    const { reason } = req.body;

    if (!migrationId) {
      res.status(400).json({ error: 'migrationId is required' });
      return;
    }

    const record = await migrationService.rollbackMigration(
      Number(migrationId),
      reason || 'User initiated rollback'
    );

    res.json(record);
  } catch (error) {
    next(error);
  }
}

export async function getBulkMigrationPreview(
  req: Request,
  res: Response,
  next: NextFunction
): Promise<void> {
  try {
    const { sourcePool, destinationPool, asset } = req.body;

    if (!sourcePool || !destinationPool || !asset) {
      res.status(400).json({ error: 'sourcePool, destinationPool, and asset are required' });
      return;
    }

    const result = await migrationService.getBulkMigrationPreview({
      sourcePool,
      destinationPool,
      asset,
    });

    res.json(result);
  } catch (error) {
    next(error);
  }
}
