# StellarLend Contract Test Framework

Unified fixtures and scenario helpers for contract test suites.

## Categories

- `Unit`: pure function and single-contract checks.
- `Integration`: multi-contract flows such as deposit -> borrow -> liquidate -> repay.
- `Fuzz`: randomized critical-function inputs and invariant smoke runs.
- `GasBenchmark`: per-operation gas snapshots with regression thresholds.

## Included Building Blocks

- `ContractFixture` and `FixtureBuilder` for reusable deployment/state setup.
- `Scenario` and `ScenarioRunner` for JSON/YAML-driven user journeys.
- `EdgeCaseCatalog` documenting expected failure modes per function.
- `GasMetrics` and `benchmark_operation` for gas diff reports.

## CI

The `.github/workflows/contract-integration-tests.yml` workflow runs framework
tests and the gas benchmark binary on every pull request that touches contract,
benchmark, or test-framework code.
