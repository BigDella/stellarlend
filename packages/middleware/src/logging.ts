import { Request, Response, NextFunction } from 'express';
import winston from 'winston';
import { RequestWithUser, LoggerConfig } from './types';

export const createLogger = (config: LoggerConfig = {}): winston.Logger => {
  return winston.createLogger({
    level: config.level || 'info',
    format: winston.format.combine(
      winston.format.timestamp(),
      winston.format.errors({ stack: true }),
      winston.format.json()
    ),
    silent: config.silent || false,
    transports: [
      new winston.transports.Console({
        format: winston.format.combine(
          winston.format.colorize(),
          winston.format.simple()
        ),
      }),
    ],
  });
};

const logger = createLogger();

export const requestLogger = (customLogger?: winston.Logger) => {
  const loggerInstance = customLogger || logger;

  return (req: RequestWithUser, res: Response, next: NextFunction): void => {
    const startTime = Date.now();
    const correlationId = req.correlationId || 'unknown';

    res.on('finish', () => {
      const duration = Date.now() - startTime;
      const logData = {
        method: req.method,
        url: req.originalUrl,
        status: res.statusCode,
        duration: `${duration}ms`,
        correlationId,
        userAgent: req.get('user-agent') || 'unknown',
        ip: req.ip || req.connection.remoteAddress,
      };

      if (res.statusCode >= 500) {
        loggerInstance.error('Request error', logData);
      } else if (res.statusCode >= 400) {
        loggerInstance.warn('Request warning', logData);
      } else {
        loggerInstance.info('Request completed', logData);
      }
    });

    next();
  };
};

export { logger };
