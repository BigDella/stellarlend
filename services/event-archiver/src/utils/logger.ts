import winston from 'winston';

export function createLogger(level: string = 'info') {
  return winston.createLogger({
    level,
    format: winston.format.combine(
      winston.format.timestamp(),
      winston.format.errors({ stack: true }),
      winston.format.json()
    ),
    defaultMeta: { service: 'event-archiver' },
    transports: [new winston.transports.Console()],
  });
}

export type Logger = ReturnType<typeof createLogger>;
