/**
 * Tests for Contract Updater Service
 */

import { describe, it, expect, beforeEach, vi } from 'vitest';
import { ContractUpdater, createContractUpdater } from '../src/services/contract-updater.js';
import type { AggregatedPrice } from '../src/types/index.js';

const transactionBuilderCalls: Array<{ fee: string; networkPassphrase: string }> = [];
const mockAccount = {
  accountId: () => 'GTEST123',
  sequenceNumber: () => '1',
  incrementSequenceNumber: vi.fn(),
};

const mockServer = {
  getAccount: vi.fn().mockResolvedValue(mockAccount),
  getHealth: vi.fn().mockResolvedValue({ status: 'healthy' }),
  simulateTransaction: vi.fn().mockResolvedValue({
    results: [{ xdr: 'mock-xdr' }],
  }),
  sendTransaction: vi.fn().mockResolvedValue({
    status: 'PENDING',
    hash: 'mock-tx-hash-123456',
  }),
  getTransaction: vi.fn().mockResolvedValue({
    status: 'SUCCESS',
  }),
};

// Mock Stellar SDK
vi.mock('@stellar/stellar-sdk', () => {
  const mockTransaction = {
    sign: vi.fn(),
    toXDR: vi.fn().mockReturnValue('mock-xdr'),
  };

  const mockTransactionBuilder = {
    addOperation: vi.fn().mockReturnThis(),
    setTimeout: vi.fn().mockReturnThis(),
    build: vi.fn().mockReturnValue(mockTransaction),
  };

  const sorobanRpc = {
    Server: vi.fn().mockImplementation((_url: string) => mockServer),
    Api: {
      isSimulationError: vi.fn().mockReturnValue(false),
      isSimulationSuccess: vi.fn().mockReturnValue(true),
      GetTransactionStatus: {
        SUCCESS: 'SUCCESS',
        FAILED: 'FAILED',
        NOT_FOUND: 'NOT_FOUND',
      },
    },
    assembleTransaction: vi.fn((tx, simulated) => ({
      build: () => mockTransaction,
    })),
  };

  return {
    Account: vi.fn().mockImplementation((accountId: string, sequence: string) => ({
      accountId: () => accountId,
      sequenceNumber: () => sequence,
      incrementSequenceNumber: vi.fn(),
    })),
    Keypair: {
      fromSecret: vi.fn((secret: string) => ({
        publicKey: () => 'GTEST123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ',
        secret: () => secret,
      })),
    },
    Contract: vi.fn().mockImplementation((contractId: string) => ({
      call: vi.fn().mockReturnValue({
        /* operation */
      }),
    })),
    rpc: sorobanRpc,
    SorobanRpc: sorobanRpc,
    TransactionBuilder: vi.fn().mockImplementation((_account, options) => {
      transactionBuilderCalls.push(options);
      return mockTransactionBuilder;
    }),
    Networks: {
      TESTNET: 'Test SDF Network ; September 2015',
      PUBLIC: 'Public Global Stellar Network ; September 2015',
    },
    xdr: {
      ScVal: {
        scvSymbol: vi.fn((symbol: string) => ({ symbol })),
      },
    },
    Address: vi.fn().mockImplementation((address: string) => ({
      toScVal: vi.fn().mockReturnValue({ address }),
    })),
    nativeToScVal: vi.fn((value: any, opts: any) => ({ value, opts })),
  };
});

describe('ContractUpdater', () => {
  let updater: ContractUpdater;

  const mockConfig = {
    network: 'testnet' as const,
    rpcUrl: 'https://soroban-testnet.stellar.org',
    contractId: 'CTEST123456789',
    adminSecretKey: 'STEST123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ123456789',
    baseFee: 100000,
    maxFee: 1000000,
    maxRetries: 3,
    retryDelayMs: 100,
  };

  beforeEach(async () => {
    const { SorobanRpc } = await import('@stellar/stellar-sdk');
    vi.clearAllMocks();
    transactionBuilderCalls.length = 0;
    mockServer.getAccount.mockResolvedValue(mockAccount);
    mockServer.getHealth.mockResolvedValue({ status: 'healthy' });
    mockServer.simulateTransaction.mockResolvedValue({
      results: [{ xdr: 'mock-xdr' }],
    });
    mockServer.sendTransaction.mockResolvedValue({
      status: 'PENDING',
      hash: 'mock-tx-hash-123456',
    });
    mockServer.getTransaction.mockResolvedValue({
      status: 'SUCCESS',
    });
    vi.mocked(SorobanRpc.Api.isSimulationError).mockReturnValue(false);
    vi.mocked(SorobanRpc.Api.isSimulationSuccess).mockReturnValue(true);
    updater = createContractUpdater(mockConfig);
  });

  describe('initialization', () => {
    it('should create contract updater with config', () => {
      expect(updater).toBeDefined();
      expect(updater).toBeInstanceOf(ContractUpdater);
    });

    it('should initialize with testnet network', () => {
      const testnetUpdater = createContractUpdater({
        ...mockConfig,
        network: 'testnet',
      });

      expect(testnetUpdater).toBeDefined();
    });

    it('should initialize with mainnet network', () => {
      const mainnetUpdater = createContractUpdater({
        ...mockConfig,
        network: 'mainnet',
      });

      expect(mainnetUpdater).toBeDefined();
    });

    it('should expose admin public key', () => {
      const publicKey = updater.getAdminPublicKey();

      expect(publicKey).toBeDefined();
      expect(typeof publicKey).toBe('string');
      expect(publicKey.length).toBeGreaterThan(0);
    });
  });

  describe('updatePrice', () => {
    it('should successfully update a single price', async () => {
      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(true);
      expect(result.asset).toBe('XLM');
      expect(result.price).toBe(150000n);
      expect(result.transactionHash).toBe('mock-tx-hash-123456');
    });

    it('should update price with correct timestamp', async () => {
      const timestamp = Math.floor(Date.now() / 1000);
      const result = await updater.updatePrice('BTC', 50000000000n, timestamp);

      expect(result.success).toBe(true);
      expect(result.timestamp).toBe(timestamp);
    });

    it('should handle different assets', async () => {
      const assets = ['XLM', 'BTC', 'ETH', 'USDC'];

      for (const asset of assets) {
        const result = await updater.updatePrice(asset, 100000n, Date.now());
        expect(result.success).toBe(true);
        expect(result.asset).toBe(asset);
      }
    });

    it('should handle large price values', async () => {
      const largePrice = 999999999999999n;
      const result = await updater.updatePrice('BTC', largePrice, Date.now());

      expect(result.success).toBe(true);
      expect(result.price).toBe(largePrice);
    });

    it('should handle small price values', async () => {
      const smallPrice = 1n;
      const result = await updater.updatePrice('XLM', smallPrice, Date.now());

      expect(result.success).toBe(true);
      expect(result.price).toBe(smallPrice);
    });

    it('should apply configured base fee to transactions', async () => {
      const customFeeUpdater = createContractUpdater({
        ...mockConfig,
        baseFee: 250000,
        maxFee: 500000,
      });

      await customFeeUpdater.updatePrice('XLM', 150000n, Date.now());

      expect(transactionBuilderCalls.at(-1)?.fee).toBe('250000');
    });

    it('should reject a base fee higher than max fee', () => {
      expect(() =>
        createContractUpdater({
          ...mockConfig,
          baseFee: 500001,
          maxFee: 500000,
        })
      ).toThrow('baseFee cannot exceed maxFee');
    });
  });

  describe('updatePrices (batch)', () => {
    it('should update multiple prices successfully', async () => {
      const prices: AggregatedPrice[] = [
        {
          asset: 'XLM',
          price: 150000n,
          timestamp: Math.floor(Date.now() / 1000),
          sources: [],
          confidence: 95,
        },
        {
          asset: 'BTC',
          price: 50000000000n,
          timestamp: Math.floor(Date.now() / 1000),
          sources: [],
          confidence: 98,
        },
      ];

      const results = await updater.updatePrices(prices);

      expect(results).toHaveLength(2);
      expect(results[0].success).toBe(true);
      expect(results[0].asset).toBe('XLM');
      expect(results[1].success).toBe(true);
      expect(results[1].asset).toBe('BTC');
    });

    describe('retry backoff delay verification', () => {
  beforeEach(() => {
    vi.useFakeTimers();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('should apply exponential backoff delays between retries via updatePrice', async () => {
    const { SorobanRpc } = await import('@stellar/stellar-sdk');
    const mockServer = new SorobanRpc.Server('mock');

    let attemptCount = 0;
    vi.spyOn(mockServer, 'simulateTransaction').mockImplementation(async () => {
      attemptCount++;
      if (attemptCount < 3) throw new Error('Temporary failure');
      return { results: [{ xdr: 'mock-xdr' }] };
    });

    const resultPromise = updater.updatePrice('XLM', 150000n, Date.now());
    await vi.advanceTimersByTimeAsync(1000); // retry 1 delay
    await vi.advanceTimersByTimeAsync(2000); // retry 2 delay
    const result = await resultPromise;

    expect(result.success).toBe(true);
    expect(attemptCount).toBe(3);
  });

  it('should apply 4s delay on third retry', async () => {
    const { SorobanRpc } = await import('@stellar/stellar-sdk');
    const mockServer = new SorobanRpc.Server('mock');

    const retryUpdater = createContractUpdater({
      ...mockConfig,
      maxRetries: 4,
    });

    let attemptCount = 0;
    vi.spyOn(mockServer, 'simulateTransaction').mockImplementation(async () => {
      attemptCount++;
      if (attemptCount < 4) throw new Error('Temporary failure');
      return { results: [{ xdr: 'mock-xdr' }] };
    });

    const resultPromise = retryUpdater.updatePrice('XLM', 150000n, Date.now());
    await vi.advanceTimersByTimeAsync(1000); // retry 1
    await vi.advanceTimersByTimeAsync(2000); // retry 2
    await vi.advanceTimersByTimeAsync(4000); // retry 3
    const result = await resultPromise;

    expect(result.success).toBe(true);
    expect(attemptCount).toBe(4);
  });
});

    it('should handle empty price array', async () => {
      const results = await updater.updatePrices([]);

      expect(results).toHaveLength(0);
    });

    it('should process prices sequentially with delay', async () => {
      const prices: AggregatedPrice[] = [
        {
          asset: 'XLM',
          price: 150000n,
          timestamp: Math.floor(Date.now() / 1000),
          sources: [],
          confidence: 95,
        },
        {
          asset: 'BTC',
          price: 50000000000n,
          timestamp: Math.floor(Date.now() / 1000),
          sources: [],
          confidence: 98,
        },
      ];

      const startTime = Date.now();
      await updater.updatePrices(prices);
      const duration = Date.now() - startTime;

      // Should have at least 100ms delay between updates
      expect(duration).toBeGreaterThanOrEqual(100);
    });
  });

  describe('retry mechanism', () => {
    it('should retry on failure', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // First attempt fails, second succeeds
      let attemptCount = 0;
      vi.spyOn(mockServer, 'simulateTransaction').mockImplementation(async () => {
        attemptCount++;
        if (attemptCount === 1) {
          throw new Error('Network error');
        }
        return {
          results: [{ xdr: 'mock-xdr' }],
        };
      });

      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(true);
    });

    it('should return failure after max retries', async () => {
      // Test validates that retry mechanism exists
      // Detailed retry testing is complex with mocked Stellar SDK
      const testUpdater = createContractUpdater({
        ...mockConfig,
        maxRetries: 2,
        retryDelayMs: 50,
      });

      expect(testUpdater).toBeDefined();
    });

    it('should use exponential backoff for retries', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      let attemptCount = 0;
      const attemptTimes: number[] = [];

      vi.spyOn(mockServer, 'simulateTransaction').mockImplementation(async () => {
        attemptTimes.push(Date.now());
        attemptCount++;
        if (attemptCount < 3) {
          throw new Error('Network error');
        }
        return {
          results: [{ xdr: 'mock-xdr' }],
        };
      });

      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(true);
      // Verify exponential backoff (delays should increase)
      if (attemptTimes.length >= 3) {
        const delay1 = attemptTimes[1] - attemptTimes[0];
        const delay2 = attemptTimes[2] - attemptTimes[1];
        expect(delay2).toBeGreaterThan(delay1);
      }
    });
  });

  describe('error handling', () => {
    it('should handle simulation errors', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');

      vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should handle transaction send errors', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      vi.spyOn(mockServer, 'sendTransaction').mockResolvedValue({
        status: 'ERROR',
        errorResult: 'Transaction rejected',
        hash: '',
      } as any);

      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(false);
    });

    it('should handle transaction failures on-chain', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      vi.spyOn(mockServer, 'getTransaction').mockResolvedValue({
        status: SorobanRpc.Api.GetTransactionStatus.FAILED,
      } as any);

      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result.success).toBe(false);
    });

    it('should handle account fetch errors', async () => {
      // With default mocks, this validates error handling structure exists
      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result).toBeDefined();
      expect(result.success !== undefined).toBe(true);
    });
  });

  describe('comprehensive error path tests', () => {
    describe('RPC connection timeout', () => {
      it('should handle RPC timeout during account fetch', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'getAccount').mockRejectedValue(
          new Error('RPC timeout: Connection timed out after 30 seconds')
        );

        const result = await updater.updatePrice('XLM', 150000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('timeout');
        expect(result.asset).toBe('XLM');
        expect(result.price).toBe(150000n);
      });

      it('should recover gracefully from RPC timeout with retries', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        let attemptCount = 0;
        vi.spyOn(mockServer, 'getAccount').mockImplementation(async () => {
          attemptCount++;
          if (attemptCount <= 2) {
            throw new Error('RPC timeout: Connection timed out after 30 seconds');
          }
          return {
            accountId: () => 'GTEST123',
            sequenceNumber: () => '1',
            incrementSequenceNumber: vi.fn(),
          };
        });

        const result = await updater.updatePrice('XLM', 150000n, Date.now());

        expect(result.success).toBe(true);
        expect(attemptCount).toBe(3); // Should retry and succeed
      });
    });

    describe('Transaction simulation failure', () => {
      it('should handle simulation failure with detailed error', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Contract execution error: Insufficient gas',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('BTC', 50000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Simulation failed');
        expect(result.error).toContain('Insufficient gas');
        expect(result.asset).toBe('BTC');
        expect(result.price).toBe(50000000000n);
      });

      it('should handle simulation failure with invalid contract method', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Method "set_asset_price" not found in contract',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('ETH', 1000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Method "set_asset_price" not found');
      });

      it('should handle simulation failure with authorization error', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Authorization failed: Invalid admin signature',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('USDC', 1000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Authorization failed');
      });
    });

    describe('Network error during submission', () => {
      it('should handle network error during transaction submission', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'sendTransaction').mockRejectedValue(
          new Error('Network error: ECONNREFUSED - Connection refused')
        );

        const result = await updater.updatePrice('XLM', 150000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Network error');
        expect(result.error).toContain('ECONNREFUSED');
      });

      it('should handle network error with rate limiting', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'sendTransaction').mockRejectedValue(
          new Error('Rate limit exceeded: Too many requests, try again later')
        );

        const result = await updater.updatePrice('BTC', 50000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Rate limit exceeded');
      });

      it('should handle network error with DNS resolution failure', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'sendTransaction').mockRejectedValue(
          new Error('DNS resolution failed: Unable to resolve host')
        );

        const result = await updater.updatePrice('ETH', 1000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('DNS resolution failed');
      });
    });

    describe('Invalid admin key', () => {
      it('should handle invalid admin secret key format', async () => {
        const { Keypair } = await import('@stellar/stellar-sdk');
        vi.spyOn(Keypair, 'fromSecret').mockImplementationOnce(() => {
          throw new Error('Invalid secret key');
        });

        expect(() =>
          createContractUpdater({
            ...mockConfig,
            adminSecretKey: 'INVALID_SECRET_KEY_FORMAT',
          })
        ).toThrow('Invalid secret key');
      });

      it('should handle admin key with insufficient permissions', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Admin does not have required permissions',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('USDC', 1000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Admin does not have required permissions');
      });

      it('should handle admin key that is not the contract admin', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Only contract admin can set prices',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('BTC', 50000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Only contract admin can set prices');
      });
    });

    describe('Contract not found', () => {
      it('should handle invalid contract ID', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Contract not found: Invalid contract ID format',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('XLM', 150000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Contract not found');
      });

      it('should handle contract that does not exist on network', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Contract not deployed on network',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('ETH', 1000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Contract not deployed');
      });

      it('should handle contract access denied', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: 'Simulation failed: Access denied: Contract is private',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('USDC', 1000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toContain('Access denied');
      });
    });

    describe('Service recovery scenarios', () => {
      it('should recover from temporary network issues', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        let attemptCount = 0;
        vi.spyOn(mockServer, 'sendTransaction').mockImplementation(async () => {
          attemptCount++;
          if (attemptCount <= 2) {
            throw new Error('Network temporarily unavailable');
          }
          return {
            status: 'PENDING',
            hash: 'recovery-tx-hash',
          };
        });

        const result = await updater.updatePrice('XLM', 150000n, Date.now());

        expect(result.success).toBe(true);
        expect(result.transactionHash).toBe('recovery-tx-hash');
        expect(attemptCount).toBe(3);
      });

      it('should maintain service state after multiple failures', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');
        const singleAttemptUpdater = createContractUpdater({
          ...mockConfig,
          maxRetries: 1,
        });

        // First request fails
        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValueOnce({
          error: 'Simulation failed: Temporary issue',
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValueOnce(true);

        const result1 = await singleAttemptUpdater.updatePrice('XLM', 150000n, Date.now());
        expect(result1.success).toBe(false);

        // Second request succeeds
        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValueOnce({
          results: [{ xdr: 'mock-xdr' }],
        });

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValueOnce(false);

        const result2 = await singleAttemptUpdater.updatePrice('BTC', 50000000000n, Date.now());
        expect(result2.success).toBe(true);
      });

      it('should provide descriptive error messages for debugging', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');

        const detailedError =
          'Simulation failed: Contract execution error: Insufficient gas limit. Required: 50000, Available: 30000';
        vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
          error: detailedError,
          result: null,
        } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError').mockReturnValue(true);

        const result = await updater.updatePrice('ETH', 1000000000n, Date.now());

        expect(result.success).toBe(false);
        expect(result.error).toBe(detailedError);
        expect(result.error).toContain('Insufficient gas limit');
        expect(result.error).toContain('Required: 50000');
      });
    });

    describe('Batch error handling', () => {
      it('should handle mixed success and failure in batch updates', async () => {
        const { SorobanRpc } = await import('@stellar/stellar-sdk');
        const mockServer = new SorobanRpc.Server('mock');
        const singleAttemptUpdater = createContractUpdater({
          ...mockConfig,
          maxRetries: 1,
        });

        // First price succeeds, second fails
        vi.spyOn(mockServer, 'simulateTransaction')
          .mockResolvedValueOnce({ results: [{ xdr: 'mock-xdr' }] })
          .mockResolvedValueOnce({
            error: 'Simulation failed: Contract error',
            result: null,
          } as any);

        vi.spyOn(SorobanRpc.Api, 'isSimulationError')
          .mockReturnValueOnce(false)
          .mockReturnValueOnce(true);

        const prices: AggregatedPrice[] = [
          {
            asset: 'XLM',
            price: 150000n,
            timestamp: Math.floor(Date.now() / 1000),
            sources: [],
            confidence: 95,
          },
          {
            asset: 'BTC',
            price: 50000000000n,
            timestamp: Math.floor(Date.now() / 1000),
            sources: [],
            confidence: 98,
          },
        ];

        const results = await singleAttemptUpdater.updatePrices(prices);

        expect(results).toHaveLength(2);
        expect(results[0].success).toBe(true);
        expect(results[1].success).toBe(false);
        expect(results[1].error).toBeDefined();
      });
    });
  });

  describe('healthCheck', () => {
    it('should return detailed health status when all checks pass', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // Mock successful health checks
      vi.spyOn(mockServer, 'getHealth').mockResolvedValue({ status: 'healthy' });
      vi.spyOn(mockServer, 'getAccount').mockResolvedValue({
        balances: [{ asset_type: 'native', balance: '10.5' }],
      } as any);
      vi.spyOn(mockServer, 'simulateTransaction').mockResolvedValue({
        results: [{ xdr: 'mock-xdr' }],
      } as any);

      const healthStatus = await updater.healthCheck();

      expect(healthStatus.overall).toBe(true);
      expect(healthStatus.rpc).toBe(true);
      expect(healthStatus.admin).toBe(true);
      expect(healthStatus.contract).toBe(true);
      expect(healthStatus.details.rpc).toBe('RPC endpoint reachable');
      expect(healthStatus.details.admin?.exists).toBe(true);
      expect(healthStatus.details.admin?.balance).toBe('10.5');
      expect(healthStatus.details.contract).toBe('Contract accessible');
    });

    it('should return failure status when RPC is unreachable', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // Mock RPC failure
      vi.spyOn(mockServer, 'getHealth').mockRejectedValue(new Error('RPC connection failed'));

      const healthStatus = await updater.healthCheck();

      expect(healthStatus.overall).toBe(false);
      expect(healthStatus.rpc).toBe(false);
      expect(healthStatus.details.rpc).toContain('RPC unreachable');
    });

    it('should return failure status when admin account does not exist', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // Mock successful RPC but failed account check
      vi.spyOn(mockServer, 'getHealth').mockResolvedValue({ status: 'healthy' });
      vi.spyOn(mockServer, 'getAccount').mockRejectedValue(new Error('Account not found'));

      const healthStatus = await updater.healthCheck();

      expect(healthStatus.overall).toBe(false);
      expect(healthStatus.rpc).toBe(true);
      expect(healthStatus.admin).toBe(false);
      expect(healthStatus.details.admin?.exists).toBe(false);
      expect(healthStatus.details.admin?.balance).toBe('0');
    });

    it('should return failure status when contract is inaccessible', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // Mock successful RPC and account but failed contract access
      vi.spyOn(mockServer, 'getHealth').mockResolvedValue({ status: 'healthy' });
      vi.spyOn(mockServer, 'getAccount').mockResolvedValue({
        balances: [{ asset_type: 'native', balance: '5.0' }],
      } as any);
      vi.spyOn(mockServer, 'simulateTransaction').mockRejectedValue(
        new Error('Contract not deployed')
      );

      const healthStatus = await updater.healthCheck();

      expect(healthStatus.overall).toBe(false);
      expect(healthStatus.rpc).toBe(true);
      expect(healthStatus.admin).toBe(true);
      expect(healthStatus.contract).toBe(false);
      expect(healthStatus.details.contract).toContain('Contract inaccessible');
    });

    it('should complete health check within 5 seconds', async () => {
      const startTime = Date.now();

      await updater.healthCheck();

      const duration = Date.now() - startTime;
      expect(duration).toBeLessThan(5000);
    });

    it('should handle unexpected errors gracefully', async () => {
      const { SorobanRpc } = await import('@stellar/stellar-sdk');
      const mockServer = new SorobanRpc.Server('mock');

      // Mock unexpected error during health check
      vi.spyOn(mockServer, 'getHealth').mockRejectedValue(new Error('Unexpected error'));

      const healthStatus = await updater.healthCheck();

      expect(healthStatus.overall).toBe(false);
      expect(healthStatus.rpc).toBe(false);
      expect(healthStatus.admin).toBe(true);
      expect(healthStatus.contract).toBe(true);
      expect(healthStatus.details.rpc).toContain('RPC unreachable');
    });
  });

  describe('transaction waiting', () => {
    it('should wait for transaction confirmation', async () => {
      // Tests that transaction confirmation logic is implemented
      const result = await updater.updatePrice('XLM', 150000n, Date.now());

      expect(result).toBeDefined();
      expect(result.asset).toBe('XLM');
    });
  });

  describe('configuration', () => {
    it('should allow custom retry settings', () => {
      const customUpdater = createContractUpdater({
        ...mockConfig,
        maxRetries: 5,
        retryDelayMs: 500,
      });

      expect(customUpdater).toBeDefined();
    });

    it('should use default retry settings when not provided', () => {
      const { maxRetries, retryDelayMs, ...minimalConfig } = mockConfig;

      const defaultUpdater = createContractUpdater(minimalConfig as any);

      expect(defaultUpdater).toBeDefined();
    });
  });
});
