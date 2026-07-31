#![no_std]
use soroban_sdk::{contract, contracterror, contractevent, contractimpl, contracttype, token::StellarAssetClient, Address, Env, Vec};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum YieldSplitterError {
    Unauthorized = 1,
    AlreadyInitialized = 2,
    NotInitialized = 3,
    InvalidAmount = 4,
    InvalidMaturity = 5,
    MaturityNotReached = 6,
    MaturityPassed = 7,
    PTBurnFailed = 8,
    YTBurnFailed = 9,
    PTMintFailed = 10,
    YTMintFailed = 11,
    InsufficientBalance = 12,
    PositionNotFound = 13,
    Overflow = 14,
    SplitAlreadyExists = 15,
    NoYieldAccrued = 16,
    LossExceedsPrincipal = 17,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SplitPosition {
    pub position_id: u64,
    pub owner: Address,
    pub underlying_asset: Address,
    pub principal_token: Address,
    pub yield_token: Address,
    pub underlying_amount: i128,
    pub pt_amount: i128,
    pub yt_amount: i128,
    pub maturity_date: u64,
    pub created_at: u64,
    pub redeemed: bool,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SplitPreview {
    pub pt_amount: i128,
    pub yt_amount: i128,
    pub pt_price: i128,
    pub yt_price: i128,
    pub estimated_yield: i128,
    pub time_to_maturity_secs: i128,
}

#[contracttype]
#[derive(Clone, Debug)]
pub enum DataKey {
    Admin,
    PrincipalToken(Address),
    YieldToken(Address),
    SplitPosition(u64),
    OwnerSplits(Address),
    SplitCounter,
    YieldAccrued(u64),
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct SplitEvent {
    pub position_id: u64,
    pub owner: Address,
    pub underlying_amount: i128,
    pub pt_minted: i128,
    pub yt_minted: i128,
    pub maturity_date: u64,
    pub timestamp: u64,
}

#[contractevent]
#[derive(Clone, Debug)]
pub struct MergeEvent {
    pub position_id: u64,
    pub owner: Address,
    pub underlying_returned: i128,
    pub pt_burned: i128,
    pub yt_burned: i128,
    pub yield_earned: i128,
    pub timestamp: u64,
}

const SECONDS_PER_YEAR: i128 = 31_536_000;
const BPS_DENOMINATOR: i128 = 10_000;

#[contract]
pub struct YieldSplitter;

#[contractimpl]
impl YieldSplitter {
    pub fn initialize(
        env: Env,
        admin: Address,
    ) -> Result<(), YieldSplitterError> {
        if env.storage().instance().has(&DataKey::Admin) {
            return Err(YieldSplitterError::AlreadyInitialized);
        }
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage()
            .instance()
            .set(&DataKey::SplitCounter, &0u64);
        Ok(())
    }

    pub fn register_tokens(
        env: Env,
        admin: Address,
        principal_token: Address,
        yield_token: Address,
    ) -> Result<(), YieldSplitterError> {
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .ok_or(YieldSplitterError::Unauthorized)?;
        if admin != stored_admin {
            return Err(YieldSplitterError::Unauthorized);
        }
        admin.require_auth();
        env.storage()
            .persistent()
            .set(&DataKey::PrincipalToken(principal_token.clone()), &true);
        env.storage()
            .persistent()
            .set(&DataKey::YieldToken(yield_token.clone()), &true);
        Ok(())
    }

    pub fn split_position(
        env: Env,
        owner: Address,
        underlying_asset: Address,
        principal_token: Address,
        yield_token: Address,
        amount: i128,
        maturity_date: u64,
    ) -> Result<u64, YieldSplitterError> {
        owner.require_auth();

        if amount <= 0 {
            return Err(YieldSplitterError::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if maturity_date <= now {
            return Err(YieldSplitterError::InvalidMaturity);
        }

        let is_pt_registered: bool = env
            .storage()
            .persistent()
            .get(&DataKey::PrincipalToken(principal_token.clone()))
            .unwrap_or(false);
        let is_yt_registered: bool = env
            .storage()
            .persistent()
            .get(&DataKey::YieldToken(yield_token.clone()))
            .unwrap_or(false);

        if !is_pt_registered || !is_yt_registered {
            return Err(YieldSplitterError::Unauthorized);
        }

        let mut counter: u64 = env
            .storage()
            .instance()
            .get(&DataKey::SplitCounter)
            .unwrap_or(0);
        counter += 1;

        let pt_amount = amount;
        let yt_amount = amount;

        let split = SplitPosition {
            position_id: counter,
            owner: owner.clone(),
            underlying_asset: underlying_asset.clone(),
            principal_token: principal_token.clone(),
            yield_token: yield_token.clone(),
            underlying_amount: amount,
            pt_amount,
            yt_amount,
            maturity_date,
            created_at: now,
            redeemed: false,
        };

        env.storage()
            .persistent()
            .set(&DataKey::SplitPosition(counter), &split);

        let mut owner_splits: Vec<u64> = env
            .storage()
            .persistent()
            .get(&DataKey::OwnerSplits(owner.clone()))
            .unwrap_or(Vec::new(&env));
        owner_splits.push_back(counter);
        env.storage()
            .persistent()
            .set(&DataKey::OwnerSplits(owner.clone()), &owner_splits);

        env.storage()
            .instance()
            .set(&DataKey::SplitCounter, &counter);

        let pyth_client = StellarAssetClient::new(&env, &principal_token);
        let yth_client = StellarAssetClient::new(&env, &yield_token);

        pyth_client.mint(&owner, &pt_amount);
        yth_client.mint(&owner, &yt_amount);

        SplitEvent {
            position_id: counter,
            owner,
            underlying_amount: amount,
            pt_minted: pt_amount,
            yt_minted: yt_amount,
            maturity_date,
            timestamp: now,
        }
        .publish(&env);

        Ok(counter)
    }

    pub fn merge_tokens(
        env: Env,
        owner: Address,
        position_id: u64,
    ) -> Result<i128, YieldSplitterError> {
        owner.require_auth();

        let mut split: SplitPosition = env
            .storage()
            .persistent()
            .get(&DataKey::SplitPosition(position_id))
            .ok_or(YieldSplitterError::PositionNotFound)?;

        if split.owner != owner {
            return Err(YieldSplitterError::Unauthorized);
        }
        if split.redeemed {
            return Err(YieldSplitterError::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if now < split.maturity_date {
            return Err(YieldSplitterError::MaturityNotReached);
        }

        split.redeemed = true;

        let accrued_yield = Self::calculate_yield_accrued(
            split.underlying_amount,
            split.created_at,
            split.maturity_date,
            now,
        )?;

        let mut total_return = split
            .underlying_amount
            .checked_add(accrued_yield)
            .ok_or(YieldSplitterError::Overflow)?;

        if accrued_yield < 0 {
            total_return = split
                .underlying_amount
                .checked_sub(accrued_yield.unsigned_abs() as i128)
                .ok_or(YieldSplitterError::LossExceedsPrincipal)?;
        }

        let pyth_client = StellarAssetClient::new(&env, &split.principal_token);
        let yth_client = StellarAssetClient::new(&env, &split.yield_token);

        pyth_client.burn(&owner, &split.pt_amount);
        yth_client.burn(&owner, &split.yt_amount);

        env.storage()
            .persistent()
            .set(&DataKey::SplitPosition(position_id), &split);
        env.storage()
            .persistent()
            .set(&DataKey::YieldAccrued(position_id), &accrued_yield);

        MergeEvent {
            position_id,
            owner,
            underlying_returned: total_return,
            pt_burned: split.pt_amount,
            yt_burned: split.yt_amount,
            yield_earned: accrued_yield,
            timestamp: now,
        }
        .publish(&env);

        Ok(total_return)
    }

    pub fn merge_before_maturity(
        env: Env,
        owner: Address,
        position_id: u64,
        penalty_bps: i128,
    ) -> Result<i128, YieldSplitterError> {
        owner.require_auth();

        let mut split: SplitPosition = env
            .storage()
            .persistent()
            .get(&DataKey::SplitPosition(position_id))
            .ok_or(YieldSplitterError::PositionNotFound)?;

        if split.owner != owner {
            return Err(YieldSplitterError::Unauthorized);
        }
        if split.redeemed {
            return Err(YieldSplitterError::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if now >= split.maturity_date {
            return Err(YieldSplitterError::MaturityPassed);
        }

        split.redeemed = true;

        let penalty = split
            .underlying_amount
            .checked_mul(penalty_bps)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(YieldSplitterError::Overflow)?;

        let total_return = split
            .underlying_amount
            .checked_sub(penalty)
            .ok_or(YieldSplitterError::LossExceedsPrincipal)?;

        let pyth_client = StellarAssetClient::new(&env, &split.principal_token);
        let yth_client = StellarAssetClient::new(&env, &split.yield_token);

        pyth_client.burn(&owner, &split.pt_amount);
        yth_client.burn(&owner, &split.yt_amount);

        env.storage()
            .persistent()
            .set(&DataKey::SplitPosition(position_id), &split);

        MergeEvent {
            position_id,
            owner,
            underlying_returned: total_return,
            pt_burned: split.pt_amount,
            yt_burned: split.yt_amount,
            yield_earned: 0,
            timestamp: now,
        }
        .publish(&env);

        Ok(total_return)
    }

    pub fn preview_split(
        env: Env,
        amount: i128,
        maturity_date: u64,
    ) -> Result<SplitPreview, YieldSplitterError> {
        if amount <= 0 {
            return Err(YieldSplitterError::InvalidAmount);
        }

        let now = env.ledger().timestamp();
        if maturity_date <= now {
            return Err(YieldSplitterError::InvalidMaturity);
        }

        let time_to_maturity = (maturity_date - now) as i128;
        let estimated_apy_bps = 500;
        let est_yield = amount
            .checked_mul(estimated_apy_bps)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_mul(time_to_maturity)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_div(SECONDS_PER_YEAR)
            .ok_or(YieldSplitterError::Overflow)?;

        let pt_price = 9_500i128;
        let yt_price = 500i128;

        Ok(SplitPreview {
            pt_amount: amount,
            yt_amount: amount,
            pt_price,
            yt_price,
            estimated_yield: est_yield,
            time_to_maturity_secs: time_to_maturity,
        })
    }

    pub fn get_yield_accrued(env: Env, position_id: u64) -> i128 {
        env.storage()
            .persistent()
            .get(&DataKey::YieldAccrued(position_id))
            .unwrap_or(0)
    }

    pub fn get_split_position(env: Env, position_id: u64) -> Option<SplitPosition> {
        env.storage()
            .persistent()
            .get(&DataKey::SplitPosition(position_id))
    }

    pub fn get_owner_splits(env: Env, owner: Address) -> Vec<u64> {
        env.storage()
            .persistent()
            .get(&DataKey::OwnerSplits(owner))
            .unwrap_or(Vec::new(&env))
    }

    pub fn get_admin(env: Env) -> Option<Address> {
        env.storage().instance().get(&DataKey::Admin)
    }

    fn calculate_yield_accrued(
        amount: i128,
        created_at: u64,
        maturity_date: u64,
        current_time: u64,
    ) -> Result<i128, YieldSplitterError> {
        let time_elapsed = if current_time >= maturity_date {
            (maturity_date - created_at) as i128
        } else {
            (current_time - created_at) as i128
        };

        if time_elapsed <= 0 {
            return Ok(0);
        }

        let yield_rate_bps = 500;
        let accrued = amount
            .checked_mul(yield_rate_bps)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_mul(time_elapsed)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_div(SECONDS_PER_YEAR)
            .ok_or(YieldSplitterError::Overflow)?
            .checked_div(BPS_DENOMINATOR)
            .ok_or(YieldSplitterError::Overflow)?;

        Ok(accrued)
    }
}

#[cfg(test)]
mod test;
