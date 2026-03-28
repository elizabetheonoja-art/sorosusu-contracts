use soroban_sdk::{Address, Env, String, Symbol};
use sorosusu_contracts::{SoroSusu, SoroSusuTrait, DataKey, DissolutionVoteChoice, DissolutionStatus, RefundStatus};

#[test]
fn test_initiate_dissolution() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0, // 100 XLM
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Global crisis - need emergency exit");
    client.initiate_dissolution(&initiator, &circle_id, reason);
    
    // Verify dissolution was initiated
    let proposal = client.get_dissolution_proposal(circle_id);
    assert_eq!(proposal.initiator, initiator);
    assert_eq!(proposal.circle_id, circle_id);
    assert_eq!(proposal.status, DissolutionStatus::Voting);
    assert_eq!(proposal.approve_votes, 0);
    assert_eq!(proposal.reject_votes, 0);
    
    // Check circle status
    let circle_key = DataKey::Circle(circle_id);
    let circle = env.storage().instance().get::<_, sorosusu_contracts::CircleInfo>(&circle_key).unwrap();
    assert_eq!(circle.dissolution_status, DissolutionStatus::Voting);
    assert!(circle.dissolution_deadline.is_some());
}

#[test]
fn test_dissolution_double_initiation_prevention() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator1 = Address::generate(&env);
    let initiator2 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator1, &circle_id, &1u32, &None);
    client.join_circle(&initiator2, &circle_id, &1u32, &None);
    
    // First initiation
    let reason = String::from_str(&env, "First dissolution attempt");
    client.initiate_dissolution(&initiator1, &circle_id, reason);
    
    // Try second initiation - should fail
    let reason2 = String::from_str(&env, "Second dissolution attempt");
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "initiate_dissolution"),
        (initiator2, circle_id, reason2),
    );
    assert!(result.is_err());
}

#[test]
fn test_vote_to_dissolve_supermajority() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    let voter4 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);
    client.join_circle(&voter3, &circle_id, &1u32, &None);
    client.join_circle(&voter4, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Need emergency exit");
    client.initiate_dissolution(&initiator, &circle_id, reason);
    
    // Vote for dissolution (need 75% = 4/5 votes)
    client.vote_to_dissolve(&voter1, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter2, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter3, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter4, &circle_id, &DissolutionVoteChoice::Approve);
    
    // Verify dissolution was approved
    let proposal = client.get_dissolution_proposal(circle_id);
    assert_eq!(proposal.status, DissolutionStatus::Approved);
    assert_eq!(proposal.approve_votes, 4);
    assert_eq!(proposal.reject_votes, 0);
    assert!(proposal.dissolution_timestamp.is_some());
    
    // Check circle status
    let circle_key = DataKey::Circle(circle_id);
    let circle = env.storage().instance().get::<_, sorosusu_contracts::CircleInfo>(&circle_key).unwrap();
    assert_eq!(circle.dissolution_status, DissolutionStatus::Approved);
}

#[test]
fn test_vote_to_dissolve_insufficient_majority() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator = Address::generate(&env);
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    let voter4 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator, &circle_id, &1u32, &None);
    client.join_circle(&voter1, &circle_id, &1u32, &None);
    client.join_circle(&voter2, &circle_id, &1u32, &None);
    client.join_circle(&voter3, &circle_id, &1u32, &None);
    client.join_circle(&voter4, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Need emergency exit");
    client.initiate_dissolve(&initiator, &circle_id, reason);
    
    // Vote with insufficient majority (need 75%, only get 60%)
    client.vote_to_dissolve(&voter1, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter2, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter3, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&voter4, &circle_id, &DissolutionVoteChoice::Reject);
    
    // Verify still in voting phase
    let proposal = client.get_dissolution_proposal(circle_id);
    assert_eq!(proposal.status, DissolutionStatus::Voting);
    assert_eq!(proposal.approve_votes, 3);
    assert_eq!(proposal.reject_votes, 1);
    
    // Advance time and finalize
    env.ledger().set_timestamp(env.ledger().timestamp() + 13000000); // 15+ days later
    client.finalize_dissolution(&admin, &circle_id);
    
    // Should fail due to insufficient majority
    let final_proposal = client.get_dissolution_proposal(circle_id);
    assert_eq!(final_proposal.status, DissolutionStatus::Failed);
}

#[test]
fn test_double_voting_prevention() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Need emergency exit");
    client.initiate_dissolve(&initiator, &circle_id, reason);
    
    // Vote once
    client.vote_to_dissolve(&voter, &circle_id, &DissolutionVoteChoice::Approve);
    
    // Try to vote again - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "vote_to_dissolve"),
        (voter, circle_id, DissolutionVoteChoice::Reject),
    );
    assert!(result.is_err());
}

#[test]
fn test_net_position_calculation() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &3u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&member1, &circle_id, &1u32, &None);
    client.join_circle(&member2, &circle_id, &1u32, &None);
    client.join_circle(&member3, &circle_id, &1u32, &None);
    
    // Simulate some contributions (in real implementation, this would be done through deposits)
    // For this test, we'll directly trigger dissolution to test net position calculation
    
    // Initiate dissolution with supermajority
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&member1, &circle_id, reason);
    client.vote_to_dissolve(&member1, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&member2, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&member3, &circle_id, &DissolutionVoteChoice::Approve);
    
    // Verify net positions were calculated
    let dissolved_circle = client.get_dissolved_circle(circle_id);
    assert!(dissolved_circle.total_contributions > 0);
    assert_eq!(dissolved_circle.total_members, 3);
    assert_eq!(dissolved_circle.dissolution_status, DissolutionStatus::Refunding);
}

#[test]
fn test_refund_claim_for_unreimbursed_member() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let recipient = Address::generate(&env);
    let unreimbursed = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &2u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&recipient, &circle_id, &1u32, &None);
    client.join_circle(&unreimbursed, &circle_id, &1u32, &None);
    
    // Initiate dissolution with supermajority
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&recipient, &circle_id, reason);
    client.vote_to_dissolve(&recipient, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&unreimbursed, &circle_id, &DissolutionVoteChoice::Approve);
    
    // In a real implementation, we'd need to set up the net positions properly
    // For this test, we'll verify the refund claim structure exists
    
    // Check net position for unreimbursed member
    let net_position = client.get_net_position(&unreimbursed, circle_id);
    assert_eq!(net_position.member, unreimbursed);
    assert_eq!(net_position.circle_id, circle_id);
    assert!(!net_position.has_received_pot); // Should not have received pot
    assert!(!net_position.refund_claimed);
    
    // Check refund claim
    let refund_claim = client.get_refund_claim(&unreimbursed, circle_id);
    assert_eq!(refund_claim.member, unreimbursed);
    assert_eq!(refund_claim.circle_id, circle_id);
    assert_eq!(refund_claim.status, RefundStatus::Pending);
}

#[test]
fn test_cannot_refund_pot_recipient() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let recipient = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &2u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&recipient, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&recipient, &circle_id, reason);
    
    // Try to claim refund as recipient - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "claim_refund"),
        (recipient, circle_id),
    );
    assert!(result.is_err());
}

#[test]
fn test_dissolution_voting_period_expiration() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let initiator = Address::generate(&env);
    let voter = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &5u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&initiator, &circle_id, &1u32, &None);
    client.join_circle(&voter, &circle_id, &1u32, &None);
    
    // Initiate dissolution
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&initiator, &circle_id, reason);
    
    // Advance time past voting period
    env.ledger().set_timestamp(env.ledger().timestamp() + 13000000); // 15+ days later
    
    // Try to vote - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "vote_to_dissolve"),
        (voter, circle_id, DissolutionVoteChoice::Approve),
    );
    assert!(result.is_err());
}

#[test]
fn test_dissolved_circle_statistics() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &3u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&member1, &circle_id, &1u32, &None);
    client.join_circle(&member2, &circle_id, &1u32, &None);
    client.join_circle(&member3, &circle_id, &1u32, &None);
    
    // Initiate dissolution with supermajority
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&member1, &circle_id, reason);
    client.vote_to_dissolve(&member1, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&member2, &circle_id, &DissolutionVoteChoice::Approve);
    client.vote_to_dissolve(&member3, &circle_id, &DissolutionVoteChoice::Approve);
    
    // Check dissolved circle statistics
    let dissolved_circle = client.get_dissolved_circle(circle_id);
    assert_eq!(dissolved_circle.circle_id, circle_id);
    assert!(dissolved_circle.dissolution_timestamp > 0);
    assert!(dissolved_circle.total_contributions > 0);
    assert_eq!(dissolved_circle.total_members, 3);
    assert_eq!(dissolved_circle.refunded_members, 0); // No refunds yet
    assert!(dissolved_circle.remaining_funds >= 0);
}

#[test]
fn test_refund_period_expiration() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create circle
    let circle_id = client.create_circle(
        &creator,
        &100_000_0,
        &2u32,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Join circle
    client.join_circle(&member, &circle_id, &1u32, &None);
    
    // Initiate dissolution and approve
    let reason = String::from_str(&env, "Test dissolution");
    client.initiate_dissolve(&member, &circle_id, reason);
    client.vote_to_dissolve(&member, &circle_id, &DissolutionVoteChoice::Approve);
    
    // Advance time past refund period (30 days + dissolution time)
    env.ledger().set_timestamp(env.ledger().timestamp() + 2700000); // 31+ days later
    
    // Try to claim refund - should fail due to expired period
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "claim_refund"),
        (member, circle_id),
    );
    assert!(result.is_err());
}
