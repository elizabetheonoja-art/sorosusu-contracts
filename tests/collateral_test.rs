use soroban_sdk::{testutils::Address as _, Address, Env, String, Symbol, token, contract, contractimpl, Vec};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, DataKey, CollateralStatus, MemberStatus, CircleInfo, CollateralInfo};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_collateral_required_for_high_value_circles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let circle_key = DataKey::Circle(circle_id);
    let circle_info: CircleInfo = env.as_contract(&contract_id, || {
        env.storage().instance().get(&circle_key).unwrap()
    });
    
    assert!(circle_info.requires_collateral);
}

#[test]
fn test_collateral_not_required_for_low_value_circles() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let low_amount = 100_000_000;
    let circle_id = client.create_circle(&creator, &low_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let circle_key = DataKey::Circle(circle_id);
    let circle_info: CircleInfo = env.as_contract(&contract_id, || {
        env.storage().instance().get(&circle_key).unwrap()
    });
    assert!(!circle_info.requires_collateral);
}

#[test]
fn test_stake_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
    let collateral_info: CollateralInfo = env.as_contract(&contract_id, || {
        env.storage().instance().get(&collateral_key).unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Staked);
}

#[test]
fn test_join_circle_requires_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let result = client.try_join_circle(&user, &circle_id, &1, &None);
    assert!(result.is_err());
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    token_client.mint(&user, &(high_amount * 2));
    client.join_circle(&user, &circle_id, &1, &None);
}

#[test]
fn test_mark_member_defaulted_and_slash_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    token_client.mint(&user, &(high_amount * 2));
    client.join_circle(&user, &circle_id, &1, &None);
    
    client.mark_member_defaulted(&creator, &circle_id, &user);
    
    let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
    let collateral_info: CollateralInfo = env.as_contract(&contract_id, || {
        env.storage().instance().get(&collateral_key).unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Slashed);
}

#[test]
fn test_release_collateral_after_completion() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    token_client.mint(&user, &(high_amount * 2));
    client.join_circle(&user, &circle_id, &1, &None);
    
    let member_key = DataKey::Member(user.clone());
    env.as_contract(&contract_id, || {
        let mut member_info = sorosusu_contracts::Member {
            address: user.clone(),
            index: 0,
            contribution_count: 5,
            on_time_count: 5,
            last_contribution_time: env.ledger().timestamp(),
            status: MemberStatus::Active,
            tier_multiplier: 1,
            referrer: None,
            buddy: None,
        };
        env.storage().instance().set(&member_key, &member_info);
    });
    
    client.release_collateral(&user, &circle_id, &user);
    
    let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
    let collateral_info: CollateralInfo = env.as_contract(&contract_id, || {
        env.storage().instance().get(&collateral_key).unwrap()
    });
    assert_eq!(collateral_info.status, CollateralStatus::Released);
}

#[test]
fn test_insufficient_collateral_amount() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    let insufficient_amount = required_collateral - 100_000_000;
    
    let result = client.try_stake_collateral(&user, &circle_id, &insufficient_amount);
    assert!(result.is_err());
}

#[test]
fn test_double_collateral_staking() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let nft_contract = env.register_contract(None, MockNft);
    
    client.init(&admin);
    
    let high_amount = 3_000_000_000;
    let circle_id = client.create_circle(&creator, &high_amount, &5, &token_id, &86400, &100, &nft_contract, &0);
    
    let required_collateral = (high_amount * 5 * 2000) / 10000;
    token_client.mint(&user, &required_collateral);
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    let result = client.try_stake_collateral(&user, &circle_id, &required_collateral);
    assert!(result.is_err());
}
