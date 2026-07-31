#![no_std]
use soroban_sdk::{contract, contractimpl, contracttype, symbol_short, Address, Env, Symbol};

const CREDIT_COUNTER: Symbol = symbol_short!("cntr");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreditStatus {
    Active,
    Drawn,
    Repaid,
    Defaulted,
    Transferred,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CreditLine {
    pub id: u64,
    pub delegator: Address,
    pub delegate: Address,
    pub max_amount: i128,
    pub interest_rate_bps: i128,
    pub maturity: u64,
    pub collateral: Option<i128>,
    pub drawn_amount: i128,
    pub repaid_amount: i128,
    pub status: CreditStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub transfer_count: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DrawRecord {
    pub credit_line_id: u64,
    pub amount: i128,
    pub drawn_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RepaymentRecord {
    pub credit_line_id: u64,
    pub amount: i128,
    pub accrued_interest: i128,
    pub repaid_at: u64,
}

#[contracttype]
#[derive(Clone)]
pub enum CreditDataKey {
    CreditLine(u64),
    DelegatorLines(Address),
    DelegateLines(Address),
    Draws(u64),
    Repayments(u64),
}

#[contract]
pub struct CreditDelegationContract;

#[contractimpl]
impl CreditDelegationContract {
    pub fn create_credit_line(
        env: Env,
        delegator: Address,
        delegate: Address,
        max_amount: i128,
        interest_rate_bps: i128,
        maturity: u64,
        collateral: Option<i128>,
    ) -> u64 {
        delegator.require_auth();

        if max_amount <= 0 {
            panic!("max amount must be positive");
        }
        if interest_rate_bps < 0 || interest_rate_bps > 10000 {
            panic!("interest rate must be between 0 and 10000 bps");
        }

        let mut counter: u64 = env.storage().instance().get(&CREDIT_COUNTER).unwrap_or(0);
        counter += 1;
        env.storage().instance().set(&CREDIT_COUNTER, &counter);

        let credit_line = CreditLine {
            id: counter,
            delegator: delegator.clone(),
            delegate: delegate.clone(),
            max_amount,
            interest_rate_bps,
            maturity,
            collateral,
            drawn_amount: 0,
            repaid_amount: 0,
            status: CreditStatus::Active,
            created_at: env.ledger().sequence().into(),
            updated_at: env.ledger().sequence().into(),
            transfer_count: 0,
        };

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(counter), &credit_line);
        counter
    }

    pub fn draw(env: Env, credit_line_id: u64, delegate: Address, amount: i128) {
        delegate.require_auth();
        let mut credit_line: CreditLine = env
            .storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
            .unwrap_or_else(|| panic!("credit line not found"));

        if credit_line.delegate != delegate {
            panic!("not authorized as delegate");
        }
        if credit_line.status != CreditStatus::Active && credit_line.status != CreditStatus::Drawn {
            panic!("credit line is not active");
        }
        if u64::from(env.ledger().sequence()) > credit_line.maturity {
            panic!("credit line has matured");
        }

        let new_drawn = credit_line.drawn_amount + amount;
        if new_drawn > credit_line.max_amount {
            panic!("draw exceeds credit limit");
        }

        credit_line.drawn_amount = new_drawn;
        credit_line.status = CreditStatus::Drawn;
        credit_line.updated_at = env.ledger().sequence().into();

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(credit_line_id), &credit_line);
    }

    pub fn repay(env: Env, credit_line_id: u64, delegate: Address, amount: i128) {
        delegate.require_auth();
        let mut credit_line: CreditLine = env
            .storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
            .unwrap_or_else(|| panic!("credit line not found"));

        if credit_line.delegate != delegate {
            panic!("not authorized as delegate");
        }
        if credit_line.status != CreditStatus::Active && credit_line.status != CreditStatus::Drawn {
            panic!("credit line is not active");
        }

        let new_repaid = credit_line.repaid_amount + amount;
        if new_repaid > credit_line.drawn_amount {
            panic!("repayment exceeds drawn amount");
        }

        credit_line.repaid_amount = new_repaid;
        if new_repaid >= credit_line.drawn_amount {
            credit_line.status = CreditStatus::Repaid;
        }
        credit_line.updated_at = env.ledger().sequence().into();

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(credit_line_id), &credit_line);
    }

    pub fn claim_default(env: Env, credit_line_id: u64, delegator: Address) {
        delegator.require_auth();
        let mut credit_line: CreditLine = env
            .storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
            .unwrap_or_else(|| panic!("credit line not found"));

        if credit_line.delegator != delegator {
            panic!("not authorized as delegator");
        }
        if u64::from(env.ledger().sequence()) <= credit_line.maturity {
            panic!("credit line has not matured yet");
        }
        if credit_line.drawn_amount <= credit_line.repaid_amount {
            panic!("no outstanding debt");
        }
        if credit_line.status == CreditStatus::Defaulted {
            panic!("already defaulted");
        }

        credit_line.status = CreditStatus::Defaulted;
        credit_line.updated_at = env.ledger().sequence().into();

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(credit_line_id), &credit_line);
    }

    pub fn adjust_limit(env: Env, credit_line_id: u64, delegator: Address, new_max: i128) {
        delegator.require_auth();
        let mut credit_line: CreditLine = env
            .storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
            .unwrap_or_else(|| panic!("credit line not found"));

        if credit_line.delegator != delegator {
            panic!("not authorized as delegator");
        }
        if new_max < credit_line.drawn_amount {
            panic!("new limit below drawn amount");
        }

        credit_line.max_amount = new_max;
        credit_line.updated_at = env.ledger().sequence().into();

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(credit_line_id), &credit_line);
    }

    pub fn transfer(
        env: Env,
        credit_line_id: u64,
        current_delegator: Address,
        new_delegator: Address,
    ) {
        current_delegator.require_auth();
        let mut credit_line: CreditLine = env
            .storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
            .unwrap_or_else(|| panic!("credit line not found"));

        if credit_line.delegator != current_delegator {
            panic!("not authorized as delegator");
        }

        credit_line.delegator = new_delegator;
        credit_line.transfer_count += 1;
        credit_line.updated_at = env.ledger().sequence().into();

        env.storage()
            .instance()
            .set(&CreditDataKey::CreditLine(credit_line_id), &credit_line);
    }

    pub fn get_credit_line(env: Env, credit_line_id: u64) -> Option<CreditLine> {
        env.storage()
            .instance()
            .get(&CreditDataKey::CreditLine(credit_line_id))
    }
}
