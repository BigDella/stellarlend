# Storage Impact Analyzer

Automated PR analysis tool that detects breaking changes in Soroban contract storage layouts before they reach production.

## Why This Exists

Soroban smart contracts serialize storage data using XDR, which is **order-dependent**. Changing the order of fields in a `#[contracttype]` struct, removing a field, or reordering enum variants will silently break deserialization of existing on-chain data. This analyzer catches those problems at PR time.

## What It Detects

| Category | Details |
|----------|---------|
| **New storage variables** | New `env.storage().{instance,persistent,temporary}().set()` / `.get()` calls |
| **Removed storage variables** | Storage keys that existed in base but not in head |
| **Struct field changes** | Added, removed, reordered, or type-changed fields in `#[contracttype]` structs |
| **Enum variant changes** | Added, removed, or reordered variants in `#[contracttype]` key enums |
| **Key collisions** | Multiple contracts using the same string-literal key in the same storage tier |
| **Breaking changes** | Removed keys, removed/reordered fields, type changes, removed/reordered enum variants |

## Architecture

```
analyze.sh              Main entry point — orchestrates extraction and comparison
extract-storage.sh      Parses Rust source files, emits JSON storage layout
compare.js              Reads base/head JSON, computes diffs, outputs Markdown report
```

### Data Flow

```
┌──────────┐    extract-storage.sh    ┌──────────────┐
│ base ref │ ──────────────────────── │ base.json    │
└──────────┘                          └──────┬───────┘
                                             │
                                      compare.js ──── Markdown report (stdout)
                                             │
┌──────────┐    extract-storage.sh    ┌──────┴───────┐
│ head ref │ ──────────────────────── │ head.json    │
└──────────┘                          └──────────────┘
```

## Usage

### Local

```bash
# Make scripts executable (first time only)
chmod +x scripts/storage-analyzer/analyze.sh
chmod +x scripts/storage-analyzer/extract-storage.sh

# Run against two local checkouts
scripts/storage-analyzer/analyze.sh /path/to/base /path/to/head

# Or compare the current tree against main using git worktrees
git worktree add /tmp/stellarlend-base main
scripts/storage-analyzer/analyze.sh /tmp/stellarlend-base .
git worktree remove /tmp/stellarlend-base
```

The report is written to stdout. Exit code 0 means no breaking changes; exit code 1 means breaking changes were found.

### CI (GitHub Actions)

The workflow at `.github/workflows/storage-impact.yml` runs automatically on PRs targeting `main` that touch files under `stellar-lend/contracts/**/*.rs`. It:

1. Checks out both the base and head commits
2. Runs the analyzer
3. Posts (or updates) a Markdown comment on the PR with the impact report
4. Fails CI if breaking changes are detected

## JSON Schema

The intermediate JSON produced by `extract-storage.sh` has this shape:

```json
{
  "contracts": {
    "<contract-name>": {
      "accesses": [
        {
          "contract": "bridge",
          "file": "stellar-lend/contracts/bridge/src/bridge.rs",
          "tier": "instance",
          "operation": "set",
          "key": "DataKey::SecurityConfig",
          "key_kind": "enum_variant"
        }
      ],
      "types": [
        {
          "contract": "bridge",
          "file": "stellar-lend/contracts/bridge/src/bridge.rs",
          "kind": "struct",
          "name": "BridgeConfig",
          "fields": [
            { "name": "bridge_id", "type": "String" },
            { "name": "fee_bps", "type": "u64" }
          ]
        }
      ],
      "key_enums": [
        {
          "contract": "bridge",
          "kind": "enum",
          "name": "DataKey",
          "variants": [
            { "name": "Bridge", "params": "String" },
            { "name": "BridgeList" }
          ]
        }
      ]
    }
  }
}
```

## Breaking Change Rules

The analyzer flags these as breaking (CI will fail):

- **Removed storage key access** — a `set`/`get` call that existed in base is gone in head
- **Removed `#[contracttype]` struct** — existing on-chain data becomes unreadable
- **Removed struct field** — XDR deserialization will fail
- **Reordered struct fields** — XDR is position-based, reordering corrupts data
- **Field type change** — incompatible serialization
- **Removed enum variant** — existing stored keys become unresolvable
- **Reordered enum variants** — discriminant values shift, breaking key lookup
- **Enum variant param change** — stored key data becomes incompatible

## Requirements

- **Bash** (4.0+)
- **Node.js** (18+)
- Standard Unix tools: `grep`, `sed`, `tr`, `find`, `sort`
