import { Contract, Address, TransactionBuilder, scValToNative } from '@stellar/stellar-sdk';
import { Server as SorobanServer } from '@stellar/stellar-sdk/rpc';
import { config } from '../config';
import logger from '../utils/logger';

const RISK_SCORING_CONTRACT_ID = process.env.RISK_SCORING_CONTRACT_ID ?? '';

export interface RiskScoreData {
  pool: string;
  assetVolatilityScore: number;
  oracleDeviationScore: number;
  poolUtilizationScore: number;
  liquidationHistoryScore: number;
  overallScore: number;
  letterGrade: string;
  timestamp: number;
}

export interface RiskFactorBreakdown {
  assetVolatilityBps: number;
  oracleDeviationBps: number;
  poolUtilizationBps: number;
  liquidationHistoryBps: number;
}

export interface RiskWeights {
  assetVolatilityWeight: number;
  oracleDeviationWeight: number;
  poolUtilizationWeight: number;
  liquidationHistoryWeight: number;
}

export interface PoolRiskProfile {
  pool: string;
  currentScore: RiskScoreData;
  history: RiskScoreData[];
  trend: 'improving' | 'stable' | 'declining';
  factors: RiskFactorBreakdown;
}

class RiskScoringService {
  private server: SorobanServer;
  private mockScores: Map<string, RiskScoreData> = new Map();
  private mockHistory: Map<string, RiskScoreData[]> = new Map();

  constructor() {
    this.server = new SorobanServer(config.stellar.sorobanRpcUrl);
    this.seedMockData();
  }

  private seedMockData(): void {
    const now = Math.floor(Date.now() / 1000);
    const pools = [
      { pool: 'pool-xlm-usdc', vol: 1800, oracle: 45, util: 7200, liq: 300 },
      { pool: 'pool-usdc-usdt', vol: 200, oracle: 10, util: 4500, liq: 50 },
      { pool: 'pool-eth-usdc', vol: 3500, oracle: 80, util: 8500, liq: 1200 },
      { pool: 'pool-xlm-usdt', vol: 1500, oracle: 35, util: 6800, liq: 200 },
    ];

    for (const p of pools) {
      const score = this.computeScore(p.pool, p.vol, p.oracle, p.util, p.liq, now);
      this.mockScores.set(p.pool, score);
      this.mockHistory.set(p.pool, [
        { ...score, timestamp: now - 86400 * 7, overallScore: Math.max(score.overallScore - 50, 0) },
        { ...score, timestamp: now - 86400 * 3, overallScore: Math.max(score.overallScore - 20, 0) },
        score,
      ]);
    }
  }

  private computeScore(
    pool: string,
    assetVolatilityBps: number,
    oracleDeviationBps: number,
    poolUtilizationBps: number,
    liquidationHistoryBps: number,
    timestamp: number,
  ): RiskScoreData {
    const volScore = assetVolatilityBps < 1000 ? 250 : assetVolatilityBps < 2500 ? 200 : assetVolatilityBps < 5000 ? 150 : 100;
    const oracleScore = oracleDeviationBps < 50 ? 250 : oracleDeviationBps < 100 ? 200 : oracleDeviationBps < 300 ? 150 : 100;
    const utilScore = poolUtilizationBps < 6000 ? 250 : poolUtilizationBps < 8000 ? 200 : poolUtilizationBps < 9500 ? 150 : 100;
    const liqScore = liquidationHistoryBps < 100 ? 250 : liquidationHistoryBps < 500 ? 200 : liquidationHistoryBps < 2000 ? 150 : 100;

    const overallScore = Math.min(
      Math.round(
        (volScore * 0.3 + oracleScore * 0.25 + utilScore * 0.25 + liqScore * 0.2)
      ),
      1000
    );

    let letterGrade: string;
    if (overallScore >= 950) letterGrade = 'A+';
    else if (overallScore >= 900) letterGrade = 'A';
    else if (overallScore >= 850) letterGrade = 'A-';
    else if (overallScore >= 800) letterGrade = 'B+';
    else if (overallScore >= 750) letterGrade = 'B';
    else if (overallScore >= 700) letterGrade = 'B-';
    else if (overallScore >= 650) letterGrade = 'C+';
    else if (overallScore >= 600) letterGrade = 'C';
    else if (overallScore >= 550) letterGrade = 'C-';
    else letterGrade = 'D';

    return {
      pool,
      assetVolatilityScore: volScore,
      oracleDeviationScore: oracleScore,
      poolUtilizationScore: utilScore,
      liquidationHistoryScore: liqScore,
      overallScore,
      letterGrade,
      timestamp,
    };
  }

  async getPoolRiskScore(pool: string): Promise<RiskScoreData | null> {
    if (!RISK_SCORING_CONTRACT_ID) {
      return this.mockScores.get(pool) ?? null;
    }
    try {
      const contract = new Contract(RISK_SCORING_CONTRACT_ID);
      const account = await this.server.getAccount(config.stellar.readOnlySimulationAccount);
      const tx = new TransactionBuilder(account, {
        fee: '100',
        networkPassphrase: config.stellar.networkPassphrase,
      })
        .addOperation(contract.call('get_pool_risk_score', new Address(pool).toScVal()))
        .setTimeout(30)
        .build();

      const sim = await this.server.simulateTransaction(tx);
      if (!('result' in sim) || !sim.result) return this.mockScores.get(pool) ?? null;

      const raw = scValToNative(sim.result.retval) as Record<string, unknown>;
      return {
        pool,
        assetVolatilityScore: Number((raw as any)['asset_volatility_score'] ?? 0),
        oracleDeviationScore: Number((raw as any)['oracle_deviation_score'] ?? 0),
        poolUtilizationScore: Number((raw as any)['pool_utilization_score'] ?? 0),
        liquidationHistoryScore: Number((raw as any)['liquidation_history_score'] ?? 0),
        overallScore: Number((raw as any)['overall_score'] ?? 0),
        letterGrade: String((raw as any)['letter_grade'] ?? 'N/A'),
        timestamp: Number((raw as any)['timestamp'] ?? 0),
      };
    } catch (err) {
      logger.warn('Risk score fetch failed', { pool, err: String(err) });
      return this.mockScores.get(pool) ?? null;
    }
  }

  async getAllPoolScores(): Promise<RiskScoreData[]> {
    const scores: RiskScoreData[] = [];
    for (const [pool] of this.mockScores) {
      const score = await this.getPoolRiskScore(pool);
      if (score) scores.push(score);
    }
    return scores;
  }

  async getPoolRiskProfile(pool: string): Promise<PoolRiskProfile | null> {
    const currentScore = await this.getPoolRiskScore(pool);
    if (!currentScore) return null;

    const history = this.mockHistory.get(pool) ?? [currentScore];
    const trend = history.length >= 2
      ? history[history.length - 1].overallScore > history[0].overallScore
        ? 'improving'
        : history[history.length - 1].overallScore < history[0].overallScore
        ? 'declining'
        : 'stable'
      : 'stable';

    return {
      pool,
      currentScore,
      history,
      trend,
      factors: {
        assetVolatilityBps: currentScore.assetVolatilityScore,
        oracleDeviationBps: currentScore.oracleDeviationScore,
        poolUtilizationBps: currentScore.poolUtilizationScore,
        liquidationHistoryBps: currentScore.liquidationHistoryScore,
      },
    };
  }

  async getScoreDistribution(): Promise<Record<string, number>> {
    const scores = await this.getAllPoolScores();
    const distribution: Record<string, number> = {};
    for (const s of scores) {
      distribution[s.letterGrade] = (distribution[s.letterGrade] || 0) + 1;
    }
    return distribution;
  }

  async getDefaultWeights(): Promise<RiskWeights> {
    return {
      assetVolatilityWeight: 3000,
      oracleDeviationWeight: 2500,
      poolUtilizationWeight: 2500,
      liquidationHistoryWeight: 2000,
    };
  }
}

export const riskScoringService = new RiskScoringService();