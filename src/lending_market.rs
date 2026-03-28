#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map, i128, u64, u32,
};

use crate::{
    SoroSusuTrait, Error, DataKey, CircleInfo, Member, UserStats, 
    SusuNftClient, SusuNftTrait, AuditEntry, AuditAction
};

// --- INTER-SUSU LENDING MARKET LIQUIDITY HOOK ---

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum LendingMarketStatus {
    Active,
    Paused,
    Emergency,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum LendingVoteChoice {
    Approve,
    Reject,
    Abstain,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum LoanStatus {
    Active,
    Repaying,
    Defaulted,
    Liquidated,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum RiskCategory {
    LowRisk,      // 3000+ credit score, 70% LTV
    MediumRisk,   // 2000-2999 credit score, 80% LTV
    HighRisk,     // 1000-1999 credit score, 90% LTV
    VeryHighRisk, // <1000 credit score, 100% LTV
}

#[contracttype]
#[derive(Clone)]
pub struct LendingMarketConfig {
    pub is_enabled: bool,
    pub emergency_mode: bool,
    pub min_participation_bps: u32,    // Minimum participation for proposals
    pub voting_period: u64,            // Voting period in seconds
    pub quorum_bps: u32,              // Quorum requirement in basis points
    pub emergency_quorum_bps: u32,       // Higher quorum for emergencies
    pub max_ltv_ratio: u32,            // Maximum LTV ratio allowed
    pub base_interest_rate_bps: u32,       // Base interest rate
    pub risk_adjustment_bps: u32,          // Risk-based interest adjustment
}

#[contracttype]
#[derive(Clone)]
pub struct LendingPoolInfo {
    pub pool_id: u64,
    pub lender_circle_id: u64,           // Circle providing liquidity
    pub borrower_circle_id: u64,          // Circle receiving loan
    pub total_liquidity: i128,            // Total liquidity in pool
    pub utilized_amount: i128,             // Currently utilized amount
    pub available_amount: i128,             // Available for new loans
    pub interest_rate_bps: u32,             // Current interest rate
    pub participant_count: u32,             // Number of participants
    pub is_active: bool,                   // Pool status
    pub created_timestamp: u64,            // Pool creation time
    pub last_activity: u64,               // Last activity timestamp
}

#[contracttype]
#[derive(Clone)]
pub struct LendingPosition {
    pub position_id: u64,
    pub borrower: Address,
    pub lender_circle_id: u64,
    pub principal_amount: i128,
    pub interest_rate_bps: u32,
    pub loan_amount: i128,              // Principal + accrued interest
    pub collateral_amount: i128,
    pub risk_category: RiskCategory,
    pub status: LoanStatus,
    pub created_timestamp: u64,
    pub due_timestamp: u64,
    pub last_payment_timestamp: Option<u64>,
    pub remaining_balance: i128,
    pub repayment_schedule_id: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct LendingOffer {
    pub offer_id: u64,
    pub lender_circle_id: u64,
    pub max_amount: i128,
    pub min_amount: i128,
    pub interest_rate_bps: u32,
    pub loan_duration: u64,             // Loan duration in seconds
    pub risk_category: RiskCategory,
    pub is_active: bool,
    pub created_timestamp: u64,
    pub expires_timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct LiquidityProvider {
    pub provider_circle_id: u64,
    pub total_contributed: i128,
    pub current_locked: i128,
    pub unlock_timestamp: u64,
    pub yield_rate_bps: u32,
    pub is_active: bool,
    pub rewards_earned: i128,
    pub last_yield_compound: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct RepaymentSchedule {
    pub schedule_id: u64,
    pub position_id: u64,
    pub total_amount: i128,
    pub payment_amount: i128,
    pub payment_frequency: u64,         // Payment interval in seconds
    pub next_payment_due: u64,
    pub total_payments_made: u32,
    pub remaining_payments: u32,
    pub is_active: bool,
    pub created_timestamp: u64,
    pub last_payment_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct EmergencyLoan {
    pub loan_id: u64,
    pub requester_circle_id: u64,
    pub borrower_circle_id: u64,
    pub amount: i128,
    pub reason: String,
    pub required_votes: u32,
    pub current_votes: u32,
    pub status: LendingMarketStatus,
    pub created_timestamp: u64,
    pub voting_deadline: u64,
    pub execution_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct LendingMarketStats {
    pub total_pools_created: u64,
    pub active_pools: u64,
    pub total_loans_issued: u64,
    pub active_loans: u64,
    pub defaulted_loans: u64,
    pub total_volume_lent: i128,
    pub total_interest_earned: i128,
    pub average_loan_size: i128,
    pub default_rate: u32,
    pub last_updated: u64,
}

// --- LENDING MARKET CONTRACT ---

#[contract]
pub struct InterSusuLendingMarket;

#[contractimpl]
impl InterSusuLendingMarket {
    // Initialize lending market
    pub fn init_lending_market(env: Env, admin: Address) {
        admin.require_auth();
        
        // Set admin
        env.storage().instance().set(&DataKey::Admin, &admin);
        
        // Initialize lending market configuration
        let config = LendingMarketConfig {
            is_enabled: true,
            emergency_mode: false,
            min_participation_bps: 4000, // 40% minimum participation
            voting_period: LENDING_MARKET_VOTING_PERIOD,
            quorum_bps: 6000,            // 60% quorum
            emergency_quorum_bps: 8000,     // 80% emergency quorum
            max_ltv_ratio: 9000,            // 90% maximum LTV
            base_interest_rate_bps: BASE_INTEREST_RATE_BPS,
            risk_adjustment_bps: 500,          // 5% risk adjustment
        };
        
        env.storage().instance().set(&DataKey::LendingMarketConfig, &config);
        
        // Initialize statistics
        let stats = LendingMarketStats {
            total_pools_created: 0,
            active_pools: 0,
            total_loans_issued: 0,
            active_loans: 0,
            defaulted_loans: 0,
            total_volume_lent: 0,
            total_interest_earned: 0,
            average_loan_size: 0,
            default_rate: BASE_INTEREST_RATE_BPS,
            last_updated: env.ledger().timestamp(),
        };
        
        env.storage().instance().set(&DataKey::LendingMarketStats, &stats);
    }

    // Create lending pool between two circles
    pub fn create_lending_pool(
        env: Env,
        lender_circle_id: u64,
        borrower_circle_id: u64,
        initial_liquidity: i128,
    ) -> u64 {
        // Get lending market config
        let config: LendingMarketConfig = env.storage().instance()
            .get(&DataKey::LendingMarketConfig)
            .unwrap_or_else(|| panic!("Lending market not initialized"));
        
        if !config.is_enabled || config.emergency_mode {
            panic!("Lending market is disabled or in emergency mode");
        }
        
        // Verify circles exist and are active
        let lender_circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(lender_circle_id))
            .unwrap_or_else(|| panic!("Lender circle not found"));
        
        let borrower_circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(borrower_circle_id))
            .unwrap_or_else(|| panic!("Borrower circle not found"));
        
        // Validate initial liquidity
        if initial_liquidity < MIN_LENDING_AMOUNT {
            panic!("Insufficient initial liquidity");
        }
        
        // Create pool ID
        let pool_id = env.ledger().sequence();
        
        // Create lending pool
        let pool = LendingPoolInfo {
            pool_id,
            lender_circle_id,
            borrower_circle_id,
            total_liquidity: initial_liquidity,
            utilized_amount: 0,
            available_amount: initial_liquidity,
            interest_rate_bps: config.base_interest_rate_bps,
            participant_count: 2, // Lender + Borrower circles
            is_active: true,
            created_timestamp: env.ledger().timestamp(),
            last_activity: env.ledger().timestamp(),
        };
        
        // Store pool
        env.storage().instance().set(&DataKey::LendingPoolInfo(pool_id), &pool);
        
        // Update statistics
        let mut stats: LendingMarketStats = env.storage().instance()
            .get(&DataKey::LendingMarketStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.total_pools_created += 1;
        stats.active_pools += 1;
        stats.last_updated = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::LendingMarketStats, &stats);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: lender_circle.creator,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: pool_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
        
        pool_id
    }

    // Lend from pool to borrower
    pub fn lend_from_pool(
        env: Env,
        pool_id: u64,
        borrower: Address,
        amount: i128,
        loan_duration: u64,
    ) -> u64 {
        // Get pool and validate
        let mut pool: LendingPoolInfo = env.storage().instance()
            .get(&DataKey::LendingPoolInfo(pool_id))
            .unwrap_or_else(|| panic!("Pool not found"));
        
        if !pool.is_active {
            panic!("Pool is not active");
        }
        
        if amount > pool.available_amount {
            panic!("Insufficient pool liquidity");
        }
        
        if amount < MIN_LENDING_AMOUNT {
            panic!("Amount below minimum");
        }
        
        // Get borrower circle for risk assessment
        let borrower_circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(pool.borrower_circle_id))
            .unwrap_or_else(|| panic!("Borrower circle not found"));
        
        // Get borrower's reputation for risk assessment
        let borrower_reputation: UserStats = env.storage().instance()
            .get(&DataKey::UserStats(borrower.clone()))
            .unwrap_or_else(|| UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });
        
        // Calculate risk category and interest rate
        let risk_category = Self::assess_risk_category(&borrower_reputation);
        let risk_adjustment = match risk_category {
            RiskCategory::LowRisk => 0,
            RiskCategory::MediumRisk => 200,    // 2% increase
            RiskCategory::HighRisk => 500,      // 5% increase
            RiskCategory::VeryHighRisk => 1000, // 10% increase
        };
        
        let adjusted_interest_rate = pool.interest_rate_bps + risk_adjustment;
        let max_ltv = match risk_category {
            RiskCategory::LowRisk => 7000,     // 70% LTV
            RiskCategory::MediumRisk => 8000,   // 80% LTV
            RiskCategory::HighRisk => 9000,     // 90% LTV
            RiskCategory::VeryHighRisk => 10000, // 100% LTV
        };
        
        // Validate LTV ratio
        let max_loan_amount = (amount * max_ltv) / 10000;
        if amount > max_loan_amount {
            panic!("Amount exceeds maximum LTV ratio");
        }
        
        // Create lending position
        let position_id = env.ledger().sequence();
        let position = LendingPosition {
            position_id,
            borrower: borrower.clone(),
            lender_circle_id: pool.lender_circle_id,
            principal_amount: amount,
            interest_rate_bps: adjusted_interest_rate,
            loan_amount: amount,
            collateral_amount: max_loan_amount, // Use full amount as collateral
            risk_category: risk_category.clone(),
            status: LoanStatus::Active,
            created_timestamp: env.ledger().timestamp(),
            due_timestamp: env.ledger().timestamp() + loan_duration,
            last_payment_timestamp: None,
            remaining_balance: amount,
            repayment_schedule_id: None,
        };
        
        // Create repayment schedule
        let schedule_id = env.ledger().sequence();
        let schedule = RepaymentSchedule {
            schedule_id,
            position_id,
            total_amount: amount,
            payment_amount: amount / 10, // 10 payments over loan duration
            payment_frequency: loan_duration / 10,
            next_payment_due: env.ledger().timestamp() + (loan_duration / 10),
            total_payments_made: 0,
            remaining_payments: 10,
            is_active: true,
            created_timestamp: env.ledger().timestamp(),
            last_payment_timestamp: None,
        };
        
        // Update pool state
        pool.utilized_amount += amount;
        pool.available_amount -= amount;
        pool.last_activity = env.ledger().timestamp();
        
        // Store position and schedule
        env.storage().instance().set(&DataKey::LendingPosition(position_id), &position);
        env.storage().instance().set(&DataKey::RepaymentSchedule(schedule_id), &schedule);
        env.storage().instance().set(&DataKey::LendingPoolInfo(pool_id), &pool);
        
        // Update statistics
        let mut stats: LendingMarketStats = env.storage().instance()
            .get(&DataKey::LendingMarketStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.total_loans_issued += 1;
        stats.active_loans += 1;
        stats.total_volume_lent += amount;
        stats.last_updated = env.ledger().timestamp();
        
        // Update average loan size
        if stats.total_loans_issued > 0 {
            stats.average_loan_size = stats.total_volume_lent / stats.total_loans_issued as i128;
        }
        
        env.storage().instance().set(&DataKey::LendingMarketStats, &stats);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: borrower,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: position_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
        
        position_id
    }

    // Add liquidity to existing pool
    pub fn add_liquidity(
        env: Env,
        pool_id: u64,
        provider: Address,
        amount: i128,
        lock_duration: u64,
    ) -> u64 {
        // Get pool and validate
        let mut pool: LendingPoolInfo = env.storage().instance()
            .get(&DataKey::LendingPoolInfo(pool_id))
            .unwrap_or_else(|| panic!("Pool not found"));
        
        if !pool.is_active {
            panic!("Pool is not active");
        }
        
        // Get provider's circle
        let provider_circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(pool.lender_circle_id))
            .unwrap_or_else(|| panic!("Provider circle not found"));
        
        // Validate amount
        if amount < MIN_LENDING_AMOUNT {
            panic!("Amount below minimum");
        }
        
        // Create or update liquidity provider
        let provider_id = env.ledger().sequence();
        let provider = LiquidityProvider {
            provider_circle_id: pool.lender_circle_id,
            total_contributed: amount,
            current_locked: amount,
            unlock_timestamp: env.ledger().timestamp() + lock_duration,
            yield_rate_bps: LIQUIDITY_PROVIDER_YIELD_BPS,
            is_active: true,
            rewards_earned: 0,
            last_yield_compound: env.ledger().timestamp(),
        };
        
        // Update pool
        pool.total_liquidity += amount;
        pool.available_amount += amount;
        pool.last_activity = env.ledger().timestamp();
        
        // Store provider
        env.storage().instance().set(&DataKey::LiquidityProvider(provider_id), &provider);
        env.storage().instance().set(&DataKey::LendingPoolInfo(pool_id), &pool);
        
        provider_id
    }

    // Process loan repayment
    pub fn process_repayment(
        env: Env,
        position_id: u64,
        payment_amount: i128,
    ) {
        // Get position and validate
        let mut position: LendingPosition = env.storage().instance()
            .get(&DataKey::LendingPosition(position_id))
            .unwrap_or_else(|| panic!("Position not found"));
        
        if position.status != LoanStatus::Active {
            panic!("Loan is not active");
        }
        
        if payment_amount > position.remaining_balance {
            panic!("Payment exceeds remaining balance");
        }
        
        // Update position
        position.remaining_balance -= payment_amount;
        position.last_payment_timestamp = Some(env.ledger().timestamp());
        
        // Calculate interest for this payment
        let interest_portion = (payment_amount * position.interest_rate_bps) / 10000;
        let principal_portion = payment_amount - interest_portion;
        
        // Update repayment schedule
        if let Some(schedule_id) = position.repayment_schedule_id {
            let mut schedule: RepaymentSchedule = env.storage().instance()
                .get(&DataKey::RepaymentSchedule(schedule_id))
                .unwrap_or_else(|| panic!("Schedule not found"));
            
            schedule.total_payments_made += 1;
            schedule.remaining_payments -= 1;
            schedule.next_payment_due = env.ledger().timestamp() + schedule.payment_frequency;
            schedule.last_payment_timestamp = Some(env.ledger().timestamp());
            
            env.storage().instance().set(&DataKey::RepaymentSchedule(schedule_id), &schedule);
        }
        
        // Check if loan is fully repaid
        if position.remaining_balance == 0 {
            position.status = LoanStatus::Repaying;
            
            // Return collateral to borrower
            // In a real implementation, this would involve token transfers
            
            env.storage().instance().set(&DataKey::LendingPosition(position_id), &position);
            
            // Update pool availability (return principal + interest)
            pool = env.storage().instance()
                .get(&DataKey::LendingPoolInfo(position.lender_circle_id))
                .unwrap_or_else(|| panic!("Pool not found"));
            
            pool.available_amount += position.loan_amount;
            pool.utilized_amount -= position.loan_amount;
            env.storage().instance().set(&DataKey::LendingPoolInfo(position.lender_circle_id), &pool);
        }
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: position.borrower,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: position_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Get lending pool information
    pub fn get_lending_pool(env: Env, pool_id: u64) -> LendingPoolInfo {
        env.storage().instance()
            .get(&DataKey::LendingPoolInfo(pool_id))
            .unwrap_or_else(|| panic!("Pool not found"))
    }

    // Get lending position
    pub fn get_lending_position(env: Env, position_id: u64) -> LendingPosition {
        env.storage().instance()
            .get(&DataKey::LendingPosition(position_id))
            .unwrap_or_else(|| panic!("Position not found"))
    }

    // Get repayment schedule
    pub fn get_repayment_schedule(env: Env, schedule_id: u64) -> RepaymentSchedule {
        env.storage().instance()
            .get(&DataKey::RepaymentSchedule(schedule_id))
            .unwrap_or_else(|| panic!("Schedule not found"))
    }

    // Helper function to assess risk category
    fn assess_risk_category(user_stats: &UserStats) -> RiskCategory {
        // Simple risk assessment based on contribution history
        let total_contributions = user_stats.on_time_contributions + user_stats.late_contributions;
        
        if total_contributions == 0 {
            return RiskCategory::VeryHighRisk;
        }
        
        let on_time_rate = if total_contributions > 0 {
            (user_stats.on_time_contributions * 10000) / total_contributions
        } else {
            5000 // Default 50%
        };
        
        // Calculate reliability score (combination of on-time rate and volume)
        let reliability_score = (on_time_rate + 
            ((user_stats.total_volume_saved / 1000000).min(100) * 50)) / 100;
        
        if reliability_score >= 8000 && on_time_rate >= 9500 {
            RiskCategory::LowRisk
        } else if reliability_score >= 6000 && on_time_rate >= 8500 {
            RiskCategory::MediumRisk
        } else if reliability_score >= 4000 && on_time_rate >= 7000 {
            RiskCategory::HighRisk
        } else {
            RiskCategory::VeryHighRisk
        }
    }

    // Get lending market statistics
    pub fn get_lending_market_stats(env: Env) -> LendingMarketStats {
        env.storage().instance()
            .get(&DataKey::LendingMarketStats)
            .unwrap_or_else(|| panic!("Stats not found"))
    }

    // Emergency loan request
    pub fn request_emergency_loan(
        env: Env,
        requester_circle_id: u64,
        borrower_circle_id: u64,
        amount: i128,
        reason: String,
    ) -> u64 {
        // Get lending market config
        let config: LendingMarketConfig = env.storage().instance()
            .get(&DataKey::LendingMarketConfig)
            .unwrap_or_else(|| panic!("Lending market not initialized"));
        
        // Create emergency loan request
        let loan_id = env.ledger().sequence();
        let emergency_loan = EmergencyLoan {
            loan_id,
            requester_circle_id,
            borrower_circle_id,
            amount,
            reason,
            required_votes: config.emergency_quorum_bps,
            current_votes: 0,
            status: LendingMarketStatus::Active,
            created_timestamp: env.ledger().timestamp(),
            voting_deadline: env.ledger().timestamp() + LENDING_MARKET_EMERGENCY_PERIOD,
            execution_timestamp: None,
        };
        
        // Store emergency loan request
        env.storage().instance().set(&DataKey::EmergencyLoan(loan_id), &emergency_loan);
        
        loan_id
    }

    // Vote on emergency loan
    pub fn vote_emergency_loan(
        env: Env,
        loan_id: u64,
        vote: LendingVoteChoice,
    ) {
        // Get emergency loan and validate
        let mut loan: EmergencyLoan = env.storage().instance()
            .get(&DataKey::EmergencyLoan(loan_id))
            .unwrap_or_else(|| panic!("Emergency loan not found"));
        
        if loan.status != LendingMarketStatus::Active {
            panic!("Loan is not active");
        }
        
        if env.ledger().timestamp() > loan.voting_deadline {
            panic!("Voting period has ended");
        }
        
        // Update vote count
        loan.current_votes += 1;
        
        // Check if quorum is met
        let config: LendingMarketConfig = env.storage().instance()
            .get(&DataKey::LendingMarketConfig)
            .unwrap_or_else(|| panic!("Lending market not initialized"));
        
        let required_votes = (config.emergency_quorum_bps * 100) / 10000;
        
        if loan.current_votes >= required_votes {
            // Approve loan
            loan.status = LendingMarketStatus::Active;
            loan.execution_timestamp = Some(env.ledger().timestamp());
            
            // In a real implementation, this would trigger the actual loan disbursement
            env.events().publish(
                (Symbol::new(&env, "emergency_loan_approved"), loan_id),
                (amount, borrower_circle_id),
            );
        } else {
            // Reject loan
            loan.status = LendingMarketStatus::Paused;
            env.events().publish(
                (Symbol::new(&env, "emergency_loan_rejected"), loan_id),
                (loan.current_votes, required_votes),
            );
        }
        
        env.storage().instance().set(&DataKey::EmergencyLoan(loan_id), &loan);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: env.current_contract_address(),
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: loan_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Get emergency loan
    pub fn get_emergency_loan(env: Env, loan_id: u64) -> EmergencyLoan {
        env.storage().instance()
            .get(&DataKey::EmergencyLoan(loan_id))
            .unwrap_or_else(|| panic!("Emergency loan not found"))
    }
}
