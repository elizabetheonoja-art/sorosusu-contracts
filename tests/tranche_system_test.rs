#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, symbol_short, Address, Env, Symbol};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, TrancheStatus};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn mint(env: Env, _to: Address, amount: i128) {
        let mut balance = env.storage().instance().get::<Symbol, i128>(&symbol_short!("balance")).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&symbol_short!("balance"), &balance);
    }
    
    pub fn balance(env: Env, account: Address) -> i128 {
        if account == env.current_contract_address() {
            env.storage().instance().get::<Symbol, i128>(&symbol_short!("balance")).unwrap_or(0)
        } else {
            1000_000_000_000 // Large balance for testing
        }
    }
    
    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
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
    client.init(&admin, &0);
    
    // Create mock token
    let token_address = env.register_contract(None, MockToken);
    
    // Create circle
    let circle_id = client.create_circle(&circle_creator, &1_000_000i128, &3u32, &token_address, &86400u64, &0i128);
    
    (env, client, admin, circle_creator, member1, circle_id)
}

#[test]
fn test_tranche_schedule_creation_on_payout() {
    let (env, client, _admin, circle_creator, _member1, circle_id) = setup_test_env();
    
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    
    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&member3, &circle_id);
    
    client.deposit(&member1, &circle_id, &1);
    client.deposit(&member2, &circle_id, &1);
    client.deposit(&member3, &circle_id, &1);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    let first_recipient = member1.clone();
    let schedule = client.get_tranche_schedule(&circle_id, &first_recipient);
    
    assert!(schedule.is_some());
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
    
    client.deposit(&member1, &circle_id, &1);
    client.deposit(&member2, &circle_id, &1);
    client.deposit(&member3, &circle_id, &1);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    client.deposit(&member1, &circle_id, &1);
    client.deposit(&member2, &circle_id, &1);
    client.deposit(&member3, &circle_id, &1);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    client.claim_tranche(&member1, &circle_id, &0);
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
    
    client.deposit(&member1, &circle_id, &1);
    client.deposit(&member2, &circle_id, &1);
    client.deposit(&member3, &circle_id, &1);
    
    client.finalize_round(&circle_creator, &circle_id);
    client.distribute_payout(&circle_creator, &circle_id);
    
    client.mark_member_defaulted(&admin, &circle_id, &member1);
    client.execute_tranche_clawback(&admin, &circle_id, &member1);
}
