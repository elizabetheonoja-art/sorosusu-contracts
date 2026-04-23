#![cfg_attr(not(test), no_std)]
use soroban_sdk::{testutils::Address as _, Address, Env, token, Vec, String};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, DexSwapConfig, DexSwapRecord, GasReserve};

#[test]
fn test_dex_auto_swap_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let dex_contract = Address::generate(&env);
    let xlm_token = Address::generate(&env);
    let usdc_token = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin, &0);

    let circle_id = client.create_circle(&creator, &1_000_000, &10, &usdc_token, &86400, &0);

    let config = DexSwapConfig {
        enabled: true,
        swap_threshold_xlm: 10_000_000,
        swap_percentage_bps: 5000,
        dex_contract,
        xlm_token,
        slippage_tolerance_bps: 100,
        minimum_swap_amount: 50_000_000,
        emergency_pause: false,
        last_swap_timestamp: 0,
        total_swapped_xlm: 0,
    };

    client.configure_dex_swap(&admin, &circle_id, &config);
    
    let stored = client.get_dex_swap_config(&circle_id).unwrap();
    assert_eq!(stored.enabled, true);

    client.trigger_dex_swap(&admin, &circle_id);
    
    let record = client.get_dex_swap_record(&circle_id, &0).unwrap();
    assert!(record.success);
}
