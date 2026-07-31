import { StellarService } from './stellar.service';

export interface MempoolTransaction {
  hash: string;
  from: string;
  to: string;
  value: string;
  timestamp: number;
  inputData: string;
  operationType: 'deposit' | 'withdraw' | 'borrow' | 'repay' | 'liquidation';
}

export interface SandwichPattern {
  id: string;
  frontrunTx: MempoolTransaction;
  victimTx: MempoolTransaction;
  backrunTx: MempoolTransaction;
  estimatedProfit: string;
  detectedAt: number;
  confidence: number;
}

export interface MempoolSnapshot {
  pendingCount: number;
  oldestTxAge: number;
  largestTxValue: string;
  sandwichPatterns: SandwichPattern[];
  updatedAt: number;
}

export class MempoolMonitorService {
  private stellarService: StellarService;

  constructor() {
    this.stellarService = new StellarService();
  }

  async getMempoolSnapshot(): Promise<MempoolSnapshot> {
    const transactions = await this.fetchPendingTransactions();
    const patterns = await this.detectSandwichPatterns(transactions);

    const oldestTx = transactions.length > 0
      ? transactions.reduce((a, b) => a.timestamp < b.timestamp ? a : b)
      : null;

    const largestTx = transactions.length > 0
      ? transactions.reduce((a, b) => BigInt(a.value) > BigInt(b.value) ? a : b)
      : null;

    return {
      pendingCount: transactions.length,
      oldestTxAge: oldestTx ? Date.now() - oldestTx.timestamp : 0,
      largestTxValue: largestTx?.value ?? '0',
      sandwichPatterns: patterns,
      updatedAt: Date.now(),
    };
  }

  async getSandwichPatterns(): Promise<SandwichPattern[]> {
    const transactions = await this.fetchPendingTransactions();
    return this.detectSandwichPatterns(transactions);
  }

  private async fetchPendingTransactions(): Promise<MempoolTransaction[]> {
    try {
      const response = await fetch('/api/mempool/pending');
      if (!response.ok) return [];
      const data = await response.json();
      return data.transactions ?? [];
    } catch {
      return [];
    }
  }

  private async detectSandwichPatterns(
    transactions: MempoolTransaction[],
  ): Promise<SandwichPattern[]> {
    const patterns: SandwichPattern[] = [];
    const sorted = [...transactions].sort((a, b) => a.timestamp - b.timestamp);

    for (let i = 1; i < sorted.length - 1; i++) {
      const prev = sorted[i - 1];
      const victim = sorted[i];
      const next = sorted[i + 1];

      const sameTarget = prev.to === victim.to && victim.to === next.to;
      const valueCorrelated =
        BigInt(prev.value) >= BigInt(victim.value) &&
        BigInt(next.value) >= BigInt(victim.value);
      const closeTimestamps =
        next.timestamp - prev.timestamp < 5000;

      if (sameTarget && valueCorrelated && closeTimestamps) {
        const estimatedProfit =
          BigInt(prev.value) > BigInt(next.value)
            ? BigInt(prev.value) - BigInt(victim.value)
            : BigInt(next.value) - BigInt(victim.value);

        patterns.push({
          id: `sandwich_${victim.hash}_${Date.now()}`,
          frontrunTx: prev,
          victimTx: victim,
          backrunTx: next,
          estimatedProfit: estimatedProfit.toString(),
          detectedAt: Date.now(),
          confidence: sameTarget && valueCorrelated && closeTimestamps ? 0.85 : 0.5,
        });
      }
    }

    return patterns;
  }
}

export const mempoolMonitorService = new MempoolMonitorService();
