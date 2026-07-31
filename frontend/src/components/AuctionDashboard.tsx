import React, { useState, useEffect, useCallback } from 'react';

interface Auction {
  id: number;
  pool: string;
  collateralAsset: string;
  debtAsset: string;
  collateralAmount: number;
  debtAmount: number;
  oraclePrice: number;
  startPrice: number;
  currentPrice: number;
  startTime: number;
  endTime: number;
  status: 'Active' | 'Settled' | 'Expired';
  borrower: string;
  highestBidder: string | null;
  highestBidAmount: number | null;
}

interface AuctionAnalytics {
  totalAuctions: number;
  settledAuctions: number;
  avgPremiumBps: number;
  avgTimeToFillSecs: number;
  totalCollateralLiquidated: number;
}

interface AuctionDashboardState {
  auctions: Auction[];
  analytics: AuctionAnalytics | null;
  isLoading: boolean;
  filter: 'all' | 'active' | 'settled' | 'expired';
  sortBy: 'id' | 'currentPrice' | 'collateralAmount' | 'endTime';
  sortDir: 'asc' | 'desc';
}

export const AuctionDashboard: React.FC = () => {
  const [state, setState] = useState<AuctionDashboardState>({
    auctions: [],
    analytics: null,
    isLoading: true,
    filter: 'all',
    sortBy: 'endTime',
    sortDir: 'asc',
  });

  const loadAuctions = useCallback(async () => {
    setState(prev => ({ ...prev, isLoading: true }));
    try {
      const [auctionsRes, analyticsRes] = await Promise.all([
        fetch(`/api/auctions?filter=${state.filter}&sortBy=${state.sortBy}&sortDir=${state.sortDir}`),
        fetch('/api/auctions/analytics'),
      ]);

      const auctionsData = await auctionsRes.json();
      const analyticsData = await analyticsRes.json();

      setState(prev => ({
        ...prev,
        auctions: auctionsData.auctions || [],
        analytics: analyticsData,
        isLoading: false,
      }));
    } catch (err) {
      setState(prev => ({ ...prev, isLoading: false }));
    }
  }, [state.filter, state.sortBy, state.sortDir]);

  useEffect(() => {
    loadAuctions();
    const interval = setInterval(loadAuctions, 10000);
    return () => clearInterval(interval);
  }, [loadAuctions]);

  const handleBid = async (auctionId: number) => {
    try {
      await fetch('/api/auctions/bid', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ auctionId }),
      });
      loadAuctions();
    } catch (err) {
      console.error('Bid failed:', err);
    }
  };

  const calculateDiscountBps = (auction: Auction): number => {
    if (auction.oraclePrice <= 0) return 0;
    return Math.round(
      ((auction.oraclePrice - auction.currentPrice) * 10000) / auction.oraclePrice
    );
  };

  const formatTimeRemaining = (endTime: number): string => {
    const remaining = Math.max(0, endTime - Math.floor(Date.now() / 1000));
    const hours = Math.floor(remaining / 3600);
    const minutes = Math.floor((remaining % 3600) / 60);
    return `${hours}h ${minutes}m`;
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Dutch Auction Dashboard</h2>

      {state.analytics && (
        <div style={styles.statsGrid}>
          <div style={styles.statCard}>
            <div style={styles.statLabel}>Total Auctions</div>
            <div style={styles.statValue}>{state.analytics.totalAuctions}</div>
          </div>
          <div style={styles.statCard}>
            <div style={styles.statLabel}>Settled</div>
            <div style={styles.statValue}>{state.analytics.settledAuctions}</div>
          </div>
          <div style={styles.statCard}>
            <div style={styles.statLabel}>Avg Premium</div>
            <div style={styles.statValue}>{state.analytics.avgPremiumBps} bps</div>
          </div>
          <div style={styles.statCard}>
            <div style={styles.statLabel}>Avg Time to Fill</div>
            <div style={styles.statValue}>
              {Math.floor(state.analytics.avgTimeToFillSecs / 60)}m
            </div>
          </div>
          <div style={styles.statCard}>
            <div style={styles.statLabel}>Total Liquidated</div>
            <div style={styles.statValue}>
              {state.analytics.totalCollateralLiquidated.toLocaleString()}
            </div>
          </div>
        </div>
      )}

      <div style={styles.controls}>
        <div style={styles.filterGroup}>
          {(['all', 'active', 'settled', 'expired'] as const).map(f => (
            <button
              key={f}
              onClick={() => setState(prev => ({ ...prev, filter: f }))}
              style={
                state.filter === f ? styles.filterButtonActive : styles.filterButton
              }
            >
              {f.charAt(0).toUpperCase() + f.slice(1)}
            </button>
          ))}
        </div>
      </div>

      {state.isLoading ? (
        <div style={styles.loading}>Loading auctions...</div>
      ) : (
        <div style={styles.tableContainer}>
          <table style={styles.table}>
            <thead>
              <tr>
                <th style={styles.th}>ID</th>
                <th style={styles.th}>Pool</th>
                <th style={styles.th}>Collateral</th>
                <th style={styles.th}>Debt</th>
                <th style={styles.th}>Oracle Price</th>
                <th style={styles.th}>Current Price</th>
                <th style={styles.th}>Discount</th>
                <th style={styles.th}>Time Left</th>
                <th style={styles.th}>Status</th>
                <th style={styles.th}>Action</th>
              </tr>
            </thead>
            <tbody>
              {state.auctions.map(auction => (
                <tr key={auction.id} style={styles.tr}>
                  <td style={styles.td}>#{auction.id}</td>
                  <td style={styles.td}>{auction.pool.slice(0, 8)}...</td>
                  <td style={styles.td}>{auction.collateralAmount.toLocaleString()}</td>
                  <td style={styles.td}>{auction.debtAmount.toLocaleString()}</td>
                  <td style={styles.td}>{auction.oraclePrice.toLocaleString()}</td>
                  <td style={styles.td}>{auction.currentPrice.toLocaleString()}</td>
                  <td style={styles.td}>
                    <span style={styles.discountBadge}>
                      {calculateDiscountBps(auction)} bps
                    </span>
                  </td>
                  <td style={styles.td}>
                    {auction.status === 'Active'
                      ? formatTimeRemaining(auction.endTime)
                      : '-'}
                  </td>
                  <td style={styles.td}>
                    <span
                      style={
                        auction.status === 'Active'
                          ? styles.statusActive
                          : auction.status === 'Settled'
                          ? styles.statusSettled
                          : styles.statusExpired
                      }
                    >
                      {auction.status}
                    </span>
                  </td>
                  <td style={styles.td}>
                    {auction.status === 'Active' && (
                      <button
                        onClick={() => handleBid(auction.id)}
                        style={styles.bidButton}
                      >
                        Bid
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '24px',
    maxWidth: '1200px',
    margin: '0 auto',
    fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif',
  },
  title: {
    fontSize: '24px',
    fontWeight: 600,
    marginBottom: '24px',
    color: '#1a1a2e',
  },
  statsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(5, 1fr)',
    gap: '16px',
    marginBottom: '24px',
  },
  statCard: {
    padding: '16px',
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    textAlign: 'center',
  },
  statLabel: {
    fontSize: '12px',
    color: '#666',
    marginBottom: '4px',
    textTransform: 'uppercase',
    fontWeight: 600,
  },
  statValue: {
    fontSize: '20px',
    fontWeight: 700,
    color: '#1a1a2e',
  },
  controls: {
    marginBottom: '16px',
  },
  filterGroup: {
    display: 'flex',
    gap: '8px',
  },
  filterButton: {
    padding: '6px 16px',
    backgroundColor: '#f5f5f5',
    border: '1px solid #ddd',
    borderRadius: '6px',
    fontSize: '13px',
    cursor: 'pointer',
  },
  filterButtonActive: {
    padding: '6px 16px',
    backgroundColor: '#0066ff',
    color: '#fff',
    border: '1px solid #0066ff',
    borderRadius: '6px',
    fontSize: '13px',
    cursor: 'pointer',
  },
  loading: {
    textAlign: 'center',
    padding: '40px',
    color: '#666',
  },
  tableContainer: {
    backgroundColor: '#fff',
    borderRadius: '12px',
    border: '1px solid #e0e0e0',
    overflow: 'hidden',
  },
  table: {
    width: '100%',
    borderCollapse: 'collapse',
    fontSize: '13px',
  },
  th: {
    textAlign: 'left',
    padding: '12px 16px',
    borderBottom: '2px solid #e0e0e0',
    color: '#666',
    fontSize: '11px',
    fontWeight: 600,
    textTransform: 'uppercase',
    backgroundColor: '#f8f9fa',
  },
  tr: {
    borderBottom: '1px solid #f0f0f0',
  },
  td: {
    padding: '10px 16px',
  },
  discountBadge: {
    padding: '2px 8px',
    backgroundColor: '#e8f5e9',
    color: '#2e7d32',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 600,
  },
  statusActive: {
    padding: '2px 8px',
    backgroundColor: '#e3f2fd',
    color: '#1565c0',
    borderRadius: '4px',
    fontSize: '12px',
    fontWeight: 600,
  },
  statusSettled: {
    padding: '2px 8px',
    backgroundColor: '#d4edda',
    color: '#155724',
    borderRadius: '4px',
    fontSize: '12px',
  },
  statusExpired: {
    padding: '2px 8px',
    backgroundColor: '#f8d7da',
    color: '#721c24',
    borderRadius: '4px',
    fontSize: '12px',
  },
  bidButton: {
    padding: '4px 12px',
    backgroundColor: '#0066ff',
    color: '#fff',
    border: 'none',
    borderRadius: '4px',
    fontSize: '12px',
    cursor: 'pointer',
    fontWeight: 500,
  },
};
