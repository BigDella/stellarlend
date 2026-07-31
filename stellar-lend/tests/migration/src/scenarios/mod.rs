pub mod boundary;
pub mod empty_state;
pub mod full_state;

use crate::gas_profiler::GasProfile;
use crate::integrity_checker::IntegrityReport;
use crate::rollback_validator::RollbackReport;

pub struct ScenarioResult {
    pub name: &'static str,
    pub integrity: IntegrityReport,
    pub rollback: RollbackReport,
    pub gas_profiles: Vec<GasProfile>,
    pub passed: bool,
}
