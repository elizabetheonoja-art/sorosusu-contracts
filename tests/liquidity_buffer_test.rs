use sorosusu_contracts::liquidity_buffer::{LiquidityBuffer, LiquidityBufferClient};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, UserStats, DataKey};
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, String, Symbol, contract, contractimpl};

#[test]
fn test_liquidity_buffer_initialization() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, LiquidityBuffer);
    let client = LiquidityBufferClient::new(&env, &contract_id);
    client.init_liquidity_buffer(&admin);
}

#[test]
fn test_advance_request() {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let member = Address::generate(&env);
    let creator = Address::generate(&env);
    let contract_id = env.register_contract(None, LiquidityBuffer);
    let client = LiquidityBufferClient::new(&env, &contract_id);
    client.init_liquidity_buffer(&admin);
    
    let user_stats = UserStats {
        total_volume_saved: 1000000_000_000,
        on_time_contributions: 10,
        late_contributions: 0
    };
    env.storage().instance().set(&DataKey::UserStats(member.clone()), &user_stats);
    
    let s_id = env.register_contract(None, SoroSusu);
    let s_client = SoroSusuClient::new(&env, &s_id);
    s_client.init(&admin, &0);
    let circle_id = s_client.create_circle(&creator, &1_000_000_000, &5, &Address::generate(&env), &86400, &100i128);
    
    client.signal_advance_request(&member, &circle_id, &100_000_000, &String::from_str(&env, "Need advance"));
}
