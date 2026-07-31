/**
 * Core lending domain types for the @stellarlend/ui-lending package.
 */

export type Theme = 'light' | 'dark';

export interface ThemeColors {
  primary: string;
  secondary: string;
  success: string;
  warning: string;
  danger: string;
  background: string;
  surface: string;
  text: string;
  textMuted: string;
  border: string;
}

export interface LendingTheme {
  mode: Theme;
  colors: ThemeColors;
}

export interface Position {
  id: string;
  asset: string;
  symbol: string;
  supplied: number;
  borrowed: number;
  supplyApy: number;
  borrowApy: number;
  collateralFactor: number;
  price: number;
}

export interface PoolData {
  id: string;
  asset: string;
  symbol: string;
  totalSupply: number;
  totalBorrow: number;
  supplyApy: number;
  borrowApy: number;
  utilization: number;
  collateralFactor: number;
  liquidationThreshold: number;
  price: number;
  isActive: boolean;
}

export interface HealthFactor {
  value: number;
  status: 'safe' | 'warning' | 'danger' | 'liquidatable';
  collateralValue: number;
  borrowedValue: number;
  liquidationThreshold: number;
}

export interface RatePoint {
  timestamp: number;
  supplyApy: number;
  borrowApy: number;
  utilization: number;
}

export interface LiquidationRisk {
  healthFactor: number;
  liquidationPrice: number;
  currentPrice: number;
  safetyBuffer: number;
  riskLevel: 'low' | 'medium' | 'high' | 'critical';
}
