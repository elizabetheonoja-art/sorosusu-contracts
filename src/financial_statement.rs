#![no_std]
use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, 
    Address, Env, String, Symbol, Vec, crypto::keccak256
};

// Import types from main contract
use crate::{
    FinancialTransaction, FinancialTransactionType, CircleInfo, DataKey
};

// --- ERROR CODES ---
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum FinancialStatementError {
    Unauthorized = 1001,
    CircleNotFound = 1002,
    InvalidTimeRange = 1003,
    NoTransactionsFound = 1004,
    HashGenerationFailed = 1005,
}

// --- DATA STRUCTURES ---

// Re-use the main contract's FinancialTransaction type
pub type TransactionRecord = FinancialTransaction;

#[contracttype]
#[derive(Clone, Debug)]
pub struct FinancialStatement {
    pub circle_id: u64,
    pub statement_period_start: u64,
    pub statement_period_end: u64,
    pub total_contributions: i128,
    pub total_payouts: i128,
    pub total_penalties: i128,
    pub total_insurance_fees: i128,
    pub net_amount: i128,
    pub transaction_count: u32,
    pub member_count: u32,
    pub statement_hash: Vec<u8>,
    pub generated_at: u64,
    pub verifying_member: Address,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct StatementMetadata {
    pub circle_creator: Address,
    pub circle_token: Address,
    pub contribution_amount: i128,
    pub max_members: u32,
    pub current_round: u32,
    pub is_active: bool,
}

// --- STORAGE KEYS ---
#[contracttype]
#[derive(Clone)]
pub enum FinancialDataKey {
    TransactionHash(u64, Address), // circle_id, member_address
    StatementHash(u64, u64), // circle_id, timestamp
    CircleMetadata(u64),
}

// --- CONTRACT TRAIT ---
pub trait FinancialStatementTrait {
    // Generate financial statement hash for a member's verification
    fn generate_financial_statement(
        env: Env,
        requesting_member: Address,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
    ) -> FinancialStatement;

    // Verify a statement hash matches the data
    fn verify_statement_hash(
        env: Env,
        circle_id: u64,
        statement_hash: Vec<u8>,
        period_start: u64,
        period_end: u64,
    ) -> bool;

    // Get all transactions for a member within a period
    fn get_member_transactions(
        env: Env,
        member: Address,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
    ) -> Vec<TransactionRecord>;

    // Get statement metadata for PDF generation
    fn get_statement_metadata(env: Env, circle_id: u64) -> StatementMetadata;
}

// --- CONTRACT IMPLEMENTATION ---
#[contract]
pub struct FinancialStatementContract;

#[contractimpl]
impl FinancialStatementTrait for FinancialStatementContract {
    fn generate_financial_statement(
        env: Env,
        requesting_member: Address,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
    ) -> FinancialStatement {
        requesting_member.require_auth();

        // Validate time range
        if period_start >= period_end {
            panic!("Invalid time range");
        }

        // Get all transactions for the member in the period
        let transactions = Self::get_member_transactions(
            env.clone(),
            requesting_member.clone(),
            circle_id,
            period_start,
            period_end,
        );

        if transactions.is_empty() {
            panic!("No transactions found in period");
        }

        // Calculate totals
        let mut total_contributions = 0i128;
        let mut total_payouts = 0i128;
        let mut total_penalties = 0i128;
        let mut total_insurance_fees = 0i128;

        for tx in &transactions {
            match tx.transaction_type {
                FinancialTransactionType::Contribution => {
                    total_contributions += tx.amount;
                }
                FinancialTransactionType::Payout => {
                    total_payouts += tx.amount;
                }
                FinancialTransactionType::Penalty => {
                    total_penalties += tx.amount;
                }
                FinancialTransactionType::InsuranceFee => {
                    total_insurance_fees += tx.amount;
                }
            }
        }

        let net_amount = total_payouts - total_contributions - total_penalties - total_insurance_fees;

        // Generate statement hash
        let statement_hash = Self::generate_statement_hash(
            env.clone(),
            circle_id,
            period_start,
            period_end,
            &transactions,
            total_contributions,
            total_payouts,
            total_penalties,
            total_insurance_fees,
        );

        let statement = FinancialStatement {
            circle_id,
            statement_period_start: period_start,
            statement_period_end: period_end,
            total_contributions,
            total_payouts,
            total_penalties,
            total_insurance_fees,
            net_amount,
            transaction_count: transactions.len() as u32,
            member_count: Self::get_unique_member_count(&transactions),
            statement_hash: statement_hash.clone(),
            generated_at: env.ledger().timestamp(),
            verifying_member: requesting_member.clone(),
        };

        // Store the statement hash for future verification
        let statement_key = FinancialDataKey::StatementHash(circle_id, env.ledger().timestamp());
        env.storage().instance().set(&statement_key, &statement_hash);

        // Store transaction hash for this member
        let tx_hash_key = FinancialDataKey::TransactionHash(circle_id, requesting_member);
        env.storage().instance().set(&tx_hash_key, &statement_hash);

        statement
    }

    fn verify_statement_hash(
        env: Env,
        circle_id: u64,
        statement_hash: Vec<u8>,
        period_start: u64,
        period_end: u64,
    ) -> bool {
        // Retrieve stored hash
        let statement_key = FinancialDataKey::StatementHash(circle_id, period_end);
        if let Some(stored_hash) = env.storage().instance().get::<FinancialDataKey, Vec<u8>>(&statement_key) {
            stored_hash == statement_hash
        } else {
            false
        }
    }

    fn get_member_transactions(
        env: Env,
        member: Address,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
    ) -> Vec<TransactionRecord> {
        // Get transactions from the main contract's storage
        let tx_key = DataKey::FinancialTransaction(circle_id, member.clone());
        
        if let Some(transactions) = env.storage().instance().get::<DataKey, Vec<FinancialTransaction>>(&tx_key) {
            let mut filtered_transactions = Vec::<FinancialTransaction>::new(&env);
            
            for tx in transactions {
                if tx.timestamp >= period_start && tx.timestamp <= period_end {
                    filtered_transactions.push_back(tx);
                }
            }
            
            filtered_transactions
        } else {
            Vec::<FinancialTransaction>::new(&env)
        }
    }

    fn get_statement_metadata(env: Env, circle_id: u64) -> StatementMetadata {
        // Fetch real circle data from the main contract
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        StatementMetadata {
            circle_creator: circle.creator,
            circle_token: circle.token,
            contribution_amount: circle.contribution_amount,
            max_members: circle.max_members,
            current_round: circle.current_recipient_index,
            is_active: circle.is_active,
        }
    }
}

impl FinancialStatementContract {
    fn generate_statement_hash(
        env: Env,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
        transactions: &Vec<TransactionRecord>,
        total_contributions: i128,
        total_payouts: i128,
        total_penalties: i128,
        total_insurance_fees: i128,
    ) -> Vec<u8> {
        // Create a deterministic hash input
        let mut hash_input = Vec::<u8>::new(&env);
        
        // Add circle_id and time range
        hash_input.extend_from_slice(&circle_id.to_be_bytes());
        hash_input.extend_from_slice(&period_start.to_be_bytes());
        hash_input.extend_from_slice(&period_end.to_be_bytes());
        
        // Add totals
        hash_input.extend_from_slice(&total_contributions.to_be_bytes());
        hash_input.extend_from_slice(&total_payouts.to_be_bytes());
        hash_input.extend_from_slice(&total_penalties.to_be_bytes());
        hash_input.extend_from_slice(&total_insurance_fees.to_be_bytes());
        
        // Add transaction data in order
        for tx in transactions {
            hash_input.extend_from_slice(&tx.amount.to_be_bytes());
            hash_input.extend_from_slice(&tx.timestamp.to_be_bytes());
            hash_input.extend_from_slice(&tx.round_number.to_be_bytes());
            hash_input.extend_from_slice(&tx.penalty_amount.to_be_bytes());
            hash_input.extend_from_slice(&tx.insurance_fee.to_be_bytes());
        }
        
        // Add contract address for uniqueness
        let contract_addr = env.current_contract_address();
        hash_input.extend_from_slice(contract_addr.as_slice());
        
        // Generate keccak256 hash
        keccak256(&env, &hash_input).to_vec()
    }

    fn get_unique_member_count(transactions: &Vec<TransactionRecord>) -> u32 {
        if transactions.is_empty() {
            return 0;
        }
        
        // Simple approach: count unique members by comparing each transaction
        let mut unique_count = 0u32;
        let mut counted_members = Vec::<Address>::new(&transactions.env);
        
        for i in 0..transactions.len() {
            let current_member = &transactions.get(i).unwrap().member;
            let mut already_counted = false;
            
            // Check if we've already counted this member
            for j in 0..counted_members.len() {
                if counted_members.get(j).unwrap() == current_member {
                    already_counted = true;
                    break;
                }
            }
            
            if !already_counted {
                counted_members.push_back(current_member.clone());
                unique_count += 1;
            }
        }
        
        unique_count
    }
}

// --- HELPER FUNCTIONS FOR BACKEND INTEGRATION ---

#[contractimpl]
impl FinancialStatementContract {
    /// Get all data needed for PDF generation in a single call
    pub fn get_pdf_generation_data(
        env: Env,
        member: Address,
        circle_id: u64,
        period_start: u64,
        period_end: u64,
    ) -> (FinancialStatement, Vec<TransactionRecord>, StatementMetadata) {
        member.require_auth();
        
        let statement = Self::generate_financial_statement(
            env.clone(),
            member.clone(),
            circle_id,
            period_start,
            period_end,
        );
        
        let transactions = Self::get_member_transactions(
            env.clone(),
            member,
            circle_id,
            period_start,
            period_end,
        );
        
        let metadata = Self::get_statement_metadata(env.clone(), circle_id);
        
        (statement, transactions, metadata)
    }

    /// Batch generate statements for multiple members (admin only)
    pub fn batch_generate_statements(
        env: Env,
        admin: Address,
        circle_id: u64,
        members: Vec<Address>,
        period_start: u64,
        period_end: u64,
    ) -> Vec<FinancialStatement> {
        // In production, verify admin rights here
        admin.require_auth();
        
        let mut statements = Vec::<FinancialStatement>::new(&env);
        
        for member in members {
            let statement = Self::generate_financial_statement(
                env.clone(),
                member,
                circle_id,
                period_start,
                period_end,
            );
            statements.push_back(statement);
        }
        
        statements
    }
}
