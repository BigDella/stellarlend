# Gas Cost Estimation System

## Overview

The Gas Cost Estimation System provides comprehensive cost prediction and optimization for all lending pool operations. Users receive upfront gas cost estimates before transaction submission, along with optimization suggestions to minimize costs.

## Features

### 1. Pre-Transaction Cost Estimation
- **Real-time estimation** using Stellar network simulation
- **Detailed breakdown**: base cost, storage writes, cross-contract calls, resource fees
- **USD cost conversion** based on current XLM price
- **Confidence levels** based on historical variance (high/medium/low)

### 2. Cost Breakdown Components

| Component | Description | Example Cost |
|-----------|-------------|--------------|
| Base Fee | Standard Stellar transaction fee | 100 stroops |
| Storage Cost | Per-storage write operation | 10,000 stroops/write |
| Cross-Contract | Per cross-contract call | 5,000 stroops/call |
| Resource Fee | CPU and memory consumption | Variable |

### 3. Historical Gas Data
- **Aggregated metrics**: average, min, max, standard deviation
- **Time periods**: 24h, 7d, 30d
- **Sample counts**: number of transactions analyzed
- **Trend analysis**: identify patterns and anomalies

### 4. Optimization Suggestions

#### Timing Optimization
- **Off-peak hours**: Execute during low-traffic periods (00:00-06:00 UTC)
- **Potential savings**: Up to 15% reduction in gas costs
- **Recommendation**: Automatic timing suggestions based on historical data

#### Batching Optimization
- **Multi-operation batching**: Combine multiple operations in one transaction
- **Savings**: Up to 15% per-operation cost reduction
- **Best for**: Large amounts, multiple operations on same asset

#### Method Optimization
- **Alternative execution paths**: Different approaches for same outcome
- **Flash loan liquidations**: More efficient for large positions
- **Partial operations**: Split large operations for better gas efficiency

### 5. Cost Comparison Across Operations

Typical gas costs ranked from cheapest to most expensive:

1. **Flash Loan** (~70,000 CPU instructions) - Most efficient
2. **Withdraw** (~144,000 CPU instructions)
3. **Borrow** (~245,000 CPU instructions)
4. **Deposit** (~355,000 CPU instructions)
5. **Liquidation** (~394,000 CPU instructions)
6. **Repay** (~430,000 CPU instructions) - Most expensive

### 6. Accuracy Tracking
- **Estimated vs Actual** comparison
- **Error metrics**: mean absolute error, percentage error
- **Accuracy reports**: overall and per-operation statistics
- **Continuous improvement**: learn from actual costs

### 7. Configurable Alerts
- **Threshold-based**: Alert when cost exceeds configured limit
- **User-specific**: Per-user or global alerts
- **Operation-specific**: Configure different thresholds per operation
- **Real-time notifications**: Immediate alert on threshold breach

## API Endpoints

### Estimate Gas Cost
```http
POST /api/gas/estimate
Content-Type: application/json

{
  "operation": "deposit",
  "userAddress": "GABC...XYZ",
  "assetAddress": "CUSD...123",
  "amount": "1000000000",
  "includeOptimizations": true,
  "includeHistorical": true
}
```

**Response:**
```json
{
  "operation": "deposit",
  "breakdown": {
    "baseCost": "100",
    "storageCost": "20000",
    "crossContractCost": "5000",
    "totalCost": "379765",
    "cpuInstructions": "354765",
    "memoryBytes": "58682",
    "minResourceFee": "354665"
  },
  "optimizations": [
    {
      "type": "timing",
      "title": "Execute During Low Gas Period",
      "description": "Gas costs are typically 15% lower during off-peak hours",
      "potentialSavings": "56965",
      "priority": "medium"
    }
  ],
  "historical": {
    "operation": "deposit",
    "averageCost": "379765",
    "minCost": "341789",
    "maxCost": "417742",
    "stdDeviation": "18988",
    "sampleCount": 10080,
    "period": "30d"
  },
  "estimatedUsdCost": "0.003798",
  "timestamp": "2026-06-29T10:30:00Z",
  "confidence": "high"
}
```

### Get Historical Data
```http
GET /api/gas/historical/:operation?period=30d
```

### Get Historical Chart
```http
GET /api/gas/chart/:operation?period=7d
```

### Compare All Operations
```http
GET /api/gas/compare
```

### Configure Alert
```http
POST /api/gas/alerts
Content-Type: application/json

{
  "userAddress": "GABC...XYZ",
  "operation": "borrow",
  "threshold": "500000",
  "enabled": true
}
```

### Get User Alerts
```http
GET /api/gas/alerts?userAddress=GABC...XYZ
```

### Record Actual Cost
```http
POST /api/gas/accuracy
Content-Type: application/json

{
  "operation": "deposit",
  "estimatedCost": "379765",
  "actualCost": "385210",
  "txHash": "abc123..."
}
```

### Get Accuracy Report
```http
GET /api/gas/accuracy?period=7d
```

### Estimate Batch Cost
```http
POST /api/gas/batch-estimate
Content-Type: application/json

{
  "operations": [
    {
      "operation": "deposit",
      "userAddress": "GABC...XYZ",
      "amount": "1000000000"
    },
    {
      "operation": "borrow",
      "userAddress": "GABC...XYZ",
      "amount": "500000000"
    }
  ]
}
```

### Get Timing Recommendation
```http
GET /api/gas/timing/:operation
```

## Frontend Components

### 1. GasCostEstimator
Main component for displaying gas cost estimates with breakdown and optimizations.

```tsx
import { GasCostEstimator } from '@/components/GasCostEstimator';

<GasCostEstimator
  operation="deposit"
  userAddress="GABC...XYZ"
  assetAddress="CUSD...123"
  amount="1000000000"
  apiBaseUrl="/api"
  onEstimateComplete={(estimate) => console.log(estimate)}
/>
```

### 2. GasComparisonChart
Visual comparison of gas costs across all operations.

```tsx
import { GasComparisonChart } from '@/components/GasComparisonChart';

<GasComparisonChart
  apiBaseUrl="/api"
  autoRefresh={true}
  refreshInterval={60}
/>
```

### 3. GasHistoricalChart
Historical gas cost trends with min/max ranges.

```tsx
import { GasHistoricalChart } from '@/components/GasHistoricalChart';

<GasHistoricalChart
  operation="deposit"
  period="7d"
  apiBaseUrl="/api"
/>
```

## Implementation Details

### Cost Calculation Formula

```typescript
totalCost = baseFee + storageCost + crossContractCost + resourceFee

where:
  baseFee = 100 stroops (fixed)
  storageCost = storageWrites × 10000 stroops
  crossContractCost = crossContractCalls × 5000 stroops
  resourceFee = f(cpuInstructions, memoryBytes)
```

### Operation Complexity

| Operation | Storage Writes | Cross-Contract Calls |
|-----------|----------------|----------------------|
| Deposit | 2 | 1 |
| Withdraw | 2 | 1 |
| Borrow | 3 | 2 |
| Repay | 3 | 2 |
| Liquidation | 4 | 3 |
| Flash Loan | 2 | 2 |

### Baseline CPU Costs (from benchmarks)

```typescript
{
  deposit: 354765,
  withdraw: 144093,
  borrow: 244830,
  repay: 430316,
  liquidation: 394438,
  flash_loan: 70030
}
```

## Edge Cases Handled

### 1. Gas Price Volatility
- **Confidence levels**: Indicate estimation reliability based on historical variance
- **Real-time simulation**: Always use latest network conditions
- **Fallback to baseline**: Use benchmark data if simulation fails

### 2. Estimation vs Actual Variance
- **Accuracy tracking**: Record all estimated vs actual comparisons
- **Continuous learning**: Adjust estimation models based on accuracy data
- **Error reporting**: Track mean absolute error and percentage error

### 3. L2 vs L1 Gas Differences
- **Network-aware**: Detects Stellar network type (mainnet/testnet/futurenet)
- **Separate baselines**: Different cost models per network
- **Environment-specific**: Adjusts for network characteristics

### 4. Simulation Failures
- **Graceful fallback**: Use benchmark baseline costs
- **Error logging**: Record simulation failures for investigation
- **User notification**: Indicate when fallback is used (lower confidence)

### 5. Historical Data Gaps
- **Minimum samples**: Require sufficient data before generating trends
- **Interpolation**: Fill gaps using surrounding data points
- **Period selection**: Choose appropriate time windows for analysis

## Performance Optimizations

### 1. Caching Strategy
- **Estimate cache**: 5 minutes TTL
- **Historical data**: 1 hour TTL
- **Chart data**: 30 minutes TTL
- **Comparison data**: 1 hour TTL

### 2. Request Coalescing
- **Deduplication**: Merge identical concurrent requests
- **Batch requests**: Group multiple estimates when possible
- **Rate limiting**: Prevent excessive estimation requests

### 3. Data Aggregation
- **Pre-computed metrics**: Calculate historical stats in background
- **Incremental updates**: Update metrics as new data arrives
- **Periodic cleanup**: Remove old accuracy metrics (keep last 1000)

## Testing

### Unit Tests
```bash
cd api
npm test -- gas.controller.test.ts
npm test -- gas/estimator.test.ts
```

### Integration Tests
```bash
npm test -- integration/gas-estimation.test.ts
```

### Load Tests
```bash
npm run test:load -- --endpoint /api/gas/estimate
```

## Monitoring

### Key Metrics
- **Estimation accuracy**: Target <10% error rate
- **Response time**: Target <500ms for estimates
- **Cache hit rate**: Target >80%
- **Alert trigger rate**: Monitor for threshold tuning

### Dashboards
- Gas cost trends over time
- Estimation accuracy by operation
- Optimization suggestion effectiveness
- User alert statistics

## Future Enhancements

1. **Machine Learning Models**: Improve estimation accuracy using ML
2. **Predictive Analytics**: Forecast future gas costs based on trends
3. **Smart Scheduling**: Automatically schedule transactions for optimal gas
4. **Multi-chain Support**: Extend to other Soroban-based chains
5. **Gas Tokens**: Implement gas token system for cost stability
6. **Dynamic Pricing**: Adjust protocol fees based on gas costs

## Security Considerations

1. **Rate Limiting**: Prevent estimation API abuse
2. **Input Validation**: Sanitize all user inputs
3. **Error Handling**: Never expose internal system details
4. **Cache Poisoning**: Validate cached data integrity
5. **DoS Protection**: Limit concurrent simulation requests

## Support

For issues or questions about the gas estimation system:
- GitHub Issues: https://github.com/Smartdevs17/stellarlend-contracts/issues
- Documentation: https://stellarlend.com/docs/gas-estimation
- Email: support@stellarlend.com

## License

MIT License - See LICENSE file for details
