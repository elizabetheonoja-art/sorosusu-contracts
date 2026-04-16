#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn register_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

#[test]
fn test_full_rosca_cycle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize
    client.init(&admin, &0);

    // Deploy mock token
    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let token_token_client = token::Client::new(&env, &token_id);

    // Mint tokens to users
    token_client.mint(&user1, &10000);
    token_client.mint(&user2, &10000);
    token_client.mint(&creator, &10000); // For bond

    // Create circle
    let contribution_amount: i128 = 1000;
    let cycle_duration: u64 = 86400; // 1 day
    let circle_id = client.create_circle(&creator, &contribution_amount, &2u32, &token_id, &cycle_duration, &100i128);

    // Join circle
    client.join_circle(&user1, &circle_id);
    client.join_circle(&user2, &circle_id);

    // Deposits
    client.deposit(&user1, &circle_id, &1u32);
    client.deposit(&user2, &circle_id, &1u32);

    // Check balances (10000 - 1000)
    assert_eq!(token_token_client.balance(&user1), 9000);
    assert_eq!(token_token_client.balance(&user2), 9000);

    client.finalize_round(&creator, &circle_id);

    let circle = client.get_circle(&circle_id);
    assert!(circle.is_round_finalized);
}
