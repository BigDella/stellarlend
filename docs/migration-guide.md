# Cross-Protocol Migration Guide

This guide explains how to use the StellarLend Migration Hub to move your positions from other protocols.

## Supported Protocols

- **StellarOtherLend**: Migrate from other lending protocols on Stellar.
- **Cross-Chain Bridge**: Migrate from EVM chains (Ethereum, Arbitrum, etc.) via the Stellar Bridge.
- **Aave (Mock)**: Simulated migration from Aave for testing.

## How to Migrate

### 1. Analysis
Before migrating, you can query your current position on the source protocol to see if it's eligible for migration.

### 2. Initiation
Call the `migrate` function on the `MigrationHub` contract.

```bash
stellar contract invoke \
  --id MIGRATION_HUB_ID \
  --source USER_SECRET \
  -- migrate \
  --user USER_ADDRESS \
  --protocol StellarOther \
  --source_contract SOURCE_CONTRACT_ID \
  --asset ASSET_ADDRESS \
  --amount AMOUNT
```

### 3. Verification
After the transaction is confirmed, you can verify the migration status.

```bash
stellar contract invoke \
  --id MIGRATION_HUB_ID \
  -- verify_migration \
  --migration_id ID
```

## Migration Analytics
Protocol admins can track migration success and volume using `get_analytics`.

## Security
- **Auth**: Every migration requires the user's explicit authorization.
- **Rate Limiting**: To ensure protocol stability, migrations are rate-limited per ledger.
- **Deadlines**: Specific migration programs may have deadlines.
