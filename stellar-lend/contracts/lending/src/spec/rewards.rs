//! # Formal Specification — Rewards Distribution
//!
//! ## Mathematical model
//!
//! The rewards system uses an index-based distribution:
//!
//! ```text
//! global_index += (emission_rate * Δt * SCALAR) / total_liquidity
//! user_accrued  += balance * (global_index - user_index) / SCALAR
//! ```
//!
//! ## Properties
//!
//! | ID   | Property                                                             |
//! |------|----------------------------------------------------------------------|
//! | R-01 | Global index is monotonically non-decreasing                         |
//! | R-02 | Zero total liquidity: global index unchanged                         |
//! | R-03 | Zero emission rate: global index unchanged                           |
//! | R-04 | User accrual is non-negative for non-negative balance and index delta|
//! | R-05 | Same-timestamp update: global index unchanged (idempotent)           |
//! | R-06 | Claim zeroes user accrued rewards                                    |
//! | R-07 | No overflow for realistic emission rates and time deltas             |

pub const SCALAR: i128 = 1_000_000_000;

#[derive(Clone, Debug)]
pub struct RewardsState {
    pub global_index: i128,
    pub last_update: i128,
    pub emission_rate: i128,
    pub total_liquidity: i128,
    pub user_index: i128,
    pub user_accrued: i128,
}

impl RewardsState {
    pub fn new(emission_rate: i128, total_liquidity: i128) -> Self {
        Self {
            global_index: 0,
            last_update: 0,
            emission_rate,
            total_liquidity,
            user_index: 0,
            user_accrued: 0,
        }
    }

    /// Update global index to `now`.
    pub fn update_global(&mut self, now: i128) {
        if now == self.last_update {
            return;
        }
        if self.total_liquidity == 0 {
            self.last_update = now;
            return;
        }
        let delta = now - self.last_update;
        self.global_index += (self.emission_rate * delta * SCALAR) / self.total_liquidity;
        self.last_update = now;
    }

    /// Update user with current balance at `now`.
    pub fn update_user(&mut self, now: i128, balance: i128) {
        self.update_global(now);
        let delta = self.global_index - self.user_index;
        self.user_accrued += (balance * delta) / SCALAR;
        self.user_index = self.global_index;
    }

    /// Claim all accrued rewards.
    pub fn claim(&mut self) -> i128 {
        let rewards = self.user_accrued;
        self.user_accrued = 0;
        rewards
    }
}

/// **R-01**: Global index is non-decreasing.
#[test]
fn lemma_r01_global_index_non_decreasing() {
    let mut state = RewardsState::new(1_000, 1_000_000);
    let times: &[i128] = &[0, 1, 60, 3600, 86_400, 31_536_000];
    let mut prev = state.global_index;
    for &t in times {
        state.update_global(t);
        assert!(state.global_index >= prev, "R-01: index decreased at t={t}");
        prev = state.global_index;
    }
}

/// **R-02**: Zero total liquidity keeps global index unchanged.
#[test]
fn lemma_r02_zero_liquidity_index_unchanged() {
    let mut state = RewardsState::new(1_000, 0);
    let initial_index = state.global_index;
    state.update_global(3600);
    assert_eq!(
        state.global_index, initial_index,
        "R-02: index changed with zero liquidity"
    );
}

/// **R-03**: Zero emission rate keeps global index unchanged.
#[test]
fn lemma_r03_zero_emission_rate_index_unchanged() {
    let mut state = RewardsState::new(0, 1_000_000);
    let initial_index = state.global_index;
    state.update_global(86_400);
    assert_eq!(
        state.global_index, initial_index,
        "R-03: index changed with zero emission"
    );
}

/// **R-04**: User accrual is non-negative for non-negative balance and positive index delta.
#[test]
fn lemma_r04_user_accrual_is_non_negative() {
    let mut state = RewardsState::new(500, 1_000_000);
    state.update_global(1000); // Advance global index
    let balance = 100_000i128;
    state.update_user(1000, balance);
    assert!(
        state.user_accrued >= 0,
        "R-04: negative user accrual={}",
        state.user_accrued
    );
}

/// **R-05**: Same-timestamp update is idempotent (global index unchanged).
#[test]
fn lemma_r05_same_timestamp_is_idempotent() {
    let mut state = RewardsState::new(1_000, 1_000_000);
    state.update_global(3600);
    let index_after_first = state.global_index;
    state.update_global(3600); // Same timestamp
    assert_eq!(
        state.global_index, index_after_first,
        "R-05: second update at same time changed index"
    );
}

/// **R-06**: Claim zeroes user accrued rewards.
#[test]
fn lemma_r06_claim_zeroes_accrued() {
    let mut state = RewardsState::new(500, 1_000_000);
    state.update_user(10_000, 500_000);
    state.claim();
    assert_eq!(
        state.user_accrued, 0,
        "R-06: accrued not zeroed after claim"
    );
}

/// **R-07**: No overflow for realistic emission and time values.
#[test]
fn lemma_r07_no_overflow_on_realistic_values() {
    // emission=1e12, total_liquidity=1e18, delta=86_400 (1 day)
    let emission = 1_000_000_000_000i128;
    let liquidity = 1_000_000_000_000_000_000i128;
    let mut state = RewardsState::new(emission, liquidity);
    state.update_global(86_400);
    // Should complete without panic
    assert!(state.global_index >= 0, "R-07: negative index");
}
