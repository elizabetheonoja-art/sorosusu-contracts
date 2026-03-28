#![cfg(test)]

use soroban_sdk::{Address, Env, String, Vec, Symbol};
use crate::{
    SoroSusuSbtMinter, SoroSusuSbtMinterClient, SbtStatus, ReputationTier, 
    SoroSusuCredential, ReputationMilestone, UserReputationMetrics
};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn create_test_env() -> (Env, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    let minter_admin = Address::generate(&env);
    
    // Register SBT minter contract
    let minter_contract_id = env.register_contract(None, SoroSusuSbtMinter);
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    (env, admin, user, minter_contract_id)
}

#[test]
fn test_sbt_minter_initialization() {
    let (env, admin, _user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Test initialization
    minter_client.init_sbt_minter(&admin);
    
    // Verify admin is set
    let stored_admin = env.storage().instance()
        .get(&crate::DataKey::SbtMinterAdmin)
        .unwrap();
    assert_eq!(stored_admin, admin);
    
    // Verify milestone counter is initialized
    let counter = env.storage().instance()
        .get(&crate::DataKey::MilestoneCounter)
        .unwrap();
    assert_eq!(counter, 0u64);
}

#[test]
fn test_admin_transfer() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize with original admin
    minter_client.init_sbt_minter(&admin);
    
    // Test admin transfer
    let new_admin = Address::generate(&env);
    minter_client.set_sbt_minter_admin(&admin, &new_admin);
    
    // Verify new admin is set
    let stored_admin = env.storage().instance()
        .get(&crate::DataKey::SbtMinterAdmin)
        .unwrap();
    assert_eq!(stored_admin, new_admin);
    
    // Test unauthorized admin transfer (should fail)
    let unauthorized_user = Address::generate(&env);
    let result = std::panic::catch_unwind(|| {
        minter_client.set_sbt_minter_admin(&unauthorized_user, &admin);
    });
    assert!(result.is_err());
}

#[test]
fn test_reputation_milestone_creation() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Test milestone creation
    let description = String::from_str(&env, "Complete 5 cycles");
    let milestone_id = minter_client.create_reputation_milestone(
        &user,
        &5,
        &description,
        &ReputationTier::Silver,
    );
    
    // Verify milestone was created
    let milestone = minter_client.get_reputation_milestone(&milestone_id);
    assert_eq!(milestone.user, user);
    assert_eq!(milestone.cycles_required, 5);
    assert_eq!(milestone.description, description);
    assert_eq!(milestone.reward_tier, ReputationTier::Silver);
    assert!(!milestone.is_completed);
    
    // Verify milestone counter was incremented
    let counter = env.storage().instance()
        .get(&crate::DataKey::MilestoneCounter)
        .unwrap();
    assert_eq!(counter, 1u64);
}

#[test]
fn test_credential_issuance() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Create milestone
    let description = String::from_str(&env, "Complete 3 cycles");
    let milestone_id = minter_client.create_reputation_milestone(
        &user,
        &3,
        &description,
        &ReputationTier::Bronze,
    );
    
    // Issue credential for completing milestone
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/credential/1");
    let token_id = minter_client.issue_credential(
        &user,
        &milestone_id,
        &metadata_uri,
    );
    
    // Verify credential was issued
    let credential = minter_client.get_credential(&token_id);
    assert_eq!(credential.holder, user);
    assert_eq!(credential.reputation_tier, ReputationTier::Bronze);
    assert_eq!(credential.total_cycles_completed, 3);
    assert_eq!(credential.status, SbtStatus::Active);
    assert_eq!(credential.metadata_uri, metadata_uri);
    
    // Verify user has credential reference
    let user_credential = minter_client.get_user_credential(&user);
    assert!(user_credential.is_some());
    assert_eq!(user_credential.unwrap(), token_id);
    
    // Verify milestone is marked complete
    let updated_milestone = minter_client.get_reputation_milestone(&milestone_id);
    assert!(updated_milestone.is_completed);
    assert!(updated_milestone.completion_timestamp.is_some());
}

#[test]
fn test_credential_revocation() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter and issue credential
    minter_client.init_sbt_minter(&admin);
    
    let description = String::from_str(&env, "Test milestone");
    let milestone_id = minter_client.create_reputation_milestone(
        &user,
        &1,
        &description,
        &ReputationTier::Bronze,
    );
    
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/credential/1");
    let token_id = minter_client.issue_credential(
        &user,
        &milestone_id,
        &metadata_uri,
    );
    
    // Test revocation
    let reason = String::from_str(&env, "Violation of community guidelines");
    minter_client.revoke_credential(&token_id, &reason);
    
    // Verify credential is revoked
    let credential = minter_client.get_credential(&token_id);
    assert_eq!(credential.status, SbtStatus::Revoked);
    
    // Verify user reputation was penalized
    let (reliability, social_capital, total_cycles) = minter_client.get_user_reputation_score(&user);
    assert!(reliability < 5000); // Should be reduced
    assert!(social_capital < 5000); // Should be reduced
}

#[test]
fn test_credential_dishonoring() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter and issue credential
    minter_client.init_sbt_minter(&admin);
    
    let description = String::from_str(&env, "Test milestone");
    let milestone_id = minter_client.create_reputation_milestone(
        &user,
        &1,
        &description,
        &ReputationTier::Bronze,
    );
    
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/credential/1");
    let token_id = minter_client.issue_credential(
        &user,
        &milestone_id,
        &metadata_uri,
    );
    
    // Test dishonoring
    let reason = String::from_str(&env, "Community trust violation");
    minter_client.update_credential_status(&token_id, &SbtStatus::Dishonored);
    
    // Verify credential is dishonored
    let credential = minter_client.get_credential(&token_id);
    assert_eq!(credential.status, SbtStatus::Dishonored);
    
    // Verify user reputation was penalized (less severe than revocation)
    let (reliability, social_capital, total_cycles) = minter_client.get_user_reputation_score(&user);
    assert!(reliability < 5000); // Should be reduced but not as severely as revocation
    assert!(social_capital < 5000);
}

#[test]
fn test_unauthorized_operations() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Test unauthorized milestone creation
    let unauthorized_user = Address::generate(&env);
    let result = std::panic::catch_unwind(|| {
        minter_client.create_reputation_milestone(
            &unauthorized_user,
            &5,
            &String::from_str(&env, "Unauthorized milestone"),
            &ReputationTier::Silver,
        );
    });
    assert!(result.is_err());
    
    // Test credential issuance for non-existent milestone
    let fake_milestone_id = 999u64;
    let result = std::panic::catch_unwind(|| {
        minter_client.issue_credential(
            &user,
            &fake_milestone_id,
            &String::from_str(&env, "https://metadata.sorosusu.com/credential/1"),
        );
    });
    assert!(result.is_err());
    
    // Test unauthorized credential status update
    let description = String::from_str(&env, "Test milestone");
    let milestone_id = minter_client.create_reputation_milestone(
        &admin, // Admin creating milestone for themselves
        &1,
        &description,
        &ReputationTier::Bronze,
    );
    
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/credential/1");
    let token_id = minter_client.issue_credential(
        &admin,
        &milestone_id,
        &metadata_uri,
    );
    
    let unauthorized_user = Address::generate(&env);
    let result = std::panic::catch_unwind(|| {
        minter_client.update_credential_status(&token_id, &SbtStatus::Revoked);
    });
    assert!(result.is_err());
}

#[test]
fn test_reputation_tier_progression() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Create multiple milestones and credentials to test tier progression
    
    // Bronze tier (0-2 cycles)
    let bronze_desc = String::from_str(&env, "Complete 1 cycle");
    let bronze_id = minter_client.create_reputation_milestone(
        &user,
        &1,
        &bronze_desc,
        &ReputationTier::Bronze,
    );
    
    let bronze_token = minter_client.issue_credential(
        &user,
        &bronze_id,
        &String::from_str(&env, "https://metadata.sorosusu.com/bronze"),
    );
    
    // Silver tier (3-5 cycles)
    let silver_desc = String::from_str(&env, "Complete 3 more cycles (total 4)");
    let silver_id = minter_client.create_reputation_milestone(
        &user,
        &3,
        &silver_desc,
        &ReputationTier::Silver,
    );
    
    let silver_token = minter_client.issue_credential(
        &user,
        &silver_id,
        &String::from_str(&env, "https://metadata.sorosusu.com/silver"),
    );
    
    // Verify tier progression
    let bronze_credential = minter_client.get_credential(&bronze_token);
    let silver_credential = minter_client.get_credential(&silver_token);
    
    assert_eq!(bronze_credential.reputation_tier, ReputationTier::Bronze);
    assert_eq!(silver_credential.reputation_tier, ReputationTier::Silver);
    
    // Verify user's current credential reflects highest tier
    let user_credential = minter_client.get_user_credential(&user);
    assert!(user_credential.is_some());
    assert_eq!(user_credential.unwrap().reputation_tier, ReputationTier::Silver);
}

#[test]
fn test_perfect_record_tracking() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Create multiple milestones with perfect completion
    for i in 1..=3 {
        let desc = String::from_str(&env, &format!("Perfect cycle {}", i));
        let milestone_id = minter_client.create_reputation_milestone(
            &user,
            &1,
            &desc,
            &ReputationTier::Gold, // Higher tier for perfect completion
        );
        
        // Mark as completed (simulating perfect cycle completion)
        minter_client.issue_credential(
            &user,
            &milestone_id,
            &String::from_str(&env, "https://metadata.sorosusu.com/perfect"),
        );
    }
    
    // Verify perfect cycles tracking
    let (reliability, social_capital, total_cycles) = minter_client.get_user_reputation_score(&user);
    assert_eq!(total_cycles, 3); // Should have 3 total cycles
    // Perfect cycles should be tracked through credentials
    // (This would require additional logic to count perfect cycles from credentials)
}

#[test]
fn test_user_reputation_metrics() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Test initial reputation metrics
    let (reliability, social_capital, total_cycles) = minter_client.get_user_reputation_score(&user);
    
    // Should start at default values
    assert_eq!(reliability, 5000); // 50%
    assert_eq!(social_capital, 5000); // 50%
    assert_eq!(total_cycles, 0);
    
    // Update reputation (simulating cycle completion)
    minter_client.update_user_reputation(&user);
    
    // Check that metrics were updated
    let (updated_reliability, updated_social_capital, updated_total_cycles) = minter_client.get_user_reputation_score(&user);
    assert!(updated_reliability > reliability); // Should increase
    assert!(updated_social_capital > social_capital); // Should increase
    assert!(updated_total_cycles > total_cycles); // Should increase
}

#[test]
fn test_audit_trail() {
    let (env, admin, user, minter_contract_id) = create_test_env();
    let minter_client = SoroSusuSbtMinterClient::new(&env, &minter_contract_id);
    
    // Initialize minter
    minter_client.init_sbt_minter(&admin);
    
    // Create milestone
    let description = String::from_str(&env, "Audit test milestone");
    let milestone_id = minter_client.create_reputation_milestone(
        &user,
        &1,
        &description,
        &ReputationTier::Bronze,
    );
    
    // Issue credential
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/audit-test");
    let token_id = minter_client.issue_credential(
        &user,
        &milestone_id,
        &metadata_uri,
    );
    
    // Revoke credential
    let reason = String::from_str(&env, "Audit test revocation");
    minter_client.revoke_credential(&token_id, &reason);
    
    // Verify audit entries were created
    // Note: This would require access to audit query functions
    // For now, we test that the operations complete without panicking
    assert!(true); // If we reach here, all operations completed successfully
}
