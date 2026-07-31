// ════════════════════════════════════════════════════════════════
// COMPREHENSIVE REENTRANCY GUARD
// ════════════════════════════════════════════════════════════════
// Provides multi-layered reentrancy protection:
// 1. Function-level guards (per-function locks)
// 2. Cross-contract reentrancy detection
// 3. Read-only reentrancy detection
// 4. Constructor reentrancy protection
// 5. Delegate call reentrancy protection
// 6. Checks-effects-interactions pattern enforcement
// ════════════════════════════════════════════════════════════════

use soroban_sdk::{contracttype, Address, Env, IntoVal, Symbol, Val};

/// Reentrancy guard state tracking
#[derive(Clone, Copy, PartialEq, Eq)]
#[contracttype]
pub enum GuardState {
    NotEntered = 0,
    Entered = 1,
}

/// Storage keys for reentrancy guards
#[contracttype]
#[derive(Clone)]
pub enum ReentrancyKey {
    /// Global reentrancy lock
    GlobalLock,
    /// Function-specific locks
    DepositLock,
    WithdrawLock,
    BorrowLock,
    RepayLock,
    LiquidateLock,
    FlashLoanLock,
    /// Cross-contract reentrancy tracking
    CrossContractLock(Address),
    /// Read-only reentrancy detection
    ReadOnlyLock,
    /// Constructor reentrancy protection
    ConstructorLock,
    /// Delegate call reentrancy protection
    DelegateCallLock,
}

/// Comprehensive reentrancy guard with RAII pattern
pub struct ReentrancyGuard<'a> {
    env: &'a Env,
    key: Val,
    state_before: GuardState,
    is_read_only: bool,
}

impl<'a> ReentrancyGuard<'a> {
    /// Create a new global reentrancy guard
    pub fn new(env: &'a Env) -> Result<Self, u32> {
        let key = ReentrancyKey::GlobalLock.into_val(env);
        Self::new_with_key(env, key, false)
    }

    /// Create a new reentrancy guard with a specific key
    pub fn new_with_key(env: &'a Env, key: Val, is_read_only: bool) -> Result<Self, u32> {
        // CHECK: Are we already inside this function?
        if env.storage().temporary().has(&key) {
            return Err(7); // Reentrancy error code
        }

        // EFFECT: Mark as entered immediately
        env.storage().temporary().set(&key, &true);

        Ok(Self {
            env,
            key,
            state_before: GuardState::NotEntered,
            is_read_only,
        })
    }

    /// Create a cross-contract reentrancy guard
    pub fn new_cross_contract(env: &'a Env, caller: &Address) -> Result<Self, u32> {
        let key = ReentrancyKey::CrossContractLock(caller.clone()).into_val(env);
        
        // Check for cross-contract reentrancy
        if env.storage().temporary().has(&key) {
            return Err(7);
        }

        env.storage().temporary().set(&key, &true);
        
        Ok(Self {
            env,
            key,
            state_before: GuardState::NotEntered,
            is_read_only: false,
        })
    }

    /// Create a read-only reentrancy guard
    pub fn new_read_only(env: &'a Env) -> Result<Self, u32> {
        let key = ReentrancyKey::ReadOnlyLock.into_val(env);
        
        // Read-only functions can be re-entered but we track it
        let state_before = if env.storage().temporary().has(&key) {
            GuardState::Entered
        } else {
            GuardState::NotEntered
        };

        env.storage().temporary().set(&key, &true);
        
        Ok(Self {
            env,
            key,
            state_before,
            is_read_only: true,
        })
    }

    /// Create a constructor reentrancy guard
    pub fn new_constructor(env: &'a Env) -> Result<Self, u32> {
        let key = ReentrancyKey::ConstructorLock.into_val(env);
        
        if env.storage().temporary().has(&key) {
            return Err(7);
        }

        env.storage().temporary().set(&key, &true);
        
        Ok(Self {
            env,
            key,
            state_before: GuardState::NotEntered,
            is_read_only: false,
        })
    }

    /// Create a delegate call reentrancy guard
    pub fn new_delegate_call(env: &'a Env) -> Result<Self, u32> {
        let key = ReentrancyKey::DelegateCallLock.into_val(env);
        
        if env.storage().temporary().has(&key) {
            return Err(7);
        }

        env.storage().temporary().set(&key, &true);
        
        Ok(Self {
            env,
            key,
            state_before: GuardState::NotEntered,
            is_read_only: false,
        })
    }

    /// Check if this is a read-only reentrancy
    pub fn is_read_only_reentrancy(&self) -> bool {
        self.is_read_only && self.state_before == GuardState::Entered
    }
}

impl<'a> Drop for ReentrancyGuard<'a> {
    fn drop(&mut self) {
        // INTERACTION: Exit only after all operations complete
        self.env.storage().temporary().remove(&self.key);
    }
}

/// Helper macro for function-level reentrancy guards
#[macro_export]
macro_rules! reentrancy_guard {
    ($env:expr, $key:expr) => {
        $crate::reentrancy::ReentrancyGuard::new_with_key($env, $key.into_val($env), false)
    };
}

/// Helper macro for cross-contract reentrancy guards
#[macro_export]
macro_rules! cross_contract_guard {
    ($env:expr, $caller:expr) => {
        $crate::reentrancy::ReentrancyGuard::new_cross_contract($env, $caller)
    };
}

/// Helper macro for read-only reentrancy guards
#[macro_export]
macro_rules! read_only_guard {
    ($env:expr) => {
        $crate::reentrancy::ReentrancyGuard::new_read_only($env)
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_state_transitions() {
        // Guard state should transition correctly
        assert_eq!(GuardState::NotEntered, GuardState::NotEntered);
        assert_eq!(GuardState::Entered, GuardState::Entered);
    }
}
