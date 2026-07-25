import React, { useEffect, useState } from 'react';

interface ReferralStats {
  code: string;
  totalReferrals: number;
  l2Referrals: number;
  totalEarned: number;
  totalClaimed: number;
  claimable: number;
  tier: number;
}

interface ConversionFunnel {
  referralCode: string;
  referralsGenerated: number;
  referralsConverted: number;
  conversionRate: string;
  l2Referrals: number;
}

export const ReferralDashboard: React.FC = () => {
  const [stats, setStats] = useState<ReferralStats | null>(null);
  const [funnel, setFunnel] = useState<ConversionFunnel | null>(null);
  const [isLoading, setIsLoading] = useState(true);
  const [referralLink, setReferralLink] = useState<string>('');

  useEffect(() => {
    const fetchStats = async () => {
      try {
        const response = await fetch(`/api/referral/stats?userAddress=current`);
        const data = await response.json();
        if (data.success) {
          setStats(data.data);

          const funnelResponse = await fetch(`/api/referral/funnel?userAddress=current`);
          const funnelData = await funnelResponse.json();
          if (funnelData.success) {
            setFunnel(funnelData.data);
          }

          setReferralLink(data.data.code ? `https://stellarlend.com?ref=${data.data.code}` : '');
        }
        setIsLoading(false);
      } catch (error) {
        console.error('Failed to fetch referral stats:', error);
        setIsLoading(false);
      }
    };

    fetchStats();
  }, []);

  const getTierBadge = (tier: number) => {
    switch (tier) {
      case 2:
        return { label: 'Tier 2 - Gold', color: '#FFD700' };
      case 1:
        return { label: 'Tier 1 - Silver', color: '#C0C0C0' };
      default:
        return { label: 'Tier 0 - Bronze', color: '#CD7F32' };
    }
  };

  const handleClaim = async () => {
    if (!stats || stats.claimable <= 0) return;

    try {
      const response = await fetch('/api/referral/claim', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userAddress: 'current' }),
      });
      if (response.ok) {
        const newResponse = await fetch(`/api/referral/stats?userAddress=current`);
        const newData = await newResponse.json();
        if (newData.success) setStats(newData.data);
      }
    } catch (error) {
      console.error('Claim failed:', error);
    }
  };

  const handleGenerateCode = async () => {
    try {
      const response = await fetch('/api/referral/generate-code', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ userAddress: 'current' }),
      });
      const data = await response.json();
      if (data.success) {
        setReferralLink(data.data.link);
        setStats(prev => prev ? { ...prev, code: data.data.code } : null);
      }
    } catch (error) {
      console.error('Failed to generate code:', error);
    }
  };

  const copyToClipboard = () => {
    navigator.clipboard.writeText(referralLink);
    alert('Referral link copied to clipboard!');
  };

  if (isLoading) {
    return <div>Loading referral dashboard...</div>;
  }

  const tier = stats ? getTierBadge(stats.tier) : null;

  return (
    <div style={styles.container}>
      <h2>Referral Dashboard</h2>

      {stats && (
        <>
          <div style={styles.headerSection}>
            <div style={{ ...styles.tierBadge, backgroundColor: tier?.color }}>
              {tier?.label}
            </div>
          </div>

          <div style={styles.metricsGrid}>
            <div style={styles.metric}>
              <span style={styles.label}>Total Referrals</span>
              <span style={styles.value}>{stats.totalReferrals}</span>
            </div>
            <div style={styles.metric}>
              <span style={styles.label}>L2 Referrals</span>
              <span style={styles.value}>{stats.l2Referrals}</span>
            </div>
            <div style={styles.metric}>
              <span style={styles.label}>Total Earned</span>
              <span style={styles.value}>${stats.totalEarned.toFixed(2)}</span>
            </div>
            <div style={styles.metric}>
              <span style={styles.label}>Claimable</span>
              <span style={styles.value}>${stats.claimable.toFixed(2)}</span>
            </div>
          </div>

          <div style={styles.section}>
            <h3>Your Referral Link</h3>
            <div style={styles.referralLink}>
              <input
                type="text"
                value={referralLink || 'No code generated yet'}
                readOnly
                style={styles.linkInput}
              />
              <button onClick={copyToClipboard} style={styles.copyButton}>
                Copy
              </button>
              {!referralLink && (
                <button onClick={handleGenerateCode} style={{ ...styles.copyButton, backgroundColor: '#17a2b8' }}>
                  Generate Code
                </button>
              )}
            </div>
          </div>

          {funnel && (
            <div style={styles.section}>
              <h3>Conversion Funnel</h3>
              <div style={styles.funnelMetrics}>
                <p>Generated: {funnel.referralsGenerated}</p>
                <p>Converted: {funnel.referralsConverted}</p>
                <p>Conversion Rate: {funnel.conversionRate}%</p>
              </div>
            </div>
          )}

          <div style={styles.section}>
            <h3>Earnings</h3>
            <div style={styles.earningsSection}>
              <p>Total Claimed: ${stats.totalClaimed.toFixed(2)}</p>
              <p>Claimable Now: ${stats.claimable.toFixed(2)}</p>
              <button
                onClick={handleClaim}
                disabled={stats.claimable <= 0}
                style={{
                  ...styles.claimButton,
                  opacity: stats.claimable > 0 ? 1 : 0.5,
                  cursor: stats.claimable > 0 ? 'pointer' : 'not-allowed',
                }}
              >
                Claim Rewards
              </button>
            </div>
          </div>
        </>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  container: {
    padding: '20px',
    maxWidth: '800px',
    margin: '0 auto',
  },
  headerSection: {
    marginBottom: '20px',
    textAlign: 'center',
  },
  tierBadge: {
    display: 'inline-block',
    padding: '8px 16px',
    color: 'black',
    borderRadius: '20px',
    fontWeight: 'bold',
  },
  metricsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(2, 1fr)',
    gap: '15px',
    marginBottom: '30px',
  },
  metric: {
    padding: '15px',
    backgroundColor: '#f9f9f9',
    borderRadius: '8px',
    textAlign: 'center',
    border: '1px solid #e0e0e0',
  },
  label: {
    display: 'block',
    fontSize: '12px',
    color: '#666',
    marginBottom: '5px',
  },
  value: {
    display: 'block',
    fontSize: '24px',
    fontWeight: 'bold',
    color: '#333',
  },
  section: {
    marginBottom: '30px',
    padding: '20px',
    backgroundColor: '#f9f9f9',
    borderRadius: '8px',
    border: '1px solid #e0e0e0',
  },
  referralLink: {
    display: 'flex',
    gap: '10px',
    marginTop: '10px',
  },
  linkInput: {
    flex: 1,
    padding: '8px 12px',
    borderRadius: '4px',
    border: '1px solid #ddd',
    fontFamily: 'monospace',
    fontSize: '12px',
  },
  copyButton: {
    padding: '8px 16px',
    backgroundColor: '#007bff',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
  },
  funnelMetrics: {
    marginTop: '10px',
  },
  earningsSection: {
    marginTop: '10px',
  },
  claimButton: {
    marginTop: '15px',
    padding: '10px 20px',
    backgroundColor: '#28a745',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
  },
};
