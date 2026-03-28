#![cfg(test)]

use soroban_sdk::{Address, Env, String, Vec, Symbol};
use crate::{
    InterSusuLendingMarket, InterSusuLendingMarketClient, LendingMarketConfig, 
    LendingPoolInfo, LendingPosition, RepaymentSchedule, LendingMarketStats,
    EmergencyLoan, LendingVoteChoice, RiskCategory, LiquidityProvider
};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn mint(_env: Env, _to: Address, _amount: i128) {}
    pub fn burn(_env: Env, _from: Address, _amount: i128) {}
    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
    pub fn balance(_env: Env, _account: Address) -> i128 { 1000000_000_000 }
}

fn create_test_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let lender_circle_creator = Address::generate(&env);
    let borrower_circle_creator = Address::generate(&env);
    let provider = Address::generate(&env);
    
    // Register lending market contract
    let contract_id = env.register_contract(None, InterSusuLendingMarket);
    let client = InterSusuLendingMarketClient::new(&env, &contract_id);
    
    (env, admin, lender_circle_creator, borrower_circle_creator)
}

#[test]
fn test_lending_market_initialization() {
    let (env, admin, _lender_creator, _borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Test initialization
    client.init_lending_market(&admin);
    
    // Verify config was set
    let config = client.get_lending_market_config();
    assert!(config.is_enabled);
    assert!(!config.emergency_mode);
    assert_eq!(config.min_participation_bps, 4000); // 40%
    assert_eq!(config.quorum_bps, 6000); // 60%
    assert_eq!(config.emergency_quorum_bps, 8000); // 80%
    assert_eq!(config.max_ltv_ratio, 9000); // 90%
    assert_eq!(config.base_interest_rate_bps, 500); // 5%
    assert_eq!(config.risk_adjustment_bps, 500); // 5%
}

#[test]
fn test_lending_pool_creation() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Initialize
    client.init_lending_market(&admin);
    
    // Create circles for testing
    let lender_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env), // Mock token
        &86400, // 1 day
        &100, // 1% fee
        &Address::generate(&env), // Mock NFT
        &admin, // arbitrator
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env), // Mock token
        &86400, // 1 day
        &100, // 1% fee
        &Address::generate(&env), // Mock NFT
        &admin, // arbitrator
    );
    
    // Test pool creation
    let pool_id = client.create_lending_pool(
        &lender_circle_id,
        &borrower_circle_id,
        &500_000_000, // 500 tokens initial liquidity
    );
    
    // Verify pool was created
    let pool = client.get_lending_pool(&pool_id);
    assert_eq!(pool.lender_circle_id, lender_circle_id);
    assert_eq!(pool.borrower_circle_id, borrower_circle_id);
    assert_eq!(pool.total_liquidity, 500_000_000);
    assert_eq!(pool.available_amount, 500_000_000);
    assert_eq!(pool.utilized_amount, 0);
    assert!(pool.participant_count, 2);
    assert!(pool.is_active);
    
    // Verify market stats
    let stats = client.get_lending_market_stats();
    assert_eq!(stats.total_pools_created, 1);
    assert_eq!(stats.active_pools, 1);
    assert_eq!(stats.total_loans_issued, 0);
}

#[test]
fn test_lending_from_pool() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Setup
    client.init_lending_market(&admin);
    
    let lender_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Create pool
    let pool_id = client.create_lending_pool(
        &lender_circle_id,
        &borrower_circle_id,
        &1000_000_000,
    );
    
    // Create borrower with good reputation
    let borrower = Address::generate(&env);
    
    // Test successful lending
    let position_id = client.lend_from_pool(
        &pool_id,
        &borrower,
        &100_000_000, // 100 tokens
        &1209600, // 14 days
    );
    
    // Verify position
    let position = client.get_lending_position(&position_id);
    assert_eq!(position.borrower, borrower);
    assert_eq!(position.principal_amount, 100_000_000);
    assert_eq!(position.loan_amount, 100_000_000);
    assert_eq!(position.remaining_balance, 100_000_000);
    assert_eq!(position.status, crate::LoanStatus::Active);
    
    // Verify pool state updated
    let pool = client.get_lending_pool(&pool_id);
    assert_eq!(pool.utilized_amount, 100_000_000);
    assert_eq!(pool.available_amount, 400_000_000); // 500 - 100
    assert_eq!(pool.participant_count, 2);
    
    // Verify stats
    let stats = client.get_lending_market_stats();
    assert_eq!(stats.total_loans_issued, 1);
    assert_eq!(stats.active_loans, 1);
    assert_eq!(stats.total_volume_lent, 100_000_000);
}

#[test]
fn test_lending_risk_assessment() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Test different risk categories
    let test_cases = vec![
        (3000, crate::RiskCategory::LowRisk),      // Excellent credit
        (2000, crate::RiskCategory::MediumRisk),   // Good credit
        (1000, crate::RiskCategory::HighRisk),     // Fair credit
        (500, crate::RiskCategory::VeryHighRisk),  // Poor credit
    ];
    
    for (credit_score, expected_risk) in test_cases {
        // Create a mock user stats for testing
        let user_stats = crate::UserStats {
            total_volume_saved: credit_score * 1000, // Simulate volume
            on_time_contributions: if credit_score >= 2000 { 10 } else { 5 },
            late_contributions: if credit_score >= 2000 { 0 } else { 5 },
        };
        
        // Test risk assessment
        let assessed_risk = client.assess_risk_category(&user_stats);
        assert_eq!(assessed_risk, expected_risk);
    }
}

#[test]
fn test_liquidity_provision() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Setup
    client.init_lending_market(&admin);
    
    let lender_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Create pool
    let pool_id = client.create_lending_pool(
        &lender_circle_id,
        &borrower_circle_id,
        &100_000_000,
    );
    
    // Test adding liquidity
    let provider = Address::generate(&env);
    let provider_id = client.add_liquidity(
        &pool_id,
        &provider,
        &200_000_000, // 200 tokens
        &604800, // 7 days lock
    );
    
    // Verify liquidity provider
    // Note: This would require a separate get_liquidity_provider function
    // For now, we test that the function completes without error
    assert!(provider_id > 0);
}

#[test]
fn test_repayment_processing() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Setup
    client.init_lending_market(&admin);
    
    let lender_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Create pool and position
    let pool_id = client.create_lending_pool(
        &lender_circle_id,
        &borrower_circle_id,
        &100_000_000,
    );
    
    let borrower = Address::generate(&env);
    let position_id = client.lend_from_pool(
        &pool_id,
        &borrower,
        &100_000_000,
        &1209600,
    );
    
    // Test partial repayment
    client.process_repayment(&position_id, &10_000_000);
    
    // Verify position updated
    let position = client.get_lending_position(&position_id);
    assert_eq!(position.remaining_balance, 90_000_000);
    assert_eq!(position.last_payment_timestamp, Some(env.ledger().timestamp()));
    
    // Test full repayment
    client.process_repayment(&position_id, &90_000_000);
    
    // Verify position is fully repaid
    let position = client.get_lending_position(&position_id);
    assert_eq!(position.remaining_balance, 0);
    assert_eq!(position.status, crate::LoanStatus::Repaying);
}

#[test]
fn test_emergency_loan_system() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Setup
    client.init_lending_market(&admin);
    
    let requester_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Test emergency loan request
    let loan_id = client.request_emergency_loan(
        &requester_circle_id,
        &borrower_circle_id,
        &50_000_000,
        &String::from_str(&env, "Emergency medical expense"),
    );
    
    // Test voting
    let borrower = Address::generate(&env);
    client.vote_emergency_loan(&loan_id, &crate::LendingVoteChoice::Approve);
    client.vote_emergency_loan(&loan_id, &crate::LendingVoteChoice::Approve);
    
    // Check if loan is approved (80% quorum = 0.8 * 2 = 1.6, need 2 votes)
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.current_votes, 2);
    assert_eq!(loan.status, crate::LendingMarketStatus::Active);
    
    // Test rejection
    let rejector = Address::generate(&env);
    client.vote_emergency_loan(&loan_id, &crate::LendingVoteChoice::Reject);
    
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.current_votes, 3);
    assert_eq!(loan.status, crate::LendingMarketStatus::Paused);
}

#[test]
fn test_lending_market_statistics() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Initialize
    client.init_lending_market(&admin);
    
    // Create some test data
    for i in 1..=3 {
        let lender_circle_id = client.create_circle(
            &lender_creator,
            &1000_000_000,
            &5,
            &Address::generate(&env),
            &86400,
            &100,
            &Address::generate(&env),
            &admin,
        );
        
        let borrower_circle_id = client.create_circle(
            &borrower_creator,
            &1000_000_000,
            &5,
            &Address::generate(&env),
            &86400,
            &100,
            &Address::generate(&env),
            &admin,
        );
        
        let pool_id = client.create_lending_pool(
            &lender_circle_id,
            &borrower_circle_id,
            &(i * 100_000_000),
        );
        
        // Create some loans
        for j in 1..=2 {
            let borrower = Address::generate(&env);
            client.lend_from_pool(
                &pool_id,
                &borrower,
                &(50_000_000),
                &1209600,
            );
        }
    }
    
    // Check statistics
    let stats = client.get_lending_market_stats();
    assert_eq!(stats.total_pools_created, 3);
    assert_eq!(stats.active_pools, 3);
    assert_eq!(stats.total_loans_issued, 6);
    assert_eq!(stats.total_volume_lent, 300_000_000);
    assert!(stats.average_loan_size, 50_000_000);
}

#[test]
fn test_lending_market_edge_cases() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Initialize
    client.init_lending_market(&admin);
    
    // Test minimum amount violation
    let lender_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let pool_id = client.create_lending_pool(
        &lender_circle_id,
        &borrower_circle_id,
        &100_000_000,
    );
    
    let borrower = Address::generate(&env);
    
    // Should panic for amount below minimum
    let result = std::panic::catch_unwind(|| {
        client.lend_from_pool(
            &pool_id,
            &borrower,
            &50_000_000, // Below MIN_LENDING_AMOUNT (100_000_000)
            &1209600,
        );
    });
    assert!(result.is_err());
    
    // Should panic for insufficient pool liquidity
    let result = std::panic::catch_unwind(|| {
        client.lend_from_pool(
            &pool_id,
            &Address::generate(&env),
            &200_000_000, // More than available (100_000_000)
            &1209600,
        );
    });
    assert!(result.is_err());
    
    // Should panic for excessive LTV ratio
    let result = std::panic::catch_unwind(|| {
        client.lend_from_pool(
            &pool_id,
            &Address::generate(&env),
            &150_000_000, // Would exceed 90% LTV for most collateral
            &1209600,
        );
    });
    assert!(result.is_err());
}

#[test]
fn test_lending_market_configuration() {
    let (env, admin, lender_creator, borrower_creator) = create_test_env();
    let client = InterSusuLendingMarketClient::new(&env, &env.register_contract(None, InterSusuLendingMarket));
    
    // Initialize with custom config
    client.init_lending_market(&admin);
    
    // Test emergency mode activation
    // Note: This would require admin functions to toggle emergency mode
    // For now, test that emergency loans can be created
    
    let requester_circle_id = client.create_circle(
        &lender_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let borrower_circle_id = client.create_circle(
        &borrower_creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Emergency loan should work even in normal mode
    let loan_id = client.request_emergency_loan(
        &requester_circle_id,
        &borrower_circle_id,
        &25_000_000,
        &String::from_str(&env, "Test emergency loan"),
    );
    
    let loan = client.get_emergency_loan(&loan_id);
    assert_eq!(loan.amount, 25_000_000);
    assert!(loan.status, crate::LendingMarketStatus::Active);
}
