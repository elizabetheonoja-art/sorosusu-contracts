#![cfg(test)]

use soroban_sdk::{contract, contractimpl, Address, Env};
use soroban_sdk::testutils::Address as _;
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_collateral_required_for_high_value_circles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);
    
    // Initialize contract
    client.init(&admin);
    
    // Create a high-value circle (above threshold)
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64, // 1 day cycle
        &100u32,   // 1% insurance fee
        &nft_contract,
    );
    
    // Joining should fail without prior collateral stake for high-value circles.
    let user = Address::generate(&env);
    let result = client.try_join_circle(&user, &circle_id, &1u32, &None);
    assert!(result.is_err());
}

#[test]
fn test_join_circle_rejected_without_collateral_when_required() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);
    
    // Initialize contract
    client.init(&admin);
    
    // Create a high-value circle (collateral required)
    let high_amount = 2_000_000_0;
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    let result = client.try_join_circle(&user, &circle_id, &1u32, &None);
    assert!(result.is_err());
}

#[test]
fn test_join_circle_succeeds_for_low_value_without_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);
    let circle_id = client.create_circle(&creator, &100_000_0, &5u32, &token, &86400u64, &100u32, &nft_contract);

    // Low-value circle should not require collateral at join time.
    client.join_circle(&user, &circle_id, &1u32, &None);
}
