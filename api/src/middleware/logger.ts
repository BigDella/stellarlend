import winston from "winston";
import { getNamespace } from "cls-hooked";

const PII_FIELDS = ["password", "email", "phone", "ssn", "token", "creditCard"];

const redactPII = (obj: Record<string, any>): Record<string, any> => {
  const redacted = { ...obj };
  for (const key of Object.keys(redacted)) {
    if (PII_FIELDS.some(f => key.toLowerCase().includes(f))) {
      redacted[key] = "[REDACTED]";
    }
  }
  return redacted;
};

const getCorrelationId = (): string => {
  const ns = getNamespace("request");
  const id = ns?.get("correlationId");
  return typeof id === "string" ? id : "no-correlation-id";
};

export const logger = winston.createLogger({
  level: process.env.LOG_LEVEL || "info",
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.errors({ stack: true }),
    winston.format.printf(({ timestamp, level, message, ...meta }) => {
      const safe = redactPII(meta as Record<string, any>);
      return JSON.stringify({
        timestamp,
        level,
        correlationId: getCorrelationId(),
        message,
        ...safe,
      });
    })
  ),
  transports: [new winston.transports.Console()],
});

export default logger;
