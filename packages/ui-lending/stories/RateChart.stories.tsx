import type { Meta, StoryObj } from '@storybook/react';
import React from 'react';
import { RateChart } from '../src/components/RateChart';
import { createTheme } from '../src/utils/theme';

const meta: Meta<typeof RateChart> = {
  title: 'Lending/RateChart',
  component: RateChart,
  parameters: { layout: 'centered' },
  tags: ['autodocs'],
};

export default meta;
type Story = StoryObj<typeof RateChart>;

const data = Array.from({ length: 12 }, (_, i) => ({
  timestamp: Date.now() - (11 - i) * 86_400_000,
  supplyApy: 3.2 + i * 0.09,
  borrowApy: 5.8 + i * 0.14,
  utilization: 0.51 + i * 0.015,
}));

export const SupplyAndBorrow: Story = {
  args: { data },
};

export const BorrowOnly: Story = {
  args: { data, showSupply: false },
};

export const Empty: Story = {
  args: { data: [], isEmpty: true },
};

export const DarkMode: Story = {
  args: { data, theme: createTheme('dark') },
  parameters: { backgrounds: { default: 'dark' } },
};

export const Loading: Story = {
  args: { data, isLoading: true },
};
