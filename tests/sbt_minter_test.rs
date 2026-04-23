#![cfg(test)]
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{Address, Env, String};
use sorosusu_contracts::sbt_minter::{
    SoroSusuSbtMinter, SoroSusuSbtMinterClient, SbtStatus, ReputationTier
};

#[test]
fn test_sbt_minter_flow() {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let user = Address::generate(&env);
    
    let minter_id = env.register_contract(None, SoroSusuSbtMinter);
    let client = SoroSusuSbtMinterClient::new(&env, &minter_id);
    
    client.init_sbt_minter(&admin);
    
    let desc = String::from_str(&env, "Complete 5 cycles");
    let mid = client.create_reputation_milestone(&user, &5u32, &desc, &ReputationTier::Silver);
    
    let m = client.get_reputation_milestone(&mid);
    assert_eq!(m.required_cycles, 5);

    let tid = client.issue_credential(&user, &mid, &String::from_str(&env, "uri"));
    let cred = client.get_credential(&tid);
    assert_eq!(cred.user, user);
    assert_eq!(cred.status, SbtStatus::Pathfinder);
}
