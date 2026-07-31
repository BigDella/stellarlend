# Interest Rate Models

The lending contract now stores the active interest rate model in `InterestRateConfig.model`.
Governance can switch the model with `update_interest_rate_model` while preserving the existing
configuration parameters.

## Models

| Model | Behavior |
| --- | --- |
| `Linear` | `base + utilization * slope` |
| `Kink` | Piecewise linear. Uses `slope` below `kink_utilization_bps` and `jump_slope` above it. |
| `Jump` | Linear slope across all utilization plus an additional jump slope above the kink. |
| `Exponential` | Quadratic/cubic integer approximation for markets that need sharper high-utilization pricing. |

All calculations use basis points and checked integer arithmetic. The final borrow rate is clamped
between `rate_floor_bps` and `rate_ceiling_bps`. The supply rate is derived from the borrow rate
minus `spread_bps`, then clamped to the floor.

## Migration Behavior

Variable-rate positions are migrated lazily because they read the current model on each accrual.
Stable-rate positions keep their stored `stable_rate_bps` snapshot, so switching models does not
rewrite existing stable debt. New stable borrows use the active model at the time of borrow.

## Benchmarking

The benchmark suite includes model-switch measurements for all four pre-built models:

- `lending::interest_rate_model_linear`
- `lending::interest_rate_model_kink`
- `lending::interest_rate_model_jump`
- `lending::interest_rate_model_exponential`
