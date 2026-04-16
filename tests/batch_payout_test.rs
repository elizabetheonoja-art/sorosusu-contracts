#![cfg(test)]
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{contract, contractimpl, Address, Env, token, Symbol, Vec};
use sorosusu_contracts::{SoroSusuClient, SoroSusu, CircleInfo, Member, BatchPayoutRecord, IndividualPayoutClaim, Error};

#[contract]
pub struct MockTokenSusu;

#[contractimpl]
impl MockTokenSusu {
    pub fn init_mock(_env: Env, _admin: Address) {}
    pub fn mint(_env: Env, _to: Address, _amount: i128) {}
    pub fn balance(_env: Env, _addr: Address) -> i128 { 1_000_000_000 }
}

#[contract]
pub struct MockNftSusu;

#[contractimpl]
impl MockNftSusu {
    pub fn init_mock_1(_env: Env, _admin: Address) {}
    pub fn mint_nft(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_configure_batch_payout_single_winner() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    let token_contract = env.register_contract(None, MockTokenSusu);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin, &0);
    
    let circle_id = client.create_circle(&creator, &100_000_000i128, &10u32, &token_contract, &86400u64, &100i128);
    
    // Add members
    client.join_circle(&creator, &circle_id);
    client.join_circle(&user1, &circle_id);
    client.join_circle(&user2, &circle_id);
    
    // Configure batch payout with 1 winner
    client.configure_batch_payout(&creator, &circle_id, &1);
    
    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.winners_per_round, 1);
}

#[test]
fn test_configure_batch_payout_multiple_winners() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    
    let token_contract = env.register_contract(None, MockTokenSusu);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin, &0);
    
    let circle_id = client.create_circle(&creator, &100_000_000i128, &10u32, &token_contract, &86400u64, &100i128);
    
    for _ in 0..5 {
        let user = Address::generate(&env);
        client.join_circle(&user, &circle_id);
    }
    
    client.configure_batch_payout(&creator, &circle_id, &5);
    
    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.winners_per_round, 5);
}

#[test]
fn test_batch_payout_two_winners_precise_math() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    
    let token_contract = env.register_contract(None, MockTokenSusu);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin, &0);
    
    let contribution_amount = 100_000_000i128;
    let circle_id = client.create_circle(&creator, &contribution_amount, &4u32, &token_contract, &86400u64, &100i128);
    
    client.join_circle(&creator, &circle_id);
    client.join_circle(&user1, &circle_id);
    client.join_circle(&user2, &circle_id);
    client.join_circle(&user3, &circle_id);
    
    client.configure_batch_payout(&creator, &circle_id, &2);
    
    client.deposit(&creator, &circle_id, &1);
    client.deposit(&user1, &circle_id, &1);
    client.deposit(&user2, &circle_id, &1);
    client.deposit(&user3, &circle_id, &1);
    
    client.finalize_round(&creator, &circle_id);
    client.distribute_batch_payout(&creator, &circle_id);
    
    let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
    assert_eq!(batch_record.total_winners, 2);
}
