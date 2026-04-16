use soroban_sdk::{contract, contractimpl, Address, Env, Vec, Symbol, String, Map};
use sorosusu_contracts::{SoroSusu, SoroSusuClient, GrantSettlement, VotingSnapshot, ImpactCertificateMetadata, MilestoneProgress};
use soroban_sdk::testutils::{Address as _, Ledger as _};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn balance(_env: Env, _account: Address) -> i128 { 100_000_000_000 }
    pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
}

fn setup_test_env() -> (Env, SoroSusuClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let admin = Address::generate(&env);
    let treasury = Address::generate(&env);
    let grantee = Address::generate(&env);
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin, &0);
    (env, client, admin, treasury, grantee)
}

#[test]
fn test_grant_settlement_calculation() {
    let (env, client, admin, treasury, grantee) = setup_test_env();
    let grant_id = 1u64;
    let total_grant = 10_000_000_000i128;
    let grant_duration = 86400u64 * 30;
    let start_timestamp = env.ledger().timestamp();
    env.ledger().set_timestamp(start_timestamp + (86400 * 15));
    let token_address = env.register_contract(None, MockToken);
    let settlement = client.terminate_grant_amicably(&admin, &grant_id, &grantee, &total_grant, &grant_duration, &start_timestamp, &treasury, &token_address);
    assert_eq!(settlement.grant_id, grant_id);
    assert_eq!(settlement.grantee, grantee);
}

#[test]
fn test_voting_snapshot_creation() {
    let (env, client, _admin, _treasury, _grantee) = setup_test_env();
    let proposal_id = 1u64;
    let voter = Address::generate(&env);
    let mut votes = Vec::new(&env);
    votes.push_back((voter, 100u32, Symbol::new(&env, "For")));
    let snapshot = client.create_voting_snapshot_for_audit(&proposal_id, &votes, &50u64);
    assert_eq!(snapshot.proposal_id, proposal_id);
    assert_eq!(snapshot.total_votes, 100);
}

#[test]
fn test_impact_certificate_initialization() {
    let (env, client, _admin, _treasury, grantee) = setup_test_env();
    let certificate_id = 1u128;
    let metadata_uri = String::from_str(&env, "https://metadata.sorosusu.com/impact/1");
    client.initialize_impact_certificate(&grantee, &certificate_id, &5u32, &metadata_uri);
    let progress_data = client.get_progress_bar_data(&certificate_id);
    assert!(progress_data.is_some());
}
