# Risk Monitoring Dashboard

## Overview

Real-time risk monitoring dashboard for protocol health, liquidation risks, oracle status, and safety metrics.

## Features

### 1. Pool Health Metrics

Monitor health of individual lending pools:

**Endpoint**: `GET /api/risk/pool-health/:poolId`

**Metrics**:
- Utilization rate (borrowed / supplied)
- Total supplied assets
- Total borrowed assets
- Available liquidity
- Average LTV across borrowers
- Concentration risk score

**Example Response**:
```json
{
  "poolId": "pool-xlm",
  "utilizationRate": 8000,
  "totalSupplied": "1000000",
  "totalBorrowed": "800000",
  "availableLiquidity": "200000",
  "averageLtv": 6500,
  "concentrationRisk": 2500
}
```

### 2. Liquidation Risk Heatmap

Visual representation of liquidation risks across pools and users:

**Endpoint**: `GET /api/risk/liquidation-heatmap?poolId=pool-1`

**Risk Levels**:
- **Safe**: Health factor >= 1.5 (15000)
- **Warning**: Health factor >= 1.2 (12000)
- **Danger**: Health factor >= 1.0 (10000)
- **Critical**: Health factor < 1.0 (liquidatable)

**Example Response**:
```json
{
  "pool-1": [
    {
      "user": "GABC...",
      "poolId": "pool-1",
      "healthFactor": 11000,
      "collateralValue": "100000",
      "debtValue": "80000",
      "liquidationThreshold": 8000,
      "riskLevel": "Danger"
    }
  ]
}
```

### 3. Oracle Health Monitoring

Track oracle status and price feed reliability:

**Endpoint**: `GET /api/risk/oracle-health`

**Checks**:
- Price staleness (time since last update)
- Deviation from TWAP
- Source reliability
- Update frequency

**Example Response**:
```json
[
  {
    "asset": "XLM",
    "lastUpdateTimestamp": 1234567890,
    "price": "0.12",
    "stalenessSeconds": 120,
    "deviationFromTwap": 150,
    "isHealthy": true
  }
]
```

### 4. Protocol Safety Score

Composite score aggregating all risk factors:

**Endpoint**: `GET /api/risk/safety-score`

**Components**:
- **Liquidity Score (25%)**: Pool liquidity and utilization
- **Solvency Score (35%)**: Protocol capital adequacy
- **Oracle Health Score (20%)**: Price feed reliability
- **Concentration Score (20%)**: Asset and user concentration

**Example Response**:
```json
{
  "overallScore": 8500,
  "liquidityScore": 8500,
  "solvencyScore": 9000,
  "oracleHealthScore": 8800,
  "concentrationScore": 7500,
  "timestamp": 1234567890
}
```

### 5. Historical Metric Trends

Track risk metrics over time:

**Endpoint**: `GET /api/risk/metric-trends?metric=utilization&period=24h`

**Supported Metrics**:
- utilization
- average_ltv
- liquidation_count
- protocol_tvl
- health_factor_distribution

**Periods**: `24h`, `7d`, `30d`

**Example Response**:
```json
[
  {
    "timestamp": 1234567890,
    "value": 7500,
    "metric": "utilization"
  }
]
```

### 6. Alert Configuration

Configure thresholds for automated alerts:

**Endpoint**: `PUT /api/risk/alert-config`

**Request Body**:
```json
{
  "healthFactorThreshold": 12000,
  "utilizationThreshold": 9000,
  "concentrationThreshold": 3000,
  "oracleStalenessThreshold": 300
}
```

### 7. Active Alerts

Retrieve current risk alerts:

**Endpoint**: `GET /api/risk/alerts?severity=high&limit=50`

**Severity Levels**: `low`, `medium`, `high`, `critical`

**Example Response**:
```json
[
  {
    "id": "1",
    "severity": "high",
    "type": "liquidation_risk",
    "message": "User GABC... health factor below threshold",
    "timestamp": 1234567890,
    "acknowledged": false
  }
]
```

### 8. User Risk Profile

Detailed risk analysis for individual users:

**Endpoint**: `GET /api/risk/user/:address/risk-profile`

**Example Response**:
```json
{
  "address": "GABC...",
  "healthFactor": 13500,
  "totalCollateralValue": "150000",
  "totalDebtValue": "90000",
  "ltv": 6000,
  "liquidationPrice": "0.08",
  "riskLevel": "Warning",
  "positions": [
    {
      "poolId": "pool-1",
      "collateral": "100000",
      "debt": "60000",
      "healthFactor": 14000
    }
  ]
}
```

## Smart Contract Integration

The dashboard integrates with on-chain risk monitoring:

```rust
use stellar_lend::risk_dashboard::*;

// Calculate pool health
let health = calculate_pool_health(&env, &pool_id, total_supplied, total_borrowed);

// Check liquidation risk
let risk_entry = calculate_liquidation_risk(
    &user,
    &pool_id,
    health_factor,
    collateral_value,
    debt_value,
    liquidation_threshold
);

// Get protocol safety score
let score = calculate_protocol_safety_score(
    &env,
    liquidity_score,
    solvency_score,
    oracle_score,
    concentration_score
);
```

## Dashboard UI Components

### Pool Health Grid
Display all pools with color-coded health indicators:
- Green: Healthy (utilization < 80%)
- Yellow: Warning (utilization 80-90%)
- Red: Critical (utilization > 90%)

### Liquidation Heatmap
Matrix view showing:
- Rows: Users
- Columns: Pools
- Color intensity: Risk level

### Oracle Status Panel
Real-time feed status with:
- Price ticker
- Last update timestamp
- Health indicator
- TWAP deviation chart

### Safety Score Gauge
Circular gauge showing overall protocol safety:
- 90-100: Excellent
- 75-90: Good
- 60-75: Caution
- <60: Critical

### Historical Charts
Line charts for trend analysis:
- Utilization over time
- Average LTV trends
- Liquidation event frequency
- Protocol TVL changes

### Alert Feed
Live feed of risk alerts with:
- Severity badges
- Timestamp
- Quick action buttons
- Acknowledgment tracking

## Implementation Notes

### Data Refresh
- Real-time metrics: WebSocket updates every 5 seconds
- Historical data: Fetched on demand with caching
- Alert checks: Continuous background monitoring

### Performance Optimization
- Pagination for large datasets
- Aggregated metrics to reduce contract calls
- Client-side caching with TTL
- Lazy loading of detailed views

### Alert Rules
Alerts trigger when:
1. Health factor < configured threshold
2. Utilization > 90%
3. Oracle price stale > 5 minutes
4. Single asset concentration > 30%
5. Protocol safety score < 70

## Security Considerations

### Access Control
- Public read access for aggregate metrics
- Admin-only access for alert configuration
- Rate limiting on API endpoints

### Data Privacy
- User addresses truncated in public views
- Detailed positions require authentication
- Sensitive data encrypted at rest

## Future Enhancements

1. **Predictive Analytics**: ML models for liquidation prediction
2. **Automated Actions**: Auto-pause on critical risk levels
3. **Multi-chain Support**: Cross-chain risk aggregation
4. **Advanced Alerts**: Telegram/Discord notifications
5. **Risk Reports**: Automated daily/weekly reports
6. **Simulation Tools**: What-if scenario testing

## API Authentication

All endpoints support optional authentication for enhanced data:

```bash
# Public access (limited data)
curl https://api.stellarlend.com/risk/pool-health/pool-1

# Authenticated access (full data)
curl -H "Authorization: Bearer <token>" \
     https://api.stellarlend.com/risk/user/GABC.../risk-profile
```

## Monitoring Best Practices

1. **Set Appropriate Thresholds**: Balance sensitivity vs alert fatigue
2. **Regular Health Checks**: Review oracle status daily
3. **Trend Analysis**: Look for gradual deterioration
4. **Correlation Monitoring**: Cross-reference multiple metrics
5. **Documentation**: Document alert responses and actions taken
