import { apiKeyService } from '../services/apiKey.service';

jest.mock('../utils/logger');
jest.mock('../services/auditLog.service', () => ({
  auditLogService: { record: jest.fn() },
}));

beforeEach(() => {
  apiKeyService._reset();
});

describe('ApiKeyService.create()', () => {
  it('returns a raw key and a public record (no hash)', async () => {
    const result = await apiKeyService.create('test-key', 'alice');

    expect(result.rawKey).toBeDefined();
    expect(result.rawKey.length).toBe(64); // 32 bytes hex
    expect(result.record.prefix).toBe(result.rawKey.slice(0, 8));
    expect((result.record as any).hash).toBeUndefined();
  });

  it('stores the key so it can be verified', async () => {
    const { rawKey } = await apiKeyService.create('my-key', 'alice');
    const verify = await apiKeyService.verify(rawKey);

    expect(verify.valid).toBe(true);
    expect(verify.record?.name).toBe('my-key');
  });

  it('does not store the raw key (only bcrypt hash)', async () => {
    const { rawKey } = await apiKeyService.create('my-key', 'alice');
    const listed = apiKeyService.list();

    expect(listed.length).toBe(1);
    expect(JSON.stringify(listed)).not.toContain(rawKey);
  });
});

describe('ApiKeyService.verify()', () => {
  it('returns valid: false for an unknown key', async () => {
    const result = await apiKeyService.verify('a'.repeat(64));
    expect(result.valid).toBe(false);
  });

  it('returns valid: false for a revoked key', async () => {
    const { rawKey, record } = await apiKeyService.create('revoke-me', 'alice');
    apiKeyService.revoke(record.id, 'alice');

    const result = await apiKeyService.verify(rawKey);
    expect(result.valid).toBe(false);
  });

  it('updates lastUsedAt on successful verification', async () => {
    const { rawKey, record } = await apiKeyService.create('track-me', 'alice');
    expect(record.lastUsedAt).toBeUndefined();

    await apiKeyService.verify(rawKey);
    const listed = apiKeyService.list();
    expect(listed[0].lastUsedAt).toBeDefined();
  });
});

describe('ApiKeyService.rotate()', () => {
  it('creates a new key while old key remains valid', async () => {
    const { rawKey: oldKey, record } = await apiKeyService.create('original', 'alice');
    const rotated = await apiKeyService.rotate(record.id, 'alice');

    // Old key still valid
    const oldVerify = await apiKeyService.verify(oldKey);
    expect(oldVerify.valid).toBe(true);

    // New key also valid
    const newVerify = await apiKeyService.verify(rotated.rawKey);
    expect(newVerify.valid).toBe(true);

    // They are different keys
    expect(rotated.rawKey).not.toBe(oldKey);
  });

  it('throws NotFoundError for unknown key id', async () => {
    await expect(apiKeyService.rotate('nonexistent-id', 'alice')).rejects.toThrow('not found');
  });
});

describe('ApiKeyService.revoke()', () => {
  it('immediately invalidates the key', async () => {
    const { rawKey, record } = await apiKeyService.create('to-revoke', 'alice');
    apiKeyService.revoke(record.id, 'alice');

    const result = await apiKeyService.verify(rawKey);
    expect(result.valid).toBe(false);
  });

  it('throws ConflictError when revoking an already-revoked key', async () => {
    const { record } = await apiKeyService.create('double-revoke', 'alice');
    apiKeyService.revoke(record.id, 'alice');

    expect(() => apiKeyService.revoke(record.id, 'alice')).toThrow('already revoked');
  });
});

describe('ApiKeyService.migratePlaintextKeys()', () => {
  it('migrates plaintext keys to bcrypt hashes', async () => {
    const rawKey = 'a'.repeat(64);
    const migrated = await apiKeyService.migratePlaintextKeys([
      { id: 'legacy-id-1', rawKey, name: 'legacy', createdBy: 'system' },
    ]);

    expect(migrated).toBe(1);

    const result = await apiKeyService.verify(rawKey);
    expect(result.valid).toBe(true);
  });
});
