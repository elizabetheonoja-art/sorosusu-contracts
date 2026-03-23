use soroban_sdk::{Address, Env, String, Symbol};
use sorosusu_contracts::{SoroSusu, SoroSusuTrait, DataKey, CollateralStatus, MemberStatus};

#[test]
fn test_collateral_required_for_high_value_circles() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create a high-value circle (above threshold)
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64, // 1 day cycle
        &100u32,   // 1% insurance fee
        &nft_contract,
    );
    
    // Verify collateral is required
    let circle_key = DataKey::Circle(circle_id);
    let circle_info = env.storage().instance().get::<_, sorosusu_contracts::CircleInfo>(&circle_key).unwrap();
    assert!(circle_info.requires_collateral);
    assert_eq!(circle_info.collateral_bps, 2000); // 20%
    assert_eq!(circle_info.total_cycle_value, high_amount * 5);
}

#[test]
fn test_collateral_not_required_for_low_value_circles() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create a low-value circle (below threshold)
    let low_amount = 100_000_0; // 100 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &low_amount,
        &max_members,
        &token,
        &86400u64, // 1 day cycle
        &100u32,   // 1% insurance fee
        &nft_contract,
    );
    
    // Verify collateral is not required
    let circle_key = DataKey::Circle(circle_id);
    let circle_info = env.storage().instance().get::<_, sorosusu_contracts::CircleInfo>(&circle_key).unwrap();
    assert!(!circle_info.requires_collateral);
    assert_eq!(circle_info.collateral_bps, 0);
}

#[test]
fn test_stake_collateral() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Calculate required collateral (20% of total cycle value)
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000; // 20%
    
    // Mock token transfer (in real test, you'd use token contract)
    // For this test, we'll assume the transfer succeeds
    
    // Stake collateral
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    // Verify collateral is staked
    let collateral_key = DataKey::CollateralVault(user, circle_id);
    let collateral_info = env.storage().instance().get::<_, sorosusu_contracts::CollateralInfo>(&collateral_key).unwrap();
    assert_eq!(collateral_info.status, CollateralStatus::Staked);
    assert_eq!(collateral_info.amount, required_collateral);
}

#[test]
fn test_join_circle_requires_collateral() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Try to join without staking collateral - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "join_circle"),
        (user.clone(), circle_id, 1u32, Option::<Address>::None),
    );
    assert!(result.is_err());
    
    // Stake collateral first
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    // Now joining should work (assuming token transfer is mocked)
    // In a real test, you'd need to set up token contracts properly
}

#[test]
fn test_mark_member_defaulted_and_slash_collateral() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Stake collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    // Mark member as defaulted
    client.mark_member_defaulted(&creator, &circle_id, &user);
    
    // Verify member is marked as defaulted
    let member_key = DataKey::Member(user.clone());
    let member_info = env.storage().instance().get::<_, sorosusu_contracts::Member>(&member_key).unwrap();
    assert_eq!(member_info.status, MemberStatus::Defaulted);
    
    // Verify collateral is slashed
    let collateral_key = DataKey::CollateralVault(user, circle_id);
    let collateral_info = env.storage().instance().get::<_, sorosusu_contracts::CollateralInfo>(&collateral_key).unwrap();
    assert_eq!(collateral_info.status, CollateralStatus::Slashed);
    
    // Verify slashed amount is in group reserve
    let reserve = env.storage().instance().get::<_, i128>(&DataKey::GroupReserve).unwrap_or(0);
    assert_eq!(reserve, required_collateral);
}

#[test]
fn test_release_collateral_after_completion() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Stake collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    // Simulate member completing all contributions
    let member_key = DataKey::Member(user.clone());
    let mut member_info = sorosusu_contracts::Member {
        address: user.clone(),
        index: 0,
        contribution_count: max_members, // Completed all contributions
        last_contribution_time: env.ledger().timestamp(),
        status: MemberStatus::Active,
        tier_multiplier: 1,
        referrer: None,
        buddy: None,
    };
    env.storage().instance().set(&member_key, &member_info);
    
    // Release collateral
    client.release_collateral(&user, &circle_id, &user);
    
    // Verify collateral is released
    let collateral_key = DataKey::CollateralVault(user, circle_id);
    let collateral_info = env.storage().instance().get::<_, sorosusu_contracts::CollateralInfo>(&collateral_key).unwrap();
    assert_eq!(collateral_info.status, CollateralStatus::Released);
    assert!(collateral_info.release_timestamp.is_some());
}

#[test]
fn test_insufficient_collateral_amount() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Calculate required collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    let insufficient_amount = required_collateral - 100_000_0; // Less than required
    
    // Try to stake insufficient collateral - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "stake_collateral"),
        (user, circle_id, insufficient_amount),
    );
    assert!(result.is_err());
}

#[test]
fn test_double_collateral_staking() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuTrait::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    // Initialize contract
    client.init(&admin);
    
    // Create high-value circle
    let high_amount = 2_000_000_0; // 2000 XLM
    let max_members = 5u32;
    let circle_id = client.create_circle(
        &creator,
        &high_amount,
        &max_members,
        &token,
        &86400u64,
        &100u32,
        &nft_contract,
    );
    
    // Calculate required collateral
    let total_cycle_value = high_amount * 5;
    let required_collateral = (total_cycle_value * 2000) / 10000;
    
    // Stake collateral first time
    client.stake_collateral(&user, &circle_id, &required_collateral);
    
    // Try to stake again - should fail
    let result = env.try_invoke_contract::<_, ()>(
        &contract_id,
        &Symbol::new(&env, "stake_collateral"),
        (user, circle_id, required_collateral),
    );
    assert!(result.is_err());
}
