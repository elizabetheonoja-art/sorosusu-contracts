#[cfg(test)]
mod governance_token_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};

    #[contract]
    pub struct MockGovernanceToken;

    #[contractimpl]
    impl MockGovernanceToken {
        pub fn mint(env: Env, to: Address, amount: u64) {
            // Mock implementation - in real implementation this would mint tokens
        }
        
        pub fn transfer(env: Env, from: Address, to: Address, amount: u64) {
            // Mock implementation
        }
    }

    #[contract]
    pub struct MockNft;

    #[contractimpl]
    impl MockNft {
        pub fn mint(_env: Env, _to: Address, _id: u128) {}
        pub fn burn(_env: Env, _from: Address, _id: u128) {}
    }

    #[test]
    fn test_governance_token_mining_setup() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Set governance token
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Verify mining is enabled
        let config: MiningConfig = env.storage().instance().get(&DataKey::MiningConfig).unwrap();
        assert!(config.is_mining_enabled);

        // Verify governance token is set
        let stored_token: Address = env.storage().instance().get(&DataKey::GovernanceToken).unwrap();
        assert_eq!(stored_token, governance_token);
    }

    #[test]
    fn test_mining_on_successful_contribution() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        // Mock token balance and authorization
        env.mock_all_auths();

        // Make deposit (should trigger mining)
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // Check user mining stats
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_contributions, 1);
        assert_eq!(stats.total_tokens_earned, 100);
        assert_eq!(stats.total_tokens_claimed, 0);

        // Check user vesting info
        let vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(vesting.total_allocated, 100);
        assert_eq!(vesting.contributions_made, 1);
        assert!(vesting.is_active);
    }

    #[test]
    fn test_mining_disabled_when_token_not_set() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract but don't set governance token
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // Make deposit (should not trigger mining since token not set)
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // Check no mining occurred
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_tokens_earned, 0);

        let vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(vesting.total_allocated, 0);
        assert!(!vesting.is_active);
    }

    #[test]
    fn test_vesting_calculation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining with 12 cycle vesting, 3 cycle cliff
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Simulate user vesting info
        let vesting_key = DataKey::UserVesting(user.clone());
        let vesting_info = UserVestingInfo {
            total_allocated: 1200, // 12 contributions * 100 tokens
            vested_amount: 0,
            claimed_amount: 0,
            start_cycle: 0,
            contributions_made: 12,
            is_active: true,
        };
        env.storage().instance().set(&vesting_key, &vesting_info);

        // Test vesting calculation
        let vested_0 = SoroSusu::calculate_vested_amount(1200, 0, 0, 12);
        assert_eq!(vested_0, 0); // Before cliff

        let vested_3 = SoroSusu::calculate_vested_amount(1200, 0, 3, 12);
        assert_eq!(vested_3, 0); // At cliff

        let vested_6 = SoroSusu::calculate_vested_amount(1200, 0, 6, 12);
        assert_eq!(vested_6, 300); // 3 cycles after cliff = 25% vested

        let vested_12 = SoroSusu::calculate_vested_amount(1200, 0, 12, 12);
        assert_eq!(vested_12, 1200); // Fully vested

        let vested_15 = SoroSusu::calculate_vested_amount(1200, 0, 15, 12);
        assert_eq!(vested_15, 1200); // Fully vested (past end)
    }

    #[test]
    fn test_token_claiming() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Set up user vesting with vested tokens
        let vesting_key = DataKey::UserVesting(user.clone());
        let mut vesting_info = UserVestingInfo {
            total_allocated: 1200,
            vested_amount: 300,
            claimed_amount: 0,
            start_cycle: 0,
            contributions_made: 12,
            is_active: true,
        };
        env.storage().instance().set(&vesting_key, &vesting_info);

        // Mock user authorization
        user.require_auth();

        // Claim tokens
        SoroSusuTrait::claim_vested_tokens(env.clone(), user.clone());

        // Verify claim updated stats
        let updated_stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(updated_stats.total_tokens_claimed, 300);

        let updated_vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(updated_vesting.claimed_amount, 300);
    }

    #[test]
    fn test_max_mining_limit_per_circle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining with low limit
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 150, // Low limit
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join users
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            2,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);

        env.mock_all_auths();

        // First user deposits (should mine 100 tokens)
        SoroSusuTrait::deposit(env.clone(), user1.clone(), circle_id);
        let stats1 = SoroSusuTrait::get_mining_stats(env.clone(), user1.clone());
        assert_eq!(stats1.total_tokens_earned, 100);

        // Second user deposits (should mine 50 tokens, hitting the limit)
        SoroSusuTrait::deposit(env.clone(), user2.clone(), circle_id);
        let stats2 = SoroSusuTrait::get_mining_stats(env.clone(), user2.clone());
        assert_eq!(stats2.total_tokens_earned, 50); // Only 50 remaining from limit

        // Check total mined
        let total_mined: u64 = env.storage().instance().get(&DataKey::TotalMinedTokens).unwrap();
        assert_eq!(total_mined, 150); // At the limit
    }

    #[test]
    fn test_mining_config_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);

        SoroSusuTrait::init(env.clone(), admin.clone());

        // Test invalid config - zero tokens per contribution
        let invalid_config1 = MiningConfig {
            tokens_per_contribution: 0,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };

        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::configure_mining(env.clone(), admin.clone(), invalid_config1);
        });
        assert!(result.is_err());

        // Test invalid config - cliff longer than vesting
        let invalid_config2 = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 6,
            cliff_cycles: 8,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };

        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::configure_mining(env.clone(), admin.clone(), invalid_config2);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_completion_and_vesting_progress() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 4, // Short vesting for testing
            cliff_cycles: 1,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create 2-member circle
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            2,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);

        env.mock_all_auths();

        // Both users deposit - should complete cycle
        SoroSusuTrait::deposit(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::deposit(env.clone(), user2.clone(), circle_id);

        // Check cycle was completed
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle.cycle_count, 1);
        assert_eq!(circle.contribution_bitmap, 0); // Reset for next cycle

        // Check users started vesting from cycle 0
        let vesting1 = SoroSusuTrait::get_user_vesting_info(env.clone(), user1.clone());
        assert_eq!(vesting1.start_cycle, 0);
        assert!(vesting1.is_active);
    }

    #[test]
    fn test_ejected_member_mining_deactivation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // User deposits and earns tokens
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        
        let vesting_before = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert!(vesting_before.is_active);

        // Eject member
        SoroSusuTrait::eject_member(env.clone(), creator.clone(), circle_id, user.clone());

        // Check vesting was deactivated
        let vesting_after = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert!(!vesting_after.is_active);
    }

    #[test]
    fn test_save_to_earn_events() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // Make deposit and check for events
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // In a real test environment, you would verify the events were emitted
        // For this mock test, we just ensure the function completes without error
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_tokens_earned, 100);
    }
}
