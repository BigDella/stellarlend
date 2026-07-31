import winston from 'winston';

export function createLogger(level: string = 'info') {
  return winston.createLogger({
    level,
    format: winston.format.combine(winston.format.timestamp(), winston.format.json()),
    defaultMeta: { service: 'data-lake' },
    transports: [new winston.transports.Console()],
  });
}

export type Logger = ReturnType<typeof createLogger>;
