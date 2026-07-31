import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { PoolCard } from '../src/components/PoolCard';
import { createTheme } from '../src/utils/theme';

const meta: Meta<typeof PoolCard> = {
  title: 'Lending/PoolCard',
  component: PoolCard,
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof PoolCard>;

const pool = {
  id: 'xlm-usdc',
  asset: 'USDC Market',
  symbol: 'USDC',
  totalSupply: 4_200_000,
  totalBorrow: 2_730_000,
  supplyApy: 4.18,
  borrowApy: 6.92,
  utilization: 0.65,
  collateralFactor: 0.8,
  liquidationThreshold: 0.85,
  price: 1,
  isActive: true,
};

export const Active: Story = {
  args: { pool },
};

export const WithActions: Story = {
  args: {
    pool,
    onSupply: (selected) => alert(`Supply ${selected.symbol}`),
    onBorrow: (selected) => alert(`Borrow ${selected.symbol}`),
  },
};

export const Inactive: Story = {
  args: { pool: { ...pool, isActive: false } },
};

export const DarkMode: Story = {
  args: { pool, theme: createTheme('dark') },
  parameters: { backgrounds: { default: 'dark' } },
};

export const Loading: Story = {
  args: { pool, isLoading: true },
};
