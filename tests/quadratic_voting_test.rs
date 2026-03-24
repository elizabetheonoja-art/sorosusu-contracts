#![cfg(test)]

use soroban_sdk::{contract, contractimpl, Address, Env, String};
use soroban_sdk::testutils::{Address as _, Ledger};
use sorosusu_contracts::{ProposalStatus, ProposalType, QuadraticVoteChoice, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_quadratic_voting_enabled_for_large_groups() {
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
    
    // Create large group (>= 10 members) - quadratic voting should be enabled
    let circle_id = client.create_circle(
        &creator,
        &50_000_0, // 50 XLM (below collateral threshold even at 15 members)
        &15u32,      // 15 members
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    let proposer = Address::generate(&env);
    client.join_circle(&proposer, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Enabled");
    let description = String::from_str(&env, "Large group allows proposals");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );
    assert!(proposal_id > 0);
    
    // Create small group (< 10 members) - quadratic voting should be disabled
        let small_creator = Address::generate(&env);
    let small_circle_id = client.create_circle(
            &small_creator,
        &50_000_0,
        &5u32,       // 5 members
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    let small_proposer = Address::generate(&env);
    client.join_circle(&small_proposer, &small_circle_id, &1u32, &None);

    let small_title = String::from_str(&env, "Disabled");
    let small_description = String::from_str(&env, "Small group rejects proposals");
    let small_execution_data = String::from_str(&env, "{}");

    let result = client.try_create_proposal(
        &small_proposer,
        &small_circle_id,
        &ProposalType::ChangeLateFee,
        &small_title,
        &small_description,
        &small_execution_data,
    );
    assert!(result.is_err());
}

#[test]
fn test_proposal_lifecycle_vote_and_execute() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);
    
    // Initialize contract
    client.init(&admin);
    
    // Create large group
    let circle_id = client.create_circle(&creator, &90_000_0, &10u32, &token, &86400u64, &100u32, &nft_contract);
    
    // Join circle
    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);
    
    // Create proposal
    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");
    
    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );
    
    client.update_voting_power(&voter, &circle_id, &10_000_000_0);
    client.quadratic_vote(&voter, &proposal_id, &2u32, &QuadraticVoteChoice::For);

    let voted = client.get_proposal(&proposal_id);
    assert_eq!(voted.status, ProposalStatus::Active);
    assert_eq!(voted.for_votes, 4);

    env.ledger().set_timestamp(voted.voting_end_timestamp + 1);
    
    // Execute proposal
    client.execute_proposal(&admin, &proposal_id);
    
    let proposal = client.get_proposal(&proposal_id);
    assert_eq!(proposal.status, ProposalStatus::Approved);
    assert_eq!(proposal.for_votes, 4);

    let stats = client.get_proposal_stats(&circle_id);
    assert_eq!(stats.executed_proposals, 0);
    assert_eq!(stats.approved_proposals, 1);
}

#[test]
fn test_vote_rejected_when_voting_power_insufficient() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);
    let circle_id = client.create_circle(&creator, &90_000_0, &10u32, &token, &86400u64, &100u32, &nft_contract);
    client.join_circle(&proposer, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");
    let proposal_id = client.create_proposal(
        &proposer,
        &circle_id,
        &ProposalType::ChangeLateFee,
        &title,
        &description,
        &execution_data,
    );

    client.update_voting_power(&voter, &circle_id, &1_000_0);

    let result = client.try_quadratic_vote(&voter, &proposal_id, &10u32, &QuadraticVoteChoice::For);
    assert!(result.is_err());
}
