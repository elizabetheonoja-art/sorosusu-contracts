#![cfg(test)]

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[allow(deprecated)]
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
    client.init(&admin);
    
    // Deploy mock token
    let token_admin = Address::generate(&env);

    let token_client = token::StellarAssetClient::new(&env, &token_id);
    let token_token_client = token::Client::new(&env, &token_id);
    
    // Deploy mock NFT
    let nft_id = env.register_contract(None, MockNft);
    
    // Mint tokens to users
    token_client.mint(&user1, &10000);
    token_client.mint(&user2, &10000);
    
    // Create circle
    let contribution_amount: i128 = 1000;
    let cycle_duration: u64 = 86400; // 1 day
    let arbitrator = Address::generate(&env);
    let circle_id = client.create_circle(
        &creator,
        &contribution_amount,
        &2, // max_members
        &token_id,
        &cycle_duration,
        &100, // 1% insurance fee
        &nft_id,
        &arbitrator,
    );
    
    // Join circle
    client.join_circle(&user1, &circle_id, &1, &None);
    client.join_circle(&user2, &circle_id, &1, &None);
    
    // Deposits
    client.deposit(&user1, &circle_id);
    client.deposit(&user2, &circle_id);
    
    // Check balances
    assert_eq!(token_token_client.balance(&user1), 10000 - 1000 - 10); // 1000 + 1% fee
    assert_eq!(token_token_client.balance(&user2), 10000 - 1000 - 10);
    
    client.finalize_round(&creator, &circle_id);

    env.as_contract(&contract_id, || {
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap();
        let scheduled_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ScheduledPayoutTime(circle_id))
            .unwrap();
        assert!(circle.is_round_finalized);
        assert_eq!(circle.current_pot_recipient, Some(user1.clone()));
        assert_eq!(scheduled_time, env.ledger().timestamp() + 86400);
    });

    let audit_entries = client.query_audit_by_resource(&circle_id, &0, &u64::MAX, &0, &10);
    assert!(audit_entries.len() >= 1);

    let finalize_audit = audit_entries.get(audit_entries.len() - 1).unwrap();
    assert_eq!(finalize_audit.actor, creator);
    assert_eq!(finalize_audit.action, AuditAction::AdminAction);
    assert_eq!(finalize_audit.resource_id, circle_id);
}
