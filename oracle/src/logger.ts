import winston from "winston";

const PII_FIELDS = ["password", "email", "phone", "token"];

const redactPII = (obj: Record<string, any>): Record<string, any> => {
  const redacted = { ...obj };
  for (const key of Object.keys(redacted)) {
    if (PII_FIELDS.some(f => key.toLowerCase().includes(f))) {
      redacted[key] = "[REDACTED]";
    }
  }
  return redacted;
};

export const oracleLogger = winston.createLogger({
  level: process.env.ORACLE_LOG_LEVEL || "info",
  format: winston.format.combine(
    winston.format.timestamp(),
    winston.format.printf(({ timestamp, level, message, ...meta }) => {
      return JSON.stringify({
        timestamp,
        level,
        service: "oracle",
        message,
        ...redactPII(meta as Record<string, any>),
      });
    })
  ),
  transports: [new winston.transports.Console()],
});

export default oracleLogger;
