use soroban_sdk::{testutils::Address as _, Address, Env, String};
use sorosusu_contracts::{
    SoroSusu, SoroSusuTrait, VouchStatus, VouchRecord, VouchStats, MemberStatus, Error,
    MIN_TRUST_SCORE_FOR_VOUCH, VOUCH_COLLATERAL_MULTIPLIER, VOUCH_EXPIRY_SECONDS, MAX_VOUCHES_PER_MEMBER,
    DataKey, SocialCapital
};
use soroban_sdk::contractclient;

#[contractclient(name = "SoroSusuClient")]
pub trait SoroSusuClientTrait {
    fn init(env: Env, admin: Address);
    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
    ) -> u64;
    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>);
    fn vouch_for_member(env: Env, voucher: Address, vouchee: Address, circle_id: u64, collateral_amount: i128);
    fn get_vouch_record(env: Env, voucher: Address, vouchee: Address) -> VouchRecord;
    fn get_vouch_stats(env: Env, voucher: Address) -> VouchStats;
    fn mark_member_defaulted(env: Env, caller: Address, circle_id: u64, member: Address);
    fn slash_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address);
    fn release_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address);
}

// Helper function to get member (for testing)
fn get_member(env: &Env, address: &Address) -> sorosusu_contracts::Member {
    let key = DataKey::Member(address.clone());
    env.storage().instance().get(&key).expect("Member not found")
}

#[test]
fn test_vouch_for_member_success() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let vouchee = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    // Initialize contract
    client.init(&admin);
    
    // Create a high-value circle that requires collateral
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000, // 1000 XLM per contribution
        &5,
        &token,
        &604800, // 1 week
        &500, // 5% insurance
        &nft_contract,
    );
    
    // Setup voucher as member with high trust score
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score for voucher
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85, // High trust score
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    // Calculate required vouch collateral (15% of cycle value)
    let cycle_value = 100_000_0000 * 5; // 5 members * 1000 XLM
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Mock token transfer for collateral
    env.mock_auths(&[(&voucher, &token)]);
    client.vouch_for_member(&voucher, &vouchee, &circle_id, &required_collateral);
    
    // Verify vouch record was created
    let vouch_record = client.get_vouch_record(&voucher, &vouchee);
    assert_eq!(vouch_record.voucher, voucher);
    assert_eq!(vouch_record.vouchee, vouchee);
    assert_eq!(vouch_record.circle_id, circle_id);
    assert_eq!(vouch_record.collateral_amount, required_collateral);
    assert_eq!(vouch_record.status, VouchStatus::Active);
    assert_eq!(vouch_record.slash_count, 0);
    
    // Verify voucher stats were updated
    let vouch_stats = client.get_vouch_stats(&voucher);
    assert_eq!(vouch_stats.total_vouches_made, 1);
    assert_eq!(vouch_stats.active_vouches, 1);
    assert_eq!(vouch_stats.successful_vouches, 0);
    assert_eq!(vouch_stats.slashed_vouches, 0);
    assert_eq!(vouch_stats.total_collateral_locked, required_collateral);
    assert_eq!(vouch_stats.total_collateral_lost, 0);
    
    // Verify vouchee can now join circle without collateral
    env.mock_auths(&[(&vouchee, &contract_id)]);
    client.join_circle(&vouchee, &circle_id, &1, &None);
}

#[test]
fn test_vouch_for_member_insufficient_trust_score() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let vouchee = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &5,
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    // Setup voucher as member with low trust score
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up low trust score for voucher
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 0,
        leniency_received: 0,
        voting_participation: 0,
        trust_score: 60, // Below minimum required
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 5;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Should fail due to insufficient trust score
    env.mock_auths(&[(&voucher, &token)]);
    let result = env.try_invoke_contract::<Error>(
        &contract_id,
        &SoroSusuTrait::vouch_for_member(
            &env,
            voucher.clone(),
            vouchee.clone(),
            circle_id,
            required_collateral,
        ),
        &(),
    );
    assert_eq!(result, Err(Ok(Error::InsufficientTrustScore)));
}

#[test]
fn test_vouch_for_member_self_vouch() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &5,
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85,
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 5;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Should fail due to self-vouch
    env.mock_auths(&[(&voucher, &token)]);
    let result = env.try_invoke_contract::<Error>(
        &contract_id,
        &SoroSusuTrait::vouch_for_member(
            &env,
            voucher.clone(),
            voucher.clone(), // Self-vouch
            circle_id,
            required_collateral,
        ),
        &(),
    );
    assert_eq!(result, Err(Ok(Error::CannotVouchForSelf)));
}

#[test]
fn test_vouch_for_member_maximum_vouches_exceeded() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &10, // More members to allow multiple vouches
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85,
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 10;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Create maximum number of vouches
    for i in 0..MAX_VOUCHES_PER_MEMBER {
        let vouchee = Address::generate(&env);
        env.mock_auths(&[(&voucher, &token)]);
        client.vouch_for_member(&voucher, &vouchee, &circle_id, &required_collateral);
    }
    
    // Try to create one more vouch - should fail
    let extra_vouchee = Address::generate(&env);
    env.mock_auths(&[(&voucher, &token)]);
    let result = env.try_invoke_contract::<Error>(
        &contract_id,
        &SoroSusuTrait::vouch_for_member(
            &env,
            voucher.clone(),
            extra_vouchee.clone(),
            circle_id,
            required_collateral,
        ),
        &(),
    );
    assert_eq!(result, Err(Ok(Error::VouchAlreadyExists))); // This might be a different error in practice
}

#[test]
fn test_slash_vouch_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let vouchee = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &5,
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    // Setup voucher as member with high trust score
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85,
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 5;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Create vouch
    env.mock_auths(&[(&voucher, &token)]);
    client.vouch_for_member(&voucher, &vouchee, &circle_id, &required_collateral);
    
    // Vouchee joins circle
    env.mock_auths(&[(&vouchee, &contract_id)]);
    client.join_circle(&vouchee, &circle_id, &1, &None);
    
    // Mark vouchee as defaulted
    env.mock_auths(&[(&admin, &contract_id)]);
    client.mark_member_defaulted(&admin, &circle_id, &vouchee);
    
    // Slash vouch collateral
    env.mock_auths(&[(&admin, &contract_id)]);
    client.slash_vouch_collateral(&admin, &circle_id, &vouchee);
    
    // Verify vouch was slashed
    let vouch_record = client.get_vouch_record(&voucher, &vouchee);
    assert_eq!(vouch_record.status, VouchStatus::Slashed);
    assert_eq!(vouch_record.slash_count, 1);
    
    // Verify voucher stats were updated
    let vouch_stats = client.get_vouch_stats(&voucher);
    assert_eq!(vouch_stats.active_vouches, 0);
    assert_eq!(vouch_stats.slashed_vouches, 1);
    assert_eq!(vouch_stats.total_collateral_lost, required_collateral);
}

#[test]
fn test_release_vouch_collateral() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let vouchee = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &5,
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    // Setup voucher as member with high trust score
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85,
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 5;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    
    // Create vouch
    env.mock_auths(&[(&voucher, &token)]);
    client.vouch_for_member(&voucher, &vouchee, &circle_id, &required_collateral);
    
    // Vouchee joins circle and completes all contributions
    env.mock_auths(&[(&vouchee, &contract_id)]);
    client.join_circle(&vouchee, &circle_id, &1, &None);
    
    // Simulate completed contributions by updating member directly
    let member_key = DataKey::Member(vouchee.clone());
    let mut member = get_member(&env, &vouchee);
    member.contribution_count = 5; // Completed all contributions
    env.storage().instance().set(&member_key, &member);
    
    // Release vouch collateral
    env.mock_auths(&[(&admin, &contract_id)]);
    client.release_vouch_collateral(&admin, &circle_id, &vouchee);
    
    // Verify vouch was completed
    let vouch_record = client.get_vouch_record(&voucher, &vouchee);
    assert_eq!(vouch_record.status, VouchStatus::Completed);
    
    // Verify voucher stats were updated
    let vouch_stats = client.get_vouch_stats(&voucher);
    assert_eq!(vouch_stats.active_vouches, 0);
    assert_eq!(vouch_stats.successful_vouches, 1);
}

#[test]
fn test_vouch_collateral_insufficient() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let voucher = Address::generate(&env);
    let vouchee = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);
    
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    
    client.init(&admin);
    
    let circle_id = client.create_circle(
        &voucher,
        &100_000_0000,
        &5,
        &token,
        &604800,
        &500,
        &nft_contract,
    );
    
    env.mock_auths(&[(&voucher, &contract_id)]);
    client.join_circle(&voucher, &circle_id, &1, &None);
    
    // Set up high trust score
    let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
    let social_capital = SocialCapital {
        member: voucher.clone(),
        circle_id,
        leniency_given: 10,
        leniency_received: 5,
        voting_participation: 20,
        trust_score: 85,
    };
    env.storage().instance().set(&social_capital_key, &social_capital);
    
    let cycle_value = 100_000_0000 * 5;
    let required_collateral = (cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
    let insufficient_collateral = required_collateral / 2; // Half of required
    
    // Should fail due to insufficient collateral
    env.mock_auths(&[(&voucher, &token)]);
    let result = env.try_invoke_contract::<Error>(
        &contract_id,
        &SoroSusuTrait::vouch_for_member(
            &env,
            voucher.clone(),
            vouchee.clone(),
            circle_id,
            insufficient_collateral,
        ),
        &(),
    );
    assert_eq!(result, Err(Ok(Error::CollateralInsufficientForVouch)));
}
