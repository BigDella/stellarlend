use soroban_sdk::Address;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FlashLoanLiquidationScenario {
    ProfitableLiquidation,
    UnprofitableLiquidation,
    CircuitBreakerIntervention,
    CascadingFlashLoans,
    BoundaryHealthFactor,
    PartialLiquidationWithFlashLoan,
    MultiPositionLiquidation,
    FlashLoanSandwichProtection,
    EmptyFlashLoan,
}

pub struct ScenarioConfig {
    pub scenario: FlashLoanLiquidationScenario,
    pub flash_loan_amount: i128,
    pub flash_loan_fee_bps: i128,
    pub liquidation_incentive_bps: i128,
    pub max_price_impact_bps: i128,
}

impl ScenarioConfig {
    pub fn profitable_liquidation() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::ProfitableLiquidation,
            flash_loan_amount: 5_000_000,
            flash_loan_fee_bps: 500,      // 5% fee
            liquidation_incentive_bps: 1_000,  // 10% incentive
            max_price_impact_bps: 2_000,       // 20% max impact
        }
    }

    pub fn unprofitable_liquidation() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::UnprofitableLiquidation,
            flash_loan_amount: 5_000_000,
            flash_loan_fee_bps: 500,
            liquidation_incentive_bps: 500,   // 5% incentive (not profitable)
            max_price_impact_bps: 2_000,
        }
    }

    pub fn cascading_flash_loans() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::CascadingFlashLoans,
            flash_loan_amount: 1_000_000,
            flash_loan_fee_bps: 1_000,    // 10% fee
            liquidation_incentive_bps: 1_000,
            max_price_impact_bps: 3_000,
        }
    }

    pub fn boundary_health_factor() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::BoundaryHealthFactor,
            flash_loan_amount: 1_000_000,
            flash_loan_fee_bps: 500,
            liquidation_incentive_bps: 1_000,
            max_price_impact_bps: 2_000,
        }
    }

    pub fn partial_liquidation() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::PartialLiquidationWithFlashLoan,
            flash_loan_amount: 2_000_000,  // Partial
            flash_loan_fee_bps: 500,
            liquidation_incentive_bps: 1_000,
            max_price_impact_bps: 2_000,
        }
    }

    pub fn sandwich_protection() -> Self {
        ScenarioConfig {
            scenario: FlashLoanLiquidationScenario::FlashLoanSandwichProtection,
            flash_loan_amount: 5_000_000,
            flash_loan_fee_bps: 500,
            liquidation_incentive_bps: 1_000,
            max_price_impact_bps: 100,  // Very tight protection
        }
    }

    pub fn is_profitable(&self) -> bool {
        self.liquidation_incentive_bps > self.flash_loan_fee_bps
    }

    pub fn expected_profit(&self, liquidated_collateral: i128) -> i128 {
        let incentive = (liquidated_collateral * self.liquidation_incentive_bps) / 10_000;
        let fee = (liquidated_collateral * self.flash_loan_fee_bps) / 10_000;
        incentive - fee
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profitable_config() {
        let config = ScenarioConfig::profitable_liquidation();
        assert!(config.is_profitable());
    }

    #[test]
    fn test_unprofitable_config() {
        let config = ScenarioConfig::unprofitable_liquidation();
        assert!(!config.is_profitable());
    }

    #[test]
    fn test_profit_calculation() {
        let config = ScenarioConfig::profitable_liquidation();
        let collateral = 1_000_000;
        let profit = config.expected_profit(collateral);
        assert!(profit > 0);
    }

    #[test]
    fn test_sandwich_protection_strict() {
        let config = ScenarioConfig::sandwich_protection();
        assert_eq!(config.max_price_impact_bps, 100);
    }
}
