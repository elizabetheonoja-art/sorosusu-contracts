#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error,
    Address, Env, Vec,
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

    // payout tracking
    has_received_payout: Vec<bool>,
    cycle_number: u32,
    current_payout_index: u32,
    total_volume_distributed: i128,

    // governance
    is_dissolved: bool,
    dissolution_votes: Vec<Address>,

    // accounting
    contributions_paid: Vec<i128>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[contracterror]
pub enum Error {
    CircleNotFound = 1001,
    Unauthorized = 1002,
    AlreadyJoined = 1003,
    MaxMembersReached = 1004,
    AlreadyVoted = 1005,
    NotMember = 1006,
    AlreadyDissolved = 1007,
    NotDissolved = 1008,
}

#[contract]
pub struct SoroSusu;

fn read_circle(env: &Env, id: u32) -> Circle {
    match env.storage().instance().get(&DataKey::Circle(id)) {
        Some(c) => c,
        None => panic_with_error!(env, Error::CircleNotFound),
    }
}

fn write_circle(env: &Env, id: u32, circle: &Circle) {
    env.storage().instance().set(&DataKey::Circle(id), circle);
}

fn next_circle_id(env: &Env) -> u32 {
    let key = DataKey::CircleCount;
    let current: u32 = env.storage().instance().get(&key).unwrap_or(0);
    let next = current + 1;
    env.storage().instance().set(&key, &next);
    next
}

#[contractimpl]
impl SoroSusu {

    // ============================================================
    // CREATE
    // ============================================================

    pub fn create_circle(env: Env, contribution: i128, is_random_queue: bool) -> u32 {
        let admin = env.invoker();
        let id = next_circle_id(&env);

        let circle = Circle {
            admin,
            contribution,
            members: Vec::new(&env),
            is_random_queue,
            payout_queue: Vec::new(&env),
            has_received_payout: Vec::new(&env),
            cycle_number: 1,
            current_payout_index: 0,
            total_volume_distributed: 0,
            is_dissolved: false,
            dissolution_votes: Vec::new(&env),
            contributions_paid: Vec::new(&env),
        };

        write_circle(&env, id, &circle);
        id
    }

    // ============================================================
    // JOIN
    // ============================================================

    pub fn join_circle(env: Env, circle_id: u32) {
        let invoker = env.invoker();
        let mut circle = read_circle(&env, circle_id);

        if circle.is_dissolved {
            panic_with_error!(&env, Error::AlreadyDissolved);
        }

        if circle.members.contains(&invoker) {
            panic_with_error!(&env, Error::AlreadyJoined);
        }

        if circle.members.len() >= MAX_MEMBERS {
            panic_with_error!(&env, Error::MaxMembersReached);
        }

        circle.members.push_back(invoker);
        circle.has_received_payout.push_back(false);
        circle.contributions_paid.push_back(circle.contribution);

        write_circle(&env, circle_id, &circle);
    }

    // ============================================================
    // FINALIZE
    // ============================================================

    pub fn finalize_circle(env: Env, circle_id: u32) {
        let mut circle = read_circle(&env, circle_id);

        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        if circle.is_dissolved {
            panic_with_error!(&env, Error::AlreadyDissolved);
        }

        if !circle.payout_queue.is_empty() {
            return;
        }

        if circle.is_random_queue {
            let mut shuffled = circle.members.clone();
            env.prng().shuffle(&mut shuffled);
            circle.payout_queue = shuffled;
        } else {
            circle.payout_queue = circle.members.clone();
        }

        write_circle(&env, circle_id, &circle);
    }

    // ============================================================
    // PROCESS PAYOUT
    // ============================================================

    pub fn process_payout(env: Env, circle_id: u32, recipient: Address) {
        let mut circle = read_circle(&env, circle_id);

        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        if circle.is_dissolved {
            panic_with_error!(&env, Error::AlreadyDissolved);
        }

        let mut index = None;
        for (i, member) in circle.members.iter().enumerate() {
            if member == recipient {
                index = Some(i);
                break;
            }
        }

        let i = index.unwrap_or_else(|| panic_with_error!(&env, Error::NotMember));

        if circle.has_received_payout.get(i).unwrap() {
            panic_with_error!(&env, Error::Unauthorized);
        }

        circle.has_received_payout.set(i, true);
        circle.current_payout_index += 1;
        circle.total_volume_distributed += circle.contribution;

        write_circle(&env, circle_id, &circle);
    }

    // ============================================================
    // GOVERNANCE — DISSOLUTION
    // ============================================================

    pub fn propose_dissolution(env: Env, circle_id: u32) {
        let invoker = env.invoker();
        let mut circle = read_circle(&env, circle_id);

        if circle.is_dissolved {
            panic_with_error!(&env, Error::AlreadyDissolved);
        }

        if !circle.members.contains(&invoker) {
            panic_with_error!(&env, Error::NotMember);
        }

        if !circle.dissolution_votes.contains(&invoker) {
            circle.dissolution_votes.push_back(invoker);
        }

        write_circle(&env, circle_id, &circle);
    }

    pub fn vote_dissolve(env: Env, circle_id: u32) {
        let invoker = env.invoker();
        let mut circle = read_circle(&env, circle_id);

        if circle.is_dissolved {
            panic_with_error!(&env, Error::AlreadyDissolved);
        }

        if !circle.members.contains(&invoker) {
            panic_with_error!(&env, Error::NotMember);
        }

        if circle.dissolution_votes.contains(&invoker) {
            panic_with_error!(&env, Error::AlreadyVoted);
        }

        circle.dissolution_votes.push_back(invoker);

        let total_members = circle.members.len();
        let votes = circle.dissolution_votes.len();

        if votes * 2 > total_members {
            circle.is_dissolved = true;
        }

        write_circle(&env, circle_id, &circle);
    }

    // ============================================================
    // WITHDRAW AFTER DISSOLUTION
    // ============================================================

    pub fn withdraw_pro_rata(env: Env, circle_id: u32) -> i128 {
        let invoker = env.invoker();
        let mut circle = read_circle(&env, circle_id);

        if !circle.is_dissolved {
            panic_with_error!(&env, Error::NotDissolved);
        }

        let mut index = None;
        for (i, member) in circle.members.iter().enumerate() {
            if member == invoker {
                index = Some(i);
                break;
            }
        }

        let i = index.unwrap_or_else(|| panic_with_error!(&env, Error::NotMember));

        let contributed = circle.contributions_paid.get(i).unwrap();
        let received = if circle.has_received_payout.get(i).unwrap() {
            circle.contribution
        } else {
            0
        };

        let refundable = contributed - received;

        if refundable > 0 {
            circle.contributions_paid.set(i, 0);
            write_circle(&env, circle_id, &circle);
        }

        refundable
    }

    // ============================================================
    // VIEW
    // ============================================================

    pub fn get_circle(env: Env, circle_id: u32) -> Circle {
        read_circle(&env, circle_id)
    }
}
