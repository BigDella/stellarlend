import { Request, Response, NextFunction } from 'express';
import { config, reloadConfig, getConfigSource } from '../config/index';
import { configAuditService } from '../services/configAudit.service';
import { assertValidConfig } from '../config/validators';
import { AppConfig } from '../config/types';

export const getConfig = (_req: Request, res: Response) => {
  const safeConfig: Record<string, unknown> = JSON.parse(JSON.stringify(config));
  if (safeConfig.auth && typeof safeConfig.auth === 'object') {
    (safeConfig.auth as Record<string, unknown>).jwtSecret = '***REDACTED***';
  }
  if (safeConfig.stellar && typeof safeConfig.stellar === 'object') {
    const s = safeConfig.stellar as Record<string, unknown>;
    if (s.relayerSecret) s.relayerSecret = '***REDACTED***';
  }
  res.status(200).json({
    config: safeConfig,
    source: getConfigSource(),
    timestamp: new Date().toISOString(),
  });
};

export const validateCurrentConfig = (_req: Request, res: Response) => {
  const errors: string[] = [];
  try {
    assertValidConfig(config);
  } catch (e) {
    errors.push((e as Error).message);
  }
  res.status(errors.length === 0 ? 200 : 400).json({
    valid: errors.length === 0,
    errors,
    timestamp: new Date().toISOString(),
  });
};

export const reloadConfiguration = async (_req: Request, res: Response, next: NextFunction) => {
  try {
    const previousSource = getConfigSource();
    reloadConfig();
    assertValidConfig(config);
    configAuditService.record({
      timestamp: new Date().toISOString(),
      action: 'reloaded',
      source: getConfigSource(),
    });
    res.status(200).json({
      success: true,
      message: 'Configuration reloaded',
      source: getConfigSource(),
      previousSource,
    });
  } catch (error) {
    next(error);
    return;
  }
};

export const updateConfigSection = (req: Request, res: Response, next: NextFunction) => {
  try {
    const { section, values } = req.body as { section: keyof AppConfig; values: Record<string, unknown> };
    const currentSection = config[section];
    if (!currentSection) {
      res.status(400).json({ success: false, error: `Unknown config section: ${section}` });
      return;
    }
    Object.assign(currentSection, values);
    assertValidConfig(config);
    configAuditService.record({
      timestamp: new Date().toISOString(),
      action: 'updated',
      source: getConfigSource(),
      changes: Object.entries(values).map(([k]) => `${section}.${k}`),
    });
    res.status(200).json({ success: true, message: `${section} configuration updated` });
  } catch (error) {
    next(error);
    return;
  }
};

export const getConfigAuditLog = (req: Request, res: Response) => {
  const limit = req.query.limit ? Number(req.query.limit) : 100;
  const offset = req.query.offset ? Number(req.query.offset) : 0;
  const entries = configAuditService.getLog(limit, offset);
  res.status(200).json({ total: configAuditService.count(), entries });
};
