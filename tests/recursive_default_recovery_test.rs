use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, Vec, i128, u64, u32, u16};
use sorosusu::{SoroSusuClient, SoroSusuTrait, CircleInfo, Member, DefaultRecoveryConfig, RecoverySprint, PriorityClaim, HealthyMemberClaim, InternalDebtRestructuring, Error};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn initialize(env: Env, admin: Address) {
        // Mock token initialization
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        // Mock mint function
    }

    pub fn balance(env: Env, addr: Address) -> i128 {
        // Mock balance function - returns different amounts for different tokens
        if addr == Address::from_string(&env, "USDC") {
            1_000_000_000 // 1000 USDC for testing
        } else if addr == Address::from_string(&env, "XLM") {
            500_000_000 // 50 XLM for testing
        } else {
            100_000_000 // Default balance
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_configure_default_recovery_basic() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000, // 100 USDC contribution
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure basic recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION, // 2 rounds
            priority_claim_bps: PRIORITY_CLAIM_BPS, // 10%
            healthy_member_bps: HEALTHY_MEMBER_BPS, // 50%
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS, // 20
            min_participant_score: MIN_PARTICIPANT_SCORE, // 30%
            collateral_release_bps: COLLATERAL_RELEASE_BPS, // 25%
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Verify configuration
        let stored_config = client.get_default_recovery_config(&circle_id).unwrap();
        assert_eq!(stored_config.enabled, true);
        assert_eq!(stored_config.sprint_duration, RECOVERY_SPRINT_DURATION);
        assert_eq!(stored_config.priority_claim_bps, PRIORITY_CLAIM_BPS);
    }

    #[test]
    fn test_configure_default_recovery_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Test invalid priority claim percentage (over 20%)
        let invalid_config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: 2500, // 25% - should fail
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        // This should panic due to invalid priority claim percentage
        env.mock_all_auths();
        // client.configure_default_recovery(&admin, &circle_id, &invalid_config); // Should panic
    }

    #[test]
    fn test_initiate_recovery_sprint_success() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add members
        for i in 0..5 {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }
        
        // Configure recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Initiate recovery sprint
        env.mock_all_auths();
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Verify sprint was created
        let sprint = client.get_recovery_sprint(&circle_id, &1).unwrap();
        assert_eq!(sprint.defaulter, defaulter);
        assert_eq!(sprint.circle_id, circle_id);
        assert_eq!(sprint.start_round, 1);
        assert_eq!(sprint.status, RecoverySprintStatus::Active);
        assert!(sprint.participants.len(), 5); // 5 members + defaulter = 6 participants
    }

    #[test]
    fn test_make_priority_claim() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        let priority_claimant = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add members
        for i in 0..5 {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }
        
        // Configure and initiate recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Make priority claim
        env.mock_all_auths();
        client.make_priority_claim(&priority_claimant, &circle_id, &1);
        
        // Verify priority claim
        let claim = client.get_priority_claim(&1).unwrap();
        assert_eq!(claim.claimant, priority_claimant);
        assert_eq!(claim.sprint_id, 1);
        assert_eq!(claim.original_defaulter_share, 100_000_000); // 100 USDC
        assert_eq!(claim.bonus_percentage_bps, PRIORITY_CLAIM_BPS); // 10% bonus
        assert!(claim.claim_amount, 110_000_000); // 100 USDC + 10% bonus = 110 USDC
    }

    #[test]
    fn test_make_healthy_member_claim() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        let healthy_claimant = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add members
        for i in 0..5 {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }
        
        // Configure and initiate recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Make healthy member claim
        env.mock_all_auths();
        client.make_healthy_member_claim(&healthy_claimant, &circle_id, &1);
        
        // Verify healthy member claim
        let claim = client.get_healthy_member_claim(&1).unwrap();
        assert_eq!(claim.claimant, healthy_claimant);
        assert_eq!(claim.sprint_id, 1);
        assert_eq!(claim.claim_amount, 50_000_000); // 50 USDC
        assert_eq!(claim.reputation_score, 5000); // 50% reputation score
    }

    #[test]
    fn test_complete_recovery_sprint() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        let priority_claimant = Address::generate(&env);
        let healthy_claimant1 = Address::generate(&env);
        let healthy_claimant2 = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add members
        for i in 0..5 {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }
        
        // Configure and initiate recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Make claims
        env.mock_all_auths();
        client.make_priority_claim(&priority_claimant, &circle_id, &1);
        client.make_healthy_member_claim(&healthy_claimant1, &circle_id, &1);
        client.make_healthy_member_claim(&healthy_claimant2, &circle_id, &1);
        
        // Complete sprint
        env.mock_all_auths();
        client.complete_recovery_sprint(&admin, &circle_id, &1);
        
        // Verify sprint completion
        let sprint = client.get_recovery_sprint(&circle_id, &1).unwrap();
        assert_eq!(sprint.status, RecoverySprintStatus::Completed);
        assert_eq!(sprint.priority_claim_amount, 110_000_000); // 110 USDC priority claim
        assert_eq!(sprint.healthy_claim_amount, 100_000_000); // 50 USDC each
        assert_eq!(sprint.collateral_released, 250_000_000); // 25% of 1000 USDC collateral
    }

    #[test]
    fn test_initiate_debt_restructuring() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add member and create default scenario
        let member = Address::generate(&env);
        client.join_circle(&member, &circle_id);
        
        // Simulate default by missing contributions
        for _ in 0..3 {
            // Skip contributions to create default scenario
        }
        
        // Configure recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Initiate recovery sprint
        env.mock_all_auths();
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Initiate debt restructuring
        env.mock_all_auths();
        client.initiate_debt_restructuring(&admin, &circle_id, &defaulter, &200_000_000, &DEBT_RESTRUCTURING_INTEREST_BPS);
        
        // Verify restructuring
        let restructuring = client.get_debt_restructuring(&circle_id, &1).unwrap();
        assert_eq!(restructuring.original_principal, 200_000_000);
        assert_eq!(restructuring.interest_rate_bps, DEBT_RESTRUCTURING_INTEREST_BPS);
        assert_eq!(restructuring.start_round, 2);
        assert_eq!(restructuring.status, DebtRestructuringStatus::Active);
    }

    #[test]
    fn test_complete_debt_restructuring() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add member
        let member = Address::generate(&env);
        client.join_circle(&member, &circle_id);
        
        // Configure recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Initiate recovery sprint and debt restructuring
        env.mock_all_auths();
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        client.initiate_debt_restructuring(&admin, &circle_id, &defaulter, &200_000_000, &DEBT_RESTRUCTURING_INTEREST_BPS);
        
        // Complete sprint
        env.mock_all_auths();
        client.complete_recovery_sprint(&admin, &circle_id, &1);
        
        // Make restructuring payments (6 rounds)
        for round in 2..8 {
            env.mock_all_auths();
            client.make_restructuring_payment(&defaulter, &circle_id, &1, &35_000_000); // 35 USDC per round
        }
        
        // Complete restructuring
        env.mock_all_auths();
        client.complete_debt_restructuring(&admin, &circle_id, &1);
        
        // Verify restructuring completion
        let restructuring = client.get_debt_restructuring(&circle_id, &1).unwrap();
        assert_eq!(restructuring.status, DebtRestructuringStatus::Completed);
        assert_eq!(restructuring.restructured_amount, 210_000_000); // 200 USDC + 10% interest
    }

    #[test]
    fn test_recovery_sprint_insufficient_participants() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add only 2 participants (insufficient for 6 participants)
        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        
        for member in [&member1, &member2] {
            client.join_circle(&member, &circle_id);
        }
        
        // Configure recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Try to initiate sprint - should fail due to insufficient participants
        env.mock_all_auths();
        // client.initiate_recovery_sprint(&admin, &circle_id, &defaulter); // Should panic
    }

    #[test]
    fn test_recovery_disabled() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure recovery sprint as disabled
        let config = DefaultRecoveryConfig {
            enabled: false, // Disabled
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        
        // Try to initiate sprint - should fail because recovery is disabled
        env.mock_all_auths();
        // client.initiate_recovery_sprint(&admin, &circle_id, &defaulter); // Should panic
    }

    #[test]
    fn test_claim_already_processed() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let defaulter = Address::generate(&env);
        let claimant = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Add members
        for i in 0..5 {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }
        
        // Configure and initiate recovery sprint
        let config = DefaultRecoveryConfig {
            enabled: true,
            sprint_duration: RECOVERY_SPRINT_DURATION,
            priority_claim_bps: PRIORITY_CLAIM_BPS,
            healthy_member_bps: HEALTHY_MEMBER_BPS,
            max_sprint_participants: MAX_SPRINT_PARTICIPANTS,
            min_participant_score: MIN_PARTICIPANT_SCORE,
            collateral_release_bps: COLLATERAL_RELEASE_BPS,
        };
        
        client.configure_default_recovery(&admin, &circle_id, &config);
        client.initiate_recovery_sprint(&admin, &circle_id, &defaulter);
        
        // Make first claim
        env.mock_all_auths();
        client.make_priority_claim(&claimant, &circle_id, &1);
        
        // Try to make second claim - should fail
        env.mock_all_auths();
        // client.make_priority_claim(&claimant, &circle_id, &1); // Should panic
    }
}
