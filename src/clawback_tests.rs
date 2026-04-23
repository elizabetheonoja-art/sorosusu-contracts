#![cfg(test)]

use soroban_sdk::{Address, Env, Symbol, token};
use crate::{
    SoroSusu, SoroSusuTrait, DataKey, Member, CircleInfo, ClawbackDeficit, 
    RecoveryPlan, RecoveryType, PauseReason, PausedRound
};

#[test]
fn test_clawback_detection_and_pause() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000, // contribution amount
        5,     // max members
        token_address.clone(),
        604800, // 1 week cycle duration
        100,    // 1% insurance fee
        nft_contract,
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Users make deposits
    SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    SoroSusu::deposit(env.clone(), user2.clone(), circle_id);

    // Simulate clawback by reducing expected balance manually
    let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    circle.expected_balance = 3000; // Higher than actual balance
    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

    // Detect clawback
    SoroSusu::detect_and_handle_clawback(env.clone(), creator.clone(), circle_id);

    // Verify round is paused
    let paused_info = SoroSusu::get_paused_round_info(env.clone(), circle_id);
    assert!(paused_info.circle_id == circle_id);
    assert!(matches!(paused_info.pause_reason, PauseReason::ClawbackDetected));

    // Verify deficit is recorded
    let deficit = SoroSusu::get_clawback_deficit(env.clone(), circle_id);
    assert!(deficit.deficit_amount > 0);
    assert!(deficit.detected_by == creator);
}

#[test]
fn test_recovery_plan_proposal_and_voting() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token_address.clone(),
        604800,
        100,
        nft_contract,
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user3.clone(), circle_id);

    // Simulate clawback detection
    let deficit = ClawbackDeficit {
        circle_id,
        deficit_amount: 500,
        detection_timestamp: env.ledger().timestamp(),
        detected_by: creator.clone(),
        token_address: token_address.clone(),
    };
    env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &deficit);

    // Propose recovery plan
    SoroSusu::propose_recovery_plan(
        env.clone(),
        user1.clone(),
        circle_id,
        RecoveryType::MemberContribution,
    );

    // Get recovery plan and verify
    let recovery_plan = SoroSusu::get_recovery_plan(env.clone(), circle_id);
    assert!(recovery_plan.is_active);
    assert!(matches!(recovery_plan.recovery_type, RecoveryType::MemberContribution));
    assert!(recovery_plan.proposed_by == user1);
    assert!(recovery_plan.total_deficit == 500);

    // Vote for recovery plan
    SoroSusu::vote_recovery_plan(env.clone(), user1.clone(), circle_id, true);
    SoroSusu::vote_recovery_plan(env.clone(), user2.clone(), circle_id, true);

    // Plan should be executed after majority vote
    let recovery_plan_after = SoroSusu::get_recovery_plan(env.clone(), circle_id);
    assert!(!recovery_plan_after.is_active); // Should be deactivated after execution
}

#[test]
fn test_member_contribution_recovery() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token_address.clone(),
        604800,
        100,
        nft_contract,
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Simulate clawback detection
    let deficit = ClawbackDeficit {
        circle_id,
        deficit_amount: 500,
        detection_timestamp: env.ledger().timestamp(),
        detected_by: creator.clone(),
        token_address: token_address.clone(),
    };
    env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &deficit);

    // Propose and vote for member contribution recovery
    SoroSusu::propose_recovery_plan(
        env.clone(),
        user1.clone(),
        circle_id,
        RecoveryType::MemberContribution,
    );

    SoroSusu::vote_recovery_plan(env.clone(), user1.clone(), circle_id, true);
    SoroSusu::vote_recovery_plan(env.clone(), user2.clone(), circle_id, true);

    // Contribute to recovery (this would normally involve actual token transfers)
    // For testing, we'll simulate the contribution recording
    let mut recovery_plan: RecoveryPlan = env.storage().instance().get(&DataKey::RecoveryPlan(circle_id)).unwrap();
    recovery_plan.recovery_contributions.set(user1.clone(), 300);
    recovery_plan.recovery_contributions.set(user2.clone(), 200);
    env.storage().instance().set(&DataKey::RecoveryPlan(circle_id), &recovery_plan);

    // Execute recovery plan
    SoroSusu::execute_recovery_plan(env.clone(), creator.clone(), circle_id);

    // Verify deficit is cleared
    let deficit_after = SoroSusu::get_clawback_deficit(env.clone(), circle_id);
    assert!(deficit_after.deficit_amount == 0);
}

#[test]
fn test_insurance_recovery() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle with high insurance fee
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token_address.clone(),
        604800,
        1000, // 10% insurance fee
        nft_contract,
    );

    // Users join and deposit to build insurance balance
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Simulate deposits (in real scenario, this would involve token transfers)
    let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    circle.insurance_balance = 200; // Simulated insurance balance
    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

    // Simulate clawback detection
    let deficit = ClawbackDeficit {
        circle_id,
        deficit_amount: 150, // Less than insurance balance
        detection_timestamp: env.ledger().timestamp(),
        detected_by: creator.clone(),
        token_address: token_address.clone(),
    };
    env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &deficit);

    // Propose insurance recovery
    SoroSusu::propose_recovery_plan(
        env.clone(),
        user1.clone(),
        circle_id,
        RecoveryType::InsuranceUsage,
    );

    SoroSusu::vote_recovery_plan(env.clone(), user1.clone(), circle_id, true);
    SoroSusu::vote_recovery_plan(env.clone(), user2.clone(), circle_id, true);

    // Execute recovery plan
    SoroSusu::execute_recovery_plan(env.clone(), creator.clone(), circle_id);

    // Verify insurance balance was reduced
    let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert!(circle_after.insurance_balance == 50); // 200 - 150

    // Verify deficit is cleared
    let deficit_after = SoroSusu::get_clawback_deficit(env.clone(), circle_id);
    assert!(deficit_after.deficit_amount == 0);
}

#[test]
fn test_round_resume_after_recovery() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token_address.clone(),
        604800,
        100,
        nft_contract,
    );

    // Users join circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);
    SoroSusu::join_circle(env.clone(), user2.clone(), circle_id);

    // Manually pause the round
    SoroSusu::pause_round(
        env.clone(),
        creator.clone(),
        circle_id,
        PauseReason::EmergencyMaintenance,
    );

    // Verify round is paused
    let paused_info = SoroSusu::get_paused_round_info(env.clone(), circle_id);
    assert!(paused_info.circle_id == circle_id);
    assert!(matches!(paused_info.pause_reason, PauseReason::EmergencyMaintenance));

    // Try to resume without resolving deficit - should fail
    let result = std::panic::catch_unwind(|| {
        SoroSusu::resume_round(env.clone(), creator.clone(), circle_id);
    });
    assert!(result.is_err()); // Should panic because no deficit resolution

    // Now execute a recovery plan first
    let deficit = ClawbackDeficit {
        circle_id,
        deficit_amount: 100,
        detection_timestamp: env.ledger().timestamp(),
        detected_by: creator.clone(),
        token_address: token_address.clone(),
    };
    env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &deficit);

    SoroSusu::propose_recovery_plan(
        env.clone(),
        user1.clone(),
        circle_id,
        RecoveryType::PayoutReduction,
    );

    SoroSusu::vote_recovery_plan(env.clone(), user1.clone(), circle_id, true);
    SoroSusu::vote_recovery_plan(env.clone(), user2.clone(), circle_id, true);

    // Execute recovery (this should clear the deficit)
    SoroSusu::execute_recovery_plan(env.clone(), creator.clone(), circle_id);

    // Now resume should work
    SoroSusu::resume_round(env.clone(), creator.clone(), circle_id);

    // Verify round is no longer paused
    let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
    assert!(!circle_after.is_paused);
}

#[test]
fn test_deposit_blocked_when_paused() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let token_address = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusu::init(env.clone(), admin.clone());
    
    // Create circle
    let circle_id = SoroSusu::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token_address.clone(),
        604800,
        100,
        nft_contract,
    );

    // User joins circle
    SoroSusu::join_circle(env.clone(), user1.clone(), circle_id);

    // Pause the round
    SoroSusu::pause_round(
        env.clone(),
        creator.clone(),
        circle_id,
        PauseReason::ClawbackDetected,
    );

    // Try to deposit - should fail
    let result = std::panic::catch_unwind(|| {
        SoroSusu::deposit(env.clone(), user1.clone(), circle_id);
    });
    assert!(result.is_err()); // Should panic because round is paused
}
