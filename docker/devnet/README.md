# StellarLend Local Devnet

Full local Soroban development environment with preconfigured accounts, deployed contracts, and seeded test data.

## Prerequisites

- Docker and Docker Compose v2+
- 4 GB RAM available for Docker
- Ports 3000, 4000, 5432, 6379, 8000 available (or override via `.env`)

## Quick Start

```bash
# From the repository root:
cd docker/devnet

# Copy and optionally edit the environment file
cp .env.example .env

# Start everything (first run takes 2-3 minutes)
docker compose up -d

# Watch initialization progress
docker logs -f stellarlend-devnet-init

# Verify all services are healthy
./healthcheck.sh
```

That's it. The `init-devnet` service automatically creates accounts, deploys contracts, and seeds sample data on first boot.

## Services

| Service       | URL                                    | Description                     |
|---------------|----------------------------------------|---------------------------------|
| Soroban RPC   | http://localhost:8000/soroban/rpc      | Soroban JSON-RPC endpoint       |
| Horizon       | http://localhost:8000                  | Stellar Horizon API             |
| Friendbot     | http://localhost:8000/friendbot        | Fund accounts with test XLM     |
| API           | http://localhost:3000                  | StellarLend REST API            |
| Oracle        | http://localhost:4000                  | Mock price oracle               |
| PostgreSQL    | localhost:5432                         | Database (stellarlend/stellarlend) |
| Redis         | localhost:6379                         | Cache                           |

## Preconfigured Accounts

| Account     | Role                    | Initial XLM |
|-------------|-------------------------|-------------|
| admin       | Protocol admin/deployer | 100,000     |
| liquidator  | Liquidation bot         | 50,000      |
| user1       | Test borrower           | 10,000      |
| user2       | Test lender             | 10,000      |
| user3       | Mixed (near-liquidation)| 10,000      |
| user4       | Flash loan user         | 10,000      |
| user5       | Institutional wallet    | 10,000      |

Retrieve account addresses:

```bash
# From inside the init container or with stellar CLI configured
stellar keys address admin
stellar keys address user1
```

## Deployed Contracts

All 16 workspace contracts are deployed and initialized:

common, lending-types, lending-interest, lending-risk, lending-core, lending, stablecoin, amm, bridge, delegation-registry, institutional-wallet, migration-hub, stealth-address, privacy-pool, test-utils, hello-world

Contract IDs are stored in the `devnet_deploy` volume at `/deploy/contracts/<name>.id`.

## Seeded Test Data

- **Deposits**: 5 active deposits across XLM, USDC, ETH, BTC
- **Active Loans**: 3 loans with varying health factors
- **Pending Liquidations**: 2 positions below threshold (trigger with oracle scenarios)
- **AMM Pools**: XLM/USDC and XLM/slUSD liquidity pools

### Triggering Liquidation Scenarios

The oracle supports price scenarios:

```bash
# Stable prices (default)
curl http://localhost:4000/prices?scenario=stable

# Bear market (triggers user3 liquidation)
curl http://localhost:4000/prices?scenario=bear

# Crash (triggers both pending liquidations)
curl http://localhost:4000/prices?scenario=crash
```

## Reset

To wipe all state and start fresh:

```bash
./reset-devnet.sh
```

This stops containers, removes volumes, rebuilds, and re-initializes. Target time: under 2 minutes.

## Health Check

```bash
# Human-readable output
./healthcheck.sh

# JSON output (for CI/scripts)
./healthcheck.sh --json

# Block until all services are healthy
./healthcheck.sh --wait
```

## VS Code Dev Container

This project includes a Dev Container configuration. To use it:

1. Install the **Dev Containers** extension in VS Code
2. Open the repository root in VS Code
3. Click "Reopen in Container" when prompted (or use the command palette: `Dev Containers: Reopen in Container`)

The Dev Container automatically starts the devnet and configures the Stellar/Soroban CLI tools.

## Network Configuration

| Parameter          | Value                                |
|--------------------|--------------------------------------|
| Network            | Standalone (local)                   |
| Passphrase         | `Standalone Network ; February 2017` |
| RPC URL            | `http://localhost:8000/soroban/rpc`  |
| Horizon URL        | `http://localhost:8000`              |

## Troubleshooting

**Services not starting?**
```bash
docker compose -f docker-compose.yml logs
```

**Init failed?**
```bash
docker logs stellarlend-devnet-init
```

**Port conflict?**
Edit `.env` to change port mappings.

**Out of disk space?**
```bash
docker system prune -a --volumes
```
