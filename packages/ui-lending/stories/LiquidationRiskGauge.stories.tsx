import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { LiquidationRiskGauge } from '../src/components/LiquidationRiskGauge';
import { createTheme } from '../src/utils/theme';

const meta: Meta<typeof LiquidationRiskGauge> = {
  title: 'Lending/LiquidationRiskGauge',
  component: LiquidationRiskGauge,
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof LiquidationRiskGauge>;

const risk = {
  healthFactor: 1.18,
  liquidationPrice: 0.82,
  currentPrice: 1,
  safetyBuffer: 0.18,
  riskLevel: 'high' as const,
};

export const HighRisk: Story = {
  args: { risk },
};

export const Critical: Story = {
  args: {
    risk: {
      ...risk,
      healthFactor: 0.96,
      safetyBuffer: 0.02,
      riskLevel: 'critical',
    },
  },
};

export const DarkMode: Story = {
  args: { risk, theme: createTheme('dark') },
  parameters: { backgrounds: { default: 'dark' } },
};

export const Loading: Story = {
  args: { risk, isLoading: true },
};
