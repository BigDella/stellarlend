# Hello-World Contract (StellarLend Core)

This contract exposes the current public API for the main StellarLend protocol contract.

## Key Entry Points

- Initialization: `initialize(admin)`, `transfer_admin(caller, new_admin)`
- Governance setup: `gov_initialize(...)`
- Core lending: `deposit_collateral`, `borrow_asset`, `repay_debt`, `withdraw_collateral`, `liquidate`
- Risk controls: `set_risk_params`, `set_emergency_pause`, `can_be_liquidated`, `get_max_liquidatable_amount`, `get_liquidation_incentive_amount`, `require_min_collateral_ratio`
- Flash loans: `execute_flash_loan`, `repay_flash_loan`, `configure_flash_loan`
- MEV protection: `commit_*_protected`, `commit_*_guarded`, `open_liq_batch_auction`, `submit_liq_batch_bid`, `settle_liq_batch_auction`, `register_private_mev_route`, `record_mev_gas_bid`, `get_mev_dashboard`
- Treasury: `set_treasury`, `get_treasury`, `get_reserve_balance`, `claim_reserves`, `set_fee_config`, `get_fee_config`
- Analytics and queries: `get_protocol_stats`, `get_protocol_report`, `get_user_position`, `get_user_report`, `get_recent_activity`, `get_user_asset_collateral`, `get_user_asset_list`, `get_user_total_collateral_value`, `get_health_factor`
- Asset configuration: `update_asset_config`

Current note:

- `HelloContract` does not currently expose dedicated upgrade entrypoints from `src/lib.rs`.
- The separate upgrade approval manager documented in [`../../../docs/upgrade-mechanism.md`](../../../docs/upgrade-mechanism.md) lives under `stellar-lend/contracts/lending`.

Refer to `src/lib.rs` for the current public entrypoints and to [`../../../docs/event-indexing.md`](../../../docs/event-indexing.md) for event topics and indexing guidance.

