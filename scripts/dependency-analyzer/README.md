# Contract Dependency Analyzer (#496)

Maps cross-contract dependencies across the Rust/Soroban workspace and reports
the blast radius of upgrading a given contract.

## Usage

```bash
# Build the graph (JSON + Graphviz DOT)
node --experimental-strip-types scripts/dependency-analyzer/index.ts \
  --root stellar-lend --out graph.json --dot graph.dot

# Render the DOT (optional)
dot -Tsvg graph.dot -o graph.svg

# Upgrade-impact report for one contract
node --experimental-strip-types scripts/dependency-analyzer/index.ts \
  --root stellar-lend --impact oracle
```

Each crate (`Cargo.toml` `[package].name`) is a node. Edges are detected as:
- **call** — a generated cross-contract client (`<Contract>Client`)
- **import** — `use <crate>` / `mod <crate>` / `<crate>::`
- **library** — workspace crate listed in `[dependencies]`

## Impact analysis

`--impact <contract>` returns the direct and transitive **dependents** (what an
upgrade affects) plus a `riskLevel` derived from the blast radius. The engine
also detects dependency cycles.

## Tests

```bash
node --experimental-strip-types --test scripts/dependency-analyzer/analyzer.test.ts
```

## Covered acceptance criteria

- Parse contract sources + imports to build a dependency graph.
- Detect direct calls, imports, and library (crate) links.
- Impact analysis: which contracts are affected by upgrading a specific contract.
- Cycle detection.
- Output: Graphviz DOT + JSON; per-contract upgrade-impact report with risk level.

## Follow-ups (out of this PR's core)

- Interactive web visualization (`web/dashboard/dependency-graph/`, D3.js/vis.js).
- Changed-interface / storage-slot impact and test-impact mapping.
- CI integration to auto-generate the graph on PRs.
