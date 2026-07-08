describe('Config Validation', () => {
  const originalEnv = process.env;

  beforeEach(() => {
    jest.resetModules();
    process.env = { ...originalEnv };
  });

  afterAll(() => {
    process.env = originalEnv;
  });

  it('should throw an error if CONTRACT_ID is missing', () => {
    delete process.env.CONTRACT_ID;

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: CONTRACT_ID is required');
  });

  it('should not throw an error if CONTRACT_ID and secure JWT_SECRET are present', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'a-secure-secret-that-is-at-least-thirty-two-characters-long';

    expect(() => {
      require('../config/index');
    }).not.toThrow();
  });

  it('should throw an error if JWT_SECRET is missing', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    delete process.env.JWT_SECRET;

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: JWT_SECRET must be at least 32 characters');
  });

  it('should throw an error if JWT_SECRET is the default insecure value', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'default-secret-change-me';

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: JWT_SECRET must be at least 32 characters');
  });

  it('should throw an error if JWT_SECRET is too short', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'too-short';

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: JWT_SECRET must be at least 32 characters');
  });

  it('should require ALLOWED_ORIGINS in production', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'a-secure-secret-that-is-at-least-thirty-two-characters-long';
    process.env.NODE_ENV = 'production';
    process.env.REDIS_URL = 'redis://localhost:6379';
    delete process.env.ALLOWED_ORIGINS;

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: ALLOWED_ORIGINS is required in production');
  });

  it('should require REDIS_URL when REDIS_ENABLED is true', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'a-secure-secret-that-is-at-least-thirty-two-characters-long';
    process.env.REDIS_ENABLED = 'true';
    delete process.env.REDIS_URL;

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: REDIS_URL is required when REDIS_ENABLED=true');
  });

  it('should reject invalid URL values', () => {
    process.env.CONTRACT_ID = 'TEST_CONTRACT_ID';
    process.env.JWT_SECRET = 'a-secure-secret-that-is-at-least-thirty-two-characters-long';
    process.env.HORIZON_URL = 'not-a-url';

    expect(() => {
      require('../config/index');
    }).toThrow('Config validation failed: HORIZON_URL must be a valid URL');
  });
});
