//! Optional utilization-based risk alerts for indexers and off-chain monitoring.
//!
//! Emits events when protocol utilization crosses configured thresholds (deduplicated by tier
//! to reduce spam). Thresholds are admin-configured; disabled when unset (all zeros).

use soroban_sdk::{contracterror, contracttype, Address, Env};

use crate::borrow::get_admin;
use crate::events::RiskAlertSeverity;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RiskAlertThresholds {
    /// Emit warning when utilization (total_debt / debt_ceiling in bps) reaches this level.
    pub warning_utilization_bps: u32,
    pub critical_utilization_bps: u32,
    pub emergency_utilization_bps: u32,
}

#[contracttype]
#[derive(Clone)]
pub(crate) enum RiskMonitorKey {
    Thresholds,
    /// Highest severity tier already emitted for the current excursion (0 = none).
    LastEmittedTier,
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RiskMonitorError {
    Unauthorized = 1,
    InvalidThresholds = 2,
}

fn severity_discriminant(s: RiskAlertSeverity) -> u32 {
    match s {
        RiskAlertSeverity::Warning => 1,
        RiskAlertSeverity::Critical => 2,
        RiskAlertSeverity::Emergency => 3,
    }
}

/// Called after total debt is updated following a borrow.
pub fn on_utilization_changed(env: &Env, total_debt: i128, debt_ceiling: i128) {
    if debt_ceiling <= 0 {
        return;
    }

    let Some(th) = env
        .storage()
        .persistent()
        .get::<RiskMonitorKey, RiskAlertThresholds>(&RiskMonitorKey::Thresholds)
    else {
        return;
    };

    if th.warning_utilization_bps == 0 {
        return;
    }

    let util_bps_u128 = total_debt
        .saturating_mul(10000)
        .saturating_div(debt_ceiling)
        .max(0) as u128;
    let util_bps = util_bps_u128.min(u128::from(u32::MAX)) as u32;

    // Reset latch when utilization falls back below warning band.
    if util_bps < th.warning_utilization_bps {
        env.storage()
            .persistent()
            .set(&RiskMonitorKey::LastEmittedTier, &0u32);
        return;
    }

    let tier = severity_for_util(util_bps, &th);

    let prev: u32 = env
        .storage()
        .persistent()
        .get(&RiskMonitorKey::LastEmittedTier)
        .unwrap_or(0);

    let tier_u32 = severity_discriminant(tier);

    // Emit only on escalation vs last emitted tier (aggregation / anti-spam).
    if tier_u32 > prev {
        crate::events::RiskUtilizationAlertEvent {
            severity: severity_discriminant(tier),
            utilization_bps: util_bps,
            total_debt,
            debt_ceiling,
            timestamp: env.ledger().timestamp(),
        }
        .publish(env);

        env.storage()
            .persistent()
            .set(&RiskMonitorKey::LastEmittedTier, &tier_u32);
    }
}

fn severity_for_util(util_bps: u32, th: &RiskAlertThresholds) -> RiskAlertSeverity {
    if th.emergency_utilization_bps > 0 && util_bps >= th.emergency_utilization_bps {
        RiskAlertSeverity::Emergency
    } else if th.critical_utilization_bps > 0 && util_bps >= th.critical_utilization_bps {
        RiskAlertSeverity::Critical
    } else {
        RiskAlertSeverity::Warning
    }
}

/// Admin: configure utilization alert thresholds (use 0 to disable critical/emergency tiers).
pub fn set_risk_alert_thresholds(
    env: &Env,
    admin: Address,
    thresholds: RiskAlertThresholds,
) -> Result<(), RiskMonitorError> {
    admin.require_auth();
    let stored = get_admin(env).ok_or(RiskMonitorError::Unauthorized)?;
    if admin != stored {
        return Err(RiskMonitorError::Unauthorized);
    }

    if thresholds.warning_utilization_bps == 0 {
        return Err(RiskMonitorError::InvalidThresholds);
    }

    if thresholds.critical_utilization_bps > 0
        && thresholds.critical_utilization_bps < thresholds.warning_utilization_bps
    {
        return Err(RiskMonitorError::InvalidThresholds);
    }

    if thresholds.emergency_utilization_bps > 0 {
        if thresholds.critical_utilization_bps == 0 {
            return Err(RiskMonitorError::InvalidThresholds);
        }
        if thresholds.emergency_utilization_bps < thresholds.critical_utilization_bps {
            return Err(RiskMonitorError::InvalidThresholds);
        }
    }

    env.storage()
        .persistent()
        .set(&RiskMonitorKey::Thresholds, &thresholds);
    env.storage()
        .persistent()
        .set(&RiskMonitorKey::LastEmittedTier, &0u32);

    Ok(())
}

pub fn get_risk_alert_thresholds(env: &Env) -> Option<RiskAlertThresholds> {
    env.storage().persistent().get(&RiskMonitorKey::Thresholds)
}
