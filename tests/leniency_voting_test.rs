#![cfg(test)]

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use soroban_sdk::testutils::Address as _;
use sorosusu_contracts::{LeniencyRequestStatus, LeniencyVote, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_request_leniency_and_approval_updates_social_capital() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let requester = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle and join members
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );

    client.join_circle(&requester, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);
    
    // Request leniency
    let reason = String::from_str(&env, "Medical emergency - need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    let pending = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(pending.status, LeniencyRequestStatus::Pending);

    client.vote_on_leniency(&voter, &circle_id, &requester, &LeniencyVote::Approve);

    let approved = client.get_leniency_request(&circle_id, &requester);
    assert_eq!(approved.status, LeniencyRequestStatus::Approved);

    let requester_social = client.get_social_capital(&requester, &circle_id);
    assert_eq!(requester_social.leniency_received, 1);
    assert_eq!(requester_social.trust_score, 55);

    let voter_social = client.get_social_capital(&voter, &circle_id);
    assert_eq!(voter_social.leniency_given, 1);
    assert_eq!(voter_social.voting_participation, 1);
}

#[test]
fn test_cannot_vote_for_own_request() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let requester = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);
    let circle_id = client.create_circle(&creator, &100_000_0, &5u32, &token, &86400u64, &100u32, &nft_contract);
    client.join_circle(&requester, &circle_id, &1u32, &None);

    let reason = String::from_str(&env, "Need extra time");
    client.request_leniency(&requester, &circle_id, &reason);

    let result = client.try_vote_on_leniency(&requester, &circle_id, &requester, &LeniencyVote::Approve);
    assert!(result.is_err());
}
