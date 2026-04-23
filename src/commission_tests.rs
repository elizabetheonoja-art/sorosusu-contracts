#![cfg(test)]

use soroban_sdk::{Address, Env, Symbol, token};
use crate::{
    SoroSusu, SoroSusuTrait, DataKey, Member, CircleInfo
};

#[test]
fn test_organizer_fee_validation() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Test valid organizer fee (1% = 100 bps)
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract.clone(),
        100,    // 1% organizer fee
    );

    let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert_eq!(circle.organizer_fee_bps, 100);
}

#[test]
#[should_panic(expected = "Organizer fee cannot exceed 100%")]
fn test_organizer_fee_too_high() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Test invalid organizer fee (> 100%)
    SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract,
        10001,  // 100.01% organizer fee (should panic)
    );
}

#[test]
fn test_zero_organizer_fee() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle with 0% organizer fee
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        3,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract,
        0,      // 0% organizer fee
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user3.clone(), circle_id);

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user3.clone(), circle_id);

    // Distribute payout (no commission should be taken)
    SoroSusu::distribute_payout(env.clone(), user1.clone(), circle_id);

    // Verify creator received no commission
    let token_client = token::Client::new(&env, &token_address);
    let creator_balance = token_client.balance(&creator);
    assert_eq!(creator_balance, 0);
}

#[test]
fn test_commission_calculation() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle with 5% organizer fee (500 bps)
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        3,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract,
        500,    // 5% organizer fee
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user3.clone(), circle_id);

    // Users make deposits (1000 each + 10 insurance fee each = 1010 each)
    // Total pool: 3030
    // Payout amount: 3000 (excluding insurance)
    // Commission: 5% of 3000 = 150
    // Net payout: 2850

    // Mint tokens to users for testing
    let token_client = token::Client::new(&env, &token_address);
    for user in [&user1, &user2, &user3] {
        token_client.mint(user, &1010);
    }

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user3.clone(), circle_id);

    // Get initial balances
    let creator_initial_balance = token_client.balance(&creator);
    let recipient_initial_balance = token_client.balance(&user1);

    // Distribute payout
    SoroSusu::distribute_payout(env.clone(), user1.clone(), circle_id);

    // Verify commission was paid to creator
    let creator_final_balance = token_client.balance(&creator);
    let commission_paid = creator_final_balance - creator_initial_balance;
    assert_eq!(commission_paid, 150); // 5% of 3000

    // Verify recipient received net payout
    let recipient_final_balance = token_client.balance(&user1);
    let net_payout_received = recipient_final_balance - recipient_initial_balance;
    assert_eq!(net_payout_received, 2850); // 3000 - 150 commission
}

#[test]
fn test_multiple_payouts_with_commission() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle with 2% organizer fee (200 bps)
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        2,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract,
        200,    // 2% organizer fee
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Mint tokens to users for testing
    let token_client = token::Client::new(&env, &token_address);
    for user in [&user1, &user2] {
        token_client.mint(user, &1010);
    }

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);

    // Get initial balances
    let creator_initial_balance = token_client.balance(&creator);

    // First payout (user1 should receive first as they joined first)
    SoroSusu::distribute_payout(env.clone(), user1.clone(), circle_id);

    // Second payout (user2 should receive second)
    SoroSusu::distribute_payout(env.clone(), user2.clone(), circle_id);

    // Verify total commission paid (2% of 2000 = 40 for each payout = 80 total)
    let creator_final_balance = token_client.balance(&creator);
    let total_commission = creator_final_balance - creator_initial_balance;
    assert_eq!(total_commission, 80); // 40 + 40
}

#[test]
#[should_panic(expected = "Not all members have contributed this cycle")]
fn test_payout_before_all_contributions() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

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
        nft_contract,
        100,    // 1% organizer fee
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Only user1 makes deposit
    let token_client = token::Client::new(&env, &token_address);
    token_client.mint(&user1, &1010);
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);

    // Try to distribute payout before user2 contributes (should panic)
    SoroSusu::distribute_payout(env.clone(), user1.clone(), circle_id);
}
