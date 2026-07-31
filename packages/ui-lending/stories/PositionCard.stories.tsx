import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { PositionCard } from '../src/components/PositionCard';
import { createTheme } from '../src/utils/theme';

const meta: Meta<typeof PositionCard> = {
  title: 'Lending/PositionCard',
  component: PositionCard,
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof PositionCard>;

const samplePosition = {
  id: '1',
  asset: 'USDC',
  symbol: 'USDC',
  supplied: 1000,
  borrowed: 250,
  supplyApy: 4.5,
  borrowApy: 6.2,
  collateralFactor: 0.8,
  price: 1,
};

export const Default: Story = {
  args: { position: samplePosition },
};

export const DarkMode: Story = {
  args: { position: samplePosition, theme: createTheme('dark') },
  parameters: { backgrounds: { default: 'dark' } },
};

export const Loading: Story = {
  args: { position: samplePosition, isLoading: true },
};

export const WithActions: Story = {
  args: {
    position: samplePosition,
    onSupply: (p) => alert(`Supply ${p.asset}`),
    onBorrow: (p) => alert(`Borrow ${p.asset}`),
    onRepay: (p) => alert(`Repay ${p.asset}`),
    onWithdraw: (p) => alert(`Withdraw ${p.asset}`),
  },
};
