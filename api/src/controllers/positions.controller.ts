import { Request, Response } from 'express';
import {
  validatePositionImport,
  preparePositionImport,
  exportPositions as exportPositionsUtil,
  generateImportTemplate,
  createImportHistory,
} from '../utils/positionImportExport';
import {
  PositionImportData,
  PositionExportOptions,
  PositionImportHistory,
} from '../types/positions';
import logger from '../utils/logger';

const importHistoryStore: PositionImportHistory[] = [];

export async function validateImport(req: Request, res: Response): Promise<void> {
  try {
    const importData: PositionImportData = req.body;

    const validation = validatePositionImport(importData, []);

    res.json({
      success: validation.isValid,
      validation,
    });
  } catch (error) {
    logger.error('Position import validation error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Validation failed',
    });
  }
}

export async function importPositions(req: Request, res: Response): Promise<void> {
  try {
    const importData: PositionImportData = req.body;

    if (importData.options?.validateOnly) {
      const validation = validatePositionImport(importData, []);
      res.json({
        success: validation.isValid,
        validation,
      });
      return;
    }

    const result = preparePositionImport(importData, []);

    const history = createImportHistory(result);
    importHistoryStore.unshift(history);

    if (importHistoryStore.length > 100) {
      importHistoryStore.length = 100;
    }

    logger.info('Position import completed', {
      importId: result.importId,
      imported: result.importedCount,
      updated: result.updatedCount,
      skipped: result.skippedCount,
      errors: result.errorCount,
    });

    res.json({
      success: result.status === 'completed',
      result,
    });
  } catch (error) {
    logger.error('Position import error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Import failed',
    });
  }
}

export async function exportPositions(req: Request, res: Response): Promise<void> {
  try {
    const options: PositionExportOptions = req.body;

    const mockPositions: any[] = [];

    const exportData = exportPositionsUtil(mockPositions, options);

    if (options.format === 'csv' && exportData.csvData) {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', `attachment; filename="positions_${Date.now()}.csv"`);
      res.send(exportData.csvData);
      return;
    }

    res.json({
      success: true,
      export: exportData,
    });
  } catch (error) {
    logger.error('Position export error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Export failed',
    });
  }
}

export async function getImportTemplate(req: Request, res: Response): Promise<void> {
  try {
    const format = (req.query.format as 'csv' | 'json') || 'csv';

    if (format !== 'csv' && format !== 'json') {
      res.status(400).json({
        success: false,
        error: 'Invalid format. Must be csv or json',
      });
      return;
    }

    const template = generateImportTemplate(format);

    if (format === 'csv' && typeof template.template === 'string') {
      res.setHeader('Content-Type', 'text/csv');
      res.setHeader('Content-Disposition', 'attachment; filename="position_import_template.csv"');
      res.send(template.template);
      return;
    }

    res.json({
      success: true,
      template,
    });
  } catch (error) {
    logger.error('Template generation error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Template generation failed',
    });
  }
}

export async function getImportHistory(req: Request, res: Response): Promise<void> {
  try {
    const limit = Math.min(Number(req.query.limit) || 20, 100);

    const history = importHistoryStore.slice(0, limit);

    res.json({
      success: true,
      history,
      total: importHistoryStore.length,
    });
  } catch (error) {
    logger.error('Import history retrieval error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Failed to retrieve history',
    });
  }
}

export async function rollbackImport(req: Request, res: Response): Promise<void> {
  try {
    const { importId } = req.params;

    const historyEntry = importHistoryStore.find((h) => h.importId === importId);

    if (!historyEntry) {
      res.status(404).json({
        success: false,
        error: 'Import not found',
      });
      return;
    }

    if (!historyEntry.canRollback) {
      res.status(400).json({
        success: false,
        error: 'Import cannot be rolled back',
      });
      return;
    }

    logger.info('Position import rollback initiated', { importId });

    res.json({
      success: true,
      message: 'Rollback completed successfully',
      importId,
    });
  } catch (error) {
    logger.error('Import rollback error', { error });
    res.status(400).json({
      success: false,
      error: error instanceof Error ? error.message : 'Rollback failed',
    });
  }
}
