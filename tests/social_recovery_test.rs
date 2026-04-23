#![cfg_attr(not(test), no_std)]
use soroban_sdk::{testutils::Address as _, Address, Env};
use sorosusu_contracts::{SoroSusu, SoroSusuClient};

#[test]
fn test_social_recovery_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let old_member = Address::generate(&env);
    let new_member = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &0);
    let circle_id = client.create_circle(&creator, &1_000, &6, &token, &604_800, &0);

    client.join_circle(&member1, &circle_id);
    client.join_circle(&member2, &circle_id);
    client.join_circle(&old_member, &circle_id);

    client.propose_address_change(&member1, &circle_id, &old_member, &new_member.clone());
    client.vote_for_recovery(&member2, &circle_id);

    let circle = client.get_circle(&circle_id);
    assert_eq!(circle.recovery_new_address, Some(new_member));
}
