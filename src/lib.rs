#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, Address, Env, Vec,
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
    is_random_queue: bool,
    payout_queue: Vec<Address>,
    cycle_number: u32,
    has_received_payout: Vec<bool>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
pub enum Error {
    CycleNotComplete = 1001,
    InsufficientAllowance = 1002,
    AlreadyJoined = 1003,
    CircleNotFound = 1004,
    Unauthorized = 1005,
    MaxMembersReached = 1006,
    CircleNotFinalized = 1007,
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
    pub fn create_circle(env: Env, contribution: i128, is_random_queue: bool) -> u32 {
        let admin = env.invoker();
        let id = next_circle_id(&env);
        let members = Vec::new(&env);
        let payout_queue = Vec::new(&env);
        let has_received_payout = Vec::new(&env);
        let circle = Circle {
            admin,
            contribution,
            members,
            is_random_queue,
            payout_queue,
            cycle_number: 1,
            has_received_payout,
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
        circle.has_received_payout.push_back(false);
        write_circle(&env, circle_id, &circle);
    }

    pub fn finalize_circle(env: Env, circle_id: u32) {
        let mut circle = read_circle(&env, circle_id);

        // Only admin can finalize the circle
        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Check if payout_queue is already finalized
        if !circle.payout_queue.is_empty() {
            return; // Already finalized
        }

        if circle.is_random_queue {
            // Use Soroban's PRNG to shuffle the members
            let mut shuffled_members = circle.members.clone();
            env.prng().shuffle(&mut shuffled_members);
            circle.payout_queue = shuffled_members;
        } else {
            // Use the order members joined
            circle.payout_queue = circle.members.clone();
        }

        write_circle(&env, circle_id, &circle);
    }

    pub fn rollover_group(env: Env, circle_id: u32) {
        let mut circle = read_circle(&env, circle_id);

        // Only admin can rollover the group
        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Check if payout_queue is finalized
        if circle.payout_queue.is_empty() {
            panic_with_error!(&env, Error::CircleNotFinalized);
        }

        // Check if all members have received payout for current cycle
        for received in circle.has_received_payout.iter() {
            if !received {
                panic_with_error!(&env, Error::CycleNotComplete);
            }
        }

        // Reset for next cycle
        circle.cycle_number += 1;

        // Reset payout flags
        for i in 0..circle.has_received_payout.len() {
            circle.has_received_payout.set(i, false);
        }

        // Reshuffle payout queue if random queue is enabled
        if circle.is_random_queue {
            let mut shuffled_members = circle.members.clone();
            env.prng().shuffle(&mut shuffled_members);
            circle.payout_queue = shuffled_members;
        }

        write_circle(&env, circle_id, &circle);
    }

    pub fn get_payout_queue(env: Env, circle_id: u32) -> Vec<Address> {
        let circle = read_circle(&env, circle_id);
        circle.payout_queue
    }

    pub fn get_cycle_number(env: Env, circle_id: u32) -> u32 {
        let circle = read_circle(&env, circle_id);
        circle.cycle_number
    }

    pub fn get_payout_status(env: Env, circle_id: u32) -> Vec<bool> {
        let circle = read_circle(&env, circle_id);
        circle.has_received_payout
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{Address as _, Env as _};

    #[test]
    fn join_circle_enforces_max_members() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;
        let circle_id = client.create_circle(&contribution, &false);

        for _ in 0..MAX_MEMBERS {
            let member = Address::generate(&env);
            client.join_circle(&circle_id);
        }

        let extra_member = Address::generate(&env);
        let result = std::panic::catch_unwind(|| {
            client.join_circle(&circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_random_queue_finalization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        // Create circle with random queue enabled
        let circle_id = client.create_circle(&contribution, &true);

        // Add some members
        let members: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(&circle_id);
        }

        // Finalize the circle (admin is the creator)
        client.finalize_circle(&circle_id);

        // Get the payout queue
        let payout_queue = client.get_payout_queue(&circle_id);

        // Verify that all members are in the queue
        assert_eq!(payout_queue.len(), 5);

        // Verify that the queue contains all members (order may be different due to shuffle)
        for member in &members {
            assert!(payout_queue.contains(member));
        }
    }

    #[test]
    fn test_sequential_queue_finalization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        // Create circle with random queue disabled
        let circle_id = client.create_circle(&contribution, &false);

        // Add some members in a specific order
        let members: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(&circle_id);
        }

        // Finalize the circle (admin is the creator)
        client.finalize_circle(&circle_id);

        // Get the payout queue
        let payout_queue = client.get_payout_queue(&circle_id);

        // Verify that the queue preserves the join order
        assert_eq!(payout_queue.len(), 5);
        for (i, member) in members.iter().enumerate() {
            assert_eq!(payout_queue.get(i as u32), Some(member));
        }
    }

    #[test]
    fn test_rollover_group() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        // Create circle with random queue enabled
        let circle_id = client.create_circle(&contribution, &true);

        // Add some members
        let members: Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(&circle_id);
        }

        // Finalize the circle
        client.finalize_circle(&circle_id);

        // Verify initial cycle number
        assert_eq!(client.get_cycle_number(&circle_id), 1);

        // Simulate all members receiving payout (manually set for test)
        // Note: In a real implementation, this would be done through payout functions

        // For testing purposes, we'll skip the payout check and test rollover directly
        // In a real scenario, all has_received_payout flags would be true

        // Test rollover (this will fail in test due to cycle check, but shows the structure)
        let result = std::panic::catch_unwind(|| {
            client.rollover_group(&circle_id);
        });
        // Should fail because not all members have received payout
        assert!(result.is_err());
    }

    #[test]
    fn test_finalize_circle_unauthorized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution, &true);

        // Try to finalize with non-admin
        let unauthorized_user = Address::generate(&env);
        env.set_source_account(&unauthorized_user);

        let result = std::panic::catch_unwind(|| {
            client.finalize_circle(&circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rollover_group_unauthorized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution, &true);

        // Try to rollover with non-admin
        let unauthorized_user = Address::generate(&env);
        env.set_source_account(&unauthorized_user);

        let result = std::panic::catch_unwind(|| {
            client.rollover_group(&circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rollover_group_not_finalized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution, &true);

        // Try to rollover without finalizing
        let result = std::panic::catch_unwind(|| {
            client.rollover_group(&circle_id);
        });
        assert!(result.is_err());
    }
}
