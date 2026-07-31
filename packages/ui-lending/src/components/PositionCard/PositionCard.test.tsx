import React from 'react';
import { render, screen, fireEvent } from '@testing-library/react';
import { PositionCard } from './index';

const mockPosition = {
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

describe('PositionCard', () => {
  it('renders asset name and symbol', () => {
    render(<PositionCard position={mockPosition} />);
    expect(screen.getByText('USDC')).toBeTruthy();
  });

  it('shows loading skeleton when isLoading is true', () => {
    render(<PositionCard position={mockPosition} isLoading />);
    expect(screen.getByRole('status')).toBeTruthy();
  });

  it('calls onSupply when supply button clicked', () => {
    const onSupply = jest.fn();
    render(<PositionCard position={mockPosition} onSupply={onSupply} />);
    fireEvent.click(screen.getByText('Supply'));
    expect(onSupply).toHaveBeenCalledWith(mockPosition);
  });

  it('calls onBorrow when borrow button clicked', () => {
    const onBorrow = jest.fn();
    render(<PositionCard position={mockPosition} onBorrow={onBorrow} />);
    fireEvent.click(screen.getByText('Borrow'));
    expect(onBorrow).toHaveBeenCalledWith(mockPosition);
  });

  it('renders supplied and borrowed amounts', () => {
    render(<PositionCard position={mockPosition} />);
    expect(screen.getByText('1000.0000')).toBeTruthy();
    expect(screen.getByText('250.0000')).toBeTruthy();
  });
});
