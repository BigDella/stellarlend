use soroban_sdk::Address;

pub struct TestEnvironment {
    pub flash_loan_contract: Address,
    pub lending_pool: Address,
    pub liquidation_engine: Address,
    pub borrower: Address,
    pub liquidator: Address,
    pub oracle: Address,
}

impl TestEnvironment {
    pub fn new(
        flash_loan: Address,
        lending_pool: Address,
        liquidation_engine: Address,
        borrower: Address,
        liquidator: Address,
        oracle: Address,
    ) -> Self {
        TestEnvironment {
            flash_loan_contract: flash_loan,
            lending_pool,
            liquidation_engine,
            borrower,
            liquidator,
            oracle,
        }
    }
}

pub struct PositionSetup {
    pub collateral_amount: i128,
    pub collateral_price: i128,
    pub borrowed_amount: i128,
    pub borrowed_price: i128,
    pub min_health_factor: i128,
}

impl PositionSetup {
    pub fn create_profitable_liquidation() -> Self {
        PositionSetup {
            collateral_amount: 10_000_000,      // 10M of collateral
            collateral_price: 1_000_000,        // $1 each
            borrowed_amount: 8_000_000,         // 8M borrowed
            borrowed_price: 1_000_000,          // $1 each
            min_health_factor: 15_000,          // 1.5x minimum
        }
    }

    pub fn create_boundary_position() -> Self {
        PositionSetup {
            collateral_amount: 10_000_000,
            collateral_price: 1_000_000,
            borrowed_amount: 6_666_667,         // Just at boundary
            borrowed_price: 1_000_000,
            min_health_factor: 15_000,
        }
    }

    pub fn create_unprofitable_liquidation() -> Self {
        PositionSetup {
            collateral_amount: 5_000_000,
            collateral_price: 1_000_000,
            borrowed_amount: 2_000_000,
            borrowed_price: 1_000_000,
            min_health_factor: 15_000,
        }
    }

    pub fn health_factor(&self) -> i128 {
        if self.borrowed_amount == 0 {
            i128::MAX
        } else {
            let collateral_value = self.collateral_amount * self.collateral_price;
            let debt_value = self.borrowed_amount * self.borrowed_price;
            (collateral_value * 10_000) / debt_value
        }
    }

    pub fn is_liquidatable(&self) -> bool {
        self.health_factor() < self.min_health_factor
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_profitable_liquidation_setup() {
        let setup = PositionSetup::create_profitable_liquidation();
        assert!(setup.is_liquidatable());
        let health = setup.health_factor();
        assert!(health < setup.min_health_factor);
    }

    #[test]
    fn test_boundary_position() {
        let setup = PositionSetup::create_boundary_position();
        let health = setup.health_factor();
        assert!(health >= setup.min_health_factor - 1000);
        assert!(health <= setup.min_health_factor + 1000);
    }

    #[test]
    fn test_unprofitable_liquidation_healthy() {
        let setup = PositionSetup::create_unprofitable_liquidation();
        assert!(!setup.is_liquidatable());
        let health = setup.health_factor();
        assert!(health >= setup.min_health_factor);
    }
}
