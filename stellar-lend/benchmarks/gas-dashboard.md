# Gas Benchmark Dashboard

This dashboard documents how to read and maintain StellarLend gas benchmark results.

## CI gate

The workflow at `.github/workflows/gas-benchmarks.yml` runs the benchmark binary, uploads `benchmark-results.json`, and fails the build when an operation exceeds its configured gas budget. A practical regression policy is:

- fail when any function exceeds its explicit budget;
- review any operation whose CPU instruction cost increases by more than 10% from the committed baseline;
- update `benchmarks/baseline.json` only after an intentional optimization or feature change is reviewed.

## Result fields

Each benchmark result should include:

- `contract`
- `operation`
- `instructions`
- `memory_bytes`
- `budget`
- `within_budget`
- `storage_reads`
- `storage_writes`
- `cross_contract_calls`

## Optimization checklist

- Prefer packed storage records over many small keys when values are read together.
- Cache config records loaded more than once inside the same entrypoint.
- Avoid repeated cross-contract calls in loops.
- Emit compact events and avoid duplicating data already present in storage.
- Add a focused benchmark before and after any storage-layout change.

## Storage slot analysis

When a benchmark regresses, inspect whether the operation added persistent keys, duplicate reads, or larger serialized values. Cross-contract accounting should be called out separately because token/oracle calls can dominate protocol-level gas.
