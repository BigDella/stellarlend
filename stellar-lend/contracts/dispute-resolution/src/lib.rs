#![no_std]
use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short, Address, BytesN, Env, String, Symbol, Vec,
};

const DISPUTE_COUNTER: Symbol = symbol_short!("d_counter");

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DisputeStatus {
    Filing,
    Evidence,
    Voting,
    Resolved,
    Appealed,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VoteChoice {
    Valid,
    Invalid,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub submitter: Address,
    pub description: String,
    pub data: BytesN<64>,
    pub submitted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Juror {
    pub address: Address,
    pub selected_at: u64,
    pub voted: bool,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vote {
    pub juror: Address,
    pub vote: VoteChoice,
    pub rationale: String,
    pub voted_at: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Dispute {
    pub id: u64,
    pub disputer: Address,
    pub liquidator: Address,
    pub liquidation_tx: BytesN<64>,
    pub collateral_amount: i128,
    pub dispute_fee: i128,
    pub status: DisputeStatus,
    pub evidence: Vec<Evidence>,
    pub jurors: Vec<Juror>,
    pub votes: Vec<Vote>,
    pub resolution: u32, // 0=unresolved, 1=Valid, 2=Invalid
    pub resolved_at: Option<u64>,
    pub created_at: u64,
    pub appeal_parent: Option<u64>,
    pub appeal_stake: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct JurorRegistration {
    pub address: Address,
    pub registered_at: u64,
    pub total_cases: u32,
    pub rewards: i128,
}

#[contracttype]
#[derive(Clone)]
pub enum DisputeDataKey {
    Dispute(u64),
    Juror(Address),
    JurorList,
}

#[contract]
pub struct DisputeResolutionContract;

#[contractimpl]
impl DisputeResolutionContract {
    pub fn register_juror(env: Env, address: Address) {
        address.require_auth();
        if env
            .storage()
            .instance()
            .has(&DisputeDataKey::Juror(address.clone()))
        {
            panic!("already registered as juror");
        }
        let registration = JurorRegistration {
            address: address.clone(),
            registered_at: env.ledger().sequence().into(),
            total_cases: 0,
            rewards: 0,
        };
        env.storage()
            .instance()
            .set(&DisputeDataKey::Juror(address), &registration);
    }

    pub fn file_dispute(
        env: Env,
        disputer: Address,
        liquidator: Address,
        liquidation_tx: BytesN<64>,
        collateral_amount: i128,
        _evidence_data: BytesN<64>,
    ) -> u64 {
        disputer.require_auth();

        if collateral_amount <= 0 {
            panic!("invalid collateral amount");
        }

        let mut counter: u64 = env.storage().instance().get(&DISPUTE_COUNTER).unwrap_or(0);
        counter += 1;
        env.storage().instance().set(&DISPUTE_COUNTER, &counter);

        let dispute = Dispute {
            id: counter,
            disputer: disputer.clone(),
            liquidator,
            liquidation_tx,
            collateral_amount,
            dispute_fee: collateral_amount / 10, // 10% fee
            status: DisputeStatus::Evidence,
            evidence: Vec::new(&env),
            jurors: Vec::new(&env),
            votes: Vec::new(&env),
            resolution: 0,
            resolved_at: None,
            created_at: env.ledger().sequence().into(),
            appeal_parent: None,
            appeal_stake: 0,
        };

        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(counter), &dispute);
        counter
    }

    pub fn submit_evidence(
        env: Env,
        dispute_id: u64,
        submitter: Address,
        description: String,
        data: BytesN<64>,
    ) {
        submitter.require_auth();
        let mut dispute: Dispute = env
            .storage()
            .instance()
            .get(&DisputeDataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic!("dispute not found"));

        if dispute.status != DisputeStatus::Evidence && dispute.status != DisputeStatus::Filing {
            panic!("evidence period has ended");
        }

        let evidence = Evidence {
            submitter: submitter.clone(),
            description,
            data,
            submitted_at: env.ledger().sequence().into(),
        };

        let mut evidence_list = dispute.evidence;
        evidence_list.push_back(evidence);
        dispute.evidence = evidence_list;
        dispute.status = DisputeStatus::Evidence;

        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(dispute_id), &dispute);
    }

    pub fn select_jurors(env: Env, dispute_id: u64, selected: Vec<Address>) {
        // Only callable by the contract admin or automated system
        let mut dispute: Dispute = env
            .storage()
            .instance()
            .get(&DisputeDataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic!("dispute not found"));

        if !dispute.jurors.is_empty() {
            panic!("jurors already selected");
        }

        let mut jurors = Vec::new(&env);
        for address in selected.iter() {
            let reg: Option<JurorRegistration> = env
                .storage()
                .instance()
                .get(&DisputeDataKey::Juror(address.clone()));
            if reg.is_some() && address != dispute.disputer {
                jurors.push_back(Juror {
                    address: address.clone(),
                    selected_at: env.ledger().sequence().into(),
                    voted: false,
                });
            }
        }

        if jurors.len() < 3 {
            panic!("insufficient jurors");
        }

        dispute.jurors = jurors;
        dispute.status = DisputeStatus::Voting;
        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(dispute_id), &dispute);
    }

    pub fn cast_vote(
        env: Env,
        dispute_id: u64,
        juror: Address,
        vote: VoteChoice,
        rationale: String,
    ) {
        juror.require_auth();
        let mut dispute: Dispute = env
            .storage()
            .instance()
            .get(&DisputeDataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic!("dispute not found"));

        if dispute.status != DisputeStatus::Voting {
            panic!("voting period is not active");
        }

        // Check if juror is selected and hasn't voted
        let mut found = false;
        let mut already_voted = false;
        let mut updated_jurors = Vec::new(&env);

        for j in dispute.jurors.iter() {
            if j.address == juror {
                found = true;
                if j.voted {
                    already_voted = true;
                } else {
                    updated_jurors.push_back(Juror {
                        address: j.address.clone(),
                        selected_at: j.selected_at,
                        voted: true,
                    });
                }
            } else {
                updated_jurors.push_back(j);
            }
        }

        if !found {
            panic!("not a selected juror");
        }
        if already_voted {
            panic!("already voted");
        }

        dispute.jurors = updated_jurors;

        let vote_record = Vote {
            juror: juror.clone(),
            vote: vote.clone(),
            rationale,
            voted_at: env.ledger().sequence().into(),
        };

        let mut votes = dispute.votes;
        votes.push_back(vote_record);
        dispute.votes = votes;

        // Check if can resolve (>66% majority)
        let total_votes = dispute.votes.len();
        if total_votes >= 3 {
            let valid_count = dispute
                .votes
                .iter()
                .filter(|v| v.vote == VoteChoice::Valid)
                .count() as u32;
            let invalid_count = total_votes - valid_count;

            if valid_count as f64 / total_votes as f64 > 0.66 {
                dispute.resolution = 1;
                dispute.status = DisputeStatus::Resolved;
                dispute.resolved_at = Some(env.ledger().sequence().into());
            } else if invalid_count as f64 / total_votes as f64 > 0.66 {
                dispute.resolution = 2;
                dispute.status = DisputeStatus::Resolved;
                dispute.resolved_at = Some(env.ledger().sequence().into());
            }
        }

        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(dispute_id), &dispute);
    }

    pub fn appeal(env: Env, dispute_id: u64, appellant: Address, stake: i128) -> u64 {
        appellant.require_auth();
        let dispute: Dispute = env
            .storage()
            .instance()
            .get(&DisputeDataKey::Dispute(dispute_id))
            .unwrap_or_else(|| panic!("dispute not found"));

        if dispute.status != DisputeStatus::Resolved {
            panic!("dispute is not resolved");
        }

        let required_stake = dispute.dispute_fee * 2;
        if stake < required_stake {
            panic!("appeal stake must be at least double the dispute fee");
        }

        let mut counter: u64 = env.storage().instance().get(&DISPUTE_COUNTER).unwrap_or(0);
        counter += 1;
        env.storage().instance().set(&DISPUTE_COUNTER, &counter);

        let appealed = Dispute {
            id: counter,
            disputer: dispute.disputer.clone(),
            liquidator: dispute.liquidator.clone(),
            liquidation_tx: dispute.liquidation_tx.clone(),
            collateral_amount: dispute.collateral_amount,
            dispute_fee: stake,
            status: DisputeStatus::Filing,
            evidence: dispute.evidence.clone(),
            jurors: Vec::new(&env),
            votes: Vec::new(&env),
            resolution: 0,
            resolved_at: None,
            created_at: env.ledger().sequence().into(),
            appeal_parent: Some(dispute_id),
            appeal_stake: stake,
        };

        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(counter), &appealed);

        // Mark original as appealed
        let mut original = dispute;
        original.status = DisputeStatus::Appealed;
        env.storage()
            .instance()
            .set(&DisputeDataKey::Dispute(dispute_id), &original);

        counter
    }

    pub fn get_dispute(env: Env, dispute_id: u64) -> Option<Dispute> {
        env.storage()
            .instance()
            .get(&DisputeDataKey::Dispute(dispute_id))
    }

    pub fn get_juror(env: Env, address: Address) -> Option<JurorRegistration> {
        env.storage()
            .instance()
            .get(&DisputeDataKey::Juror(address))
    }
}
