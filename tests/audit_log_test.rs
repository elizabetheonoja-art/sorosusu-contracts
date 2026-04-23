#![cfg_attr(not(test), no_std)]
use soroban_sdk::{testutils::Address as _, Address, Env};
use sorosusu_contracts::{AuditAction, SoroSusu, SoroSusuClient};

#[test]
fn test_audit_log_flow() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let token = Address::generate(&env);

    client.init(&admin, &0);
    let circle_id = client.create_circle(&creator, &1_000, &6, &token, &604_800, &0);
    client.join_circle(&member1, &circle_id);

    let start_ts = env.ledger().timestamp();
    client.pair_with_member(&member1, &Address::generate(&env));
    let end_ts = env.ledger().timestamp();

    let actor_entries = client.query_audit_by_actor(&member1, &start_ts, &end_ts, &0, &20);
    assert!(actor_entries.len() > 0);

    let res_entries = client.query_audit_by_resource(&circle_id, &start_ts, &end_ts, &0, &20);
    assert!(res_entries.len() > 0);
}
