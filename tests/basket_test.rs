#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, token, Address, Env, Vec};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNftBasket;

#[contractimpl]
impl MockNftBasket {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

fn register_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

#[test]
fn test_basket_circle_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let usdc_admin = Address::generate(&env);
    let yxlm_admin = Address::generate(&env);

    let usdc_addr = register_token(&env, &usdc_admin);
    let yxlm_addr = register_token(&env, &yxlm_admin);

    let usdc_client = token::Client::new(&env, &usdc_addr);
    let yxlm_client = token::Client::new(&env, &yxlm_addr);
    let usdc_asset = token::StellarAssetClient::new(&env, &usdc_addr);
    let yxlm_asset = token::StellarAssetClient::new(&env, &yxlm_addr);

    usdc_asset.mint(&creator, &10_000_000);
    usdc_asset.mint(&user1, &10_000_000);
    yxlm_asset.mint(&creator, &10_000_000);
    yxlm_asset.mint(&user1, &10_000_000);

    let nft_id = env.register_contract(None, MockNftBasket);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin, &0);

    let contribution_amount: i128 = 1_000_000;
    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(5000); // 50%
    weights.push_back(5000); // 50%

    let circle_id = client.create_basket_circle(&creator, &contribution_amount, &2, &assets, &weights, &86400, &0, &nft_id, &arbitrator);

    client.join_circle(&creator, &circle_id);
    client.join_circle(&user1, &circle_id);

    client.deposit_basket(&creator, &circle_id);
    client.deposit_basket(&user1, &circle_id);

    // Verify contract balances (500k + 500k = 1M each)
    assert_eq!(usdc_client.balance(&contract_id), 1_000_000);
    assert_eq!(yxlm_client.balance(&contract_id), 1_000_000);
}
