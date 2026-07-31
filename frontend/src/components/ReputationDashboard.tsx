import React, { useState, useEffect, useCallback } from 'react';

interface ReputationScore {
  address: string;
  total_repayments: number;
  on_time_repayments: number;
  defaults: number;
  total_borrowed: string;
  score: number;
  tier: string;
  last_activity_timestamp: number;
}

interface ReputationTier {
  tier: string;
  min_score: number;
  max_score: number;
  benefits: {
    interest_rate_discount_bps: number;
    borrowing_limit_multiplier_bps: number;
    collateral_reduction_bps: number;
  };
}

interface LeaderboardEntry extends ReputationScore {}

export const ReputationDashboard: React.FC = () => {
  const [address, setAddress] = useState('');
  const [reputation, setReputation] = useState<ReputationScore | null>(null);
  const [tiers, setTiers] = useState<ReputationTier[]>([]);
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [activeTab, setActiveTab] = useState<'search' | 'leaderboard' | 'tiers'>('search');

  const fetchReputation = useCallback(async () => {
    if (!address) return;
    setIsLoading(true);
    try {
      const res = await fetch(`/api/reputation/${address}`);
      const data = await res.json();
      if (data.success) setReputation(data.reputation);
    } catch (err) {
      console.error('Failed to fetch reputation', err);
    } finally {
      setIsLoading(false);
    }
  }, [address]);

  const fetchTiers = useCallback(async () => {
    try {
      const res = await fetch('/api/reputation/tiers');
      const data = await res.json();
      if (data.success) setTiers(data.tiers);
    } catch (err) {
      console.error('Failed to fetch tiers', err);
    }
  }, []);

  const fetchLeaderboard = useCallback(async () => {
    setIsLoading(true);
    try {
      const res = await fetch('/api/reputation/leaderboard?limit=20');
      const data = await res.json();
      if (data.success) setLeaderboard(data.leaderboard);
    } catch (err) {
      console.error('Failed to fetch leaderboard', err);
    } finally {
      setIsLoading(false);
    }
  }, []);

  useEffect(() => { fetchTiers(); }, [fetchTiers]);

  const getTierColor = (tier: string): string => {
    const colors: Record<string, string> = {
      Bronze: '#cd7f32',
      Silver: '#c0c0c0',
      Gold: '#ffd700',
      Platinum: '#e5e4e2',
    };
    return colors[tier] ?? '#888';
  };

  const getScoreBarWidth = (score: number): string => {
    return `${Math.min((score / 1000) * 100, 100)}%`;
  };

  return (
    <div style={styles.container}>
      <h2 style={styles.title}>Reputation System</h2>

      <div style={styles.tabBar}>
        {(['search', 'leaderboard', 'tiers'] as const).map((tab) => (
          <button
            key={tab}
            onClick={() => setActiveTab(tab)}
            style={activeTab === tab ? styles.tabActive : styles.tab}
          >
            {tab.charAt(0).toUpperCase() + tab.slice(1)}
          </button>
        ))}
      </div>

      {activeTab === 'search' && (
        <div>
          <div style={styles.searchBar}>
            <input
              type="text"
              placeholder="Enter Stellar address..."
              value={address}
              onChange={(e) => setAddress(e.target.value)}
              style={styles.input}
            />
            <button onClick={fetchReputation} style={styles.searchButton} disabled={isLoading}>
              {isLoading ? 'Searching...' : 'Search'}
            </button>
          </div>

          {reputation && (
            <div style={styles.reputationCard}>
              <div style={styles.cardHeader}>
                <div style={styles.addressDisplay}>
                  {reputation.address.slice(0, 12)}...{reputation.address.slice(-8)}
                </div>
                <span
                  style={{
                    ...styles.tierBadge,
                    backgroundColor: getTierColor(reputation.tier),
                    color: reputation.tier === 'Gold' || reputation.tier === 'Platinum' ? '#333' : '#fff',
                  }}
                >
                  {reputation.tier}
                </span>
              </div>

              <div style={styles.scoreSection}>
                <div style={styles.scoreLabel}>Reputation Score</div>
                <div style={styles.scoreValue}>{reputation.score}/1000</div>
                <div style={styles.scoreBarBg}>
                  <div
                    style={{
                      ...styles.scoreBarFill,
                      width: getScoreBarWidth(reputation.score),
                      backgroundColor: getTierColor(reputation.tier),
                    }}
                  />
                </div>
              </div>

              <div style={styles.statsGrid}>
                <div style={styles.statItem}>
                  <div style={styles.statLabel}>Repayments</div>
                  <div style={styles.statValue}>{reputation.total_repayments}</div>
                </div>
                <div style={styles.statItem}>
                  <div style={styles.statLabel}>On-Time</div>
                  <div style={styles.statValue}>{reputation.on_time_repayments}</div>
                </div>
                <div style={styles.statItem}>
                  <div style={styles.statLabel}>Defaults</div>
                  <div style={{ ...styles.statValue, color: reputation.defaults > 0 ? '#d32f2f' : '#2e7d32' }}>
                    {reputation.defaults}
                  </div>
                </div>
                <div style={styles.statItem}>
                  <div style={styles.statLabel}>Total Borrowed</div>
                  <div style={styles.statValue}>{parseInt(reputation.total_borrowed).toLocaleString()}</div>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {activeTab === 'leaderboard' && (
        <div>
          <button onClick={fetchLeaderboard} style={styles.refreshButton}>
            Refresh Leaderboard
          </button>
          <div style={styles.tableContainer}>
            <table style={styles.table}>
              <thead>
                <tr>
                  <th style={styles.th}>#</th>
                  <th style={styles.th}>Address</th>
                  <th style={styles.th}>Score</th>
                  <th style={styles.th}>Tier</th>
                  <th style={styles.th}>Repayments</th>
                  <th style={styles.th}>Defaults</th>
                </tr>
              </thead>
              <tbody>
                {leaderboard.map((entry, i) => (
                  <tr key={entry.address} style={styles.tr}>
                    <td style={styles.td}>{i + 1}</td>
                    <td style={styles.td}>
                      {entry.address.slice(0, 8)}...{entry.address.slice(-6)}
                    </td>
                    <td style={styles.td}>
                      <span style={{ fontWeight: 600 }}>{entry.score}</span>
                    </td>
                    <td style={styles.td}>
                      <span
                        style={{
                          ...styles.tierBadgeSmall,
                          backgroundColor: getTierColor(entry.tier),
                          color: entry.tier === 'Gold' || entry.tier === 'Platinum' ? '#333' : '#fff',
                        }}
                      >
                        {entry.tier}
                      </span>
                    </td>
                    <td style={styles.td}>{entry.total_repayments}</td>
                    <td style={styles.td}>{entry.defaults}</td>
                  </tr>
                ))}
                {leaderboard.length === 0 && !isLoading && (
                  <tr>
                    <td colSpan={6} style={{ ...styles.td, textAlign: 'center', color: '#888' }}>
                      No entries yet. Search for addresses to populate the leaderboard.
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>
      )}

      {activeTab === 'tiers' && (
        <div style={styles.tiersGrid}>
          {tiers.map((tier) => (
            <div key={tier.tier} style={styles.tierCard}>
              <div
                style={{
                  ...styles.tierHeader,
                  backgroundColor: getTierColor(tier.tier),
                  color: tier.tier === 'Gold' || tier.tier === 'Platinum' ? '#333' : '#fff',
                }}
              >
                {tier.tier}
              </div>
              <div style={styles.tierBody}>
                <div style={styles.tierRange}>
                  {tier.min_score} - {tier.max_score} pts
                </div>
                <div style={styles.tierBenefits}>
                  <div style={styles.benefitRow}>
                    <span>Interest Discount</span>
                    <span style={styles.benefitValue}>{tier.benefits.interest_rate_discount_bps} bps</span>
                  </div>
                  <div style={styles.benefitRow}>
                    <span>Borrow Limit</span>
                    <span style={styles.benefitValue}>
                      {(tier.benefits.borrowing_limit_multiplier_bps / 100).toFixed(0)}%
                    </span>
                  </div>
                  <div style={styles.benefitRow}>
                    <span>Collateral Reduction</span>
                    <span style={styles.benefitValue}>{tier.benefits.collateral_reduction_bps} bps</span>
                  </div>
                </div>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: { padding: '24px', maxWidth: '1000px', margin: '0 auto', fontFamily: '-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif' },
  title: { fontSize: '24px', fontWeight: 600, marginBottom: '24px', color: '#1a1a2e' },
  tabBar: { display: 'flex', gap: '8px', marginBottom: '24px' },
  tab: { padding: '8px 20px', backgroundColor: '#f5f5f5', border: '1px solid #ddd', borderRadius: '6px', fontSize: '14px', cursor: 'pointer' },
  tabActive: { padding: '8px 20px', backgroundColor: '#0066ff', color: '#fff', border: '1px solid #0066ff', borderRadius: '6px', fontSize: '14px', cursor: 'pointer' },
  searchBar: { display: 'flex', gap: '12px', marginBottom: '24px' },
  input: { flex: 1, padding: '10px 16px', border: '1px solid #ddd', borderRadius: '8px', fontSize: '14px' },
  searchButton: { padding: '10px 24px', backgroundColor: '#0066ff', color: '#fff', border: 'none', borderRadius: '8px', fontSize: '14px', cursor: 'pointer', fontWeight: 500 },
  reputationCard: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', overflow: 'hidden' },
  cardHeader: { padding: '16px 20px', display: 'flex', justifyContent: 'space-between', alignItems: 'center', borderBottom: '1px solid #f0f0f0' },
  addressDisplay: { fontFamily: 'monospace', fontSize: '14px', color: '#333' },
  tierBadge: { padding: '4px 12px', borderRadius: '12px', fontSize: '12px', fontWeight: 700, textTransform: 'uppercase' },
  tierBadgeSmall: { padding: '2px 8px', borderRadius: '8px', fontSize: '11px', fontWeight: 700, textTransform: 'uppercase' },
  scoreSection: { padding: '16px 20px', borderBottom: '1px solid #f0f0f0' },
  scoreLabel: { fontSize: '12px', color: '#666', marginBottom: '4px', textTransform: 'uppercase', fontWeight: 600 },
  scoreValue: { fontSize: '28px', fontWeight: 700, color: '#1a1a2e', marginBottom: '8px' },
  scoreBarBg: { height: '8px', backgroundColor: '#eee', borderRadius: '4px', overflow: 'hidden' },
  scoreBarFill: { height: '100%', borderRadius: '4px', transition: 'width 0.5s ease' },
  statsGrid: { display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '0' },
  statItem: { padding: '16px 20px', textAlign: 'center', borderRight: '1px solid #f0f0f0' },
  statLabel: { fontSize: '11px', color: '#666', marginBottom: '4px', textTransform: 'uppercase', fontWeight: 600 },
  tableContainer: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', overflow: 'hidden' },
  table: { width: '100%', borderCollapse: 'collapse', fontSize: '13px' },
  th: { textAlign: 'left', padding: '12px 16px', borderBottom: '2px solid #e0e0e0', color: '#666', fontSize: '11px', fontWeight: 600, textTransform: 'uppercase', backgroundColor: '#f8f9fa' },
  tr: { borderBottom: '1px solid #f0f0f0' },
  td: { padding: '10px 16px' },
  refreshButton: { padding: '8px 20px', backgroundColor: '#f5f5f5', border: '1px solid #ddd', borderRadius: '6px', fontSize: '13px', cursor: 'pointer', marginBottom: '16px' },
  tiersGrid: { display: 'grid', gridTemplateColumns: 'repeat(4, 1fr)', gap: '16px' },
  tierCard: { backgroundColor: '#fff', borderRadius: '12px', border: '1px solid #e0e0e0', overflow: 'hidden' },
  tierHeader: { padding: '12px', textAlign: 'center', fontSize: '16px', fontWeight: 700, textTransform: 'uppercase' },
  tierBody: { padding: '16px' },
  tierRange: { textAlign: 'center', fontSize: '13px', color: '#666', marginBottom: '12px' },
  tierBenefits: { display: 'flex', flexDirection: 'column', gap: '8px' },
  benefitRow: { display: 'flex', justifyContent: 'space-between', fontSize: '12px', color: '#333' },
  benefitValue: { fontWeight: 600, color: '#0066ff' },
};