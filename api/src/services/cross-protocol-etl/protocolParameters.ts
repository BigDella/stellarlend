/**
 * Curated protocol-parameter reference table — Issue #482
 *
 * Reserve factors and liquidation parameters are protocol *configuration*,
 * not market data — DefiLlama's public yields API (this pipeline's only
 * peer-protocol source, see `adapters/defiLlamaAdapter.ts`) doesn't expose
 * them. Unlike APY/TVL, these parameters are relatively stable and are
 * published directly by each protocol (docs, per-market configs, or
 * on-chain reserve config), so — rather than fabricate live values or
 * silently omit the comparison — this is a small, manually-curated
 * reference table, refreshed periodically from public sources.
 *
 * Each protocol's own convention for "liquidation threshold" differs
 * (StellarLend expresses it as a minimum collateral/debt ratio; most EVM
 * money markets express it as a max loan-to-value against collateral), so
 * values are kept in each protocol's native convention and labeled via
 * `thresholdConvention` rather than forced into a false apples-to-apples
 * percentage.
 *
 * StellarLend's own figures are the actual contract defaults (see
 * `stellar-lend/contracts/hello-world/src/risk_params.rs` and
 * `reserve.rs`), not estimates.
 */

export type LiquidationThresholdConvention = 'min-collateral-ratio' | 'max-loan-to-value';

export interface ProtocolFeeParameters {
  protocol: string;
  displayName: string;
  /** Blended/representative reserve factor across assets, in basis points. */
  reserveFactorBps: number;
  source: 'onchain-config' | 'public-docs';
  note: string;
}

export interface ProtocolLiquidationParameters {
  protocol: string;
  displayName: string;
  liquidationThresholdBps: number;
  thresholdConvention: LiquidationThresholdConvention;
  liquidationBonusBps: number;
  closeFactorBps: number;
  source: 'onchain-config' | 'public-docs';
  note: string;
}

const AS_OF = '2026-01';

export const PROTOCOL_FEE_PARAMETERS: ProtocolFeeParameters[] = [
  {
    protocol: 'stellarlend',
    displayName: 'StellarLend',
    reserveFactorBps: 1000, // DEFAULT_RESERVE_FACTOR_BPS in reserve.rs; governance-adjustable per asset, 0-5000bps
    source: 'onchain-config',
    note: 'Contract default (governance can override 0-50% per asset via a reserve-factor curve).',
  },
  {
    protocol: 'aave-v3',
    displayName: 'Aave V3',
    reserveFactorBps: 1500,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; varies per reserve (typically 10-20%).`,
  },
  {
    protocol: 'compound-v3',
    displayName: 'Compound III',
    reserveFactorBps: 1000,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; Comet markets set this per base asset.`,
  },
  {
    protocol: 'morpho-blue',
    displayName: 'Morpho Blue',
    reserveFactorBps: 0,
    source: 'public-docs',
    note: `Morpho Blue markets have no protocol reserve factor by default; fee switch is per-market and governance-controlled, ${AS_OF}.`,
  },
  {
    protocol: 'spark',
    displayName: 'Spark',
    reserveFactorBps: 1500,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; Spark is an Aave-v3-based deployment with similar per-reserve configuration.`,
  },
  {
    protocol: 'exactly',
    displayName: 'Exactly',
    reserveFactorBps: 1000,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; varies per market.`,
  },
];

export const PROTOCOL_LIQUIDATION_PARAMETERS: ProtocolLiquidationParameters[] = [
  {
    protocol: 'stellarlend',
    displayName: 'StellarLend',
    liquidationThresholdBps: 10500, // 105% minimum collateral/debt ratio
    thresholdConvention: 'min-collateral-ratio',
    liquidationBonusBps: 1000, // liquidation_incentive default
    closeFactorBps: 5000, // close_factor default
    source: 'onchain-config',
    note: 'Contract defaults from risk_params.rs; admin-adjustable within safety bounds.',
  },
  {
    protocol: 'aave-v3',
    displayName: 'Aave V3',
    liquidationThresholdBps: 8000, // ~80% max LTV, representative across majors
    thresholdConvention: 'max-loan-to-value',
    liquidationBonusBps: 500,
    closeFactorBps: 5000,
    source: 'public-docs',
    note: `Representative blended figure, ${AS_OF}; varies per asset (e.g. stablecoins are typically higher).`,
  },
  {
    protocol: 'compound-v3',
    displayName: 'Compound III',
    liquidationThresholdBps: 8500,
    thresholdConvention: 'max-loan-to-value',
    liquidationBonusBps: 700,
    closeFactorBps: 10000, // Comet liquidates the full position rather than a partial close factor
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; Comet allows full liquidation of an unhealthy position.`,
  },
  {
    protocol: 'morpho-blue',
    displayName: 'Morpho Blue',
    liquidationThresholdBps: 8600, // common LLTV tier
    thresholdConvention: 'max-loan-to-value',
    liquidationBonusBps: 500,
    closeFactorBps: 10000,
    source: 'public-docs',
    note: `LLTV is configured per isolated market, ${AS_OF}; 86% is a common tier, not a protocol-wide constant.`,
  },
  {
    protocol: 'spark',
    displayName: 'Spark',
    liquidationThresholdBps: 8000,
    thresholdConvention: 'max-loan-to-value',
    liquidationBonusBps: 500,
    closeFactorBps: 5000,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; Aave-v3-based configuration.`,
  },
  {
    protocol: 'exactly',
    displayName: 'Exactly',
    liquidationThresholdBps: 8000,
    thresholdConvention: 'max-loan-to-value',
    liquidationBonusBps: 500,
    closeFactorBps: 5000,
    source: 'public-docs',
    note: `Representative figure, ${AS_OF}; varies per market.`,
  },
];
