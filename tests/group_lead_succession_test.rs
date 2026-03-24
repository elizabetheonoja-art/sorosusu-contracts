#![cfg(test)]
use std::panic::{catch_unwind, AssertUnwindSafe};

use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, Env};
use sorosusu_contracts::{MemberStatus, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn group_lead_succession_handoffs_permissions_at_two_thirds_majority() {
    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let nominee = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);

    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);
    client.init(&admin);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract(token_admin);
    let nft_id = env.register_contract(None, MockNft);

    let circle_id = client.create_circle(
        &creator, &1000i128, &3u32, &token_id, &86400u64, &100u32, &nft_id,
    );
    client.join_circle(&member1, &circle_id, &1u32, &None);
    client.join_circle(&member2, &circle_id, &1u32, &None);
    client.join_circle(&member3, &circle_id, &1u32, &None);

    let before_handoff = client.get_circle(&circle_id);
    assert_eq!(before_handoff.current_lead, creator);
    assert_eq!(before_handoff.organizer_fee_recipient, creator);

    client.propose_group_lead_succession(&member1, &circle_id, &nominee);

    let proposal = client
        .get_succession_proposal(&circle_id)
        .expect("proposal should exist before threshold is met");
    assert_eq!(proposal.nominee, nominee);

    let after_first_vote = client.get_circle(&circle_id);
    assert_eq!(after_first_vote.current_lead, creator);
    assert_eq!(after_first_vote.organizer_fee_recipient, creator);

    client.approve_group_lead_succession(&member2, &circle_id);

    let after_handoff = client.get_circle(&circle_id);
    assert_eq!(after_handoff.current_lead, nominee);
    assert_eq!(after_handoff.organizer_fee_recipient, nominee);
    assert!(client.get_succession_proposal(&circle_id).is_none());

    let old_lead_attempt = catch_unwind(AssertUnwindSafe(|| {
        client.eject_member(&creator, &circle_id, &member3);
    }));
    assert!(old_lead_attempt.is_err());

    client.eject_member(&nominee, &circle_id, &member3);
    let updated_member = client.get_member(&circle_id, &member3);
    assert_eq!(updated_member.status, MemberStatus::Ejected);
}
