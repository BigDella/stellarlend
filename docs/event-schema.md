# StellarLend — Standardized Event Schema

All smart-contract events in the StellarLend workspace follow a single set of
conventions so that indexers, block explorers, and off-chain consumers can
process them consistently without per-contract logic.

---

## 1. Mandatory fields

Every event struct **must** include the following field:

| Field | Type | Description |
|-------|------|-------------|
| `timestamp` | `u64` | `env.ledger().timestamp()` at the time of emission (ledger seconds since Unix epoch) |

Where an event is associated with a user action, the struct **should** also include:

| Field | Type | Description |
|-------|------|-------------|
| `user` / `actor` / `caller` | `Address` | The initiating party |

---

## 2. Topic naming convention

Every `#[contractevent]` **must** declare an explicit `topics` list using a
short `snake_case` identifier (≤ 12 chars). The Soroban SDK default (struct
name in camel-case) must **not** be used without an override.

```
#[contractevent(topics = ["br_dep"])]
pub struct BridgeDepositEvent { … }
```

Topic prefixes by contract:

| Contract | Prefix |
|----------|--------|
| Lending  | `lend_` |
| AMM      | `amm_` |
| Bridge   | `br_` |
| Upgrade manager | `up_` |
| Data store | `ds_` |

---

## 3. Per-contract event catalogue

### 3.1 Lending contract

| Event struct | Topic | Key fields |
|---|---|---|
| `BorrowEvent` | `borrow_event` (default) | user, asset, amount, collateral, timestamp |
| `VaultDepositEvent` | `deposit_event` | user, asset, amount, new_balance, timestamp |
| `BorrowCollateralDepositEvent` | `deposit_event` | user, asset, amount, timestamp |
| `RepayEvent` | `repay_event` (default) | user, asset, amount, timestamp |
| `WithdrawEvent` | `withdraw_event` (default) | user, asset, amount, remaining_balance, timestamp |
| `FlashLoanEvent` | `flash_loan_event` (default) | receiver, asset, amount, fee, timestamp |
| `InterestRateModelUpdatedEvent` | `interest_rate_model_updated` | caller, previous, updated, timestamp |

### 3.2 Upgrade manager

| Event struct | Topic | Key fields |
|---|---|---|
| `UpgradeInitEvent` | `up_init` | admin, required_approvals |
| `UpgradeApproverAddedEvent` | `up_apadd` | caller, approver |
| `UpgradeApproverRemovedEvent` | `up_aprem` | caller, approver |
| `UpgradeProposedEvent` | `up_prop` | caller, id, new_version |
| `UpgradeApprovalRecordedEvent` | `up_appr` | caller, proposal_id, approval_count |
| `UpgradeTimelockQueuedEvent` | `up_tlock` | caller, proposal_id, execute_after, is_emergency |
| `UpgradeEmergencyProposedEvent` | `up_emrg` | caller, id, new_version, execute_after |
| `UpgradeExecutedEvent` | `up_exec` | caller, proposal_id, new_version |
| `UpgradeRollbackEvent` | `up_roll` | caller, proposal_id, prev_version |

### 3.3 AMM contract

| Event struct | Topic | Key fields |
|---|---|---|
| `SwapExecutedEvent` | `amm_swap` | user, protocol, amount_in, amount_out, effective_price, timestamp |
| `LiquidityAddedEvent` | `amm_liq_add` | user, protocol, amount_a, amount_b, lp_tokens, timestamp |
| `LiquidityRemovedEvent` | `amm_liq_rm` | user, protocol, lp_tokens, timestamp |
| `AmmOperationEvent` | `amm_op` | user, operation, amount_in, amount_out, timestamp |
| `CallbackValidatedEvent` | `amm_cb_valid` | caller, user, operation, nonce, timestamp |

### 3.4 Bridge contract

| Event struct | Topic | Key fields |
|---|---|---|
| `BridgeRegisteredEvent` | `br_reg` | bridge_id, fee_bps, min_amount, timestamp |
| `BridgeFeeUpdatedEvent` | `br_fee` | bridge_id, fee_bps, timestamp |
| `BridgeActiveUpdatedEvent` | `br_active` | bridge_id, active, timestamp |
| `BridgeDepositEvent` | `br_dep` | bridge_id, amount, fee, net, timestamp |
| `BridgeWithdrawalEvent` | `br_wdraw` | bridge_id, amount, timestamp |
| `BridgeAcceptancePauseEvent` | `br_pause` | paused, admin, timestamp |
| `ValidatorUpdatedEvent` | `br_val_upd` | validator, stake, active, timestamp |
| `SecurityConfigUpdatedEvent` | `br_sec_cfg` | (config fields), timestamp |
| `ValidatorSlashedEvent` | `br_slash` | validator, amount, remaining_stake, timestamp |
| `ChannelEmergencyCloseEvent` | `br_ch_emrg` | channel_id, closed, reason, timestamp |
| `BridgeAnomalyEvent` | `br_anomaly` | channel_id, anomaly_count, reason, timestamp |

---

## 4. Backward-compatibility note

Events in section 3.1 that previously lacked an explicit `topics` attribute
retain their Soroban default topic (struct name in snake_case) for backward
compatibility with existing indexers. New events **must** declare explicit topics.

---

## 5. Checklist for new events

Before merging a PR that adds or modifies a contract event:

- [ ] `timestamp: u64` field present and populated with `env.ledger().timestamp()`
- [ ] Explicit `topics = ["…"]` attribute declared (prefix matches contract table above)
- [ ] Event struct registered in the catalogue table in this document
- [ ] CI script `scripts/check_event_schema.sh` passes (no missing timestamps)
