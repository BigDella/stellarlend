import { describe, expect, it } from 'vitest';
import { quoteBasket, debtCollateralRatio } from '../src/services/multi-asset-prices.js';

describe('multi-asset price feeds', () => {
  it('quotes a basket with fallbacks for liquidation decisions', () => {
    const quotes = quoteBasket(['XLM', 'USDC', 'ETH']);
    expect(quotes).toHaveLength(3);
    expect(quotes.find((q) => q.asset === 'USDC')?.price).toBe(1);
  });

  it('computes a debt/collateral ratio across assets', () => {
    const ratio = debtCollateralRatio('USDC', 100, 'XLM', 2000, []);
    expect(ratio).toBeGreaterThan(1);
  });
});
