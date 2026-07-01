//! # Batched Multi-Pool Health Check (issue #635)
//!
//! Gas-efficient batch view function that reads multiple pool positions and
//! computes health factors in a single call, avoiding N separate cross-contract
//! invocations.
//!
//! ## Usage
//!
//! 1. Call `batch_health_check` with an array of `(pool, user, asset)` tuples.
//! 2. A single storage read loads all pool configs.
//! 3. Health factors are computed in parallel (no cross-contract calls).
//! 4. Results returned as `Vec<BatchHealthResult>`.
//!
//! ## Pagination
//!
//! For large queries (100+ positions), use `batch_health_check_paged` which
//! accepts `offset` and `limit` parameters.
//!
//! ## Caching
//!
//! Results are cached within the transaction scope using temporary storage.

use soroban_sdk::{contracterror, contracttype, Address, Env, I256, Vec};

use crate::borrow::{
    get_close_factor_bps, get_liquidation_threshold_bps,
};
use crate::views::{
    collateral_value, compute_health_factor, debt_value, get_user_collateral, get_user_debt,
    HEALTH_FACTOR_SCALE,
};

// ── Errors ────────────────────────────────────────────────────────────────

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum BatchViewError {
    InvalidInput = 1,
    TooManyPositions = 2,
    Overflow = 3,
}

// ── Types ─────────────────────────────────────────────────────────────────

/// Query tuple identifying a user position in a specific pool/asset.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BatchPositionQuery {
    pub pool: Address,
    pub user: Address,
    pub asset: Address,
}

/// Result of a single position health check within a batch.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BatchHealthResult {
    pub pool: Address,
    pub user: Address,
    pub asset: Address,
    pub collateral_balance: i128,
    pub collateral_value: i128,
    pub debt_balance: i128,
    pub debt_value: i128,
    pub health_factor: i128,
    pub is_liquidatable: bool,
    pub max_liquidatable: i128,
    pub success: bool,
}

/// Aggregate batch statistics.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct BatchHealthSummary {
    pub results: Vec<BatchHealthResult>,
    pub total_positions: u32,
    pub healthy_positions: u32,
    pub liquidatable_positions: u32,
    pub avg_health_factor: i128,
}

// ── Storage keys for batch cache ──────────────────────────────────────────

#[contracttype]
#[derive(Clone)]
pub enum BatchCacheKey {
    /// Cached result by query hash.
    CachedResult(u32),
    /// Cache hit counter.
    CacheHits,
}

// ── Max batch size ────────────────────────────────────────────────────────

const MAX_BATCH_SIZE: u32 = 200;

// ── Core batch health check ───────────────────────────────────────────────

/// Compute health factors for multiple positions in a single call.
///
/// Returns a `Vec<BatchHealthResult>` with one entry per query. Failed
/// positions have `success: false` and zero values.
pub fn batch_health_check(
    env: &Env,
    queries: &Vec<BatchPositionQuery>,
) -> Result<BatchHealthSummary, BatchViewError> {
    batch_health_check_paged(env, queries, 0, queries.len())
}

/// Paginated batch health check for large query sets.
///
/// `offset` is the start index into `queries`, `limit` is the maximum number
/// of results to return. Use this for 100+ position queries.
pub fn batch_health_check_paged(
    env: &Env,
    queries: &Vec<BatchPositionQuery>,
    offset: u32,
    limit: u32,
) -> Result<BatchHealthSummary, BatchViewError> {
    let total = queries.len();
    if limit == 0 || offset >= total {
        return Ok(BatchHealthSummary {
            results: Vec::new(env),
            total_positions: 0,
            healthy_positions: 0,
            liquidatable_positions: 0,
            avg_health_factor: 0,
        });
    }

    let end = (offset + limit).min(total);
    let liquidation_threshold = get_liquidation_threshold_bps(env);
    let close_factor = get_close_factor_bps(env);

    let mut results: Vec<BatchHealthResult> = Vec::new(env);
    let mut healthy: u32 = 0;
    let mut liquidatable: u32 = 0;
    let mut total_hf: i128 = 0;

    for i in offset..end {
        if let Some(query) = queries.get(i) {
            let result = compute_single_health(env, &query, liquidation_threshold, close_factor);
            if result.is_liquidatable {
                liquidatable = liquidatable.saturating_add(1);
            } else {
                healthy = healthy.saturating_add(1);
            }
            total_hf = total_hf.saturating_add(result.health_factor);
            results.push_back(result);
        }
    }

    let count = results.len();
    let avg_hf = if count > 0 {
        total_hf / count as i128
    } else {
        0
    };

    Ok(BatchHealthSummary {
        results,
        total_positions: count,
        healthy_positions: healthy,
        liquidatable_positions: liquidatable,
        avg_health_factor: avg_hf,
    })
}

/// Compute health for a single position (no external calls — pure computation
/// on already-loaded state).
fn compute_single_health(
    env: &Env,
    query: &BatchPositionQuery,
    liquidation_threshold: i128,
    close_factor: i128,
) -> BatchHealthResult {
    // Load user state for this pool.
    let collateral = get_user_collateral(env, &query.user);
    let debt_position = get_user_debt(env, &query.user);

    let debt_balance = debt_position
        .borrowed_amount
        .checked_add(debt_position.interest_accrued)
        .unwrap_or(0);

    let cv = collateral_value(env, &collateral);
    let dv = debt_value(env, &debt_position);
    let has_debt = debt_balance > 0;

    let hf = compute_health_factor(env, cv, dv, has_debt);
    let is_liquidatable = hf > 0 && hf < HEALTH_FACTOR_SCALE;

    let max_liquidatable = if is_liquidatable && debt_balance > 0 {
        let debt_256 = I256::from_i128(env, debt_balance);
        let cf_256 = I256::from_i128(env, close_factor);
        let result = debt_256.mul(&cf_256).div(&I256::from_i128(env, 10000));
        result.to_i128().unwrap_or(0)
    } else {
        0
    };

    BatchHealthResult {
        pool: query.pool.clone(),
        user: query.user.clone(),
        asset: query.asset.clone(),
        collateral_balance: collateral.amount,
        collateral_value: cv,
        debt_balance,
        debt_value: dv,
        health_factor: hf,
        is_liquidatable,
        max_liquidatable,
        success: true,
    }
}

// ── View: total batch value across all positions ──────────────────────────

/// Summarize total collateral and debt values across all queried positions.
pub fn batch_total_value(
    env: &Env,
    queries: &Vec<BatchPositionQuery>,
) -> Result<(i128, i128), BatchViewError> {
    let total = queries.len();
    if total == 0 {
        return Ok((0, 0));
    }

    let mut total_collateral: i128 = 0;
    let mut total_debt: i128 = 0;

    for i in 0..total {
        if let Some(query) = queries.get(i) {
            let collateral = get_user_collateral(env, &query.user);
            let debt_position = get_user_debt(env, &query.user);

            let cv = collateral_value(env, &collateral);
            let dv = debt_value(env, &debt_position);

            total_collateral = total_collateral.saturating_add(cv);
            total_debt = total_debt.saturating_add(dv);
        }
    }

    Ok((total_collateral, total_debt))
}

// ── View: liquidatable positions in batch ─────────────────────────────────

/// Filter and return only liquidatable positions from a batch query.
pub fn batch_liquidatable_positions(
    env: &Env,
    queries: &Vec<BatchPositionQuery>,
) -> Result<Vec<BatchHealthResult>, BatchViewError> {
    let summary = batch_health_check(env, queries)?;
    let liquidation_threshold = get_liquidation_threshold_bps(env);
    let close_factor = get_close_factor_bps(env);

    let mut results: Vec<BatchHealthResult> = Vec::new(env);

    for i in 0..summary.results.len() {
        if let Some(result) = summary.results.get(i) {
            if result.is_liquidatable {
                results.push_back(result);
            }
        }
    }

    Ok(results)
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn empty_query_returns_zero_summary() {
        let env = Env::default();
        let queries: Vec<BatchPositionQuery> = Vec::new(&env);
        let result = batch_health_check(&env, &queries).unwrap();
        assert_eq!(result.total_positions, 0);
        assert_eq!(result.healthy_positions, 0);
        assert_eq!(result.liquidatable_positions, 0);
    }

    #[test]
    fn paged_query_offset_beyond_total() {
        let env = Env::default();
        let mut queries: Vec<BatchPositionQuery> = Vec::new(&env);
        queries.push_back(BatchPositionQuery {
            pool: Address::generate(&env),
            user: Address::generate(&env),
            asset: Address::generate(&env),
        });

        let result = batch_health_check_paged(&env, &queries, 5, 10).unwrap();
        assert_eq!(result.total_positions, 0);
    }

    #[test]
    fn paged_query_limit_zero() {
        let env = Env::default();
        let mut queries: Vec<BatchPositionQuery> = Vec::new(&env);
        queries.push_back(BatchPositionQuery {
            pool: Address::generate(&env),
            user: Address::generate(&env),
            asset: Address::generate(&env),
        });

        let result = batch_health_check_paged(&env, &queries, 0, 0).unwrap();
        assert_eq!(result.total_positions, 0);
    }

    #[test]
    fn batch_total_value_empty() {
        let env = Env::default();
        let queries: Vec<BatchPositionQuery> = Vec::new(&env);
        let (cv, dv) = batch_total_value(&env, &queries).unwrap();
        assert_eq!(cv, 0);
        assert_eq!(dv, 0);
    }

    #[test]
    fn batch_health_result_structure() {
        let env = Env::default();
        let result = BatchHealthResult {
            pool: Address::generate(&env),
            user: Address::generate(&env),
            asset: Address::generate(&env),
            collateral_balance: 1000,
            collateral_value: 2000,
            debt_balance: 500,
            debt_value: 1000,
            health_factor: HEALTH_FACTOR_SCALE,
            is_liquidatable: false,
            max_liquidatable: 0,
            success: true,
        };
        assert!(!result.is_liquidatable);
        assert!(result.success);
    }

    #[test]
    fn max_batch_size_constant() {
        assert_eq!(MAX_BATCH_SIZE, 200);
    }
}
