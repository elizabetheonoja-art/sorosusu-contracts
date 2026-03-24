#![cfg(test)]

use soroban_sdk::{contract, contractimpl, testutils::Address as _, token, Address, Env};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, SoroSusuTrait};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn insurance_default_adds_member_to_global_blacklist() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let defaulter = Address::generate(&env);
    let paying_member = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_id = env.register_contract(None, MockNft);

    token_admin_client.mint(&paying_member, &1_000);

    let circle_id = client.create_circle(&creator, &100, &2, &token_id, &86_400, &10_000, &nft_id);
    client.join_circle(&defaulter, &circle_id, &1, &None);
    client.join_circle(&paying_member, &circle_id, &1, &None);

    client.deposit(&paying_member, &circle_id);
    client.trigger_insurance_coverage(&creator, &circle_id, &defaulter);

    assert!(client.is_globally_blacklisted(&defaulter));
}

#[test]
#[should_panic(expected = "User is globally blacklisted")]
fn blacklisted_address_cannot_join_circle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let second_creator = Address::generate(&env);
    let blacklisted_member = Address::generate(&env);
    let paying_member = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_id = env.register_contract(None, MockNft);

    token_admin_client.mint(&paying_member, &1_000);

    let first_circle_id = client.create_circle(&creator, &100, &2, &token_id, &86_400, &10_000, &nft_id);
    client.join_circle(&blacklisted_member, &first_circle_id, &1, &None);
    client.join_circle(&paying_member, &first_circle_id, &1, &None);
    client.deposit(&paying_member, &first_circle_id);
    client.trigger_insurance_coverage(&creator, &first_circle_id, &blacklisted_member);

    let second_circle_id = client.create_circle(&second_creator, &100, &3, &token_id, &86_400, &100, &nft_id);
    client.join_circle(&blacklisted_member, &second_circle_id, &1, &None);
}

#[test]
fn admin_can_clear_blacklist_entry() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let nft_id = env.register_contract(None, MockNft);

    let circle_id = client.create_circle(&creator, &100, &3, &token_id, &86_400, &100, &nft_id);
    client.join_circle(&member, &circle_id, &1, &None);
    client.flag_member_for_default(&admin, &circle_id, &member);
    assert!(client.is_globally_blacklisted(&member));

    client.clear_global_blacklist(&admin, &member);
    assert!(!client.is_globally_blacklisted(&member));
}
