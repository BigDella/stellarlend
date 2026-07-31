import request from 'supertest';
import app from '../app';

describe('Response compression', () => {
  it('compresses large JSON responses when client accepts gzip', async () => {
    const res = await request(app)
      .get('/api/v1/openapi.json')
      .set('Accept-Encoding', 'gzip');

    expect(res.headers['content-encoding']).toBe('gzip');
  });

  it('does not send content-encoding when client does not accept it', async () => {
    const res = await request(app)
      .get('/api/v1/openapi.json')
      .set('Accept-Encoding', 'identity');

    expect(res.headers['content-encoding']).toBeUndefined();
  });

  it('does not compress responses below the threshold', async () => {
    const res = await request(app)
      .get('/api/health/live')
      .set('Accept-Encoding', 'gzip');

    expect(res.headers['content-encoding']).toBeUndefined();
  });
});
