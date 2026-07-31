import React, { useEffect, useState } from 'react';

interface EmergencyState {
  paused: boolean;
  reason: string | null;
  since: number | null;
  queueLength: number;
}

interface EmergencyNotification {
  id: string;
  message: string;
  severity: 'info' | 'warning' | 'critical';
  timestamp: string;
  read: boolean;
}

interface EmergencyStatusBannerProps {
  userAddress?: string;
  onEmergencyWithdraw?: () => void;
}

export const EmergencyStatusBanner: React.FC<EmergencyStatusBannerProps> = ({
  userAddress,
  onEmergencyWithdraw,
}) => {
  const [state, setState] = useState<EmergencyState | null>(null);
  const [notifications, setNotifications] = useState<EmergencyNotification[]>([]);
  const [showNotifications, setShowNotifications] = useState(false);

  useEffect(() => {
    const fetchStatus = async () => {
      try {
        const response = await fetch('/api/emergency/status');
        const data = await response.json();
        if (data.success) {
          setState({
            paused: data.data.paused,
            reason: data.data.reason,
            since: data.data.since,
            queueLength: data.data.queueLength,
          });
        }
      } catch (error) {
        console.error('Failed to fetch emergency status:', error);
      }
    };

    const fetchNotifications = async () => {
      try {
        const response = await fetch('/api/emergency/notifications');
        const data = await response.json();
        if (data.success) {
          setNotifications(data.data);
        }
      } catch (error) {
        console.error('Failed to fetch notifications:', error);
      }
    };

    fetchStatus();
    fetchNotifications();
    const interval = setInterval(fetchStatus, 30000);
    return () => clearInterval(interval);
  }, []);

  const unreadCount = notifications.filter(n => !n.read).length;

  const getBannerColor = (): string => {
    if (!state?.paused) return '#28a745';
    switch (state.reason) {
      case 'auto-failure-threshold': return '#dc3545';
      case 'oracle-failure': return '#fd7e14';
      case 'governance-vote': return '#ffc107';
      default: return '#6c757d';
    }
  };

  const getReasonLabel = (reason: string | null): string => {
    switch (reason) {
      case 'manual': return 'Manual Pause';
      case 'auto-failure-threshold': return 'System Failure Threshold';
      case 'governance-vote': return 'Governance Vote';
      case 'oracle-failure': return 'Oracle Failure';
      default: return 'Unknown';
    }
  };

  const formatTimeSince = (timestamp: number): string => {
    const seconds = Math.floor((Date.now() - timestamp) / 1000);
    if (seconds < 60) return `${seconds}s ago`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m ago`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ago`;
    return `${Math.floor(seconds / 86400)}d ago`;
  };

  if (!state) return null;

  return (
    <div>
      <div
        style={{
          ...styles.banner,
          backgroundColor: getBannerColor(),
        }}
      >
        <div style={styles.bannerContent}>
          <div style={styles.bannerLeft}>
            <span style={styles.statusIcon}>
              {state.paused ? '\u26A0' : '\u2714'}
            </span>
            <div>
              <span style={styles.statusText}>
                {state.paused ? 'EMERGENCY PAUSE ACTIVE' : 'System Operating Normally'}
              </span>
              {state.paused && state.reason && (
                <span style={styles.reasonText}>
                  {getReasonLabel(state.reason)}
                  {state.since && ` - Since ${formatTimeSince(state.since)}`}
                </span>
              )}
            </div>
          </div>

          <div style={styles.bannerRight}>
            {state.paused && (
              <div style={styles.queueInfo}>
                <span style={styles.queueBadge}>{state.queueLength}</span>
                <span>Queued Withdrawals</span>
              </div>
            )}

            {state.paused && onEmergencyWithdraw && (
              <button onClick={onEmergencyWithdraw} style={styles.emergencyButton}>
                Emergency Withdraw
              </button>
            )}

            <button
              onClick={() => setShowNotifications(!showNotifications)}
              style={styles.notifButton}
            >
              Bell
              {unreadCount > 0 && (
                <span style={styles.notifBadge}>{unreadCount}</span>
              )}
            </button>
          </div>
        </div>
      </div>

      {showNotifications && (
        <div style={styles.notificationPanel}>
          <div style={styles.notifHeader}>
            <h4>Emergency Notifications</h4>
            <button
              onClick={() => setShowNotifications(false)}
              style={styles.closeButton}
            >
              Close
            </button>
          </div>
          <div style={styles.notifList}>
            {notifications.length === 0 ? (
              <div style={styles.noNotifs}>No notifications</div>
            ) : (
              notifications.slice(0, 20).map((notif) => (
                <div
                  key={notif.id}
                  style={{
                    ...styles.notifItem,
                    borderLeftColor: notif.severity === 'critical' ? '#dc3545'
                      : notif.severity === 'warning' ? '#ffc107' : '#007bff',
                    opacity: notif.read ? 0.6 : 1,
                  }}
                >
                  <div style={styles.notifSeverity}>
                    {notif.severity.toUpperCase()}
                  </div>
                  <div style={styles.notifMessage}>{notif.message}</div>
                  <div style={styles.notifTime}>
                    {new Date(notif.timestamp).toLocaleString()}
                  </div>
                </div>
              ))
            )}
          </div>
        </div>
      )}
    </div>
  );
};

const styles: Record<string, React.CSSProperties> = {
  banner: {
    padding: '12px 20px',
    borderRadius: '8px',
    marginBottom: '16px',
  },
  bannerContent: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    flexWrap: 'wrap',
    gap: '12px',
  },
  bannerLeft: {
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
  },
  statusIcon: {
    fontSize: '24px',
  },
  statusText: {
    display: 'block',
    color: 'white',
    fontWeight: 'bold',
    fontSize: '14px',
  },
  reasonText: {
    display: 'block',
    color: 'rgba(255,255,255,0.85)',
    fontSize: '12px',
    marginTop: '2px',
  },
  bannerRight: {
    display: 'flex',
    alignItems: 'center',
    gap: '12px',
  },
  queueInfo: {
    display: 'flex',
    alignItems: 'center',
    gap: '6px',
    color: 'white',
    fontSize: '13px',
  },
  queueBadge: {
    backgroundColor: 'rgba(255,255,255,0.25)',
    padding: '2px 8px',
    borderRadius: '12px',
    fontWeight: 'bold',
  },
  emergencyButton: {
    padding: '8px 16px',
    backgroundColor: 'white',
    color: '#dc3545',
    border: 'none',
    borderRadius: '4px',
    fontWeight: 'bold',
    cursor: 'pointer',
    fontSize: '13px',
  },
  notifButton: {
    position: 'relative',
    padding: '6px 10px',
    backgroundColor: 'rgba(255,255,255,0.2)',
    color: 'white',
    border: 'none',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '14px',
  },
  notifBadge: {
    position: 'absolute',
    top: '-4px',
    right: '-4px',
    backgroundColor: '#dc3545',
    color: 'white',
    padding: '1px 5px',
    borderRadius: '10px',
    fontSize: '10px',
    fontWeight: 'bold',
  },
  notificationPanel: {
    backgroundColor: 'white',
    border: '1px solid #e0e0e0',
    borderRadius: '8px',
    marginBottom: '16px',
    boxShadow: '0 4px 12px rgba(0,0,0,0.1)',
    overflow: 'hidden',
  },
  notifHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    padding: '12px 16px',
    borderBottom: '1px solid #e0e0e0',
  },
  closeButton: {
    padding: '4px 8px',
    backgroundColor: 'transparent',
    border: '1px solid #ddd',
    borderRadius: '4px',
    cursor: 'pointer',
    fontSize: '12px',
  },
  notifList: {
    maxHeight: '300px',
    overflowY: 'auto',
  },
  noNotifs: {
    padding: '20px',
    textAlign: 'center',
    color: '#999',
  },
  notifItem: {
    padding: '10px 16px',
    borderBottom: '1px solid #f0f0f0',
    borderLeft: '3px solid',
  },
  notifSeverity: {
    fontSize: '10px',
    fontWeight: 'bold',
    color: '#666',
    marginBottom: '2px',
  },
  notifMessage: {
    fontSize: '13px',
    color: '#333',
  },
  notifTime: {
    fontSize: '11px',
    color: '#999',
    marginTop: '4px',
  },
};
