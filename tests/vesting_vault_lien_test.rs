#![cfg(test)]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, symbol_short,
    token, Address, Env, String, Symbol, Vec,
};
use std::panic;

use sorosusu::SoroSusuClient;
use sorosusu::SoroSusuTrait;
use sorosusu::DataKey;
use sorosusu::VestingVaultLienInfo;
use sorosusu::LienInfo;
use sorosusu::LienStatus;
use sorosusu::MemberStatus;

// Mock Vesting Vault Contract for testing
#[contract]
pub struct MockVestingVault;

#[contractimpl]
impl MockVestingVault {
    pub fn initialize(env: Env, admin: Address) {
        // Mock initialization
    }
    
    pub fn get_vesting_balance(env: Env, user: Address) -> i128 {
        // Mock vesting balance - return a fixed amount for testing
        1_000_000_0 // 100 tokens
    }
    
    pub fn get_vesting_end_time(env: Env, user: Address) -> u64 {
        // Mock vesting end time - far in the future
        env.ledger().timestamp() + 86400 * 30 // 30 days from now
    }
    
    pub fn create_lien(env: Env, user: Address, amount: i128, recipient: Address) -> bool {
        // Mock lien creation - always succeeds
        true
    }
    
    pub fn claim_lien(env: Env, user: Address, amount: i128, claimant: Address) -> bool {
        // Mock lien claim - always succeeds
        true
    }
    
    pub fn release_lien(env: Env, user: Address, amount: i128) -> bool {
        // Mock lien release - always succeeds
        true
    }
}

#[test]
fn test_create_vesting_lien() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a high-value circle that requires collateral
    let circle_id = client.create_circle(
        &creator,
        &2_000_000_0, // 200 tokens per contribution
        &5u32,        // 5 members = 1000 tokens total (requires collateral)
        &token_contract,
        &86400u64,    // 1 day cycles
        &100u32,      // 1% insurance
        &nft_contract,
        &admin,
    );
    
    // Create vesting lien
    let lien_amount = 400_000_0; // 40 tokens (20% of 200 tokens contribution)
    let lien_id = client.create_vesting_lien(
        &member,
        &circle_id,
        &vesting_vault_contract,
        &lien_amount,
    );
    
    // Verify lien was created
    assert_eq!(lien_id, 1);
    
    let lien_info = client.get_vesting_lien(&member, &circle_id);
    assert!(lien_info.is_some());
    
    let lien = lien_info.unwrap();
    assert_eq!(lien.member, member);
    assert_eq!(lien.circle_id, circle_id);
    assert_eq!(lien.vesting_vault_contract, vesting_vault_contract);
    assert_eq!(lien.lien_amount, lien_amount);
    assert_eq!(lien.status, LienStatus::Active);
}

#[test]
fn test_join_circle_with_vesting_lien() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a high-value circle that requires collateral
    let circle_id = client.create_circle(
        &creator,
        &2_000_000_0, // 200 tokens per contribution
        &5u32,        // 5 members = 1000 tokens total (requires collateral)
        &token_contract,
        &86400u64,    // 1 day cycles
        &100u32,      // 1% insurance
        &nft_contract,
        &admin,
    );
    
    // Create vesting lien with sufficient amount
    let total_contribution_value = 2_000_000_0 * 5; // 1000 tokens total
    let required_lien_amount = (total_contribution_value * 2000) / 10000; // 20% = 200 tokens
    client.create_vesting_lien(
        &member,
        &circle_id,
        &vesting_vault_contract,
        &required_lien_amount,
    );
    
    // Should be able to join circle with lien
    client.join_circle(
        &member,
        &circle_id,
        &1u32,
        &None,
    );
    
    // Verify member joined successfully
    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.member_count, 2); // creator + member
}

#[test]
#[should_panic(expected = "Vesting lien amount insufficient")]
fn test_join_circle_insufficient_lien() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a high-value circle that requires collateral
    let circle_id = client.create_circle(
        &creator,
        &2_000_000_0, // 200 tokens per contribution
        &5u32,        // 5 members = 1000 tokens total (requires collateral)
        &token_contract,
        &86400u64,    // 1 day cycles
        &100u32,      // 1% insurance
        &nft_contract,
        &admin,
    );
    
    // Create vesting lien with insufficient amount
    client.create_vesting_lien(
        &member,
        &circle_id,
        &vesting_vault_contract,
        &100_000_0, // Only 10 tokens (should be 200)
    );
    
    // Should fail to join circle
    client.join_circle(
        &member,
        &circle_id,
        &1u32,
        &None,
    );
}

#[test]
fn test_claim_vesting_lien_on_default() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a high-value circle
    let circle_id = client.create_circle(
        &creator,
        &2_000_000_0,
        &5u32,
        &token_contract,
        &86400u64,
        &100u32,
        &nft_contract,
        &admin,
    );
    
    // Creator joins first
    client.join_circle(&creator, &circle_id, &1u32, &None);
    
    // Member creates lien and joins
    let lien_amount = 400_000_0;
    client.create_vesting_lien(&member, &circle_id, &vesting_vault_contract, &lien_amount);
    client.join_circle(&member, &circle_id, &1u32, &None);
    
    // Verify lien is active
    let lien_info = client.get_vesting_lien(&member, &circle_id).unwrap();
    assert_eq!(lien_info.status, LienStatus::Active);
    
    // Mark member as defaulted
    client.mark_member_defaulted(&creator, &circle_id, &member);
    
    // Verify lien was claimed
    let lien_info = client.get_vesting_lien(&member, &circle_id).unwrap();
    assert_eq!(lien_info.status, LienStatus::Claimed);
    assert!(lien_info.claim_timestamp.is_some());
}

#[test]
fn test_auto_release_lien_on_completion() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a circle
    let circle_id = client.create_circle(
        &creator,
        &1_000_000_0, // 100 tokens per contribution
        &2u32,         // 2 members (small for easier testing)
        &token_contract,
        &86400u64,
        &100u32,
        &nft_contract,
        &admin,
    );
    
    // Both members join
    client.join_circle(&creator, &circle_id, &1u32, &None);
    
    let lien_amount = 200_000_0;
    client.create_vesting_lien(&member, &circle_id, &vesting_vault_contract, &lien_amount);
    client.join_circle(&member, &circle_id, &1u32, &None);
    
    // Start round and complete contributions
    client.start_round(&creator, &circle_id);
    client.deposit(&creator, &circle_id);
    client.deposit(&member, &circle_id);
    
    // Finalize round
    client.finalize_round(&creator, &circle_id);
    
    // Member claims pot (simulating completion)
    env.ledger().set_timestamp(env.ledger().timestamp() + 86400); // Wait for payout time
    client.claim_pot(&creator, &circle_id);
    
    // Verify lien was released
    let lien_info = client.get_vesting_lien(&creator, &circle_id).unwrap();
    assert_eq!(lien_info.status, LienStatus::Released);
    assert!(lien_info.release_timestamp.is_some());
}

#[test]
fn test_get_circle_liens() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let vault_contract = Address::generate(&env);
    
    // Setup contracts
    let token_contract = env.register_contract(None, MockToken);
    let nft_contract = env.register_contract(None, MockNft);
    let vesting_vault_contract = env.register_contract(None, MockVestingVault);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin);
    
    // Create a circle
    let circle_id = client.create_circle(
        &creator,
        &1_000_000_0,
        &3u32,
        &token_contract,
        &86400u64,
        &100u32,
        &nft_contract,
        &admin,
    );
    
    // Create multiple liens
    client.create_vesting_lien(&member1, &circle_id, &vesting_vault_contract, &200_000_0);
    client.create_vesting_lien(&member2, &circle_id, &vesting_vault_contract, &200_000_0);
    
    // Get all circle liens
    let circle_liens = client.get_circle_liens(&circle_id);
    assert_eq!(circle_liens.len(), 2);
    
    // Verify lien details
    let lien1 = circle_liens.iter().find(|l| l.member == member1).unwrap();
    let lien2 = circle_liens.iter().find(|l| l.member == member2).unwrap();
    
    assert_eq!(lien1.lien_id, 1);
    assert_eq!(lien2.lien_id, 2);
    assert_eq!(lien1.status, LienStatus::Active);
    assert_eq!(lien2.status, LienStatus::Active);
}

#[test]
fn test_verify_vesting_vault() {
    let env = Env::default();
    let admin = Address::generate(&env);
    
    let contract_id = env.register_contract(None, sorosusu::SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    // Test vault verification (mock implementation returns true)
    let vault_contract = Address::generate(&env);
    let is_valid = client.verify_vesting_vault(&vault_contract);
    assert!(is_valid);
}

// Mock implementations for dependencies
#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn initialize(env: Env, admin: Address) {
        // Mock
    }
    
    pub fn allowance(env: Env, from: Address, spender: Address) -> i128 {
        1_000_000_0_000_000_0 // Large allowance
    }
    
    pub fn approve(env: Env, from: Address, spender: Address, amount: i128) {
        // Mock approval
    }
    
    pub fn balance(env: Env, account: Address) -> i128 {
        1_000_000_0_000_000_0 // Large balance
    }
    
    pub fn transfer(env: Env, from: Address, to: Address, amount: i128) {
        // Mock transfer
    }
    
    pub fn transfer_from(env: Env, spender: Address, from: Address, to: Address, amount: i128) {
        // Mock transfer_from
    }
}

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn initialize(env: Env, admin: Address) {
        // Mock
    }
    
    pub fn mint(env: Env, to: Address, token_id: u128) {
        // Mock mint
    }
    
    pub fn burn(env: Env, from: Address, token_id: u128) {
        // Mock burn
    }
    
    pub fn owner_of(env: Env, token_id: u128) -> Address {
        Address::generate(&env) // Mock owner
    }
}
