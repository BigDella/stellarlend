use soroban_sdk::{contracttype, Address, Vec};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
#[contracttype]
pub enum MarginMode {
    Isolated = 0,
    Cross = 1,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
#[contracttype]
pub enum MarginCallLevel {
    Safe = 0,
    Warning = 1,
    Liquidation = 2,
    ForcedClose = 3,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct Position {
    pub asset: Address,
    pub amount: i128,
    pub debt: i128,
    pub entry_price: i128,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracttype]
pub struct MarginAccount {
    pub owner: Address,
    pub mode: MarginMode,
    pub positions: Vec<Position>,
    pub total_collateral_value: i128,
    pub total_debt_value: i128,
}

impl MarginAccount {
    pub fn is_isolated(&self) -> bool {
        self.mode == MarginMode::Isolated
    }

    pub fn is_cross(&self) -> bool {
        self.mode == MarginMode::Cross
    }
}
