# Zero-Amount And Dust Semantics

This document defines the amount-handling rules added for issue #380.

## Amount Rules

The lending contract rejects all zero or negative amount-bearing operations:

| Operation | Rule |
| --- | --- |
| Deposit | `amount <= 0` is rejected with `DepositError::InvalidAmount` |
| Borrow | `amount <= 0` or `collateral_amount <= 0` is rejected with `BorrowError::InvalidAmount` |
| Repay | `amount <= 0` is rejected with `BorrowError::InvalidAmount` |
| Withdraw | `amount <= 0` is rejected with `WithdrawError::InvalidAmount` |

Configured minimum amounts are treated as dust thresholds. A positive amount
below the relevant minimum is dust and is rejected before any state mutation.

## Dust Prevention

Deposits, borrows, and withdrawals already have configured minimum sizes.
The implementation now also prevents withdrawals and repayments from leaving
small residual balances:

- A withdrawal that would leave a non-zero deposit balance below
  `MinWithdrawAmount` is rejected with `WithdrawError::DustAmount`.
- A repayment that would leave a non-zero debt balance below
  `BorrowMinAmount` is rejected with `BorrowError::DustAmount`.

These checks are constant-time comparisons and run after arithmetic validation
but before saving updated state.

## Dust Sweep

Users can clear existing dust that may have been created before a policy
change, migration, or manual recovery:

- `sweep_deposit_dust(user, asset)` withdraws the user's full deposit balance
  only when it is positive and below `MinWithdrawAmount`.
- `sweep_debt_dust(user, asset)` repays the user's full debt balance only when
  it is positive and below `BorrowMinAmount`.

Both sweep functions reject non-dust balances with the same `DustAmount` error
so they cannot be used to bypass normal minimum transaction sizes.

## Rounding Direction

Interest accrual uses depositor-friendly ceiling division: if a positive
interest calculation has any remainder, it rounds up by one unit instead of
rounding down to zero. Zero elapsed time and zero principal still accrue zero
interest.
