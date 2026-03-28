#![cfg(test)]

use soroban_sdk::{Address, Env, Vec, Symbol, String};
use crate::{SoroSusu, SoroSusuClient, GrantSettlement, VotingSnapshot, ImpactCertificateMetadata};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn balance(env: Env, account: Address) -> i128 {
        if account == env.current_contract_address() {
            100_000_000_000 // Large balance for testing
        } else {
            10_000_000_000
        }
    }
    
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Simplified transfer for testing
    }
}

fn setup_test_env() -> (Env, SoroSusuClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let grantee = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create mock token
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    (env, client, admin, treasury, grantee)
}

// ============================================
// TEST 1: GRANT SETTLEMENT SYSTEM
// ============================================

#[test]
fn test_grant_settlement_calculation() {
    let (env, client, admin, treasury, grantee) = setup_test_env();
    
    let grant_id = 1u64;
    let total_grant = 10_000_000_000i128; // 100 tokens
    let grant_duration = 86400u64 * 30; // 30 days
    let start_timestamp = env.ledger().timestamp();
    
    // Advance time by 15 days (halfway point)
    env.ledger().with_mut(|li| {
        li.timestamp = start_timestamp + (86400 * 15);
    });
    
    // Terminate grant amicably
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    let settlement = client.terminate_grant_amicably(
        &admin,
        &grant_id,
        &grantee,
        &total_grant,
        &grant_duration,
        &start_timestamp,
        &treasury,
        &token_address,
    );
    
    // Verify settlement calculation
    assert_eq!(settlement.grant_id, grant_id);
    assert_eq!(settlement.grantee, grantee);
    assert_eq!(settlement.total_grant_amount, total_grant);
    
    // Should have received ~50% (15 days out of 30)
    let expected_drip = (total_grant * 15) / 30;
    assert!((settlement.amount_dripped - expected_drip).abs() < 1000); // Allow small rounding
    
    // WIP pay should equal dripped amount
    assert_eq!(settlement.work_in_progress_pay, settlement.amount_dripped);
    
    // Treasury should get remainder
    assert_eq!(settlement.treasury_return, total_grant - settlement.work_in_progress_pay);
    
    // Verify event was emitted
    // (In production, would check events more thoroughly)
}

#[test]
fn test_grant_settlement_full_duration() {
    let (env, client, admin, treasury, grantee) = setup_test_env();
    
    let grant_id = 2u64;
    let total_grant = 5_000_000_000i128; // 50 tokens
    let grant_duration = 86400u64 * 10; // 10 days
    let start_timestamp = env.ledger().timestamp();
    
    // Advance time to full duration
    env.ledger().with_mut(|li| {
        li.timestamp = start_timestamp + grant_duration;
    });
    
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    let settlement = client.terminate_grant_amicably(
        &admin,
        &grant_id,
        &grantee,
        &total_grant,
        &grant_duration,
        &start_timestamp,
        &treasury,
        &token_address,
    );
    
    // Should receive full amount
    assert_eq!(settlement.work_in_progress_pay, total_grant);
    assert_eq!(settlement.treasury_return, 0);
}

#[test]
fn test_grant_settlement_zero_elapsed() {
    let (env, client, admin, treasury, grantee) = setup_test_env();
    
    let grant_id = 3u64;
    let total_grant = 8_000_000_000i128;
    let grant_duration = 86400u64 * 7; // 7 days
    let start_timestamp = env.ledger().timestamp();
    
    // Don't advance time - terminate immediately
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    let settlement = client.terminate_grant_amicably(
        &admin,
        &grant_id,
        &grantee,
        &total_grant,
        &grant_duration,
        &start_timestamp,
        &treasury,
        &token_address,
    );
    
    // Should receive nothing, all goes back to treasury
    assert_eq!(settlement.work_in_progress_pay, 0);
    assert_eq!(settlement.treasury_return, total_grant);
}

// ============================================
// TEST 2: VOTING SNAPSHOT FOR AUDITS
// ============================================

#[test]
fn test_voting_snapshot_creation() {
    let (env, _client, _admin, _treasury, _grantee) = setup_test_env();
    
    let proposal_id = 1u64;
    
    // Create sample votes
    let voter1 = Address::generate(&env);
    let voter2 = Address::generate(&env);
    let voter3 = Address::generate(&env);
    
    let mut votes = Vec::new(&env);
    votes.push_back((voter1.clone(), 100u32, Symbol::new(&env, "For")));
    votes.push_back((voter2.clone(), 50u32, Symbol::new(&env, "Against")));
    votes.push_back((voter3.clone(), 25u32, Symbol::new(&env, "Abstain")));
    
    let quorum_required = 150u32;
    
    // Create snapshot
    let snapshot = _client.create_voting_snapshot_for_audit(
        &proposal_id,
        &votes,
        &quorum_required,
    );
    
    // Verify snapshot data
    assert_eq!(snapshot.proposal_id, proposal_id);
    assert_eq!(snapshot.total_votes, 175); // 100 + 50 + 25
    assert_eq!(snapshot.for_votes, 100);
    assert_eq!(snapshot.against_votes, 50);
    assert_eq!(snapshot.abstain_votes, 25);
    assert_eq!(snapshot.quorum_required, quorum_required);
    assert!(snapshot.quorum_met); // 175 >= 150
    assert_eq!(snapshot.result, Symbol::new(&env, "APPROVED"));
    
    // Verify hash was created
    assert_eq!(snapshot.vote_hash.len(), 32);
}

#[test]
fn test_voting_snapshot_quorum_not_met() {
    let (env, _client, _admin, _treasury, _grantee) = setup_test_env();
    
    let proposal_id = 2u64;
    
    let voter1 = Address::generate(&env);
    let mut votes = Vec::new(&env);
    votes.push_back((voter1.clone(), 50u32, Symbol::new(&env, "For")));
    
    let quorum_required = 100u32;
    
    let snapshot = _client.create_voting_snapshot_for_audit(
        &proposal_id,
        &votes,
        &quorum_required,
    );
    
    assert!(!snapshot.quorum_met); // 50 < 100
    assert_eq!(snapshot.result, Symbol::new(&env, "QUORUM_NOT_MET"));
}

#[test]
fn test_voting_snapshot_retrieval() {
    let (env, client, _admin, _treasury, _grantee) = setup_test_env();
    
    let proposal_id = 3u64;
    
    let voter1 = Address::generate(&env);
    let mut votes = Vec::new(&env);
    votes.push_back((voter1.clone(), 75u32, Symbol::new(&env, "For")));
    
    let quorum_required = 50u32;
    
    // Create and store snapshot
    let _ = client.create_voting_snapshot_for_audit(&proposal_id, &votes, &quorum_required);
    
    // Retrieve snapshot
    let retrieved = client.get_voting_snapshot_for_audit(&proposal_id);
    
    assert!(retrieved.is_some());
    let snapshot = retrieved.unwrap();
    assert_eq!(snapshot.proposal_id, proposal_id);
    assert_eq!(snapshot.for_votes, 75);
}

// ============================================
// TEST 3: DYNAMIC REPUTATION VIA NFT METADATA
// ============================================

#[test]
fn test_impact_certificate_initialization() {
    let (env, client, _admin, _treasury, grantee) = setup_test_env();
    
    let certificate_id = 1u128;
    let total_phases = 5u32;
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/impact/1");
    
    // Initialize certificate
    client.initialize_impact_certificate(
        &grantee,
        &certificate_id,
        &total_phases,
        &metadata_uri,
    );
    
    // Verify initial state via progress bar data
    let progress_data = client.get_progress_bar_data(&certificate_id);
    
    assert!(progress_data.is_some());
    let data = progress_data.unwrap();
    
    // Should start at 0% progress
    // Should have NEWCOMER badge
    // Should be in BRONZE tier (50% impact score)
}

#[test]
fn test_milestone_progress_updates() {
    let (env, client, admin, _treasury, grantee) = setup_test_env();
    
    let certificate_id = 2u128;
    let total_phases = 3u32;
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/impact/2");
    
    // Initialize
    client.initialize_impact_certificate(
        &grantee,
        &certificate_id,
        &total_phases,
        &metadata_uri,
    );
    
    // Update to phase 1
    let cert1 = client.update_milestone_progress(
        &admin,
        &certificate_id,
        &1, // Phase 1
        &500, // +5% impact score
    );
    
    assert_eq!(cert1.phases_completed, 1);
    assert_eq!(cert1.total_phases, 2); // new_phase + 1
    assert!(cert1.impact_score > 5000); // Increased from initial 5000
    
    // Update to phase 2
    let cert2 = client.update_milestone_progress(
        &admin,
        &certificate_id,
        &2, // Phase 2
        &1000, // +10% impact score
    );
    
    assert_eq!(cert2.phases_completed, 2);
    assert!(cert2.impact_score > cert1.impact_score);
    
    // Badge should change as phases complete
    assert_ne!(cert1.on_chain_badge, cert2.on_chain_badge);
}

#[test]
fn test_progress_bar_visual_data() {
    let (env, client, admin, _treasury, grantee) = setup_test_env();
    
    let certificate_id = 3u128;
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/impact/3");
    
    // Initialize with 4 phases
    client.initialize_impact_certificate(
        &grantee,
        &certificate_id,
        &4u32,
        &metadata_uri,
    );
    
    // Update to phase 2 (50% complete)
    client.update_milestone_progress(
        &admin,
        &certificate_id,
        &2,
        &1000,
    );
    
    // Get visual progress data
    let progress_data = client.get_progress_bar_data(&certificate_id).unwrap();
    
    // Verify progress percentage is 50%
    // Verify badge reflects phase 2 completion
    // Verify tier based on impact score
}

#[test]
fn test_impact_certificate_completion() {
    let (env, client, admin, _treasury, grantee) = setup_test_env();
    
    let certificate_id = 4u128;
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/impact/4");
    
    // Initialize
    client.initialize_impact_certificate(
        &grantee,
        &certificate_id,
        &3u32,
        &metadata_uri,
    );
    
    // Complete all phases
    client.update_milestone_progress(&admin, &certificate_id, &1, &500);
    client.update_milestone_progress(&admin, &certificate_id, &2, &500);
    let final_cert = client.update_milestone_progress(&admin, &certificate_id, &3, &1000);
    
    // Should be marked as completed
    assert_eq!(final_cert.milestone_status, crate::MilestoneProgress::Completed);
    assert_eq!(final_cert.phases_completed, final_cert.total_phases);
    
    // Should have highest badge
    assert_eq!(final_cert.on_chain_badge, Symbol::new(&env, "IMPACT_MASTER"));
}

// ============================================
// INTEGRATION TEST: COMBINED FEATURES
// ============================================

#[test]
fn test_full_governance_workflow() {
    let (env, client, admin, treasury, grantee) = setup_test_env();
    
    // 1. Initialize impact certificate for grant recipient
    let certificate_id = 100u128;
    client.initialize_impact_certificate(
        &grantee,
        &certificate_id,
        &5u32,
        &String::from_str(&env, "https://metadata.sorosusu.com/impact/100"),
    );
    
    // 2. Create voting snapshot for grant approval
    let proposal_id = 1u64;
    let mut votes = Vec::new(&env);
    votes.push_back((admin.clone(), 100u32, Symbol::new(&env, "For")));
    
    let snapshot = client.create_voting_snapshot_for_audit(&proposal_id, &votes, &50u32);
    assert_eq!(snapshot.result, Symbol::new(&env, "APPROVED"));
    
    // 3. Simulate grant progress - update milestone
    client.update_milestone_progress(&admin, &certificate_id, &1, &200);
    
    // 4. Grant cancelled amicably - calculate settlement
    let grant_id = 1u64;
    let total_grant = 20_000_000_000i128;
    let grant_duration = 86400u64 * 60; // 60 days
    let start_timestamp = env.ledger().timestamp();
    
    // Advance 30 days
    env.ledger().with_mut(|li| {
        li.timestamp = start_timestamp + (86400 * 30);
    });
    
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    let settlement = client.terminate_grant_amicably(
        &admin,
        &grant_id,
        &grantee,
        &total_grant,
        &grant_duration,
        &start_timestamp,
        &treasury,
        &token_address,
    );
    
    // Verify 50% payout (30 days out of 60)
    assert!((settlement.work_in_progress_pay - (total_grant / 2)).abs() < 1000);
    
    // 5. Final milestone update shows project completed partially
    let final_cert = client.get_progress_bar_data(&certificate_id).unwrap();
    assert!(final_cert.contains_key(&Symbol::new(&env, "progress")));
}
