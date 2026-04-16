#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, Address, Env, String};
use soroban_sdk::testutils::Ledger;
use sorosusu_contracts::{ProposalStatus, ProposalType, QuadraticVoteChoice, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNftQuadratic;

#[contractimpl]
impl MockNftQuadratic {
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
    let _nft_contract = env.register_contract(None, MockNftQuadratic);
    
    // Initialize contract
    client.init(&admin, &0);
    
    // Create large group (>= 10 members) - quadratic voting should be enabled
    let circle_id = client.create_circle(&creator, &1_000_000i128, &15u32, &token, &86400u64, &500i128);
    
    let proposer = Address::generate(&env);
    client.join_circle(&proposer, &circle_id);

    let title = String::from_str(&env, "Enabled");
    let description = String::from_str(&env, "Large group allows proposals");
    let execution_data = String::from_str(&env, "{}");

    let proposal_id = client.create_proposal(&proposer, &circle_id, &ProposalType::ChangeLateFee, &title, &description, &execution_data);
    assert!(proposal_id > 0);
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
    let _nft_contract = env.register_contract(None, MockNftQuadratic);
    
    // Initialize contract
    client.init(&admin, &0);
    
    // Create large group
    let circle_id = client.create_circle(&creator, &90_000_0i128, &10u32, &token, &86400u64, &0i128);
    
    // Join circle
    client.join_circle(&proposer, &circle_id);
    client.join_circle(&voter, &circle_id);
    
    // Create proposal
    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");
    
    let proposal_id = client.create_proposal(&proposer, &circle_id, &ProposalType::ChangeLateFee, &title, &description, &execution_data);
    
    client.update_voting_power(&voter, &circle_id, &10_000_000i128);
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
    let _nft_contract = env.register_contract(None, MockNftQuadratic);

    client.init(&admin, &0);
    let circle_id = client.create_circle(&creator, &90_000_0i128, &10u32, &token, &86400u64, &0i128);
    client.join_circle(&proposer, &circle_id);
    client.join_circle(&voter, &circle_id);

    let title = String::from_str(&env, "Test proposal");
    let description = String::from_str(&env, "Test description");
    let execution_data = String::from_str(&env, "{}");
    let proposal_id = client.create_proposal(&proposer, &circle_id, &ProposalType::ChangeLateFee, &title, &description, &execution_data);

    client.update_voting_power(&voter, &circle_id, &10000i128);

    let result = client.try_quadratic_vote(&voter, &proposal_id, &15u32, &QuadraticVoteChoice::For);
    assert!(result.is_err());
}
