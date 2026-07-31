# Test Utilities

Shared test utilities for StellarLend smart contracts.

## Overview

This crate provides common testing infrastructure to reduce code duplication across contract tests and ensure consistent testing patterns.

## Modules

### Environment (`environment.rs`)

Test environment setup helpers:

```rust
use test_utils::TestEnv;

let mut test_env = TestEnv::new()
    .with_timestamp(1000)
    .with_ledger_sequence(1);

let user = test_env.generate_user();
test_env.advance_time(3600);
```

Helper functions:
- `setup_test_env()` - Basic env with admin
- `setup_test_env_with_users(count)` - Env with multiple users
- `create_string(env, value)` - Create Soroban strings

### Mock Contracts (`mock_contracts.rs`)

Pre-built mock implementations:

```rust
use test_utils::{register_mock_token, register_mock_oracle};

let token_address = register_mock_token(&env);
let oracle_address = register_mock_oracle(&env);
```

Available mocks:
- `MockToken` - Basic token with mint/burn/transfer
- `MockOracle` - Price oracle with configurable prices

### Assertions (`assertions.rs`)

Domain-specific assertion helpers:

```rust
use test_utils::*;

assert_non_negative(balance, "User balance");
assert_approximately_equal(actual, expected, 100, "Interest calculation");
assert_in_range(ltv, 0, 10_000, "LTV ratio");
```

Available assertions:
- `assert_non_negative` / `assert_positive`
- `assert_in_range`
- `assert_balance_non_negative`
- `assert_balances_equal`
- `assert_approximately_equal`
- `assert_percentage_in_range`
- `assert_greater_than` / `assert_less_than`
- `assert_zero` / `assert_not_zero`

### Fixtures (`fixtures.rs`)

Common test constants and builders:

```rust
use test_utils::{AmountFixtures, AssetConfigFixture};

let amount = AmountFixtures::MEDIUM;

let config = AssetConfigFixture::new()
    .with_collateral_factor(8000)
    .with_price(2_000_000);
```

Available fixtures:
- Amount constants: `MIN_AMOUNT`, `MAX_AMOUNT`, `LARGE_CEILING`
- Time constants: `TimeFixtures::HOUR`, `TimeFixtures::DAY`
- Rate constants: `RateFixtures::FIVE_PERCENT`
- `AssetConfigFixture` - Builder for asset configurations

## Usage

Add to your contract's `Cargo.toml`:

```toml
[dev-dependencies]
test-utils = { path = "../test-utils" }
```

Import in tests:

```rust
#[cfg(test)]
mod tests {
    use test_utils::*;
    
    #[test]
    fn test_example() {
        let mut test_env = TestEnv::new();
        let user = test_env.generate_user();
        
        // Your test logic
    }
}
```

## Design Principles

1. **Zero Dependencies**: Only depends on `soroban-sdk`
2. **Flexible**: Composable helpers that don't impose rigid patterns
3. **Clear**: Self-documenting names and comprehensive error messages
4. **Minimal**: Only includes truly common patterns
5. **Type-Safe**: Leverages Rust's type system for correctness

## Contributing

When adding new utilities:

1. Ensure the pattern is used in at least 3 different contracts
2. Add comprehensive documentation with examples
3. Keep functions focused and composable
4. Maintain backward compatibility
