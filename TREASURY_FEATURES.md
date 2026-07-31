# Treasury Management Features

This document describes the comprehensive treasury management system implemented for the StellarLend protocol. The system includes four major features: treasury dashboard with cash flow forecasting, portfolio rebalancing automation, yield harvesting through external protocols, and protocol-owned liquidity management.

## Overview

The treasury management system provides protocol operators with complete visibility and control over protocol finances. It enables sophisticated financial planning, automated asset management, and yield optimization.

## Features Implemented

### 1. Treasury Dashboard with Cash Flow Forecasting (Issue #473)

The treasury dashboard provides real-time visibility into protocol finances and enables comprehensive financial planning.

#### API Endpoints

**Base Path:** `/api/treasury/dashboard`

- `GET /overview` - Get current treasury balance by asset and total USD value
- `POST /asset-balance` - Set asset balance and USD price
- `POST /revenue` - Record revenue (fees, liquidation penalties, yield)
- `POST /expense` - Record expense (development grants, operational costs, marketing)
- `GET /revenue-tracking` - Get revenue by source (30-day window)
- `GET /expense-tracking` - Get expenses by category (30-day window)
- `GET /cashflow-forecasts` - Get 30/60/90 day cash flow projections
- `GET /scenario-analysis` - Get conservative, base, and optimistic scenarios
- `GET /burn-rate-runway` - Get burn rate and runway analysis
- `GET /cashflow-history` - Get historical cash flow reports (monthly/quarterly)

#### Key Features

- **Real-time Balances**: Track treasury balance by asset with USD conversion
- **Revenue Tracking**: Monitor fee collection, liquidation penalties, and yield
- **Expense Tracking**: Track development grants, operational costs, and marketing spend
- **Cash Flow Forecasting**: 30/60/90 day projections based on historical trends
- **Scenario Planning**: Adjust revenue/expense assumptions to model different scenarios
- **Burn Rate Analysis**: Calculate daily/monthly burn rate and runway in months
- **Historical Reports**: Monthly and quarterly cash flow history with detailed breakdown

#### Usage Example

```bash
# Set asset balance
curl -X POST http://localhost:3000/api/treasury/dashboard/asset-balance \
  -H "Content-Type: application/json" \
  -d '{
    "asset": "USDC",
    "balance": 1000000,
    "usdPrice": 1.0
  }'

# Record revenue
curl -X POST http://localhost:3000/api/treasury/dashboard/revenue \
  -H "Content-Type: application/json" \
  -d '{
    "source": "fees",
    "amount": 5000,
    "asset": "USDC"
  }'

# Get treasury overview
curl http://localhost:3000/api/treasury/dashboard/overview
```

---

### 2. Treasury Portfolio Rebalancing Automation (Issue #470)

Automated rebalancing ensures the protocol treasury maintains optimal asset allocation according to governance-defined targets.

#### API Endpoints

**Base Path:** `/api/treasury/rebalancer`

- `POST /target-allocations` - Set target asset allocations (must sum to 100%)
- `POST /asset-prices` - Set current asset prices for valuation
- `POST /current-allocations` - Set current portfolio allocations
- `GET /allocation` - Get current allocation status
- `GET /rebalance-trigger` - Check if rebalance is needed (time-based or deviation-based)
- `GET /simulate` - Simulate rebalance with estimated costs and slippage
- `POST /execute` - Execute rebalance based on simulation
- `GET /history` - Get rebalance execution history
- `POST /governance-proposal` - Create governance proposal for allocation changes
- `GET /governance-proposals` - Get governance proposals (pending/approved/rejected)
- `POST /governance-proposal/:proposalId/approve` - Approve governance proposal
- `POST /governance-proposal/:proposalId/reject` - Reject governance proposal
- `POST /pause` - Pause automatic rebalancing
- `POST /resume` - Resume automatic rebalancing
- `GET /status` - Get rebalancer status (paused/running)

#### Key Features

- **Target Allocation Management**: Define desired asset allocation percentages
- **Deviation Monitoring**: Track deviation from targets (threshold-based: >5% drift triggers rebalance)
- **Time-Based Rebalancing**: Weekly automatic rebalancing schedule
- **Swap Execution**: DEX integration with slippage protection
- **Rebalance Simulation**: Preview cost and resulting allocation before execution
- **Gas Optimization**: Batch swap execution to minimize gas costs
- **Partial Rebalancing**: Only correct deviations exceeding threshold
- **Governance Control**: Pause/resume and governance-approved allocation changes
- **Execution Reporting**: Detailed report of swaps, costs, and slippage

#### Usage Example

```bash
# Set target allocations
curl -X POST http://localhost:3000/api/treasury/rebalancer/target-allocations \
  -H "Content-Type: application/json" \
  -d '{
    "allocations": [
      { "asset": "USDC", "targetPercentage": 40 },
      { "asset": "BTC", "targetPercentage": 30 },
      { "asset": "ETH", "targetPercentage": 30 }
    ]
  }'

# Simulate rebalance
curl "http://localhost:3000/api/treasury/rebalancer/simulate?slippagePercentage=0.3"

# Execute rebalance
curl -X POST http://localhost:3000/api/treasury/rebalancer/execute \
  -H "Content-Type: application/json" \
  -d '{ "slippagePercentage": 0.3 }'
```

---

### 3. Treasury Yield Harvesting (Issue #472)

Automated yield harvesting deploys treasury assets to whitelisted external DeFi protocols and harvests yield.

#### API Endpoints

**Base Path:** `/api/treasury/yield`

- `POST /protocol` - Register external protocol
- `POST /whitelist` - Update protocol whitelist (daily check)
- `GET /protocols` - Get all registered protocols
- `GET /whitelisted-protocols` - Get whitelisted protocols only
- `GET /risk-score/:protocolId` - Calculate risk score (TVL, audit, age)
- `POST /deploy` - Deploy assets to external protocol
- `GET /positions` - Get yield positions (active/pending/withdrawn)
- `POST /harvest` - Harvest yield from active positions
- `GET /withdrawal-simulation/:positionId` - Preview withdrawal costs and PnL
- `POST /withdraw/:positionId` - Withdraw from protocol
- `POST /emergency-withdraw/:positionId` - Emergency withdrawal with reason
- `GET /yield-report` - Get yield report (daily/monthly)
- `GET /emergency-withdrawals` - Get emergency withdrawal history

#### Key Features

- **Protocol Whitelist**: Approve external protocols for deployment
- **Deployment Strategies**: Single-sided LP, Curve, Convex, staking
- **Risk Scoring**: Contract risk, slashing risk, bridge risk assessment
- **Maximum Allocation**: Configurable max allocation per protocol
- **Automated Harvesting**: Daily yield harvest from active positions
- **Emergency Withdrawal**: Rapid withdrawal for risk mitigation
- **Withdrawal Simulation**: Preview withdrawal costs and expected return
- **Daily P&L Tracking**: Comprehensive yield and PnL reporting
- **Risk Assessment**: TVL-based, audit status, and age scoring

#### Usage Example

```bash
# Register protocol
curl -X POST http://localhost:3000/api/treasury/yield/protocol \
  -H "Content-Type: application/json" \
  -d '{
    "protocolId": "curve-finance",
    "name": "Curve Finance",
    "tvl": 5000000000,
    "auditStatus": "audited",
    "ageMonths": 48,
    "supportedStrategies": ["curve"],
    "maxAllocationPercentage": 20
  }'

# Deploy to protocol
curl -X POST http://localhost:3000/api/treasury/yield/deploy \
  -H "Content-Type: application/json" \
  -d '{
    "protocolId": "curve-finance",
    "asset": "USDC",
    "amount": 100000,
    "strategy": "curve"
  }'

# Harvest yield
curl -X POST http://localhost:3000/api/treasury/yield/harvest
```

---

### 4. Protocol-Owned Liquidity Management (Issue #471)

POL module enables the protocol to deploy treasury assets into its own lending pools and earn yield while reducing external liquidity dependence.

#### API Endpoints

**Base Path:** `/api/treasury/pol`

- `POST /pool` - Initialize pool with liquidity metrics
- `POST /deployment-proposal` - Create governance proposal for POL deployment
- `POST /deployment-proposal/:proposalId/approve` - Approve deployment proposal
- `POST /deployment-proposal/:proposalId/execute` - Execute deployment (after timelock)
- `POST /deployment-proposal/:proposalId/reject` - Reject deployment proposal
- `POST /withdrawal-proposal` - Create governance proposal for POL withdrawal
- `POST /withdrawal-proposal/:proposalId/approve` - Approve withdrawal proposal
- `POST /withdrawal-proposal/:proposalId/execute` - Execute withdrawal (after timelock)
- `POST /withdrawal-proposal/:proposalId/reject` - Reject withdrawal proposal
- `POST /rebalance-proposal` - Create governance proposal for POL rebalancing
- `POST /rebalance-proposal/:proposalId/approve` - Approve rebalance proposal
- `POST /rebalance-proposal/:proposalId/execute` - Execute rebalance
- `POST /rebalance-proposal/:proposalId/reject` - Reject rebalance proposal
- `GET /positions` - Get POL positions (active/pending/withdrawn)
- `POST /harvest` - Harvest yield from POL positions
- `GET /dashboard` - Get POL dashboard (value, yield, % of total)
- `GET /history` - Get POL activity history (deploy/withdraw/harvest/rebalance)
- `GET /deployment-proposals` - Get deployment proposals by status
- `GET /withdrawal-proposals` - Get withdrawal proposals by status
- `GET /rebalance-proposals` - Get rebalance proposals by status

#### Key Features

- **Governance-Controlled Deployment**: Timelock-based governance for POL deployment
- **Separate Accounting**: POL vs external liquidity clearly separated
- **Yield Earning**: POL positions earn yield like any depositor
- **Withdrawal Control**: Governance approval and timelock for withdrawals
- **Non-Voting Shares**: POL cannot be used for governance (no double counting)
- **Rebalancing**: Move POL between pools based on yield and utilization
- **Liquidity Monitoring**: Real-time monitoring of POL % of total liquidity
- **Activity Tracking**: Complete history of all POL actions
- **Risk Management**: Governance can pause or reject proposals

#### Usage Example

```bash
# Initialize pool
curl -X POST http://localhost:3000/api/treasury/pol/pool \
  -H "Content-Type: application/json" \
  -d '{
    "pool": "USDC-pool",
    "totalLiquidity": 50000000,
    "utilizationRate": 0.75,
    "currentAPY": 8.5
  }'

# Create deployment proposal
curl -X POST http://localhost:3000/api/treasury/pol/deployment-proposal \
  -H "Content-Type: application/json" \
  -d '{
    "poolId": "USDC-pool",
    "amount": 1000000,
    "proposedBy": "governance-multisig",
    "timelockDuration": 172800000
  }'

# Approve deployment
curl -X POST http://localhost:3000/api/treasury/pol/deployment-proposal/prop-xxx/approve

# Execute deployment (after timelock expires)
curl -X POST http://localhost:3000/api/treasury/pol/deployment-proposal/prop-xxx/execute

# Get POL dashboard
curl http://localhost:3000/api/treasury/pol/dashboard
```

---

## Integration Guide

### Architecture

The treasury management system follows a service-oriented architecture:

```
Routes (REST API)
    ↓
Controllers (Request handling & validation)
    ↓
Services (Business logic & state management)
    ↓
Data Store (In-memory store, can be extended to DB)
```

### Adding New Features

1. **Create Service** (`api/src/services/treasury-*.service.ts`)
   - Implement business logic and state management
   - Export service singleton

2. **Create Controller** (`api/src/controllers/treasury-*.controller.ts`)
   - Handle HTTP requests
   - Validate input
   - Call service methods

3. **Create Routes** (`api/src/routes/treasury-*.routes.ts`)
   - Define REST endpoints
   - Wire controller methods

4. **Register Routes** (`api/src/app.ts`)
   - Import routes
   - Mount on appropriate path

### Error Handling

All endpoints follow consistent error handling:

```json
{
  "success": false,
  "error": "Error message"
}
```

Validation errors return 400 status code with detailed error message.

### Response Format

All successful endpoints return:

```json
{
  "success": true,
  "data": { /* response data */ }
}
```

---

## Best Practices

1. **Governance Control**: All critical operations (deployment, withdrawal) require governance approval and timelock
2. **Risk Management**: Use risk scores and maximum allocation percentages to limit exposure
3. **Monitoring**: Regular monitoring of burn rate, yield, and liquidity metrics
4. **Diversification**: Spread POL and yield positions across multiple strategies/protocols
5. **Emergency Procedures**: Maintain ability to rapidly withdraw in crisis situations

---

## Testing

The implementation includes comprehensive business logic. To test:

1. Set asset balances and prices
2. Record revenues and expenses
3. Create and execute rebalancing proposals
4. Deploy to external protocols and harvest yield
5. Create and execute POL governance proposals

All operations can be tested via the REST API endpoints.

---

## Future Enhancements

1. **Database Persistence**: Migrate from in-memory store to persistent storage
2. **On-Chain Integration**: Direct contract interaction for actual swaps and deployments
3. **Advanced Analytics**: Predictive modeling and trend analysis
4. **Alerts & Notifications**: Real-time alerts for critical events
5. **Audit Trail**: Complete immutable audit log of all treasury operations
6. **Multi-Currency**: Support for additional stablecoins and assets
7. **Advanced Strategies**: Options, derivatives, and complex yield strategies

---

## Monitoring & Analytics

The treasury system provides comprehensive monitoring:

- **Real-time Dashboard**: Current balances, allocations, and yield
- **Historical Analysis**: Monthly/quarterly trends and forecasts
- **Scenario Planning**: Model different market conditions
- **Risk Assessment**: Protocol risk scoring and exposure monitoring
- **Performance Metrics**: ROI, Sharpe ratio, and other analytics

---

## Security Considerations

1. **Governance Required**: All significant changes require governance approval
2. **Timelock**: Governance proposals include timelock periods for review
3. **Whitelisting**: External protocols must be explicitly whitelisted
4. **Risk Scoring**: Automated risk assessment of external protocols
5. **Emergency Withdrawal**: Rapid withdrawal capability for emergency situations
6. **Audit Trail**: Complete tracking of all actions

---

## Related Issues

- Issue #473: Treasury Dashboard with Cash Flow Forecasting
- Issue #470: Treasury Portfolio Rebalancing Automation
- Issue #472: Treasury Yield Harvesting
- Issue #471: Protocol-Owned Liquidity Management
