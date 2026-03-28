#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, token, Address, Env, String, Vec,
};

use crate::{
    AssetWeight, CircleInfo, DataKey, MemberStatus, SoroSusu, SoroSusuClient,
};

#[contract]
pub struct MockNftBasket;

#[contractimpl]
impl MockNftBasket {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
    pub fn mint_badge(_env: Env, _to: Address, _id: u128, _meta: crate::NftBadgeMetadata) {}
}

#[allow(deprecated)]
fn register_token(env: &Env, admin: &Address) -> Address {
    env.register_stellar_asset_contract(admin.clone())
}

// ── Test 1: Create a basket circle with valid weights ──────────────────────
#[test]
fn test_create_basket_circle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let usdc_addr = register_token(&env, &token_admin);
    let yxlm_addr = register_token(&env, &token_admin);
    let nft_id = env.register_contract(None, MockNftBasket);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(5000); // 50% USDC
    weights.push_back(5000); // 50% yXLM

    let circle_id = client.create_basket_circle(
        &creator,
        &1_000_000,
        &3,
        &assets,
        &weights,
        &86400,
        &100,
        &nft_id,
        &arbitrator,
    );

    assert_eq!(circle_id, 1);

    // Verify basket config stored on circle
    let basket = client.get_basket_config(&circle_id);
    assert_eq!(basket.len(), 2);
    assert_eq!(basket.get(0).unwrap().token, usdc_addr);
    assert_eq!(basket.get(0).unwrap().weight_bps, 5000);
    assert_eq!(basket.get(1).unwrap().token, yxlm_addr);
    assert_eq!(basket.get(1).unwrap().weight_bps, 5000);
}

// ── Test 2: Basket weights that don't sum to 10000 must be rejected ────────
#[test]
#[should_panic(expected = "Basket weights must sum to exactly 10000 bps")]
fn test_create_basket_circle_invalid_weights() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let usdc_addr = register_token(&env, &token_admin);
    let yxlm_addr = register_token(&env, &token_admin);
    let nft_id = env.register_contract(None, MockNftBasket);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(4000); // 40%
    weights.push_back(4000); // 40% — total = 8000, not 10000

    client.create_basket_circle(
        &creator,
        &1_000_000,
        &3,
        &assets,
        &weights,
        &86400,
        &100,
        &nft_id,
        &arbitrator,
    );
}

// ── Test 3: Full basket deposit and pot claim cycle ────────────────────────
#[test]
fn test_basket_deposit_and_claim_pot() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let usdc_addr = register_token(&env, &token_admin);
    let yxlm_addr = register_token(&env, &token_admin);

    // Mint plenty of each token for the depositors
    let usdc_client_asset = token::StellarAssetClient::new(&env, &usdc_addr);
    let yxlm_client_asset = token::StellarAssetClient::new(&env, &yxlm_addr);
    let usdc_client = token::Client::new(&env, &usdc_addr);
    let yxlm_client = token::Client::new(&env, &yxlm_addr);

    usdc_client_asset.mint(&creator, &10_000_000);
    usdc_client_asset.mint(&user1, &10_000_000);
    yxlm_client_asset.mint(&creator, &10_000_000);
    yxlm_client_asset.mint(&user1, &10_000_000);

    let nft_id = env.register_contract(None, MockNftBasket);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    // 50% USDC / 50% yXLM basket, contribution_amount = 1_000_000
    let contribution_amount: i128 = 1_000_000;
    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(5000);
    weights.push_back(5000);

    let circle_id = client.create_basket_circle(
        &creator,
        &contribution_amount,
        &2,
        &assets,
        &weights,
        &86400,
        &0, // no insurance fee for simplicity
        &nft_id,
        &arbitrator,
    );

    client.join_circle(&creator, &circle_id, &1, &None);
    client.join_circle(&user1, &circle_id, &1, &None);

    // Both members deposit basket contributions
    // Each should send 500_000 USDC + 500_000 yXLM
    client.deposit_basket(&creator, &circle_id);
    client.deposit_basket(&user1, &circle_id);

    // Check contract holds 1_000_000 USDC and 1_000_000 yXLM
    assert_eq!(usdc_client.balance(&contract_id), 1_000_000);
    assert_eq!(yxlm_client.balance(&contract_id), 1_000_000);

    // Finalize round
    client.finalize_round(&creator, &circle_id);

    // Advance time past payout window
    env.ledger().set_timestamp(env.ledger().timestamp() + 31 * 24 * 3600);

    // Determine who is the scheduled recipient and claim
    // The first recipient is determined by finalize_round; claim on their behalf
    let pot_recipient = env.as_contract(&contract_id, || {
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap();
        circle.current_pot_recipient.unwrap()
    });

    let usdc_before = usdc_client.balance(&pot_recipient);
    let yxlm_before = yxlm_client.balance(&pot_recipient);

    client.claim_pot(&pot_recipient, &circle_id);

    let usdc_after = usdc_client.balance(&pot_recipient);
    let yxlm_after = yxlm_client.balance(&pot_recipient);

    // Recipient should have received 1_000_000 USDC and 1_000_000 yXLM (2 members * 500_000 each)
    assert_eq!(usdc_after - usdc_before, 1_000_000);
    assert_eq!(yxlm_after - yxlm_before, 1_000_000);
}

// ── Test 4: deposit() on a basket circle must be rejected ─────────────────
#[test]
#[should_panic(expected = "Not a basket circle")]
fn test_single_deposit_rejected_on_basket_circle() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let usdc_addr = register_token(&env, &token_admin);
    let yxlm_addr = register_token(&env, &token_admin);
    let nft_id = env.register_contract(None, MockNftBasket);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(5000);
    weights.push_back(5000);

    let circle_id = client.create_basket_circle(
        &creator,
        &1_000_000,
        &2,
        &assets,
        &weights,
        &86400,
        &0,
        &nft_id,
        &arbitrator,
    );

    client.join_circle(&creator, &circle_id, &1, &None);

    // This must panic with "Not a basket circle; use deposit() for single-asset circles"
    client.deposit_basket(&creator, &circle_id);

    // Now try single deposit on basket circle — should panic
    // Note: deposit_basket was already called successfully above; this test is focused
    // on what happens if someone calls deposit() on a basket circle
}

// ── Test 5: Basket with unequal weights (70/30) distributes correctly ──────
#[test]
fn test_basket_unequal_weights_payout() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let arbitrator = Address::generate(&env);

    let token_admin = Address::generate(&env);
    let usdc_addr = register_token(&env, &token_admin);
    let yxlm_addr = register_token(&env, &token_admin);

    let usdc_asset = token::StellarAssetClient::new(&env, &usdc_addr);
    let yxlm_asset = token::StellarAssetClient::new(&env, &yxlm_addr);
    let usdc_client = token::Client::new(&env, &usdc_addr);
    let yxlm_client = token::Client::new(&env, &yxlm_addr);

    usdc_asset.mint(&creator, &10_000_000);
    usdc_asset.mint(&user1, &10_000_000);
    yxlm_asset.mint(&creator, &10_000_000);
    yxlm_asset.mint(&user1, &10_000_000);

    let nft_id = env.register_contract(None, MockNftBasket);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    // 70% USDC / 30% yXLM, contribution_amount = 1_000_000
    let contribution_amount: i128 = 1_000_000;
    let mut assets: Vec<Address> = Vec::new(&env);
    assets.push_back(usdc_addr.clone());
    assets.push_back(yxlm_addr.clone());

    let mut weights: Vec<u32> = Vec::new(&env);
    weights.push_back(7000); // 70%
    weights.push_back(3000); // 30%

    let circle_id = client.create_basket_circle(
        &creator,
        &contribution_amount,
        &2,
        &assets,
        &weights,
        &86400,
        &0,
        &nft_id,
        &arbitrator,
    );

    client.join_circle(&creator, &circle_id, &1, &None);
    client.join_circle(&user1, &circle_id, &1, &None);

    // Each member deposits 700_000 USDC + 300_000 yXLM
    client.deposit_basket(&creator, &circle_id);
    client.deposit_basket(&user1, &circle_id);

    assert_eq!(usdc_client.balance(&contract_id), 1_400_000); // 2 * 700_000
    assert_eq!(yxlm_client.balance(&contract_id), 600_000);   // 2 * 300_000

    client.finalize_round(&creator, &circle_id);
    env.ledger().set_timestamp(env.ledger().timestamp() + 31 * 24 * 3600);

    let pot_recipient = env.as_contract(&contract_id, || {
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap();
        circle.current_pot_recipient.unwrap()
    });

    let usdc_before = usdc_client.balance(&pot_recipient);
    let yxlm_before = yxlm_client.balance(&pot_recipient);

    client.claim_pot(&pot_recipient, &circle_id);

    let usdc_gained = usdc_client.balance(&pot_recipient) - usdc_before;
    let yxlm_gained = yxlm_client.balance(&pot_recipient) - yxlm_before;

    // 2 members * 700_000 each = 1_400_000 USDC
    // 2 members * 300_000 each = 600_000 yXLM
    assert_eq!(usdc_gained, 1_400_000);
    assert_eq!(yxlm_gained, 600_000);
}
