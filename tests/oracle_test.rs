#![cfg(test)]
use soroban_sdk::{testutils::Address as _, testutils::Ledger as _, Address, Env, token, contract, contractimpl, Vec, Symbol};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

mod nft {
    use super::*;
    #[contract]
    pub struct MockNft;

    #[contractimpl]
    impl MockNft {
        pub fn mint(_env: Env, _to: Address, _id: u128) {}
        pub fn burn(_env: Env, _from: Address, _id: u128) {}
    }
}

mod badge {
    use super::*;
    #[contract]
    pub struct MockBadge;

    #[contractimpl]
    impl MockBadge {
        pub fn mint(_env: Env, _to: Address, _traits: Vec<Symbol>) {}
    }
}

#[test]
fn test_reliability_score_oracle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let other_user = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_id = env.register_contract(None, nft::MockNft);
    
    token_client.mint(&user, &10_000_000_000);
    token_client.mint(&other_user, &10_000_000_000);
    
    let amount: i128 = 1000_000_000; // 100 units
    let circle_id = client.create_circle(&user, &amount, &2, &token_id, &86400, &0, &nft_id, &0);
    let circle_id = client.create_circle(&user, &amount, &2, &token_id, &86400, &0, &nft_id);
    
    client.join_circle(&user, &circle_id, &1, &None);
    client.join_circle(&other_user, &circle_id, &1, &None);

    // Initial score should be 0
    assert_eq!(client.get_reliability_score(&user), 0);
    
    // --- ROUND 1 ---
    client.deposit(&user, &circle_id);
    client.deposit(&other_user, &circle_id);
    
    let score = client.get_reliability_score(&user);
    assert_eq!(score, 403); 

    client.finalize_round(&user, &circle_id);
    client.claim_pot(&user, &circle_id);
    
    // --- ROUND 2 ---
    env.ledger().set_timestamp(env.ledger().timestamp() + 100000); // Past deadline
    client.deposit(&user, &circle_id);
    client.deposit(&other_user, &circle_id);
    
    let score2 = client.get_reliability_score(&user);
    assert_eq!(score2, 206);
    
    client.finalize_round(&user, &circle_id);
    client.claim_pot(&other_user, &circle_id);
    
    let score_other = client.get_reliability_score(&other_user);
    assert_eq!(score_other, 236);
    
    println!("Oracle Reliability Score Test Passed!");
}

#[test]
fn test_badge_minting_after_long_cycle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);
    
    // Deploy mock Badge
    let badge_id = env.register_contract(None, badge::MockBadge);
    client.set_badge_contract(&admin, &badge_id);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_id = env.register_contract(None, nft::MockNft);
    
    // Create "12-month" circle using 2 members and long cycle duration
    // 2 members * 180 days = 360 days (~12 months)
    let amount: i128 = 500_000_000;
    let cycle_duration: u64 = 180 * 24 * 60 * 60; 
    let max_members = 2;
    let circle_id = client.create_circle(&creator, &amount, &max_members, &token_id, &cycle_duration, &0, &nft_id, &0);
    let circle_id = client.create_circle(&creator, &amount, &max_members, &token_id, &cycle_duration, &0, &nft_id);
    
    let user1 = creator.clone();
    let user2 = Address::generate(&env);
    
    token_client.mint(&user1, &10_000_000_000);
    token_client.mint(&user2, &10_000_000_000);
    
    client.join_circle(&user1, &circle_id, &1, &None);
    client.join_circle(&user2, &circle_id, &1, &None);
    
    // Round 1
    client.deposit(&user1, &circle_id);
    client.deposit(&user2, &circle_id);
    client.finalize_round(&user1, &circle_id);
    client.claim_pot(&user1, &circle_id);
    
    // Round 2
    env.ledger().set_timestamp(env.ledger().timestamp() + cycle_duration);
    client.deposit(&user1, &circle_id);
    client.deposit(&user2, &circle_id);
    client.finalize_round(&user1, &circle_id);
    client.claim_pot(&user2, &circle_id);
    
    println!("Badge Minting Trigger Test Completed Successfully!");
}

#[test]
#[should_panic(expected = "Insufficient reliability score")]
fn test_reputation_gate() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let nft_id = env.register_contract(None, nft::MockNft);
    
    let amount: i128 = 1000_000_000;
    // Require reputation score of 500
    let circle_id = client.create_circle(&admin, &amount, &2, &token_id, &86400, &0, &nft_id, &500);
    
    // User has 0 score, should panic
    client.join_circle(&user, &circle_id, &1, &None);
}
