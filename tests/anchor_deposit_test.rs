#![cfg(test)]

use crate::{
    token, AnchorDeposit, AnchorInfo, DataKey, Error, SoroSusu, SoroSusuClient, SoroSusuTrait,
};
use soroban_sdk::{Address, Env, Map, String, Vec};

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
fn test_anchor_registration() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Create anchor info
    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2, // Enhanced
        compliance_level: 2,    // Enhanced KYC
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(
            &env,
            [String::from_str(&env, "US"), String::from_str(&env, "GB")],
        ),
        max_deposit_amount: 10000_000_000,  // 1000 tokens
        daily_deposit_limit: 50000_000_000, // 5000 tokens per day
    };

    // Register anchor
    client.register_anchor(&admin, &anchor_info);

    // Verify anchor info
    let retrieved_anchor = client.get_anchor_info(&anchor_address);
    assert_eq!(
        retrieved_anchor.anchor_name,
        String::from_str(&env, "Test Anchor")
    );
    assert_eq!(
        retrieved_anchor.sep_version,
        String::from_str(&env, "SEP-24")
    );
    assert!(retrieved_anchor.is_active);
}

#[test]
fn test_anchor_deposit_for_user() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Deploy mock token
    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    // Deploy mock NFT
    let nft_id = env.register_contract(None, MockNft);

    // Register and fund anchor
    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2,
        compliance_level: 2,
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(&env, [String::from_str(&env, "US")]),
        max_deposit_amount: 10000_000_000,
        daily_deposit_limit: 50000_000_000,
    };

    client.register_anchor(&admin, &anchor_info);

    // Mint tokens to anchor
    token_client.mint(&anchor_address, &10000_000_000);

    // Create circle
    let contribution_amount: i128 = 1000_000_000; // 100 tokens
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2, // max_members
        &token_id,
        &86400, // 1 day cycle
        &100,   // 1% insurance fee
        &nft_id,
        &admin, // arbitrator
    );

    // User joins circle
    client.join_circle(&user, &circle_id, &1, &None);

    // Anchor deposits for user
    let deposit_memo = String::from_str(&env, "DEP_12345");
    let fiat_reference = String::from_str(&env, "BANK_TX_67890");
    let sep_type = String::from_str(&env, "SEP-24");

    // Calculate expected total amount (contribution + insurance + group insurance)
    let insurance_fee = (contribution_amount * 100) / 10000; // 1%
    let group_insurance_premium = (contribution_amount * 50) / 10000; // 0.5%
    let total_amount = contribution_amount + insurance_fee + group_insurance_premium;

    client.deposit_for_user(
        &anchor_address,
        &user,
        &circle_id,
        &total_amount,
        &deposit_memo,
        &fiat_reference,
        &sep_type,
    );

    // Verify deposit was processed
    let deposit_id = env.ledger().sequence() - 1; // Deposit ID was set to ledger sequence
    let deposit_record = client.get_deposit_record(&deposit_id);

    assert_eq!(deposit_record.anchor_address, anchor_address);
    assert_eq!(deposit_record.beneficiary_user, user);
    assert_eq!(deposit_record.circle_id, circle_id);
    assert_eq!(deposit_record.amount, total_amount);
    assert_eq!(deposit_record.deposit_memo, deposit_memo);
    assert_eq!(deposit_record.fiat_reference, fiat_reference);
    assert_eq!(deposit_record.sep_type, sep_type);
    assert!(deposit_record.processed);
    assert!(deposit_record.compliance_verified);

    // Verify user stats updated
    let member = client.get_member(&user);
    assert!(member.has_contributed_current_round);
    assert_eq!(member.total_contributions, total_amount);
}

#[test]
fn test_anchor_deposit_validation() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let anchor_address = Address::generate(&env);
    let unauthorized_anchor = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Deploy mock token
    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    // Deploy mock NFT
    let nft_id = env.register_contract(None, MockNft);

    // Register anchor
    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2,
        compliance_level: 2,
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(&env, [String::from_str(&env, "US")]),
        max_deposit_amount: 1000_000_000, // Lower limit for testing
        daily_deposit_limit: 5000_000_000,
    };

    client.register_anchor(&admin, &anchor_info);

    // Create circle
    let contribution_amount: i128 = 1000_000_000;
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2,
        &token_id,
        &86400,
        &100,
        &nft_id,
        &admin,
    );

    // User joins circle
    client.join_circle(&user, &circle_id, &1, &None);

    let deposit_memo = String::from_str(&env, "DEP_12345");
    let fiat_reference = String::from_str(&env, "BANK_TX_67890");
    let sep_type = String::from_str(&env, "SEP-24");

    // Test 1: Unauthorized anchor should fail
    env.mock_auths(&[&mock_auth::MockAuth {
        address: &unauthorized_anchor,
        contract: &contract_id,
        fn_name: "deposit_for_user",
        args: (),
        invoked: true,
    }]);

    let result = std::panic::catch_unwind(|| {
        client.deposit_for_user(
            &unauthorized_anchor,
            &user,
            &circle_id,
            &contribution_amount,
            &deposit_memo,
            &fiat_reference,
            &sep_type,
        );
    });
    assert!(result.is_err());

    // Test 2: Amount exceeding limit should fail
    env.mock_auths(&[&mock_auth::MockAuth {
        address: &anchor_address,
        contract: &contract_id,
        fn_name: "deposit_for_user",
        args: (),
        invoked: true,
    }]);

    token_client.mint(&anchor_address, &20000_000_000);

    let result = std::panic::catch_unwind(|| {
        client.deposit_for_user(
            &anchor_address,
            &user,
            &circle_id,
            &2000_000_000, // Exceeds max_deposit_amount
            &deposit_memo,
            &fiat_reference,
            &sep_type,
        );
    });
    assert!(result.is_err());

    // Test 3: Invalid SEP type should fail
    let result = std::panic::catch_unwind(|| {
        client.deposit_for_user(
            &anchor_address,
            &user,
            &circle_id,
            &contribution_amount,
            &deposit_memo,
            &fiat_reference,
            &String::from_str(&env, "SEP-99"), // Invalid SEP
        );
    });
    assert!(result.is_err());

    // Test 4: Inactive anchor should fail
    let inactive_anchor_info = AnchorInfo {
        is_active: false,
        ..anchor_info
    };
    client.register_anchor(&admin, &inactive_anchor_info);

    let result = std::panic::catch_unwind(|| {
        client.deposit_for_user(
            &anchor_address,
            &user,
            &circle_id,
            &contribution_amount,
            &deposit_memo,
            &fiat_reference,
            &sep_type,
        );
    });
    assert!(result.is_err());
}

#[test]
fn test_anchor_deposit_double_processing_prevention() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Deploy mock token
    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    // Deploy mock NFT
    let nft_id = env.register_contract(None, MockNft);

    // Register anchor
    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2,
        compliance_level: 2,
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(&env, [String::from_str(&env, "US")]),
        max_deposit_amount: 10000_000_000,
        daily_deposit_limit: 50000_000_000,
    };

    client.register_anchor(&admin, &anchor_info);

    // Mint tokens to anchor
    token_client.mint(&anchor_address, &20000_000_000);

    // Create circle
    let contribution_amount: i128 = 1000_000_000;
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2,
        &token_id,
        &86400,
        &100,
        &nft_id,
        &admin,
    );

    // User joins circle
    client.join_circle(&user, &circle_id, &1, &None);

    let deposit_memo = String::from_str(&env, "DEP_DUPLICATE_TEST");
    let fiat_reference = String::from_str(&env, "BANK_TX_67890");
    let sep_type = String::from_str(&env, "SEP-24");

    let insurance_fee = (contribution_amount * 100) / 10000;
    let group_insurance_premium = (contribution_amount * 50) / 10000;
    let total_amount = contribution_amount + insurance_fee + group_insurance_premium;

    // First deposit should succeed
    client.deposit_for_user(
        &anchor_address,
        &user,
        &circle_id,
        &total_amount,
        &deposit_memo.clone(),
        &fiat_reference.clone(),
        &sep_type.clone(),
    );

    // Second deposit with same memo should fail
    let result = std::panic::catch_unwind(|| {
        client.deposit_for_user(
            &anchor_address,
            &user,
            &circle_id,
            &total_amount,
            &deposit_memo,
            &fiat_reference,
            &sep_type,
        );
    });
    assert!(result.is_err());
}

#[test]
fn test_sep31_compliance() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Deploy mock token
    let token_admin = Address::generate(&env);
    let token_id = register_token(&env, &token_admin);
    let token_client = token::StellarAssetClient::new(&env, &token_id);

    // Deploy mock NFT
    let nft_id = env.register_contract(None, MockNft);

    // Register SEP-31 anchor (higher compliance requirements)
    let sep31_anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "SEP-31 Compliant Anchor"),
        sep_version: String::from_str(&env, "SEP-31"),
        authorization_level: 3, // Full
        compliance_level: 3,    // Full KYC+AML
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(
            &env,
            [
                String::from_str(&env, "US"),
                String::from_str(&env, "GB"),
                String::from_str(&env, "CA"),
            ],
        ),
        max_deposit_amount: 50000_000_000, // Higher limits for SEP-31
        daily_deposit_limit: 250000_000_000,
    };

    client.register_anchor(&admin, &sep31_anchor_info);

    // Mint tokens to anchor
    token_client.mint(&anchor_address, &100000_000_000);

    // Create circle
    let contribution_amount: i128 = 5000_000_000; // Higher amount for SEP-31
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2,
        &token_id,
        &86400,
        &100,
        &nft_id,
        &admin,
    );

    // User joins circle
    client.join_circle(&user, &circle_id, &1, &None);

    // SEP-31 deposit should work
    let deposit_memo = String::from_str(&env, "SEP31_DEP_12345");
    let fiat_reference = String::from_str(&env, "BANK_WIRE_67890");
    let sep_type = String::from_str(&env, "SEP-31");

    let insurance_fee = (contribution_amount * 100) / 10000;
    let group_insurance_premium = (contribution_amount * 50) / 10000;
    let total_amount = contribution_amount + insurance_fee + group_insurance_premium;

    client.deposit_for_user(
        &anchor_address,
        &user,
        &circle_id,
        &total_amount,
        &deposit_memo,
        &fiat_reference,
        &sep_type,
    );

    // Verify SEP-31 specific compliance
    let deposit_id = env.ledger().sequence() - 1;
    let deposit_record = client.get_deposit_record(&deposit_id);

    assert_eq!(deposit_record.sep_type, String::from_str(&env, "SEP-31"));
    assert!(deposit_record.compliance_verified);
    assert!(deposit_record.processed);

    // Verify anchor info shows SEP-31 compliance
    let anchor_info = client.get_anchor_info(&anchor_address);
    assert_eq!(anchor_info.sep_version, String::from_str(&env, "SEP-31"));
    assert_eq!(anchor_info.compliance_level, 3); // Full KYC+AML
}

#[test]
fn test_anchor_audit_trail() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let anchor_address = Address::generate(&env);

    // Register contract
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    // Initialize contract
    client.init(&admin);

    // Get initial audit count
    let initial_audit_count = client
        .query_audit_by_resource(&0, &0, &u64::MAX, &0, &100)
        .len();

    // Register anchor (should create audit entry)
    let anchor_info = AnchorInfo {
        anchor_address: anchor_address.clone(),
        anchor_name: String::from_str(&env, "Audit Test Anchor"),
        sep_version: String::from_str(&env, "SEP-24"),
        authorization_level: 2,
        compliance_level: 2,
        is_active: true,
        registration_timestamp: env.ledger().timestamp(),
        last_activity: env.ledger().timestamp(),
        supported_countries: Vec::from_array(&env, [String::from_str(&env, "US")]),
        max_deposit_amount: 10000_000_000,
        daily_deposit_limit: 50000_000_000,
    };

    client.register_anchor(&admin, &anchor_info);

    // Check that audit entry was created for anchor registration
    let audit_entries = client.query_audit_by_resource(&0, &0, &u64::MAX, &0, &100);
    assert_eq!(audit_entries.len(), initial_audit_count + 1);

    let registration_audit = audit_entries.get(audit_entries.len() - 1).unwrap();
    assert_eq!(registration_audit.actor, admin);
    assert_eq!(registration_audit.action, crate::AuditAction::AdminAction);
    assert_eq!(registration_audit.resource_id, 0); // Anchor registration uses resource_id 0
}
