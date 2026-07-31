import {
  Contract,
  Address,
  TransactionBuilder,
  scValToNative,
} from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import { config } from '../config';
import logger from '../utils/logger';

const REPUTATION_CONTRACT_ID = process.env.REPUTATION_CONTRACT_ID ?? '';
const TIER_NAMES = ['Bronze', 'Silver', 'Gold', 'Platinum'];

export interface ReputationScore {
  address: string;
  total_repayments: number;
  on_time_repayments: number;
  defaults: number;
  total_borrowed: string;
  score: number;
  tier: string;
  last_activity_timestamp: number;
}

// In-memory leaderboard populated as addresses are queried
const leaderboardCache = new Map<string, ReputationScore>();

class ReputationService {
  private server: SorobanServer;

  constructor() {
    this.server = new SorobanServer(config.stellar.sorobanRpcUrl);
  }

  async getReputation(address: string): Promise<ReputationScore> {
    if (!REPUTATION_CONTRACT_ID) {
      return emptyScore(address);
    }

    try {
      const contract = new Contract(REPUTATION_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_reputation', new Address(address).toScVal()))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) {
        return emptyScore(address);
      }

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      const score: ReputationScore = {
        address,
        total_repayments: Number(raw['total_repayments'] ?? 0),
        on_time_repayments: Number(raw['on_time_repayments'] ?? 0),
        defaults: Number(raw['defaults'] ?? 0),
        total_borrowed: String(raw['total_borrowed'] ?? '0'),
        score: Number(raw['score'] ?? 0),
        tier: TIER_NAMES[Number(raw['tier'] ?? 0)] ?? 'Bronze',
        last_activity_timestamp: Number(raw['last_activity_timestamp'] ?? 0),
      };

      leaderboardCache.set(address, score);
      return score;
    } catch (err) {
      logger.warn('Reputation contract simulation failed', { address, err: String(err) });
      return emptyScore(address);
    }
  }

  getLeaderboard(limit: number): ReputationScore[] {
    return Array.from(leaderboardCache.values())
      .sort((a, b) => b.score - a.score)
      .slice(0, limit);
  }
}

function emptyScore(address: string): ReputationScore {
  return {
    address,
    total_repayments: 0,
    on_time_repayments: 0,
    defaults: 0,
    total_borrowed: '0',
    score: 0,
    tier: 'Bronze',
    last_activity_timestamp: 0,
  };
}

export const reputationService = new ReputationService();
