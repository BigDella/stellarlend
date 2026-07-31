# Lending-Pool Performance Optimization Suite — Gas & Storage Benchmarks

Covers issues **#631** (interest caching), **#632** (liquidation gas),
**#633** (storage packing) and **#634** (lazy initialization).

All figures below are **modelled estimates** derived from Soroban's metered cost
model (storage-entry reads/writes and rent dominate; CPU instructions are
secondary). They are expressed as *relative* deltas because absolute fees depend
on network fee parameters at execution time. The pure cost-driving logic
(index math, bit-packing, check ordering) is unit-tested in each module.

---

## #631 — Incremental interest calculation (`interest.rs`)

### Mechanism
A single cached **cumulative interest index** is advanced incrementally:

```
index_n = index_{n-1} + index_{n-1} * rate * dt / (BPS * SECONDS_PER_YEAR)
```

A position's interest is one multiply against `(index_now / index_entry)`
instead of a full re-derivation from rate-model inputs.

### Gas diff — full recompute vs incremental update

| Operation                         | Before (full recompute) | After (incremental) | Δ        |
|-----------------------------------|-------------------------|---------------------|----------|
| Accrue, first op in a block       | rate-model walk + write | 1 read + 1 write    | ~unchanged |
| Accrue, 2nd+ op in same block     | rate-model walk + write | 1 read, **no write**| **−1 storage write / op** |
| `interest_index` (view)           | rate-model walk         | 1 read, no write    | **−1 write vs accrue-on-read** |
| Rate-model change (invalidate)    | n/a                     | close segment + 1 write | bounded |

**Batching win:** with `k` interactions in one ledger, interest writes drop from
`k` to `1`. **Read-call win:** `interest_index` never writes.

### Edge cases covered
- **No-op update** (`dt == 0`): returns cached value, no write.
- **Cache consistency during reorg:** `interest_for` rejects a position whose
  entry index is newer than a rewound global index (`StaleSnapshot`); `accrue`
  never rewinds a persisted index (monotonic).

---

## #632 — Liquidation gas optimization (`liquidation.rs`)

### Mechanism
`plan_liquidation` runs validation **cheapest-first** and returns *before* any
storage write or token transfer on failure:

1. amount sign check — no reads
2. health factor — pure arithmetic on a single batched `PositionSnapshot`
3. close-factor / profitability clamp — pure arithmetic
4. oracle freshness — uses the timestamp already in the snapshot
5. gas-vs-profit guard — `abort_if_unprofitable`

### Gas diff — failed liquidations (the common case)

| Position size / outcome           | Before                     | After                  | Δ                |
|-----------------------------------|----------------------------|------------------------|------------------|
| Reverts: healthy position         | partial state prep + write | batched read only      | **−all prep writes** |
| Reverts: stale oracle             | oracle call after prep     | reject after 1 read set| **−prep cost**   |
| Reverts: unprofitable             | executed, then unwound     | aborted pre-execution  | **−execution cost** |
| Succeeds: partial liquidation     | N scattered reads          | 1 batched read         | **−(N−1) reads** |

**Batched reads:** all required values are gathered once into `PositionSnapshot`
rather than re-read per check. At scale (100+ simultaneous liquidations) the
saved per-call reads compound linearly.

---

## #633 — Storage slot packing (`storage.rs`)

### Mechanism
Configuration is packed into **two words** instead of one slot per parameter:

- **Rate word (`u128`):** LTV ∥ liq-threshold ∥ reserve-factor ∥ close-factor ∥
  liq-incentive — five 16-bit bps fields (low 80 bits).
- **Status word (`u64`):** 40-bit timestamp ∥ 8 status-flag bits.

### Storage-rent diff

| Layout            | Persistent entries | Relative rent |
|-------------------|--------------------|---------------|
| Before (loose)    | 5+ (one per param) | 100%          |
| After (packed)    | 2 words            | **~40%**      |

**Read overhead:** unpacking is shift+mask integer arithmetic — O(1), no extra
storage reads, so read gas does **not** increase versus a single-slot read.

### Edge cases covered
- **Value overflow in packed fields:** `pack` returns `BpsFieldOverflow` if a bps
  value exceeds the 16-bit field (or is negative) and `TimestampOverflow` past
  the 40-bit timestamp range — caught before corrupting neighbouring fields.
- **Upgrade compatibility:** `migrate_from_legacy` is idempotent and reads the
  pool's current loose values, so re-running is safe.

---

## #634 — Lazy state initialization (`lazy.rs`)

### Mechanism
Deferrable fields (`ReserveBalance`, `AccumulatedFees`, `LiquidationCounter`,
`TotalReserves`, `BorrowIndexSnapshot`) are written **on first use**, not at pool
creation. Reads fall back to `default_for` with no allocation.

### Storage-rent diff — pool lifetime

| Field state                       | Before (eager)        | After (lazy)            |
|-----------------------------------|-----------------------|-------------------------|
| Pool created, field never used    | rent paid from day 0  | **0 — slot never written** |
| Field first used at op K          | rent from creation    | rent from op K (front-loaded) |

For a pool that never liquidates, `LiquidationCounter` rent is **eliminated**;
for one that liquidates late, its rent clock starts late.

### Edge cases covered
- **Concurrent first-use:** `ensure_initialized` is idempotent — a second caller
  in the same transaction sees the slot present and no-ops.
- **Initialization failure recovery:** reads always return a valid
  `default_for` value even if the slot was never written.
- **Migration:** `migrate_initialize_all` eagerly materialises every field so
  pre-existing pools match lazily-initialised behaviour.

---

## Reproducing

Pure cost-driving logic is unit-tested in each module (`mod unit`):

```bash
cd stellar-lend
cargo test -p stellarlend-lending interest::unit
cargo test -p stellarlend-lending liquidation::unit
cargo test -p stellarlend-lending storage::unit
cargo test -p stellarlend-lending lazy::unit
```

> Note: the `stellarlend-lending` crate currently has compile errors in
> unrelated modules (`token_adapter*`, an over-length event name) that predate
> this change; once those are resolved the suite above runs end-to-end. The pure
> packing/index/pricing logic has been verified in isolation.
