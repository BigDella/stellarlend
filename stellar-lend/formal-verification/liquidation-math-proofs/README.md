# liquidation-math-proofs

Formal verification of `RiskManager::apply_liquidation` and the supporting
discount/conservation math added to `contracts/lending-risk` for this
effort.

This crate is **not** part of the main Cargo workspace (see the root
`Cargo.toml` `exclude` list), matching the existing
`formal-verification/safe-math-proofs` pattern.

## Running

```sh
# Bounded property tests (fast, run in normal CI, no extra tooling):
cargo test --manifest-path formal-verification/liquidation-math-proofs/Cargo.toml

# Exhaustive bounded model checking with Kani:
cargo install --locked kani-verifier
cargo kani setup
cargo kani --manifest-path formal-verification/liquidation-math-proofs/Cargo.toml

# SMT-LIB spec directly with Z3 (all five checks verified: unsat, unsat,
# unsat, unsat, sat):
z3 formal-verification/liquidation-math-proofs/liquidation_math.smt2
```

## Properties verified

1. **Liquidation discount bounds**: `0 <= bonus <= repay_amount` for any
   `bonus_bps` in `[0, 10_000]`.
2. **Conservation / no value leak**: collateral and debt removed from a
   position exactly match what the liquidator receives / what debt is
   cleared — nothing is silently created or destroyed.
3. **Exact liquidator payout**: `liquidator_profit` equals the configured
   discount precisely.
4. **The ratio condition for health-factor movement** (see below).
5. **No silent overflow/underflow**: `apply_liquidation` is total — always
   `Ok` with exactly the conserved values above, or `Err`, never a wrapped
   result.

## Key finding: liquidation is not unconditionally health-improving

The original brief for this work assumed liquidation always leaves a
position at least as healthy as before. **Kani (and the accompanying Z3
witness check) show that's false** for sufficiently under-collateralized
("bad debt") positions: the liquidator's bonus is carved out of collateral
that's already scarcer than the debt it backs, so the health factor can
*decrease* as a mechanical consequence of a "correct" liquidation call.

The crate instead proves the precise condition (`src/lib.rs` has the full
derivation and a concrete regression-pinned example:
`collateral=50_000, debt=90_000, repay=40_000, bonus_bps=500` moves the
health factor from `4444` to `1280`):

```text
collateral * repay >= debt * seized   =>   hf_after >= hf_before
collateral * repay <  debt * seized   =>   hf_after <= hf_before
```

Any keeper/liquidation-bot logic built on top of this math should account
for this — liquidating a position that's already below the
`1 + bonus_bps / 10_000` collateral/debt ratio does not restore its health
and should likely route to a different (bonus-free / bad-debt) handling
path instead. That routing logic is out of scope for `lending-risk` itself;
this crate's contribution is making the exact boundary provable rather than
assumed.
