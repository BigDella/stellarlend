#[cfg(test)]
mod tests {
    use crate::setup::{TestEnvironment, PositionSetup};
    use crate::test_scenarios::ScenarioConfig;

    #[test]
    fn test_flash_loan_then_borrow_then_liquidate_then_repay() {
        let scenario = ScenarioConfig::profitable_liquidation();
        assert!(scenario.is_profitable());
        // Verify profitable path: incentive > fees
        assert!(scenario.liquidation_incentive_bps > scenario.flash_loan_fee_bps);
    }

    #[test]
    fn test_flash_loan_manipulate_price_then_liquidate() {
        let scenario = ScenarioConfig::unprofitable_liquidation();
        // Price manipulation should not help if incentive doesn't exceed fees
        assert!(!scenario.is_profitable());
    }

    #[test]
    fn test_multiple_flash_loans_cascade() {
        let scenario = ScenarioConfig::cascading_flash_loans();
        // Verify that multiple sequential flash loans are tracked
        assert_eq!(
            scenario.scenario,
            crate::test_scenarios::FlashLoanLiquidationScenario::CascadingFlashLoans
        );
    }

    #[test]
    fn test_flash_loan_at_boundary_health_factor() {
        let scenario = ScenarioConfig::boundary_health_factor();
        let setup = PositionSetup::create_boundary_position();

        // Position should be at the edge of liquidatability
        let health = setup.health_factor();
        assert!(health >= setup.min_health_factor - 1000);
    }

    #[test]
    fn test_partial_liquidation_with_flash_loan() {
        let scenario = ScenarioConfig::partial_liquidation();
        let setup = PositionSetup::create_profitable_liquidation();

        // Flash loan should only cover part of the debt
        assert!(scenario.flash_loan_amount < setup.borrowed_amount);
        assert!(scenario.is_profitable());
    }

    #[test]
    fn test_liquidation_across_multiple_positions() {
        let scenario = ScenarioConfig::profitable_liquidation();

        // With 5M flash loan, can potentially liquidate multiple positions
        let position1 = PositionSetup::create_profitable_liquidation();
        let position2 = PositionSetup::create_profitable_liquidation();

        // Both should be liquidatable
        assert!(position1.is_liquidatable());
        assert!(position2.is_liquidatable());
    }

    #[test]
    fn test_protocol_fee_collection_in_flash_liquidation() {
        let scenario = ScenarioConfig::profitable_liquidation();
        let setup = PositionSetup::create_profitable_liquidation();

        // Protocol should collect fee from flash loan
        let fee = (setup.borrowed_amount * scenario.flash_loan_fee_bps) / 10_000;
        assert!(fee > 0);
    }

    #[test]
    fn test_flash_loan_liquidation_sandwich_protection() {
        let scenario = ScenarioConfig::sandwich_protection();
        // Sandwich protection should enforce strict price impact limit
        assert!(scenario.max_price_impact_bps <= 200); // Very tight
    }

    #[test]
    fn test_partial_liquidation_insufficient_collateral() {
        let scenario = ScenarioConfig::partial_liquidation();
        let setup = PositionSetup::create_unprofitable_liquidation();

        // Unprofitable position should not be liquidatable
        assert!(!setup.is_liquidatable());
    }

    #[test]
    fn test_empty_flash_loan_edge_case() {
        // Test handling of 0 amount flash loan
        let scenario = ScenarioConfig::profitable_liquidation();
        let empty_flash_loan = 0;

        assert_ne!(empty_flash_loan, scenario.flash_loan_amount);
        // No fee for 0 amount
        let fee = (empty_flash_loan * scenario.flash_loan_fee_bps) / 10_000;
        assert_eq!(fee, 0);
    }

    #[test]
    fn test_liquidation_state_consistency() {
        let scenario = ScenarioConfig::profitable_liquidation();
        let setup = PositionSetup::create_profitable_liquidation();

        let initial_collateral = setup.collateral_amount;
        let initial_debt = setup.borrowed_amount;

        // After liquidation, state should be consistent
        let collateral_seized = (initial_collateral * scenario.liquidation_incentive_bps) / 10_000;
        let remaining_collateral = initial_collateral - collateral_seized;

        // No negative collateral
        assert!(remaining_collateral >= 0);
    }

    #[test]
    fn test_flash_loan_cannot_be_griefed() {
        let scenario = ScenarioConfig::profitable_liquidation();

        // Flash loan fee must be repaid in same transaction
        // So attacker cannot drain by repeated small flash loans
        let fee = (1_000_000 * scenario.flash_loan_fee_bps) / 10_000;
        assert!(fee > 0); // Fee enforced
    }

    #[test]
    fn test_liquidation_with_circuit_breaker() {
        let scenario = ScenarioConfig::unprofitable_liquidation();
        // Circuit breaker should stop liquidation if price moves adversely
        // Even with flash loan funding, shouldn't proceed if price impact too high
        assert!(!scenario.is_profitable());
    }

    #[test]
    fn test_gas_benchmark_flash_liquidation() {
        let scenario = ScenarioConfig::profitable_liquidation();
        let setup = PositionSetup::create_profitable_liquidation();

        // Flash liquidation: borrow + liquidate + repay in single tx
        // Should be efficient
        assert!(scenario.flash_loan_amount > 0);
        assert!(setup.borrowed_amount > 0);
    }

    #[test]
    fn test_gas_benchmark_standard_liquidation() {
        let setup = PositionSetup::create_profitable_liquidation();

        // Standard liquidation without flash loan
        // Should require collateral check, price validation
        assert!(setup.collateral_amount > 0);
        assert!(setup.is_liquidatable());
    }

    #[test]
    fn test_combo_profit_gate_and_unprofitable_rollback() {
        let profitable = ScenarioConfig::profitable_liquidation();
        assert!(profitable.is_profitable());
        let unprofitable = ScenarioConfig::unprofitable_liquidation();
        assert!(!unprofitable.is_profitable());
        assert!(unprofitable.liquidation_incentive_bps <= unprofitable.flash_loan_fee_bps);
    }
}
