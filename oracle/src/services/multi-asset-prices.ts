/**
 * Multi-asset price feed helper for liquidation and health-factor decisions (Issues #661, #663).
 */

export interface AssetPriceQuote {
  asset: string;
  price: number;
  timestamp: number;
  source: string;
}

const FALLBACK: Record<string, number> = {
  XLM: 0.12,
  USDC: 1,
  USDT: 1,
  BTC: 64000,
  ETH: 3200,
};

export function quoteAsset(asset: string, live?: Partial<AssetPriceQuote>): AssetPriceQuote {
  const symbol = asset.toUpperCase();
  return {
    asset: symbol,
    price: live?.price ?? FALLBACK[symbol] ?? 1,
    timestamp: live?.timestamp ?? Date.now(),
    source: live?.source ?? 'fallback',
  };
}

export function quoteBasket(assets: string[], live: AssetPriceQuote[] = []): AssetPriceQuote[] {
  const byAsset = new Map(live.map((q) => [q.asset.toUpperCase(), q]));
  return assets.map((asset) => quoteAsset(asset, byAsset.get(asset.toUpperCase())));
}

export function debtCollateralRatio(
  debtAsset: string,
  debtAmount: number,
  collateralAsset: string,
  collateralAmount: number,
  quotes: AssetPriceQuote[]
): number {
  const debtPx = quoteAsset(debtAsset, quotes.find((q) => q.asset === debtAsset.toUpperCase())).price;
  const collPx = quoteAsset(
    collateralAsset,
    quotes.find((q) => q.asset === collateralAsset.toUpperCase())
  ).price;
  const debtValue = debtAmount * debtPx;
  if (debtValue <= 0) return Number.POSITIVE_INFINITY;
  return (collateralAmount * collPx) / debtValue;
}
