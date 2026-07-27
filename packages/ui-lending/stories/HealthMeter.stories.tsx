import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { HealthMeter } from '../src/components/HealthMeter';
import { createTheme } from '../src/utils/theme';

const meta: Meta<typeof HealthMeter> = {
  title: 'Lending/HealthMeter',
  component: HealthMeter,
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof HealthMeter>;

const safeHealth = {
  value: 2.42,
  status: 'safe' as const,
  collateralValue: 24_200,
  borrowedValue: 8_000,
  liquidationThreshold: 0.8,
};

export const Safe: Story = {
  args: { health: safeHealth },
};

export const Liquidatable: Story = {
  args: {
    health: {
      ...safeHealth,
      value: 0.94,
      status: 'liquidatable',
      borrowedValue: 20_600,
    },
  },
};

export const Compact: Story = {
  args: { health: safeHealth, showDetails: false },
};

export const DarkMode: Story = {
  args: { health: safeHealth, theme: createTheme('dark') },
  parameters: { backgrounds: { default: 'dark' } },
};

export const Loading: Story = {
  args: { health: safeHealth, isLoading: true },
};
