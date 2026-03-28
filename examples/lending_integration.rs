// Example implementation of a third-party contract using SoroSusu reputation queries
// This demonstrates how a lending protocol could integrate with SoroSusu

#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, Address, Env, String, Symbol,
};

// Import SoroSusu contract types
// In a real implementation, you would add sorosusu-contracts as a dependency
// use sorosusu_contracts::{SoroSusuClient, ReputationData};

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    InsufficientReputation = 1,
    LoanAlreadyActive = 2,
    InvalidAmount = 3,
    Unauthorized = 4,
}

#[contracttype]
#[derive(Clone)]
pub struct LoanRequest {
    pub borrower: Address,
    pub amount: i128,
    pub interest_rate: u32, // in basis points
    pub term: u64, // in seconds
    pub collateral_required: bool,
    pub reputation_threshold: u32, // minimum susu_score required
}

#[contracttype]
#[derive(Clone)]
pub struct Loan {
    pub id: u64,
    pub borrower: Address,
    pub principal: i128,
    pub interest_rate: u32,
    pub amount_due: i128,
    pub due_timestamp: u64,
    pub is_active: bool,
    pub reputation_at_approval: u32,
}

#[contracttype]
pub enum DataKey {
    Admin,
    SusuContract,
    Loan(u64),
    LoanCount,
    UserLoans(Address),
}

// Mock ReputationData for demonstration
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct ReputationData {
    pub user_address: Address,
    pub susu_score: u32,        // 0-10000 bps (0-100%)
    pub reliability_score: u32,  // 0-10000 bps (0-100%)
    pub total_contributions: u32,
    pub on_time_rate: u32,      // 0-10000 bps (0-100%)
    pub volume_saved: i128,
    pub social_capital: u32,    // 0-10000 bps (0-100%)
    pub last_updated: u64,
    pub is_active: bool,
}

#[contractclient(name = "SusuClient")]
pub trait SusuTrait {
    fn get_reputation(env: Env, user: Address) -> ReputationData;
}

pub trait LendingTrait {
    fn init(env: Env, admin: Address, susu_contract: Address);
    fn request_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        interest_rate: u32,
        term: u64,
    ) -> u64;
    fn approve_loan(env: Env, lender: Address, loan_id: u64);
    fn repay_loan(env: Env, borrower: Address, loan_id: u64, amount: i128);
    fn get_loan_terms(env: Env, user: Address) -> LoanRequest;
}

#[contract]
pub struct SoroSusuLending;

#[contractimpl]
impl LendingTrait for SoroSusuLending {
    fn init(env: Env, admin: Address, susu_contract: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::SusuContract, &susu_contract);
        env.storage().instance().set(&DataKey::LoanCount, &0u64);
    }

    fn request_loan(
        env: Env,
        borrower: Address,
        amount: i128,
        interest_rate: u32,
        term: u64,
    ) -> u64 {
        borrower.require_auth();

        // Get SoroSusu contract address
        let susu_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::SusuContract)
            .expect("Susu contract not set");

        // Query user's reputation from SoroSusu
        let susu_client = SusuClient::new(&env, &susu_contract);
        let reputation = susu_client.get_reputation(&borrower);

        // Determine loan terms based on reputation
        let (collateral_required, reputation_threshold) = match reputation.susu_score {
            8000..=10000 => (false, 7000),    // Excellent - no collateral needed
            6000..=7999 => (false, 6000),     // Good - no collateral needed
            4000..=5999 => (true, 4000),      // Fair - collateral required
            2000..=3999 => (true, 2000),      // Poor - collateral required, higher threshold
            _ => return env.panic_with_error!(Error::InsufficientReputation), // Very Poor - denied
        };

        // Check if user meets reputation threshold
        if reputation.susu_score < reputation_threshold {
            env.panic_with_error!(Error::InsufficientReputation);
        }

        // Check for existing active loans
        let user_loans_key = DataKey::UserLoans(borrower.clone());
        let active_loans: Vec<u64> = env
            .storage()
            .instance()
            .get(&user_loans_key)
            .unwrap_or(Vec::new(&env));

        for loan_id in active_loans.iter() {
            let loan_key = DataKey::Loan(loan_id);
            if let Some(loan) = env.storage().instance().get::<DataKey, Loan>(&loan_key) {
                if loan.is_active {
                    env.panic_with_error!(Error::LoanAlreadyActive);
                }
            }
        }

        // Create loan request
        let loan_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::LoanCount)
            .unwrap_or(0);
        let new_loan_id = loan_count + 1;

        let loan_request = LoanRequest {
            borrower: borrower.clone(),
            amount,
            interest_rate,
            term,
            collateral_required,
            reputation_threshold,
        };

        // Store loan request (in a real implementation, you'd have separate request/approval flow)
        let loan = Loan {
            id: new_loan_id,
            borrower: borrower.clone(),
            principal: amount,
            interest_rate,
            amount_due: amount + (amount * interest_rate as i128) / 10000,
            due_timestamp: env.ledger().timestamp() + term,
            is_active: false, // Pending approval
            reputation_at_approval: reputation.susu_score,
        };

        env.storage().instance().set(&DataKey::Loan(new_loan_id), &loan);
        env.storage().instance().set(&DataKey::LoanCount, &new_loan_id);

        // Add to user's loans
        let mut user_loans = active_loans;
        user_loans.push_back(new_loan_id);
        env.storage().instance().set(&user_loans_key, &user_loans);

        // Publish event
        env.events().publish(
            (Symbol::new(&env, "LOAN_REQUESTED"), borrower),
            (new_loan_id, amount, reputation.susu_score),
        );

        new_loan_id
    }

    fn approve_loan(env: Env, lender: Address, loan_id: u64) {
        lender.require_auth();

        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Admin not set");

        if lender != admin {
            env.panic_with_error!(Error::Unauthorized);
        }

        let loan_key = DataKey::Loan(loan_id);
        let mut loan: Loan = env
            .storage()
            .instance()
            .get(&loan_key)
            .expect("Loan not found");

        if loan.is_active {
            env.panic_with_error!(Error::LoanAlreadyActive);
        }

        // Activate the loan
        loan.is_active = true;
        env.storage().instance().set(&loan_key, &loan);

        // Publish event
        env.events().publish(
            (Symbol::new(&env, "LOAN_APPROVED"), loan.borrower),
            (loan_id, loan.amount_due),
        );
    }

    fn repay_loan(env: Env, borrower: Address, loan_id: u64, amount: i128) {
        borrower.require_auth();

        let loan_key = DataKey::Loan(loan_id);
        let mut loan: Loan = env
            .storage()
            .instance()
            .get(&loan_key)
            .expect("Loan not found");

        if loan.borrower != borrower {
            env.panic_with_error!(Error::Unauthorized);
        }

        if !loan.is_active {
            env.panic_with_error!(Error::InvalidAmount);
        }

        // In a real implementation, you would handle token transfers here
        // For now, we'll just mark as paid if amount covers the due amount

        if amount >= loan.amount_due {
            loan.is_active = false;
            env.storage().instance().set(&loan_key, &loan);

            // Publish event
            env.events().publish(
                (Symbol::new(&env, "LOAN_REPAID"), borrower),
                (loan_id, amount),
            );
        }
    }

    fn get_loan_terms(env: Env, user: Address) -> LoanRequest {
        // Get SoroSusu contract address
        let susu_contract: Address = env
            .storage()
            .instance()
            .get(&DataKey::SusuContract)
            .expect("Susu contract not set");

        // Query user's reputation
        let susu_client = SusuClient::new(&env, &susu_contract);
        let reputation = susu_client.get_reputation(&user);

        // Determine terms based on reputation
        let (collateral_required, reputation_threshold, interest_rate) = match reputation.susu_score {
            8000..=10000 => (false, 7000, 500),   // Excellent - 5% interest
            6000..=7999 => (false, 6000, 800),    // Good - 8% interest
            4000..=5999 => (true, 4000, 1200),    // Fair - 12% interest + collateral
            2000..=3999 => (true, 2000, 2000),    // Poor - 20% interest + collateral
            _ => (true, 0, 3000),                 // Very Poor - 30% interest + collateral
        };

        LoanRequest {
            borrower: user,
            amount: 0, // Will be set by caller
            interest_rate,
            term: 2592000, // 30 days default
            collateral_required,
            reputation_threshold,
        }
    }
}
