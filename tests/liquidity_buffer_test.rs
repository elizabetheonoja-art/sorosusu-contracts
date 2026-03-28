#![cfg(test)]

use soroban_sdk::{Address, Env, String, Vec, Symbol};
use crate::{
    PotLiquidityBuffer, PotLiquidityBufferClient, LiquidityBufferConfig, 
    LiquidityAdvance, LiquidityAdvanceStatus, MemberAdvanceHistory, LiquidityBufferStats,
    PlatformFeeAllocation
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
    let member = Address::generate(&env);
    let circle_creator = Address::generate(&env);
    
    // Register liquidity buffer contract
    let contract_id = env.register_contract(None, PotLiquidityBuffer);
    let client = PotLiquidityBufferClient::new(&env, &contract_id);
    
    (env, admin, member, circle_creator)
}

#[test]
fn test_liquidity_buffer_initialization() {
    let (env, admin, _member, _creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Test initialization
    client.init_liquidity_buffer(&admin);
    
    // Verify config was set
    let config = client.get_liquidity_buffer_config();
    assert!(config.is_enabled);
    assert_eq!(config.advance_period, 172800); // 48 hours
    assert_eq!(config.min_reputation, 10000); // 100% reputation
    assert_eq!(config.max_advance_bps, 10000); // 100% of contribution
    assert_eq!(config.platform_fee_allocation, 2000); // 20% of platform fees
    assert_eq!(config.min_reserve, 1_000_000_000); // 100 tokens
    assert_eq!(config.max_reserve, 10_000_000_000); // 10,000 tokens
    assert_eq!(config.advance_fee_bps, 50); // 0.5% fee
    assert_eq!(config.grace_period, 86400); // 24 hours grace period
    assert_eq!(config.max_advances_per_round, 3); // 3 advances per round
    
    // Verify statistics were initialized
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 0);
    assert_eq!(stats.total_advances_provided, 0);
    assert_eq!(stats.active_advances_count, 0);
}

#[test]
fn test_advance_eligibility_check() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create a circle for testing
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env), // Mock token
        &86400, // 1 day
        &100, // 1% fee
        &Address::generate(&env), // Mock NFT
        &admin, // arbitrator
    );
    
    // Test eligibility with perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    // Store user stats (simulating perfect reputation)
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Check eligibility
    let is_eligible = client.check_advance_eligibility(&member, &circle_id);
    assert!(is_eligible);
    
    // Test with imperfect reputation
    let imperfect_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 8,
        late_contributions: 2, // 80% on-time rate
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &imperfect_stats);
    
    let is_eligible = client.check_advance_eligibility(&member, &circle_id);
    assert!(!is_eligible); // Should not be eligible with less than 100% reputation
}

#[test]
fn test_advance_request_and_provision() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer
    client.allocate_platform_fees_to_buffer(&5_000_000_000); // 500 tokens
    
    // Test advance request
    let advance_id = client.signal_advance_request(
        &member,
        &circle_id,
        &100_000_000, // 100 tokens contribution
        &String::from_str(&env, "Bank holiday weekend"),
    );
    
    // Verify advance was created
    let advance = client.get_liquidity_advance(&advance_id);
    assert_eq!(advance.member, member);
    assert_eq!(advance.circle_id, circle_id);
    assert_eq!(advance.contribution_amount, 100_000_000);
    assert_eq!(advance.advance_amount, 100_000_000);
    assert_eq!(advance.advance_fee, 50_000); // 0.5% of 100 tokens
    assert_eq!(advance.repayment_amount, 100_050_000);
    assert_eq!(advance.status, crate::LiquidityAdvanceStatus::Pending);
    
    // Test advance provision
    client.provide_advance(&advance_id);
    
    // Verify advance is now active
    let advance = client.get_liquidity_advance(&advance_id);
    assert_eq!(advance.status, crate::LiquidityAdvanceStatus::Active);
    assert!(advance.provided_timestamp.is_some());
    
    // Verify reserve was deducted
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 4_900_000_000); // 500 - 100
    assert_eq!(stats.active_advances_count, 1);
}

#[test]
fn test_advance_cancellation() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer
    client.allocate_platform_fees_to_buffer(&5_000_000_000);
    
    // Request advance
    let advance_id = client.signal_advance_request(
        &member,
        &circle_id,
        &100_000_000,
        &String::from_str(&env, "Test advance"),
    );
    
    // Cancel advance
    client.cancel_advance_request(&advance_id);
    
    // Verify advance was cancelled
    let advance = client.get_liquidity_advance(&advance_id);
    assert_eq!(advance.status, crate::LiquidityAdvanceStatus::Cancelled);
    
    // Verify statistics were updated
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_advances_provided, 0); // Should be 0 after cancellation
    
    // Verify member history was updated
    let history = client.get_member_advance_history(&member);
    assert_eq!(history.current_round_advances, 0);
    assert_eq!(history.total_advances_taken, 0);
}

#[test]
fn test_advance_refill_from_deposit() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer
    client.allocate_platform_fees_to_buffer(&5_000_000_000);
    
    // Request and provide advance
    let advance_id = client.signal_advance_request(
        &member,
        &circle_id,
        &100_000_000,
        &String::from_str(&env, "Test advance"),
    );
    
    client.provide_advance(&advance_id);
    
    // Process refill (simulate member deposit)
    client.process_advance_refill(&member, &circle_id, &100_050_000); // Full repayment amount
    
    // Verify advance is completed
    let advance = client.get_liquidity_advance(&advance_id);
    assert_eq!(advance.status, crate::LiquidityAdvanceStatus::Completed);
    assert!(advance.repaid_timestamp.is_some());
    
    // Verify reserve was refilled
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 5_000_000_000); // Should be back to original
    assert_eq!(stats.total_advances_completed, 1);
    assert_eq!(stats.total_fees_collected, 50_000);
    assert_eq!(stats.active_advances_count, 0);
    
    // Verify member history was updated
    let history = client.get_member_advance_history(&member);
    assert_eq!(history.total_fees_paid, 50_000);
    assert_eq!(history.current_round_advances, 0);
}

#[test]
fn test_platform_fee_allocation() {
    let (env, admin, _member, _creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Test platform fee allocation
    client.allocate_platform_fees_to_buffer(&1_000_000_000); // 100 tokens
    
    // Verify allocation
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 200_000_000); // 20% of 100 tokens
    assert_eq!(stats.total_platform_fees_allocated, 200_000_000);
    
    // Test maximum reserve limit
    client.allocate_platform_fees_to_buffer(&100_000_000_000); // 10,000 tokens
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 10_000_000_000); // Should be capped at max reserve
}

#[test]
fn test_advance_limits_and_validation() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer with minimal amount
    client.allocate_platform_fees_to_buffer(&200_000_000); // Only 20 tokens
    
    // Test insufficient reserve
    let result = std::panic::catch_unwind(|| {
        client.signal_advance_request(
            &member,
            &circle_id,
            &300_000_000, // 300 tokens, more than reserve
            &String::from_str(&env, "Too large advance"),
        );
    });
    assert!(result.is_err());
    
    // Test maximum advances per round
    client.allocate_platform_fees_to_buffer(&1_000_000_000); // Add more funds
    
    // Request maximum allowed advances
    for i in 1..=3 {
        client.signal_advance_request(
            &member,
            &circle_id,
            &100_000_000,
            &String::from_str(&env, &format!("Advance {}", i)),
        );
    }
    
    // Should fail on 4th attempt
    let result = std::panic::catch_unwind(|| {
        client.signal_advance_request(
            &member,
            &circle_id,
            &100_000_000,
            &String::from_str(&env, "Fourth advance"),
        );
    });
    assert!(result.is_err());
}

#[test]
fn test_member_advance_history_tracking() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer
    client.allocate_platform_fees_to_buffer(&5_000_000_000);
    
    // Request multiple advances
    let advance_id1 = client.signal_advance_request(
        &member,
        &circle_id,
        &100_000_000,
        &String::from_str(&env, "First advance"),
    );
    
    let advance_id2 = client.signal_advance_request(
        &member,
        &circle_id,
        &50_000_000,
        &String::from_str(&env, "Second advance"),
    );
    
    // Check member history
    let history = client.get_member_advance_history(&member);
    assert_eq!(history.total_advances_taken, 2);
    assert_eq!(history.total_advance_amount, 150_000_000);
    assert_eq!(history.current_round_advances, 2);
    assert_eq!(history.repayment_history.len(), 2);
    assert!(history.repayment_history.contains(&advance_id1));
    assert!(history.repayment_history.contains(&advance_id2));
}

#[test]
fn test_liquidity_buffer_statistics() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    // Set up perfect reputation
    let user_stats = crate::UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0,
    };
    
    env.storage().instance().set(&crate::DataKey::UserStats(member.clone()), &user_stats);
    
    // Fund the reserve buffer
    client.allocate_platform_fees_to_buffer(&5_000_000_000);
    
    // Request and provide multiple advances
    let advance_id1 = client.signal_advance_request(
        &member,
        &circle_id,
        &100_000_000,
        &String::from_str(&env, "Advance 1"),
    );
    
    let advance_id2 = client.signal_advance_request(
        &member,
        &circle_id,
        &200_000_000,
        &String::from_str(&env, "Advance 2"),
    );
    
    client.provide_advance(&advance_id1);
    client.provide_advance(&advance_id2);
    
    // Check statistics
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_reserve_balance, 4_700_000_000); // 500 - 100 - 200
    assert_eq!(stats.total_platform_fees_allocated, 1_000_000_000);
    assert_eq!(stats.total_advances_provided, 2);
    assert_eq!(stats.total_advance_amount, 300_000_000);
    assert_eq!(stats.active_advances_count, 2);
    assert_eq!(stats.average_advance_size, 150_000_000); // 300 / 2
    
    // Complete one advance
    client.process_advance_refill(&member, &circle_id, &100_050_000);
    
    // Check updated statistics
    let stats = client.get_liquidity_buffer_stats();
    assert_eq!(stats.total_advances_completed, 1);
    assert_eq!(stats.active_advances_count, 1);
    assert_eq!(stats.total_fees_collected, 50_000);
}

#[test]
fn test_edge_cases_and_error_handling() {
    let (env, admin, member, creator) = create_test_env();
    let client = PotLiquidityBufferClient::new(&env, &env.register_contract(None, PotLiquidityBuffer));
    
    // Test operations without initialization
    let result = std::panic::catch_unwind(|| {
        client.signal_advance_request(
            &member,
            &1,
            &100_000_000,
            &String::from_str(&env, "Test"),
        );
    });
    assert!(result.is_err());
    
    // Initialize
    client.init_liquidity_buffer(&admin);
    
    // Test with non-existent circle
    let result = std::panic::catch_unwind(|| {
        client.signal_advance_request(
            &member,
            &999, // Non-existent circle
            &100_000_000,
            &String::from_str(&env, "Test"),
        );
    });
    assert!(result.is_err());
    
    // Test with invalid advance ID
    let result = std::panic::catch_unwind(|| {
        client.get_liquidity_advance(&999);
    });
    assert!(result.is_err());
    
    // Test with zero contribution amount
    let circle_id = client.create_circle(
        &creator,
        &1000_000_000,
        &5,
        &Address::generate(&env),
        &86400,
        &100,
        &Address::generate(&env),
        &admin,
    );
    
    let result = std::panic::catch_unwind(|| {
        client.signal_advance_request(
            &member,
            &circle_id,
            &0, // Invalid amount
            &String::from_str(&env, "Test"),
        );
    });
    assert!(result.is_err());
}
