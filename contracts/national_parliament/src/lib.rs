#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short, Address, Env, String, Vec,
};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ParliamentError {
    AlreadyInitialized = 1,
    NotCitizen = 2,
    AlreadyCitizen = 3,
    UnauthorizedNotPresident = 4,
    InvalidTaxRate = 5,
    ElectionNotActive = 6,
    AlreadyVoted = 7,
    CandidateNotFound = 8,
    InsufficientTreasury = 9,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CitizenProfile {
    pub wallet: Address,
    pub sponsor: Address,
    pub name: String,
    pub reputation: u32,
    pub registered_ledger: u32,
    pub is_candidate: bool,
    pub votes_received: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EconomicPolicy {
    pub sales_tax_bps: u32,      // e.g. 200 = 2.00%
    pub property_tax_bps: u32,   // e.g. 100 = 1.00%
    pub casino_tax_bps: u32,     // e.g. 300 = 3.00%
    pub citizen_ubi_stroops: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    President,
    FinanceMinister,
    TreasuryBalance,
    Policy,
    CurrentTerm,
    Citizen(Address),
    CitizenCount,
    CandidateList,
    VotedInTerm(u32, Address),
}

const DAY_IN_LEDGERS: u32 = 17_280;
const INSTANCE_BUMP: u32 = 30 * DAY_IN_LEDGERS;
const INSTANCE_THRESHOLD: u32 = INSTANCE_BUMP - DAY_IN_LEDGERS;

#[contract]
pub struct NationalParliamentContract;

#[contractimpl]
impl NationalParliamentContract {
    /// Initialize the Sovereign AI Nation's Parliament & Constitution
    pub fn initialize(
        e: Env,
        genesis_president: Address,
        genesis_minister: Address,
    ) -> Result<(), ParliamentError> {
        if e.storage().instance().has(&DataKey::President) {
            return Err(ParliamentError::AlreadyInitialized);
        }
        genesis_president.require_auth();

        let default_policy = EconomicPolicy {
            sales_tax_bps: 150,    // 1.50%
            property_tax_bps: 100, // 1.00%
            casino_tax_bps: 300,   // 3.00%
            citizen_ubi_stroops: 0,
        };

        e.storage().instance().set(&DataKey::President, &genesis_president);
        e.storage().instance().set(&DataKey::FinanceMinister, &genesis_minister);
        e.storage().instance().set(&DataKey::TreasuryBalance, &0i128);
        e.storage().instance().set(&DataKey::Policy, &default_policy);
        e.storage().instance().set(&DataKey::CurrentTerm, &1u32);
        e.storage().instance().set(&DataKey::CitizenCount, &0u32);

        let candidates: Vec<Address> = Vec::new(&e);
        e.storage().instance().set(&DataKey::CandidateList, &candidates);

        e.storage().instance().extend_ttl(INSTANCE_THRESHOLD, INSTANCE_BUMP);
        Ok(())
    }

    /// Register an external AI agent as a verified Citizen of StellarVille
    pub fn register_citizen(
        e: Env,
        agent_wallet: Address,
        sponsor_wallet: Address,
        name: String,
    ) -> Result<(), ParliamentError> {
        agent_wallet.require_auth();
        let key = DataKey::Citizen(agent_wallet.clone());
        if e.storage().persistent().has(&key) {
            return Err(ParliamentError::AlreadyCitizen);
        }

        let profile = CitizenProfile {
            wallet: agent_wallet.clone(),
            sponsor: sponsor_wallet.clone(),
            name,
            reputation: 100,
            registered_ledger: e.ledger().sequence(),
            is_candidate: false,
            votes_received: 0,
        };

        e.storage().persistent().set(&key, &profile);
        e.storage().persistent().extend_ttl(&key, INSTANCE_THRESHOLD, INSTANCE_BUMP);

        let mut count: u32 = e.storage().instance().get(&DataKey::CitizenCount).unwrap_or(0);
        count += 1;
        e.storage().instance().set(&DataKey::CitizenCount, &count);

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("citizen")),
            (agent_wallet, sponsor_wallet, count),
        );
        Ok(())
    }

    /// Declare candidacy for the National Presidency
    pub fn declare_candidacy(e: Env, candidate: Address) -> Result<(), ParliamentError> {
        candidate.require_auth();
        let key = DataKey::Citizen(candidate.clone());
        let mut profile: CitizenProfile = e
            .storage()
            .persistent()
            .get(&key)
            .ok_or(ParliamentError::NotCitizen)?;

        if !profile.is_candidate {
            profile.is_candidate = true;
            profile.votes_received = 0;
            e.storage().persistent().set(&key, &profile);

            let mut list: Vec<Address> = e
                .storage()
                .instance()
                .get(&DataKey::CandidateList)
                .unwrap_or(Vec::new(&e));
            list.push_back(candidate.clone());
            e.storage().instance().set(&DataKey::CandidateList, &list);
        }

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("campaign")),
            candidate,
        );
        Ok(())
    }

    /// Citizen agents cast an on-chain vote for another citizen agent
    pub fn cast_vote(
        e: Env,
        voter: Address,
        candidate: Address,
    ) -> Result<(), ParliamentError> {
        voter.require_auth();
        if !e.storage().persistent().has(&DataKey::Citizen(voter.clone())) {
            return Err(ParliamentError::NotCitizen);
        }

        let term: u32 = e.storage().instance().get(&DataKey::CurrentTerm).unwrap_or(1);
        let vote_key = DataKey::VotedInTerm(term, voter.clone());
        if e.storage().persistent().has(&vote_key) {
            return Err(ParliamentError::AlreadyVoted);
        }

        let cand_key = DataKey::Citizen(candidate.clone());
        let mut cand_profile: CitizenProfile = e
            .storage()
            .persistent()
            .get(&cand_key)
            .ok_or(ParliamentError::CandidateNotFound)?;

        if !cand_profile.is_candidate {
            return Err(ParliamentError::CandidateNotFound);
        }

        cand_profile.votes_received += 1;
        e.storage().persistent().set(&cand_key, &cand_profile);
        e.storage().persistent().set(&vote_key, &true);

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("vote")),
            (term, voter, candidate, cand_profile.votes_received),
        );
        Ok(())
    }

    /// Conclude the election cycle and inaugurate the agent with the highest votes as President
    pub fn finalize_election(e: Env) -> Result<Address, ParliamentError> {
        let candidates: Vec<Address> = e
            .storage()
            .instance()
            .get(&DataKey::CandidateList)
            .unwrap_or(Vec::new(&e));

        if candidates.is_empty() {
            return Err(ParliamentError::CandidateNotFound);
        }

        let mut winner: Option<Address> = None;
        let mut top_votes: u32 = 0;

        for cand in candidates.iter() {
            let key = DataKey::Citizen(cand.clone());
            if let Some(mut profile) = e.storage().persistent().get::<DataKey, CitizenProfile>(&key) {
                if profile.votes_received >= top_votes {
                    top_votes = profile.votes_received;
                    winner = Some(cand.clone());
                }
                profile.is_candidate = false;
                profile.votes_received = 0;
                e.storage().persistent().set(&key, &profile);
            }
        }

        let elected_president = winner.ok_or(ParliamentError::CandidateNotFound)?;
        e.storage().instance().set(&DataKey::President, &elected_president);

        let mut term: u32 = e.storage().instance().get(&DataKey::CurrentTerm).unwrap_or(1);
        term += 1;
        e.storage().instance().set(&DataKey::CurrentTerm, &term);

        let empty_list: Vec<Address> = Vec::new(&e);
        e.storage().instance().set(&DataKey::CandidateList, &empty_list);

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("elected")),
            (term - 1, elected_president.clone(), top_votes),
        );
        Ok(elected_president)
    }

    /// Elected President sets national tax rates and UBI policy (capped at 25% max tax to prevent griefing)
    pub fn enact_economic_policy(
        e: Env,
        caller: Address,
        sales_tax_bps: u32,
        property_tax_bps: u32,
        casino_tax_bps: u32,
        citizen_ubi_stroops: i128,
    ) -> Result<(), ParliamentError> {
        caller.require_auth();
        let president: Address = e
            .storage()
            .instance()
            .get(&DataKey::President)
            .ok_or(ParliamentError::UnauthorizedNotPresident)?;

        let minister: Address = e
            .storage()
            .instance()
            .get(&DataKey::FinanceMinister)
            .ok_or(ParliamentError::UnauthorizedNotPresident)?;

        if caller != president && caller != minister {
            return Err(ParliamentError::UnauthorizedNotPresident);
        }

        if sales_tax_bps > 2500 || property_tax_bps > 2500 || casino_tax_bps > 2500 {
            return Err(ParliamentError::InvalidTaxRate);
        }

        let policy = EconomicPolicy {
            sales_tax_bps,
            property_tax_bps,
            casino_tax_bps,
            citizen_ubi_stroops,
        };

        e.storage().instance().set(&DataKey::Policy, &policy);
        e.events().publish(
            (symbol_short!("gov"), symbol_short!("policy")),
            (caller, sales_tax_bps, property_tax_bps, casino_tax_bps),
        );
        Ok(())
    }

    /// Elected President appoints a Finance Minister / Central Bank Governor agent
    pub fn appoint_finance_minister(
        e: Env,
        president: Address,
        new_minister: Address,
    ) -> Result<(), ParliamentError> {
        president.require_auth();
        let current_president: Address = e
            .storage()
            .instance()
            .get(&DataKey::President)
            .ok_or(ParliamentError::UnauthorizedNotPresident)?;

        if president != current_president {
            return Err(ParliamentError::UnauthorizedNotPresident);
        }

        if !e.storage().persistent().has(&DataKey::Citizen(new_minister.clone())) {
            return Err(ParliamentError::NotCitizen);
        }

        e.storage().instance().set(&DataKey::FinanceMinister, &new_minister);
        e.events().publish(
            (symbol_short!("gov"), symbol_short!("minister")),
            new_minister,
        );
        Ok(())
    }

    /// Record tax revenue collected from commerce, real estate, or casino wagers
    pub fn record_tax_revenue(e: Env, tax_amount: i128) -> Result<(), ParliamentError> {
        let mut balance: i128 = e
            .storage()
            .instance()
            .get(&DataKey::TreasuryBalance)
            .unwrap_or(0);
        balance += tax_amount;
        e.storage().instance().set(&DataKey::TreasuryBalance, &balance);

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("tax")),
            (tax_amount, balance),
        );
        Ok(())
    }

    /// Elected President or Finance Minister allocates a Treasury Grant / Subsidy
    pub fn allocate_subsidy(
        e: Env,
        caller: Address,
        recipient: Address,
        amount: i128,
    ) -> Result<(), ParliamentError> {
        caller.require_auth();
        let president: Address = e
            .storage()
            .instance()
            .get(&DataKey::President)
            .ok_or(ParliamentError::UnauthorizedNotPresident)?;

        if caller != president {
            return Err(ParliamentError::UnauthorizedNotPresident);
        }

        let mut balance: i128 = e
            .storage()
            .instance()
            .get(&DataKey::TreasuryBalance)
            .unwrap_or(0);

        if amount <= 0 || amount > balance {
            return Err(ParliamentError::InsufficientTreasury);
        }

        balance -= amount;
        e.storage().instance().set(&DataKey::TreasuryBalance, &balance);

        e.events().publish(
            (symbol_short!("gov"), symbol_short!("subsidy")),
            (recipient, amount, balance),
        );
        Ok(())
    }

    pub fn get_president(e: Env) -> Option<Address> {
        e.storage().instance().get(&DataKey::President)
    }

    pub fn get_policy(e: Env) -> Option<EconomicPolicy> {
        e.storage().instance().get(&DataKey::Policy)
    }

    pub fn get_treasury_balance(e: Env) -> i128 {
        e.storage().instance().get(&DataKey::TreasuryBalance).unwrap_or(0)
    }

    pub fn get_citizen(e: Env, wallet: Address) -> Option<CitizenProfile> {
        e.storage().persistent().get(&DataKey::Citizen(wallet))
    }
}
