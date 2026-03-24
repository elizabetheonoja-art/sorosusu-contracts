#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl,
    testutils::Address as TestAddress,
    Address, Env,
};
use sorosusu_contracts::{CircleInfo, DataKey, Member, SoroSusu, SoroSusuClient};

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn mint(_env: Env, _to: Address, _id: u128) {}
    pub fn burn(_env: Env, _from: Address, _id: u128) {}
}

#[test]
fn social_recovery_requires_more_than_seventy_percent_votes() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let member1 = Address::generate(&env);
    let member2 = Address::generate(&env);
    let member3 = Address::generate(&env);
    let old_member = Address::generate(&env);
    let new_member = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    env.mock_all_auths();

    client.init(&admin);

    let circle_id = client.create_circle(&creator, &1_000, &6, &token, &604_800, &0, &nft_contract);

    client.join_circle(&member1, &circle_id, &1, &None);
    client.join_circle(&member2, &circle_id, &1, &None);
    client.join_circle(&member3, &circle_id, &1, &None);
    client.join_circle(&old_member, &circle_id, &1, &None);

    // Proposal auto-votes for proposer: 1/4 = 25%
    client.propose_address_change(&member1, &circle_id, &old_member, &new_member);

    // Second vote: 2/4 = 50% (still below >70%)
    client.vote_for_recovery(&member2, &circle_id);

    let old_member_key = DataKey::Member(old_member.clone());
    let new_member_key = DataKey::Member(new_member.clone());
    env.as_contract(&contract_id, || {
        assert!(env.storage().instance().has(&old_member_key));
        assert!(!env.storage().instance().has(&new_member_key));
    });

    // Third vote: 3/4 = 75% (passes >70%)
    client.vote_for_recovery(&member3, &circle_id);

    env.as_contract(&contract_id, || {
        assert!(!env.storage().instance().has(&old_member_key));
        assert!(env.storage().instance().has(&new_member_key));

        let replaced_member: Member = env.storage().instance().get(&new_member_key).unwrap();
        assert_eq!(replaced_member.address, new_member.clone());

        let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let recovered_addr = circle_after
            .member_addresses
            .get(replaced_member.index)
            .unwrap();
        assert_eq!(recovered_addr, new_member);

        assert!(circle_after.recovery_old_address.is_none());
        assert!(circle_after.recovery_new_address.is_none());
        assert_eq!(circle_after.recovery_votes_bitmap, 0);
    });
}

#[test]
#[should_panic(expected = "New address is already a member")]
fn social_recovery_rejects_new_address_if_already_member() {
    let env = Env::default();
    let contract_id = env.register_contract(None, SoroSusu);
    let client = SoroSusuClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let proposer = Address::generate(&env);
    let old_member = Address::generate(&env);
    let existing_member = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = env.register_contract(None, MockNft);

    env.mock_all_auths();

    client.init(&admin);
    let circle_id = client.create_circle(&creator, &1_000, &6, &token, &604_800, &0, &nft_contract);

    client.join_circle(&proposer, &circle_id, &1, &None);
    client.join_circle(&old_member, &circle_id, &1, &None);
    client.join_circle(&existing_member, &circle_id, &1, &None);

    client.propose_address_change(&proposer, &circle_id, &old_member, &existing_member);
}
