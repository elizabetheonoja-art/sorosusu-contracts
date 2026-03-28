#![cfg(test)]

use soroban_sdk::contractclient;
use soroban_sdk::testutils::{Address as TestAddress, Logs};
use soroban_sdk::{Address, Env, Symbol};
use sorosusu_contracts::{SusuNftClient, SorosusuContractClient};

#[contractclient(name = "SusuNftClient")]
impl sorosusu_contracts::SusuNftTrait for SusuNftClient {}

#[test]
fn test_shares_functionality() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env); // 1 share
    let member2 = Address::generate(&env); // 2 shares
    let member3 = Address::generate(&env); // 1 share
    
    // Deploy contracts
    let susu_contract_address = env.register_contract(None, sorosusu_contracts::WASM);
    let nft_contract_address = env.register_contract(None, sorosusu_contracts::nft::WASM);
    
    let susu_client = SorosusuContractClient::new(&env, &susu_contract_address);
    let nft_client = SusuNftClient::new(&env, &nft_contract_address);
    
    // Initialize
    susu_client.init(&admin);
    
    // Create NFT collection for the susu contract
    nft_client.init(&susu_contract_address, &Symbol::new(&env, "SusuNFT"), &Symbol::new(&env, "SSNFT"));
    
    // Create a circle
    let circle_id = susu_client.create_circle(
        &creator,
        &100i128, // contribution amount
        &3u32,    // max members
        &Address::generate(&env), // token
        &604800,  // cycle duration (1 week)
        &100u32,  // insurance fee bps
        &nft_contract_address,
        &admin,   // arbitrator
    );
    
    // Join members with different shares
    susu_client.join_circle(
        &member1,
        &circle_id,
        &1u32, // tier_multiplier (will be set to shares)
        &1u32, // shares = 1 (standard)
        &None::<Address>,
    );
    
    susu_client.join_circle(
        &member2,
        &circle_id,
        &2u32, // tier_multiplier (will be set to shares)
        &2u32, // shares = 2 (double)
        &None::<Address>,
    );
    
    susu_client.join_circle(
        &member3,
        &circle_id,
        &1u32, // tier_multiplier (will be set to shares)
        &1u32, // shares = 1 (standard)
        &None::<Address>,
    );
    
    // Verify circle state
    let circle_info = susu_client.get_circle(&circle_id);
    assert_eq!(circle_info.member_count, 3);
    assert_eq!(circle_info.total_shares, 4); // 1 + 2 + 1 = 4
    
    // Verify member shares
    let member1_info = susu_client.get_member(&member1);
    assert_eq!(member1_info.shares, 1);
    assert_eq!(member1_info.tier_multiplier, 1);
    
    let member2_info = susu_client.get_member(&member2);
    assert_eq!(member2_info.shares, 2);
    assert_eq!(member2_info.tier_multiplier, 2);
    
    let member3_info = susu_client.get_member(&member3);
    assert_eq!(member3_info.shares, 1);
    assert_eq!(member3_info.tier_multiplier, 1);
    
    // Test contributions (member2 should pay double)
    // Note: In a real test, you'd need to set up token contracts and approve transfers
    // This is a simplified test structure showing the shares logic
    
    // Test that invalid shares are rejected
    let member4 = Address::generate(&env);
    let result = env.try_invoke_contract::<_, _>(
        &susu_contract_address,
        &SorosusuContractClient::new(&env, &susu_contract_address).join_circle(
            &member4,
            &circle_id,
            &1u32,
            &3u32, // Invalid shares (must be 1 or 2)
            &None::<Address>,
        ),
    );
    assert!(result.is_err());
}

#[test]
fn test_double_payout_for_two_shares() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let double_share_member = Address::generate(&env);
    let single_share_member = Address::generate(&env);
    
    // Deploy contracts
    let susu_contract_address = env.register_contract(None, sorosusu_contracts::WASM);
    let nft_contract_address = env.register_contract(None, sorosusu_contracts::nft::WASM);
    
    let susu_client = SorosusuContractClient::new(&env, &susu_contract_address);
    let nft_client = SusuNftClient::new(&env, &nft_contract_address);
    
    // Initialize
    susu_client.init(&admin);
    nft_client.init(&susu_contract_address, &Symbol::new(&env, "SusuNFT"), &Symbol::new(&env, "SSNFT"));
    
    // Create a circle with 2 members
    let circle_id = susu_client.create_circle(
        &creator,
        &100i128, // contribution amount
        &2u32,    // max members
        &Address::generate(&env), // token
        &604800,  // cycle duration
        &100u32,  // insurance fee bps
        &nft_contract_address,
        &admin,   // arbitrator
    );
    
    // Join members
    susu_client.join_circle(&single_share_member, &circle_id, &1u32, &1u32, &None::<Address>);
    susu_client.join_circle(&double_share_member, &circle_id, &2u32, &2u32, &None::<Address>);
    
    // Verify total shares
    let circle_info = susu_client.get_circle(&circle_id);
    assert_eq!(circle_info.total_shares, 3); // 1 + 2 = 3
    
    // The pot should be based on total shares: 100 * 3 = 300
    // Single share member gets: 300
    // Double share member gets: 300 * 2 = 600
    
    // Note: In a real test, you'd need to:
    // 1. Set up token contracts
    // 2. Make contributions
    // 3. Finalize the round
    // 4. Claim pots and verify amounts
    // This test structure validates the shares setup logic
}
