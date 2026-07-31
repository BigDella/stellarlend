# ABI Extractor

Automated contract ABI extraction, type generation, and versioning for StellarLend Soroban smart contracts.

## Overview

This toolchain extracts ABI (Application Binary Interface) metadata from compiled Soroban WASM artifacts, generates type-safe client bindings (TypeScript and Rust), and manages semantic versioning based on ABI changes.

## Scripts

### `extract.sh`

Builds contracts and extracts ABI/metadata from compiled `.wasm` artifacts.

```bash
# Extract all contract ABIs (builds first)
./scripts/abi-extractor/extract.sh

# Extract a specific contract only
./scripts/abi-extractor/extract.sh --contract lending-core

# Skip build, use existing WASM artifacts
./scripts/abi-extractor/extract.sh --skip-build

# Custom output directory
./scripts/abi-extractor/extract.sh --output ./my-abis
```

Output: JSON ABI files in `packages/contract-abis/abi/`, one per contract.

### `generate-types.js`

Reads extracted ABI JSON files and generates typed client bindings.

```bash
# Generate TypeScript types
node scripts/abi-extractor/generate-types.js

# Also generate Rust client types
node scripts/abi-extractor/generate-types.js --rust

# Specific contract only
node scripts/abi-extractor/generate-types.js --contract lending-core

# Custom directories
node scripts/abi-extractor/generate-types.js --abi-dir ./my-abis --out-dir ./my-types
```

Output:
- `packages/contract-abis/src/types/<contract>.ts` — TypeScript definitions
- `packages/contract-abis/src/rust/<contract>.rs` — Rust client types (with `--rust`)
- `packages/contract-abis/src/index.ts` — Barrel re-export

### `version.sh`

Compares current ABIs against a baseline, detects breaking vs non-breaking changes, and bumps the version in `package.json` accordingly.

```bash
# Run versioning (compares & bumps)
./scripts/abi-extractor/version.sh

# Dry run — show what would change without modifying files
./scripts/abi-extractor/version.sh --dry-run
```

Version bump rules:
- **Major** — Breaking changes (removed functions, removed types, removed contracts)
- **Minor** — Additive changes (new functions, new types, new contracts)
- **Patch** — Implementation changes (WASM hash changed, no structural ABI diff)

### `ci-check.sh`

CI integration script that runs the full pipeline and reports results.

```bash
# Basic CI check
./scripts/abi-extractor/ci-check.sh

# Fail CI if breaking changes detected
./scripts/abi-extractor/ci-check.sh --fail-on-breaking

# Full pipeline with type generation
./scripts/abi-extractor/ci-check.sh --generate-types --generate-rust

# Skip build (assumes WASM artifacts exist)
./scripts/abi-extractor/ci-check.sh --skip-build

# Write summary to a file (for PR comments)
./scripts/abi-extractor/ci-check.sh --output-file abi-report.md
```

GitHub Actions integration: the script automatically writes to `GITHUB_STEP_SUMMARY` and `GITHUB_OUTPUT` when those environment variables are set.

## Full Workflow

```bash
# 1. Extract ABIs from contracts
./scripts/abi-extractor/extract.sh

# 2. Generate client types
node scripts/abi-extractor/generate-types.js --rust

# 3. Version and changelog
./scripts/abi-extractor/version.sh
```

Or use the CI script which combines all steps:

```bash
./scripts/abi-extractor/ci-check.sh --generate-types --generate-rust --fail-on-breaking
```

## Output Structure

```
packages/contract-abis/
  abi/                     # Raw ABI JSON files
    lending-core.json
    amm.json
    ...
    .baseline/             # Previous version ABIs (for comparison)
  src/
    index.ts               # Barrel re-export
    types/                 # TypeScript definitions
      lending-core.ts
      amm.ts
      ...
    rust/                  # Rust client types (optional)
      lending_core.rs
      amm.rs
      mod.rs
  package.json
  tsconfig.json
  CHANGELOG.md
```

## ABI JSON Format

Each extracted ABI file contains:

```json
{
  "contract_name": "lending-core",
  "version": "20240115",
  "wasm_hash": "abc123...",
  "wasm_size": 12345,
  "extracted_at": "2024-01-15T10:30:00Z",
  "wasm_artifact": "lending_core.optimized.wasm",
  "spec": [ ... ]
}
```

The `spec` field contains the Soroban contract specification, including function signatures, struct definitions, enum types, and error codes — as output by the Stellar CLI.

## Requirements

- Rust toolchain with `wasm32-unknown-unknown` target
- Stellar CLI >= v21
- Node.js >= 18 (for type generation)
- `jq` (optional, for detailed ABI diffing in `version.sh`)

## Supported Contracts

The extractor processes all deployable contracts (those with `crate-type = ["cdylib"]`):

- amm
- bridge
- delegation-registry
- hello-world
- lending
- lending-core
- lending-interest
- lending-risk
- stablecoin
- institutional-wallet
- migration-hub
- stealth-address
- privacy-pool

Library crates (common, lending-types, test-utils) are excluded as they do not produce deployable WASM artifacts.
