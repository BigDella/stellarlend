// Mock StellarService before importing app
import { StellarService } from '../services/stellar.service';
jest.mock('../services/stellar.service');

import request from 'supertest';
import app, { resetRateLimiters } from '../app';

// Override CORS config for specific tests without re-importing the module
const configModule = jest.requireMock('../config') as { config: { cors: { allowedOrigins: string[] } } };

jest.mock('../config', () => {
  const original = jest.requireActual('../config');
  return {
    ...original,
    config: {
      ...original.config,
      cors: { allowedOrigins: ['*'] },
    },
  };
});

jest.mock('../services/redisCache.service', () => ({
  redisCacheService: { warmup: jest.fn().mockResolvedValue(undefined) },
}));

beforeEach(async () => {
  jest.clearAllMocks();
  await resetRateLimiters();
});

describe('CORS headers', () => {
  it('returns Access-Control-Allow-Origin for an allowed origin (wildcard)', async () => {
    configModule.config.cors.allowedOrigins = ['*'];
    const res = await request(app)
      .get('/api/health')
      .set('Origin', 'https://any-domain.example.com');

    expect(res.headers['access-control-allow-origin']).toBeTruthy();
  });

  it('includes Access-Control-Allow-Credentials when credentials option is set', async () => {
    configModule.config.cors.allowedOrigins = ['*'];
    const res = await request(app)
      .get('/api/health')
      .set('Origin', 'https://app.stellarlend.io');

    expect(res.headers['access-control-allow-credentials']).toBe('true');
  });

  it('allows a request with no Origin header (server-to-server)', async () => {
    const res = await request(app).get('/api/health');
    expect(res.status).not.toBe(403);
  });

  it('allows an explicitly listed origin', async () => {
    configModule.config.cors.allowedOrigins = ['https://app.stellarlend.io'];
    const res = await request(app)
      .get('/api/health')
      .set('Origin', 'https://app.stellarlend.io');

    expect(res.headers['access-control-allow-origin']).toBeTruthy();
  });

  it('responds to preflight OPTIONS requests', async () => {
    configModule.config.cors.allowedOrigins = ['*'];
    const res = await request(app)
      .options('/api/health')
      .set('Origin', 'https://app.stellarlend.io')
      .set('Access-Control-Request-Method', 'GET');

    expect([200, 204]).toContain(res.status);
  });
});
