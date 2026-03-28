#![cfg(test)]

use soroban_sdk::{Address, Env, Symbol, token};
use crate::{
    SoroSusu, SoroSusuTrait, DataKey, Member, CircleInfo, GasBufferConfig
};

#[test]
fn test_gas_buffer_funding() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let funder = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Fund gas buffer with 1 XLM (10000000 stroops)
    let funding_amount = 10000000i128;
    
    // Mock XLM balance for funder
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&funder, &funding_amount);

    // Fund the gas buffer
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, funding_amount);

    // Check gas buffer balance
    let buffer_balance = SoroSusu::get_gas_buffer_balance(env.clone(), circle_id);
    assert_eq!(buffer_balance, funding_amount);

    // Verify circle state
    let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert_eq!(circle.gas_buffer_balance, funding_amount);
    assert!(circle.gas_buffer_enabled);
}

#[test]
fn test_gas_buffer_config_management() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Create custom gas buffer configuration
    let custom_config = GasBufferConfig {
        min_buffer_amount: 5000000,     // 0.005 XLM minimum
        max_buffer_amount: 500000000,   // 5 XLM maximum
        auto_refill_threshold: 2500000, // 0.0025 XLM threshold
        emergency_buffer: 25000000,     // 0.25 XLM emergency buffer
    };

    // Set custom configuration
    SoroSusu::set_gas_buffer_config(env.clone(), circle_id, custom_config.clone());

    // Verify configuration was stored
    let stored_config: GasBufferConfig = env.storage().instance()
        .get(&DataKey::GasBufferConfig(circle_id))
        .unwrap();
    
    assert_eq!(stored_config.min_buffer_amount, custom_config.min_buffer_amount);
    assert_eq!(stored_config.max_buffer_amount, custom_config.max_buffer_amount);
    assert_eq!(stored_config.auto_refill_threshold, custom_config.auto_refill_threshold);
    assert_eq!(stored_config.emergency_buffer, custom_config.emergency_buffer);
}

#[test]
#[should_panic(expected = "Amount exceeds maximum gas buffer limit")]
fn test_gas_buffer_overfunding_protection() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Try to fund beyond maximum limit (default is 10 XLM = 1000000000 stroops)
    let excessive_amount = 2000000000i128; // 20 XLM
    
    // Mock XLM balance
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&creator, &excessive_amount);

    // This should panic due to exceeding maximum buffer limit
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, excessive_amount);
}

#[test]
fn test_payout_with_gas_buffer_protection() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        2,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Fund gas buffer sufficiently
    let funding_amount = 50000000i128; // 0.5 XLM
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&creator, &funding_amount);
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, funding_amount);

    // Mock token for contributions
    let token_client = token::Client::new(&env, &token_address);
    token_client.mint(&user1, &2000); // 1000 + 10 insurance
    token_client.mint(&user2, &2000);

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id, None);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id, None);

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);

    // Finalize round
    SoroSusu::finalize_round(env.clone(), creator.clone(), circle_id);

    // Get initial gas buffer balance
    let initial_buffer_balance = SoroSusu::get_gas_buffer_balance(env.clone(), circle_id);

    // Execute payout (should succeed with gas buffer protection)
    SoroSusu::distribute_payout(env.clone(), admin.clone(), circle_id);

    // Verify gas buffer was reduced (gas cost deducted)
    let final_buffer_balance = SoroSusu::get_gas_buffer_balance(env.clone(), circle_id);
    assert!(final_buffer_balance < initial_buffer_balance);

    // Verify payout was executed by checking user balances
    let user1_balance = token_client.balance(&user1);
    let user2_balance = token_client.balance(&user2);
    
    // One user should have received the payout (net of organizer fee)
    let expected_net_payout = 2000 - 20; // 2000 - 1% organizer fee
    assert!(user1_balance == expected_net_payout || user2_balance == expected_net_payout);
}

#[test]
#[should_panic(expected = "Insufficient gas buffer for payout")]
fn test_payout_fails_without_gas_buffer() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        2,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Mock token for contributions
    let token_client = token::Client::new(&env, &token_address);
    token_client.mint(&user1, &2000); // 1000 + 10 insurance
    token_client.mint(&user2, &2000);

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id, None);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id, None);

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);

    // Finalize round
    SoroSusu::finalize_round(env.clone(), creator.clone(), circle_id);

    // Try to execute payout without gas buffer (should fail)
    SoroSusu::distribute_payout(env.clone(), admin.clone(), circle_id);
}

#[test]
fn test_emergency_gas_buffer_usage() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        2,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Set custom config with low threshold and emergency buffer
    let custom_config = GasBufferConfig {
        min_buffer_amount: 1000000,      // 0.001 XLM minimum
        max_buffer_amount: 100000000,    // 1 XLM maximum
        auto_refill_threshold: 3000000,  // 0.003 XLM threshold
        emergency_buffer: 10000000,     // 0.01 XLM emergency buffer
    };
    SoroSusu::set_gas_buffer_config(env.clone(), circle_id, custom_config);

    // Fund gas buffer with just below threshold amount
    let funding_amount = 2500000i128; // Below auto_refill_threshold
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&creator, &funding_amount);
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, funding_amount);

    // Mock token for contributions
    let token_client = token::Client::new(&env, &token_address);
    token_client.mint(&user1, &2000); // 1000 + 10 insurance
    token_client.mint(&user2, &2000);

    // Users join circle and make deposits
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id, None);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id, None);
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);

    // Finalize round
    SoroSusu::finalize_round(env.clone(), creator.clone(), circle_id);

    // Execute payout (should use emergency buffer)
    SoroSusu::distribute_payout(env.clone(), admin.clone(), circle_id);

    // Verify emergency buffer was used (check events or final state)
    let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert!(circle.gas_buffer_balance > 0); // Should have emergency buffer added
}

#[test]
fn test_gas_buffer_disabled_circle() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Manually disable gas buffer for this circle (for testing)
    let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    circle.gas_buffer_enabled = false;
    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

    // Funding gas buffer should still work but won't be used for payouts
    let funding_amount = 10000000i128;
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&creator, &funding_amount);
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, funding_amount);

    // Verify buffer was funded
    let buffer_balance = SoroSusu::get_gas_buffer_balance(env.clone(), circle_id);
    assert_eq!(buffer_balance, funding_amount);

    // Verify gas buffer is disabled
    let updated_circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert!(!updated_circle.gas_buffer_enabled);
}

#[test]
fn test_gas_buffer_events() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        arbitrator.clone(),
        100,    // 1% organizer fee
    );

    // Fund gas buffer
    let funding_amount = 10000000i128;
    let xlm_token = env.native_token();
    let xlm_client = token::Client::new(&env, &xlm_token);
    xlm_client.mint(&creator, &funding_amount);
    SoroSusu::fund_gas_buffer(env.clone(), circle_id, funding_amount);

    // Check for gas_buffer_funded event
    let events = env.events().all();
    let gas_buffer_funded_events: Vec<_> = events
        .iter()
        .filter(|event| {
            event.topics[0] == Symbol::new(&env, "gas_buffer_funded")
        })
        .collect();

    assert!(!gas_buffer_funded_events.is_empty());
    
    // Verify event data
    let event = &gas_buffer_funded_events[0];
    assert_eq!(event.data[0], circle_id);
    assert_eq!(event.data[1], funding_amount);
    assert_eq!(event.data[2], funding_amount); // balance after funding
}
