#![no_std]

use soroban_sdk::{
    contract, contractimpl, contracterror, contracttype, panic_with_error, Address, Env, Vec,
};

const MAX_MEMBERS: u32 = 50;

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Circle(u32),
    CircleCount,
}

#[derive(Clone)]
#[contracttype]
pub struct Circle {
    admin: Address,
    contribution: i128,
    members: Vec<Address>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[contracterror]
pub enum Error {
    CycleNotComplete = 1001,
    InsufficientAllowance = 1002,
    AlreadyJoined = 1003,
    CircleNotFound = 1004,
    Unauthorized = 1005,
    MaxMembersReached = 1006,
}

#[contract]
pub struct SoroSusu;

fn read_circle(env: &Env, id: u32) -> Circle {
    let key = DataKey::Circle(id);
    let storage = env.storage().instance();
    match storage.get(&key) {
        Some(circle) => circle,
        None => panic_with_error!(env, Error::CircleNotFound),
    }
}

fn write_circle(env: &Env, id: u32, circle: &Circle) {
    let key = DataKey::Circle(id);
    let storage = env.storage().instance();
    storage.set(&key, circle);
}

fn next_circle_id(env: &Env) -> u32 {
    let key = DataKey::CircleCount;
    let storage = env.storage().instance();
    let current: u32 = storage.get(&key).unwrap_or(0);
    let next = current.saturating_add(1);
    storage.set(&key, &next);
    next
}

#[contractimpl]
impl SoroSusu {
    pub fn create_circle(env: Env, contribution: i128) -> u32 {
        let admin = env.invoker();
        let id = next_circle_id(&env);
        let members = Vec::new(&env);
        let circle = Circle {
            admin,
            contribution,
            members,
        };
        write_circle(&env, id, &circle);
        id
    }

    pub fn join_circle(env: Env, circle_id: u32) {
        let invoker = env.invoker();
        let mut circle = read_circle(&env, circle_id);
        for member in circle.members.iter() {
            if member == invoker {
                panic_with_error!(&env, Error::AlreadyJoined);
            }
        }
        let member_count: u32 = circle.members.len();
        if member_count >= MAX_MEMBERS {
            panic_with_error!(&env, Error::MaxMembersReached);
        }
        circle.members.push_back(invoker);
        write_circle(&env, circle_id, &circle);
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn join_circle_enforces_max_members() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;
        let circle_id = client.create_circle(&contribution);

        for _ in 0..MAX_MEMBERS {
            let member = Address::random(&env);
            env.set_source_account(&member);
            client.join_circle(&circle_id);
        }

        let extra_member = Address::random(&env);
        env.set_source_account(&extra_member);
        let result = std::panic::catch_unwind(|| {
            client.join_circle(&circle_id);
        });
        assert!(result.is_err());
    }
}
