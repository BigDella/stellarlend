# StellarLend Developer Portal

The developer portal is served from `/api/developer` and links the interactive
REST explorer, GraphQL playground, API key management, usage status, SDK
quickstarts, and webhook test tooling.

## REST API Explorer

- Swagger UI: `/api/docs`
- OpenAPI schema: `/api/openapi.json`

The API enables CORS for `X-API-Key`, `X-Developer-Id`, and `X-User-Address`
headers so browser-based explorer and playground requests can authenticate
without custom proxy code.

## GraphQL Playground

The playground posts to `/api/developer/graphql`. It accepts an optional
`X-API-Key` header and never returns that key in the response body. The initial
federated surface exposes protocol, lending, and usage metadata and is designed
to be extended behind the same route as more subgraphs are added.

## API Keys

- `GET /api/developer/api-keys?actor=<id>` lists keys without hashes or raw
  secrets.
- `POST /api/developer/api-keys` generates a new key and returns the raw key
  once.
- `POST /api/developer/api-keys/:keyId/rotate` creates an overlapping
  replacement key.
- `DELETE /api/developer/api-keys/:keyId` revokes a key immediately.

Raw keys are never persisted. Only the bcrypt hash and prefix are retained by
the existing API key service.

## Usage And Rate Limits

`GET /api/developer/usage` returns sensitive-operation rate limit config and
current per-user counters. The sensitive operation limiter protects `borrow`,
`withdraw`, and `liquidate` with operation-specific sliding windows, standard
rate limit headers, graduated penalties, and trusted-user bypasses.

Environment overrides:

- `SENSITIVE_RATE_LIMIT_BORROW=max:windowMs:throttleAfter:blockAfter`
- `SENSITIVE_RATE_LIMIT_WITHDRAW=max:windowMs:throttleAfter:blockAfter`
- `SENSITIVE_RATE_LIMIT_LIQUIDATE=max:windowMs:throttleAfter:blockAfter`
- `RATE_LIMIT_TRUSTED_USERS=userA,userB`

## Webhook Testing

`POST /api/developer/webhooks/test` validates an HTTP(S) URL and returns a test
delivery payload. Production delivery workers can consume the same payload
shape when outbound webhook execution is wired to the queue.

## SDK Quickstarts

- `GET /api/developer/quickstart` returns machine-readable quickstart metadata.
- `GET /api/developer/sdk/typescript` returns a TypeScript client snippet.
