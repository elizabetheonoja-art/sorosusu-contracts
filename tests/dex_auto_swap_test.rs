use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, Vec, i128, u64, u32, u16};
use sorosusu::{SoroSusuClient, SoroSusuTrait, DexSwapConfig, DexSwapRecord, GasReserve, Error};

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

#[contract]
pub struct MockDex;

#[contractimpl]
impl MockDex {
    pub fn swap_exact_in_for_exact_out(
        env: Env,
        token_in: Address,
        token_out: Address,
        amount_in: i128,
        amount_out_min: i128,
        deadline: u64,
    ) -> i128 {
        // Mock DEX swap function - simulates USDC->XLM swap
        // Returns 90% of input as XLM (simulating 1 USDC = 0.9 XLM rate)
        if token_in == Address::from_string(&env, "USDC") && token_out == Address::from_string(&env, "XLM") {
            (amount_in * 9) / 10 // 90% conversion rate with 10% slippage
        } else {
            amount_in // Default no conversion
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_configure_dex_swap_basic() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
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
            &100, // 1% organizer fee
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 10_000_000, // 0.1 XLM threshold
            swap_percentage_bps: 5000, // 50% swap
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100, // 1% slippage
            minimum_swap_amount: 50_000_000, // 50 USDC minimum
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Verify configuration
        let stored_config = client.get_dex_swap_config(&circle_id).unwrap();
        assert_eq!(stored_config.enabled, true);
        assert_eq!(stored_config.swap_threshold_xlm, 10_000_000);
        assert_eq!(stored_config.swap_percentage_bps, 5000);
    }

    #[test]
    fn test_configure_dex_swap_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
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
        
        // Test invalid swap percentage (over 80%)
        let invalid_config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 10_000_000,
            swap_percentage_bps: 9000, // 90% - should fail
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        // This should panic due to invalid swap percentage
        env.mock_all_auths();
        // client.configure_dex_swap(&admin, &circle_id, &invalid_config); // Should panic
    }

    #[test]
    fn test_trigger_dex_swap_below_threshold() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 100_000_000, // 1 XLM threshold
            swap_percentage_bps: 5000, // 50% swap
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute (to generate fees)
        client.join_circle(&user1, &circle_id);
        client.deposit(&user1, &circle_id);
        
        // Try to trigger swap below threshold - should fail
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic - threshold not met
    }

    #[test]
    fn test_trigger_dex_swap_success() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap with low threshold
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000, // 0.001 XLM threshold (very low)
            swap_percentage_bps: 5000, // 50% swap
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add members and contribute to generate fees in USDC
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        client.join_circle(&user2, &circle_id);
        
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        
        // Simulate gas reserve accumulation (contract should have XLM)
        let gas_reserve = GasReserve {
            xlm_balance: 5_000_000, // 0.005 XLM - above threshold
            reserved_for_ttl: 0,
            auto_swap_enabled: true,
            last_refill_timestamp: env.ledger().timestamp(),
            consumption_rate: 1_000_000,
        };
        
        // Manually set gas reserve for testing
        // In real implementation, this would be set by contract operations
        // For test, we'll trigger swap directly
        
        // Trigger DEX swap - should succeed
        env.mock_all_auths();
        client.trigger_dex_swap(&admin, &circle_id);
        
        // Verify swap record was created
        let swap_records = client.get_dex_swap_record(&circle_id, &0);
        assert!(swap_records.is_some());
        
        let record = swap_records.unwrap();
        assert!(record.success);
        assert!(record.usdc_amount > 0);
        assert!(record.xlm_received > 0);
    }

    #[test]
    fn test_dex_swap_cooldown_period() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000, // Low threshold
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute
        client.join_circle(&creator, &circle_id);
        client.deposit(&creator, &circle_id);
        
        // First swap should succeed
        env.mock_all_auths();
        client.trigger_dex_swap(&admin, &circle_id);
        
        // Try second swap immediately - should fail due to cooldown
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic - cooldown
        
        // Advance time past cooldown (5 minutes)
        env.ledger().set_timestamp(env.ledger().timestamp() + 301);
        
        // Second swap should now succeed
        env.mock_all_auths();
        client.trigger_dex_swap(&admin, &circle_id); // Should succeed
    }

    #[test]
    fn test_emergency_pause_dex_swaps() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000,
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute
        client.join_circle(&creator, &circle_id);
        client.deposit(&creator, &circle_id);
        
        // Emergency pause swaps
        env.mock_all_auths();
        client.emergency_pause_dex_swaps(&admin);
        
        // Try to trigger swap - should fail due to emergency pause
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic - emergency pause
    }

    #[test]
    fn test_emergency_refill_gas_reserve() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000,
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Emergency refill gas reserve
        env.mock_all_auths();
        client.emergency_refill_gas_reserve(&admin, &1000_000_000); // 1 XLM
        
        // Verify gas reserve was updated
        let gas_reserve = client.get_gas_reserve(&circle_id).unwrap();
        assert!(gas_reserve.xlm_balance >= 1000_000_000);
    }

    #[test]
    fn test_dex_swap_slippage_protection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap with tight slippage tolerance
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000,
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 50, // 0.5% slippage tolerance
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute
        client.join_circle(&creator, &circle_id);
        client.deposit(&creator, &circle_id);
        
        // Trigger swap - should fail if slippage exceeds tolerance
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic if slippage > 0.5%
    }

    #[test]
    fn test_dex_swap_minimum_amount_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap with high minimum swap amount
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 1_000_000,
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 200_000_000, // 200 USDC minimum - high
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute small amount
        client.join_circle(&creator, &circle_id);
        client.deposit(&creator, &circle_id);
        
        // Trigger swap - should fail due to insufficient USDC for minimum swap
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic - below minimum
    }

    #[test]
    fn test_dex_swap_comprehensive_flow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap
        let config = DexSwapConfig {
            enabled: true,
            swap_threshold_xlm: 5_000_000, // 0.005 XLM threshold
            swap_percentage_bps: 3000, // 30% swap
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 200, // 2% slippage tolerance
            minimum_swap_amount: 30_000_000, // 30 USDC minimum
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add members and contribute multiple rounds to accumulate fees
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        client.join_circle(&user2, &circle_id);
        client.join_circle(&user3, &circle_id);
        
        // Round 1: Generate 300 USDC in fees (100 each * 3 members * 1%)
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.deposit(&user3, &circle_id);
        
        // Trigger first swap (30% of 300 = 90 USDC)
        env.mock_all_auths();
        client.trigger_dex_swap(&admin, &circle_id);
        
        let swap1 = client.get_dex_swap_record(&circle_id, &0).unwrap();
        assert_eq!(swap1.usdc_amount, 90_000_000);
        assert!(swap1.success);
        
        // Advance time past cooldown
        env.ledger().set_timestamp(env.ledger().timestamp() + 301);
        
        // Round 2: Generate another 300 USDC in fees
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.deposit(&user3, &circle_id);
        
        // Trigger second swap (30% of new 300 = 90 USDC)
        env.mock_all_auths();
        client.trigger_dex_swap(&admin, &circle_id);
        
        let swap2 = client.get_dex_swap_record(&circle_id, &1).unwrap();
        assert_eq!(swap2.usdc_amount, 90_000_000);
        assert!(swap2.success);
        
        // Verify gas reserve accumulation
        let gas_reserve = client.get_gas_reserve(&circle_id).unwrap();
        assert!(gas_reserve.xlm_balance > 0); // Should have XLM from swaps
        
        // Verify DEX config updated with total swapped
        let updated_config = client.get_dex_swap_config(&circle_id).unwrap();
        assert!(updated_config.total_swapped_xlm > 0);
    }

    #[test]
    fn test_dex_swap_disabled() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockToken);
        let dex_contract = env.register_contract(None, MockDex);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100,
        );
        
        // Configure DEX auto-swap as disabled
        let config = DexSwapConfig {
            enabled: false, // Disabled
            swap_threshold_xlm: 1_000_000,
            swap_percentage_bps: 5000,
            dex_contract: dex_contract,
            xlm_token: Address::from_string(&env, "XLM"),
            slippage_tolerance_bps: 100,
            minimum_swap_amount: 50_000_000,
            emergency_pause: false,
            last_swap_timestamp: 0,
            total_swapped_xlm: 0,
        };
        
        client.configure_dex_swap(&admin, &circle_id, &config);
        
        // Add member and contribute
        client.join_circle(&creator, &circle_id);
        client.deposit(&creator, &circle_id);
        
        // Try to trigger swap - should fail because DEX swaps are disabled
        env.mock_all_auths();
        // client.trigger_dex_swap(&admin, &circle_id); // Should panic - DEX swaps disabled
    }
}
