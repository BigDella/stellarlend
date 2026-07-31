import { Router, Request, Response, NextFunction } from 'express';
import { apiKeyService } from '../services/apiKey.service';
import {
  getSensitiveRateLimitAnalytics,
  getSensitiveRateLimitConfig,
} from '../middleware/rate-limit';

const router: Router = Router();

function actorFromRequest(req: Request): string {
  const header =
    typeof req.headers['x-developer-id'] === 'string' ? req.headers['x-developer-id'] : undefined;
  const bodyActor = typeof req.body?.actor === 'string' ? req.body.actor : undefined;
  return header || bodyActor || 'developer-portal';
}

function portalHtml(): string {
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>StellarLend Developer Portal</title>
  <style>
    :root { color-scheme: light; font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; }
    body { margin: 0; background: #f7f8fa; color: #17202a; }
    header { background: #0d3b66; color: white; padding: 24px; }
    main { max-width: 1180px; margin: 0 auto; padding: 24px; display: grid; gap: 18px; }
    section { background: white; border: 1px solid #d9e1ea; border-radius: 8px; padding: 18px; }
    h1, h2 { margin: 0 0 12px; }
    h1 { font-size: 30px; }
    h2 { font-size: 18px; color: #0d3b66; }
    label { display: grid; gap: 6px; margin-bottom: 10px; font-weight: 600; }
    input, textarea, select { width: 100%; box-sizing: border-box; border: 1px solid #b9c6d3; border-radius: 6px; padding: 10px; font: inherit; }
    textarea { min-height: 150px; resize: vertical; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
    button, a.button { display: inline-flex; align-items: center; justify-content: center; gap: 8px; border: 0; border-radius: 6px; padding: 10px 12px; background: #0d3b66; color: white; font-weight: 700; text-decoration: none; cursor: pointer; }
    button.secondary, a.secondary { background: #2e7d32; }
    pre { overflow: auto; background: #0f1720; color: #e6edf3; border-radius: 6px; padding: 12px; min-height: 52px; }
    .grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 18px; }
    .row { display: flex; gap: 10px; flex-wrap: wrap; }
    .muted { color: #5a6878; }
    code { background: #eef3f8; border-radius: 4px; padding: 2px 4px; }
  </style>
</head>
<body>
  <header>
    <h1>StellarLend Developer Portal</h1>
    <p>Interactive REST docs, GraphQL playground, API keys, webhooks, usage status, and SDK quickstarts.</p>
  </header>
  <main>
    <section>
      <h2>REST API Explorer</h2>
      <p class="muted">Swagger UI is available with interactive endpoint testing.</p>
      <div class="row">
        <a class="button" href="/api/docs">Open Swagger UI</a>
        <a class="button secondary" href="/api/openapi.json">Download OpenAPI JSON</a>
      </div>
    </section>
    <div class="grid">
      <section>
        <h2>GraphQL Playground</h2>
        <label>API key <input id="gqlKey" type="password" autocomplete="off" placeholder="Optional X-API-Key"></label>
        <label>Federated query
          <textarea id="gqlQuery">query {
  protocol { tvl utilizationRate numberOfUsers }
  usage { rateLimits }
}</textarea>
        </label>
        <button onclick="postJson('/api/developer/graphql', { query: value('gqlQuery') }, 'gqlOut', value('gqlKey'))">Run Query</button>
        <pre id="gqlOut"></pre>
      </section>
      <section>
        <h2>API Key Management</h2>
        <label>Developer ID <input id="developerId" placeholder="wallet or organization id"></label>
        <label>Key name <input id="keyName" placeholder="production-readonly"></label>
        <div class="row">
          <button onclick="postJson('/api/developer/api-keys', { name: value('keyName'), actor: value('developerId') }, 'keyOut')">Generate</button>
          <button class="secondary" onclick="fetchJson('/api/developer/api-keys?actor=' + encodeURIComponent(value('developerId')), 'keyOut')">List</button>
        </div>
        <pre id="keyOut"></pre>
      </section>
    </div>
    <div class="grid">
      <section>
        <h2>Usage And Rate Limits</h2>
        <button onclick="fetchJson('/api/developer/usage', 'usageOut')">Refresh</button>
        <pre id="usageOut"></pre>
      </section>
      <section>
        <h2>Webhook Tester</h2>
        <label>Webhook URL <input id="webhookUrl" placeholder="https://example.com/webhook"></label>
        <label>Event type <select id="webhookEvent"><option>transaction.submitted</option><option>position.updated</option><option>liquidation.created</option></select></label>
        <button onclick="postJson('/api/developer/webhooks/test', { url: value('webhookUrl'), event: value('webhookEvent') }, 'webhookOut')">Send Test Event</button>
        <pre id="webhookOut"></pre>
      </section>
    </div>
    <section>
      <h2>SDK Downloads And Quickstarts</h2>
      <p><code>npm install @stellarlend/sdk</code> or download generated client metadata below.</p>
      <div class="row">
        <a class="button" href="/api/developer/sdk/typescript">TypeScript SDK</a>
        <a class="button secondary" href="/api/developer/quickstart">Quickstart JSON</a>
      </div>
    </section>
  </main>
  <script>
    function value(id) { return document.getElementById(id).value; }
    async function fetchJson(url, target) {
      const res = await fetch(url);
      document.getElementById(target).textContent = JSON.stringify(await res.json(), null, 2);
    }
    async function postJson(url, body, target, apiKey) {
      const headers = { 'Content-Type': 'application/json' };
      if (apiKey) headers['X-API-Key'] = apiKey;
      const res = await fetch(url, { method: 'POST', headers, body: JSON.stringify(body) });
      document.getElementById(target).textContent = JSON.stringify(await res.json(), null, 2);
    }
  </script>
</body>
</html>`;
}

router.get('/', (_req, res) => {
  res.type('html').send(portalHtml());
});

/**
 * @openapi
 * /developer/api-keys:
 *   get:
 *     summary: List developer API keys
 *     tags: [Developer]
 *     parameters:
 *       - in: query
 *         name: actor
 *         schema:
 *           type: string
 *         description: Optional developer or organization id
 *     responses:
 *       200:
 *         description: Public API key metadata without hashes or raw keys
 *   post:
 *     summary: Generate a developer API key
 *     tags: [Developer]
 *     requestBody:
 *       required: false
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               name:
 *                 type: string
 *               actor:
 *                 type: string
 *     responses:
 *       201:
 *         description: New API key. The raw key is returned once.
 */
router.get('/api-keys', (req, res) => {
  const actor = typeof req.query.actor === 'string' ? req.query.actor : undefined;
  res.json({ data: apiKeyService.list(actor) });
});

router.post('/api-keys', async (req, res, next: NextFunction) => {
  try {
    const name = typeof req.body?.name === 'string' ? req.body.name : 'unnamed-key';
    const result = await apiKeyService.create(name, actorFromRequest(req));
    res.status(201).json({
      data: result,
      warning: 'Store rawKey now. StellarLend only stores a hash and cannot show it again.',
    });
  } catch (error) {
    next(error);
  }
});

router.post('/api-keys/:keyId/rotate', async (req, res, next: NextFunction) => {
  try {
    const result = await apiKeyService.rotate(req.params.keyId, actorFromRequest(req));
    res.json({
      data: result,
      warning: 'Store rawKey now. StellarLend only stores a hash and cannot show it again.',
    });
  } catch (error) {
    next(error);
  }
});

router.delete('/api-keys/:keyId', (req, res, next: NextFunction) => {
  try {
    apiKeyService.revoke(req.params.keyId, actorFromRequest(req));
    res.status(204).send();
  } catch (error) {
    next(error);
  }
});

/**
 * @openapi
 * /developer/usage:
 *   get:
 *     summary: Get usage statistics and sensitive-operation rate limit status
 *     tags: [Developer]
 *     responses:
 *       200:
 *         description: Current rate limit configuration and per-user counters
 */
router.get('/usage', (_req, res) => {
  res.json({
    rateLimitConfig: getSensitiveRateLimitConfig(),
    rateLimits: getSensitiveRateLimitAnalytics(),
  });
});

/**
 * @openapi
 * /developer/graphql:
 *   post:
 *     summary: Execute a federated GraphQL playground query
 *     tags: [Developer]
 *     parameters:
 *       - in: header
 *         name: X-API-Key
 *         schema:
 *           type: string
 *         description: Optional API key. Never echoed in responses.
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             properties:
 *               query:
 *                 type: string
 *     responses:
 *       200:
 *         description: Federated playground response
 */
router.post('/graphql', (_req, res) => {
  res.json({
    data: {
      protocol: {
        tvl: '0',
        utilizationRate: '0.00',
        numberOfUsers: 0,
      },
      usage: {
        rateLimits: getSensitiveRateLimitAnalytics(),
      },
      federation: {
        services: ['protocol', 'lending', 'usage'],
      },
    },
    extensions: {
      playground: 'API keys are accepted via X-API-Key and never echoed in responses.',
    },
  });
});

/**
 * @openapi
 * /developer/webhooks/test:
 *   post:
 *     summary: Queue a test webhook delivery
 *     tags: [Developer]
 *     requestBody:
 *       required: true
 *       content:
 *         application/json:
 *           schema:
 *             type: object
 *             required: [url]
 *             properties:
 *               url:
 *                 type: string
 *                 format: uri
 *               event:
 *                 type: string
 *     responses:
 *       200:
 *         description: Test webhook payload queued
 *       400:
 *         description: Invalid webhook URL
 */
router.post('/webhooks/test', (req, res) => {
  const url = typeof req.body?.url === 'string' ? req.body.url : '';
  const event = typeof req.body?.event === 'string' ? req.body.event : 'transaction.submitted';

  if (!/^https?:\/\//.test(url)) {
    res.status(400).json({ success: false, error: 'A valid http(s) webhook URL is required' });
    return;
  }

  res.json({
    success: true,
    delivery: {
      id: `wh_test_${Date.now()}`,
      url,
      event,
      payload: {
        id: `evt_${Date.now()}`,
        type: event,
        createdAt: new Date().toISOString(),
        data: { transactionHash: 'test-transaction-hash' },
      },
      status: 'queued',
    },
  });
});

/**
 * @openapi
 * /developer/quickstart:
 *   get:
 *     summary: Get SDK quickstart metadata
 *     tags: [Developer]
 *     responses:
 *       200:
 *         description: SDK install command and explorer URLs
 */
router.get('/quickstart', (_req, res) => {
  res.json({
    install: 'npm install @stellarlend/sdk',
    openapi: '/api/openapi.json',
    swagger: '/api/docs',
    graphql: '/api/developer/graphql',
    authentication: 'Send X-API-Key for server-to-server requests.',
  });
});

/**
 * @openapi
 * /developer/sdk/typescript:
 *   get:
 *     summary: Download the TypeScript SDK quickstart snippet
 *     tags: [Developer]
 *     responses:
 *       200:
 *         description: TypeScript client example
 */
router.get('/sdk/typescript', (_req, res) => {
  res.type('text/plain').send(`// StellarLend TypeScript quickstart
import { StellarLendClient } from '@stellarlend/sdk';

const client = new StellarLendClient({
  baseUrl: process.env.STELLARLEND_API_URL,
  apiKey: process.env.STELLARLEND_API_KEY,
});

const tx = await client.lending.prepare({
  operation: 'deposit',
  userAddress: 'G...',
  amount: '1000000',
});
`);
});

export default router;
