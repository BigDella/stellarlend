use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum MathError {
    Overflow = 1,
    DivisionByZero = 2,
    InvalidParameter = 3,
    NegativeValue = 4,
    ExceedsMax = 5,
    InsufficientCollateral = 6,
    InvalidHealthFactor = 7,
    RoundingError = 8,
}
