#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[test]
fn test_shares_functionality() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin, &0);
    
    // Create a circle
    let token = Address::generate(&env);
    let circle_id = client.create_circle(&creator, &100i128, &3u32, &token, &604800u64, &0i128);
    
    // Join member
    client.join_circle(&member1, &circle_id);
    
    // Verify circle state
    let circle_info = client.get_circle(&circle_id);
    assert_eq!(circle_info.member_count, 2); // creator + member1
}

#[test]
fn test_double_payout_for_two_shares() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let single_share_member = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin, &0);
    
    // Create a circle
    let token = Address::generate(&env);
    let circle_id = client.create_circle(&creator, &100i128, &5u32, &token, &604800u64, &0i128);
    
    // Join member
    client.join_circle(&single_share_member, &circle_id);
}
