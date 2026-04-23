#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{contract, contractimpl, Address, Env, token, Symbol};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[contract]
pub struct MockSanctionsOracle;

#[contractimpl]
impl MockSanctionsOracle {
    pub fn is_sanctioned(env: Env, address: Address) -> bool {
        env.storage().instance().has(&address)
    }
    
    pub fn set_sanctioned(env: Env, address: Address, status: bool) {
        if status {
            env.storage().instance().set(&address, &true);
        } else {
            env.storage().instance().remove(&address);
        }
    }
}

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn test_aml_sanctions_payout_gating() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let winner = Address::generate(&env);
    let other_member = Address::generate(&env);
    let _nft_contract = env.register_contract(None, MockNft);
    let oracle_id = env.register_contract(None, MockSanctionsOracle);
    let oracle_client = MockSanctionsOracleClient::new(&env, &oracle_id);

    // Register a token contract
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract(token_admin.clone());
    let token_admin_client = token::StellarAssetClient::new(&env, &token);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize
    client.init(&admin, &0);
    client.set_sanctions_oracle(&admin, &oracle_id);
    
    // Create circle
    let amount = 1000i128;
    let max_members = 3u32;
    let circle_id = client.create_circle(&creator, &amount, &max_members, &token, &86400u64, &0i128);
    
    // Mint tokens
    token_admin_client.mint(&winner, &10000);
    token_admin_client.mint(&other_member, &10000);
    
    // Join members
    client.join_circle(&winner, &circle_id);
    client.join_circle(&other_member, &circle_id);
    
    // Deposit
    client.deposit(&winner, &circle_id, &1);
    client.deposit(&other_member, &circle_id, &1);
    
    // Finalize round
    client.finalize_round(&creator, &circle_id);
    
    // CASE 1: Winner is sanctioned
    oracle_client.set_sanctioned(&winner, &true);
    
    // Claim pot should freeze the payout
    client.claim_pot(&winner, &circle_id);
    
    // Verify payout is frozen
    let (frozen_amount, frozen_winner) = client.get_frozen_payout(&circle_id);
    assert_eq!(frozen_amount, 3000); // 1000 * 3 (creator + winner + other_member)
    assert_eq!(frozen_winner, Some(winner));
}
