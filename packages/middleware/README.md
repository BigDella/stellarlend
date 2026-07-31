# @stellarlend/middleware

Shared middleware package for StellarLend API and Oracle services.

## Overview

This package provides common middleware functionality to ensure consistency across services and reduce code duplication.

## Installation

```bash
npm install @stellarlend/middleware
```

## Modules

### Authentication Middleware

Support for API key and JWT authentication:

```typescript
import { authMiddleware } from '@stellarlend/middleware';

const apiKeyAuth = authMiddleware.apiKey({
  apiKeyHeader: 'x-api-key',
  validateApiKey: async (key: string) => {
    return await checkApiKeyInDatabase(key);
  },
});

const jwtAuth = authMiddleware.jwt({
  jwtSecret: process.env.JWT_SECRET,
});

app.use('/api/protected', jwtAuth);
app.use('/api/public', apiKeyAuth);
```

### Logging Middleware

Request/response logging with correlation IDs:

```typescript
import { requestLogger, createLogger } from '@stellarlend/middleware';

const logger = createLogger({
  level: 'info',
});

app.use(requestLogger(logger));
```

Features:
- Automatic request/response logging
- Duration tracking
- Correlation ID support
- Structured JSON logs
- Error/warning detection based on status codes

### Rate Limiting

Configurable rate limiting:

```typescript
import { rateLimitMiddleware, createRateLimiter } from '@stellarlend/middleware';

app.use('/api', rateLimitMiddleware.standard);

const customLimiter = createRateLimiter({
  windowMs: 60 * 1000,
  max: 10,
  message: 'Custom rate limit message',
});

app.use('/api/sensitive', customLimiter);
```

Presets:
- `standard`: 100 requests per 15 minutes
- `strict`: 50 requests per 15 minutes
- `lenient`: 200 requests per 15 minutes

### Error Handling

Centralized error handling:

```typescript
import { errorHandler, asyncHandler, AppError } from '@stellarlend/middleware';

app.get('/api/data', asyncHandler(async (req, res) => {
  const data = await fetchData();
  if (!data) {
    throw new AppError('Data not found', 404);
  }
  res.json(data);
}));

app.use(errorHandler);
```

Features:
- Async error catching
- Operational vs programming error distinction
- Stack trace exposure control (dev vs production)
- Structured error responses

### Request ID

Correlation ID injection:

```typescript
import { requestIdMiddleware } from '@stellarlend/middleware';

app.use(requestIdMiddleware('x-correlation-id'));
```

Automatically generates or forwards correlation IDs for request tracing across services.

## Usage Examples

### Complete Express Setup

```typescript
import express from 'express';
import {
  requestIdMiddleware,
  requestLogger,
  rateLimitMiddleware,
  authMiddleware,
  errorHandler,
  createLogger,
} from '@stellarlend/middleware';

const app = express();
const logger = createLogger({ level: 'info' });

app.use(express.json());
app.use(requestIdMiddleware());
app.use(requestLogger(logger));
app.use(rateLimitMiddleware.standard);

app.use('/api/public', authMiddleware.apiKey({
  validateApiKey: async (key) => key === process.env.VALID_KEY,
}));

app.use('/api/protected', authMiddleware.jwt({
  jwtSecret: process.env.JWT_SECRET,
}));

app.use(errorHandler);

app.listen(3000);
```

### API Service Integration

```typescript
import { requestLogger, rateLimitMiddleware } from '@stellarlend/middleware';

app.use(requestLogger());
app.use('/api', rateLimitMiddleware.strict);
```

### Oracle Service Integration

```typescript
import { requestIdMiddleware, errorHandler } from '@stellarlend/middleware';

app.use(requestIdMiddleware());
app.use(errorHandler);
```

## Type Definitions

All middleware includes TypeScript type definitions:

```typescript
import type {
  AuthConfig,
  JWTPayload,
  RateLimitConfig,
  LoggerConfig,
  RequestWithUser,
} from '@stellarlend/middleware';
```

## Configuration

Environment variables:
- `NODE_ENV` - Controls error verbosity
- `JWT_SECRET` - JWT signing secret
- Custom variables as needed per service

## Testing

```bash
npm test
```

## License

MIT
