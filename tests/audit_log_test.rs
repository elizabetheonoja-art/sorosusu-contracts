#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as _,
    Address, Env,
};
use sorosusu_contracts::{AuditAction, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn audit_log_writes_and_queries_by_actor_resource_and_time() {
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
    let nft_contract = env.register_contract(None, MockNft);

    client.init(&admin);

    let circle_id = client.create_circle(&creator, &1_000, &6, &token, &604_800, &0, &nft_contract);

    client.join_circle(&member1, &circle_id, &1, &None);
    client.join_circle(&member2, &circle_id, &1, &None);
    client.join_circle(&old_member, &circle_id, &1, &None);

    let start_ts = env.ledger().timestamp();

    client.propose_address_change(&member1, &circle_id, &old_member, &new_member);
    client.vote_for_recovery(&member2, &circle_id);

    let end_ts = env.ledger().timestamp();

    let actor_entries = client.query_audit_by_actor(&member1, &start_ts, &end_ts, &0, &20);
    assert!(actor_entries.len() > 0);

    let first_actor_entry = actor_entries.get(0).unwrap();
    assert_eq!(first_actor_entry.actor, member1.clone());
    assert_eq!(first_actor_entry.resource_id, circle_id);
    assert_eq!(first_actor_entry.action, AuditAction::DisputeSubmission);

    let resource_entries = client.query_audit_by_resource(&circle_id, &start_ts, &end_ts, &0, &50);
    assert!(resource_entries.len() >= 2);

    for i in 0..resource_entries.len() {
        let e = resource_entries.get(i).unwrap();
        assert_eq!(e.resource_id, circle_id);
        assert!(e.timestamp >= start_ts);
        assert!(e.timestamp <= end_ts);
    }

    let time_entries = client.query_audit_by_time(&start_ts, &end_ts, &0, &100);
    assert!(time_entries.len() >= resource_entries.len());

    let paged = client.query_audit_by_resource(&circle_id, &start_ts, &end_ts, &1, &1);
    assert!(paged.len() <= 1);
}
