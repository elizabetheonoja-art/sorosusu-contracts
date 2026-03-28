#![cfg(test)]

use soroban_sdk::{Address, Env, Vec, Symbol};
use crate::{SoroSusu, SoroSusuClient, TrancheSchedule, TrancheStatus};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn mint(env: Env, to: Address, amount: i128) {
        let mut balance = env.storage().instance().get::<_, i128>(&("balance".into())).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&("balance".into()), &balance);
    }
    
    pub fn balance(env: Env, account: Address) -> i128 {
        if account == env.current_contract_address() {
            env.storage().instance().get::<_, i128>(&("balance".into())).unwrap_or(0)
        } else {
            1000_000_000_000 // Large balance for testing
        }
    }
    
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Simplified transfer for testing
    }
}

fn setup_test_env() -> (Env, SoroSusuClient<'static>, Address, Address, Address, u64) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let circle_creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    
    // Deploy contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create mock token
    let token_id = env.register_contract(None, MockToken);
    let token_address = Address::from_token(&token_id);
    
    // Create NFT contract (mock)
    let nft_id = env.register_contract(None, MockToken);
    let nft_address = Address::from_token(&nft_id);
    
    // Create circle with 3 members
    let circle_id = client.create_circle(
        &circle_creator,
        &100_000_000, // 1 token contribution (7 decimals)
        &3,
        &token_address,
        &86400, // 1 day cycle
        &100, // 1% insurance fee
        &nft_address,
        &admin, // arbitrator
        &100, // 1% organizer fee
    );
    
    (env, client, admin, circle_creator, member1, circle_id)
}

#[test]
fn test_tranche_schedule_creation_on_payout() {
    let (env, client, _admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    // Join circle with members
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    // All members contribute
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    // Finalize round
    client.finalize_round(&circle_creator, &circle_id);
    
    // Trigger payout (first member should receive)
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Check that tranche schedule was created
    let first_recipient = member1.clone(); // First in queue
    let schedule = client.get_tranche_schedule(&circle_id, &first_recipient);
    
    assert!(schedule.is_some());
    let sched = schedule.unwrap();
    assert_eq!(sched.circle_id, circle_id);
    assert_eq!(sched.winner, first_recipient);
    assert!(sched.total_pot > 0);
    assert!(sched.immediate_payout > 0);
    assert_eq!(sched.tranches.len(), 2); // 2 tranches
    
    // Verify 70/30 split
    let expected_immediate = (sched.total_pot * 7000) / 10000; // 70%
    let expected_locked = sched.total_pot - expected_immediate; // 30%
    
    assert_eq!(sched.immediate_payout, expected_immediate);
    
    let total_tranche_amount: i128 = sched.tranches.iter().map(|t| t.amount).sum();
    assert_eq!(total_tranche_amount, expected_locked);
}

#[test]
fn test_tranche_claim_unlocks_after_one_round() {
    let (env, client, _admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    // Round 1: All contribute
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Get tranche schedule
    let schedule = client.get_tranche_schedule(&circle_id, &member1).unwrap();
    
    // Try to claim first tranche immediately (should fail - not unlocked yet)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_tranche(&member1, &circle_id, &0);
    }));
    
    // Should fail because tranche is not unlocked yet
    assert!(result.is_err());
    
    // Complete another round (advance time/round)
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Now first tranche should be unlockable
    client.claim_tranche(&member1, &circle_id, &0);
    
    // Verify tranche status changed to Claimed
    let updated_schedule = client.get_tranche_schedule(&circle_id, &member1).unwrap();
    let first_tranche = updated_schedule.tranches.get(0).unwrap();
    assert_eq!(first_tranche.status, TrancheStatus::Claimed);
}

#[test]
fn test_clawback_on_default() {
    let (env, client, admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    // Round 1: member1 receives pot
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Get tranche schedule before default
    let schedule_before = client.get_tranche_schedule(&circle_id, &member1).unwrap();
    let total_locked: i128 = schedule_before.tranches.iter().map(|t| t.amount).sum();
    assert!(total_locked > 0);
    
    // Round 2: member1 defaults (doesn't contribute)
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    // member1 does NOT contribute - DEFAULT!
    
    // Mark member1 as defaulted
    client.mark_member_defaulted(&admin, &circle_id, &member1);
    
    // Execute clawback
    let clawback_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.execute_tranche_clawback(&admin, &circle_id, &member1);
    }));
    
    assert!(clawback_result.is_ok());
    
    // Verify all tranches are clawed back
    let schedule_after = client.get_tranche_schedule(&circle_id, &member1).unwrap();
    for i in 0..schedule_after.tranches.len() {
        let tranche = schedule_after.tranches.get(i).unwrap();
        assert_eq!(tranche.status, TrancheStatus::ClawedBack);
    }
}

#[test]
fn test_defaulted_member_cannot_claim_tranche() {
    let (env, client, admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    // Round 1
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Advance to next round where member1 can claim
    client.deposit(&member1, &circle_id);
    client.deposit(&member2, &circle_id);
    client.deposit(&member3, &circle_id);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    // Now mark member1 as defaulted
    client.mark_member_defaulted(&admin, &circle_id, &member1);
    
    // Try to claim tranche after default (should fail)
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        client.claim_tranche(&member1, &circle_id, &0);
    }));
    
    assert!(result.is_err());
}

#[test]
fn test_full_cycle_with_tranches() {
    let (env, client, _admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    // Simulate full 3-round cycle
    for round in 0..3 {
        // All members contribute
        client.deposit(&member1, &circle_id);
        client.deposit(&member2, &circle_id);
        client.deposit(&member3, &circle_id);
        
        // Finalize and distribute
        client.finalize_round(&circle_creator, &circle_id);
        client.distribute_payout(&circle_creator, &circle_id);
        
        // Current recipient gets 70% immediately, 30% in tranches
        let current_recipient = match round {
            0 => &member1,
            1 => &member2,
            _ => &member3,
        };
        
        let schedule = client.get_tranche_schedule(&circle_id, current_recipient);
        assert!(schedule.is_some());
    }
    
    // Verify all members have tranche schedules
    for member in [&member1, &member2, &member3] {
        let schedule = client.get_tranche_schedule(&circle_id, member);
        assert!(schedule.is_some());
    }
}
