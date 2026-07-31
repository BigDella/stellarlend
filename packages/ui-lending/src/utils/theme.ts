import type { LendingTheme, ThemeColors } from '../types';

export const LIGHT_COLORS: ThemeColors = {
  primary: '#2563eb',
  secondary: '#7c3aed',
  success: '#16a34a',
  warning: '#d97706',
  danger: '#dc2626',
  background: '#f8fafc',
  surface: '#ffffff',
  text: '#0f172a',
  textMuted: '#64748b',
  border: '#e2e8f0',
};

export const DARK_COLORS: ThemeColors = {
  primary: '#3b82f6',
  secondary: '#8b5cf6',
  success: '#22c55e',
  warning: '#f59e0b',
  danger: '#ef4444',
  background: '#0f172a',
  surface: '#1e293b',
  text: '#f8fafc',
  textMuted: '#94a3b8',
  border: '#334155',
};

export function createTheme(mode: 'light' | 'dark' = 'light'): LendingTheme {
  return {
    mode,
    colors: mode === 'dark' ? DARK_COLORS : LIGHT_COLORS,
  };
}

export function getHealthColor(healthFactor: number, colors: ThemeColors): string {
  if (healthFactor >= 2) return colors.success;
  if (healthFactor >= 1.5) return colors.warning;
  if (healthFactor >= 1) return colors.danger;
  return colors.danger;
}

export function getRiskColor(riskLevel: string, colors: ThemeColors): string {
  switch (riskLevel) {
    case 'low': return colors.success;
    case 'medium': return colors.warning;
    case 'high': return colors.danger;
    case 'critical': return colors.danger;
    default: return colors.textMuted;
  }
}
