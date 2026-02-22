#![no_std]
#![cfg_attr(test, allow(dead_code))]

use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, contracttype, panic_with_error,
    symbol_short, token, Address, BytesN, Env, Symbol, Vec,
};

const FEE_BASIS_POINTS_KEY: Symbol = symbol_short!("fee_bps");
const TREASURY_KEY: Symbol = symbol_short!("treasury");
const ADMIN_KEY: Symbol = symbol_short!("admin");
const MAX_BASIS_POINTS: u32 = 10_000;
const MAX_MEMBERS: u32 = 50;

contractmeta!(
    key = "Description",
    val = "SoroSusu ROSCA protocol with protocol payout fee"
);

#[derive(Clone)]
#[contracttype]
pub enum DataKey {
    Circle(u32),
    CircleCount,
}

#[derive(Clone)]
#[contracttype]
pub struct Circle {
    pub admin: Address,
    pub contribution: i128,
    pub members: Vec<Address>,
    pub is_random_queue: bool,
    pub payout_queue: Vec<Address>,
    pub has_received_payout: Vec<bool>,
    pub cycle_number: u32,
    pub current_payout_index: u32,
    pub total_volume_distributed: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct CycleCompletedEvent {
    pub group_id: u32,
    pub total_volume_distributed: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct GroupRolloverEvent {
    pub group_id: u32,
    pub new_cycle_number: u32,
}

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    CycleNotComplete = 1001,
    InsufficientAllowance = 1002,
    AlreadyJoined = 1003,
    CircleNotFound = 1004,
    Unauthorized = 1005,
    MaxMembersReached = 1006,
    CircleNotFinalized = 1007,
    InvalidFeeConfig = 1008,
    InvalidUpgradeHash = 1009,
    PayoutAlreadyReceived = 1010,
    NotMember = 1011,
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
    env.storage().instance().set(&key, circle);
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
    // ============================================================
    // GLOBAL ADMIN & FEE LOGIC
    // ============================================================

    /// Initialize the contract with an admin. Call once after deploy.
    pub fn initialize(env: Env, admin: Address) -> Result<(), Error> {
        if env.storage().instance().has(&ADMIN_KEY) {
            return Err(Error::Unauthorized);
        }
        env.storage().instance().set(&ADMIN_KEY, &admin);
        env.storage().instance().set(&FEE_BASIS_POINTS_KEY, &0u32);
        Ok(())
    }

    /// Set protocol fee (basis points, e.g. 50 = 0.5%) and treasury address. Admin only.
    pub fn set_protocol_fee(
        env: Env,
        fee_basis_points: u32,
        treasury: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        if fee_basis_points > MAX_BASIS_POINTS {
            return Err(Error::InvalidFeeConfig);
        }
        env.storage()
            .instance()
            .set(&FEE_BASIS_POINTS_KEY, &fee_basis_points);
        env.storage().instance().set(&TREASURY_KEY, &treasury);
        Ok(())
    }

    pub fn fee_basis_points(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<_, u32>(&FEE_BASIS_POINTS_KEY)
            .unwrap_or(0)
    }

    pub fn treasury_address(env: Env) -> Option<Address> {
        env.storage().instance().get::<_, Address>(&TREASURY_KEY)
    }

    pub fn compute_and_transfer_payout(
        env: Env,
        token: Address,
        from: Address,
        recipient: Address,
        gross_payout: i128,
    ) -> Result<(), Error> {
        let fee_bps = env
            .storage()
            .instance()
            .get::<_, u32>(&FEE_BASIS_POINTS_KEY)
            .unwrap_or(0);
        let fee = if fee_bps == 0 {
            0_i128
        } else {
            (gross_payout * fee_bps as i128) / MAX_BASIS_POINTS as i128
        };
        let net_payout = gross_payout - fee;

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&from, &recipient, &net_payout);

        if fee > 0 {
            let treasury: Address = env
                .storage()
                .instance()
                .get::<_, Address>(&TREASURY_KEY)
                .ok_or(Error::InvalidFeeConfig)?;
            token_client.transfer(&from, &treasury, &fee);
        }
        Ok(())
    }

    /// Upgrade contract to new WASM binary. Admin only.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        Self::require_admin(&env)?;
        if new_wasm_hash.is_empty() {
            return Err(Error::InvalidUpgradeHash);
        }
        env.deployer().update_current_contract_wasm(new_wasm_hash);
        Ok(())
    }

    // ============================================================
    // CIRCLE MANAGEMENT & GOVERNANCE
    // ============================================================

    pub fn create_circle(env: Env, admin: Address, contribution: i128, is_random_queue: bool) -> u32 {
        admin.require_auth();
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
        };
        write_circle(&env, id, &circle);
        id
    }

    pub fn join_circle(env: Env, invoker: Address, circle_id: u32) {
        invoker.require_auth();
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

    pub fn process_payout(env: Env, admin: Address, circle_id: u32, recipient: Address) {
        admin.require_auth();
        let mut circle = read_circle(&env, circle_id);

        if admin != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        let mut member_index: Option<u32> = None;
        for (i, member) in circle.members.iter().enumerate() {
            if member == recipient {
                member_index = Some(i as u32);
                break;
            }
        }

        let index = match member_index {
            Some(i) => i,
            None => panic_with_error!(&env, Error::NotMember),
        };

        if circle.has_received_payout.get(index).unwrap_or(false) {
            panic_with_error!(&env, Error::PayoutAlreadyReceived);
        }

        circle.has_received_payout.set(index, true);
        circle.current_payout_index += 1;
        circle.total_volume_distributed += circle.contribution;

        let all_paid = circle.has_received_payout.iter().all(|paid| paid);
        if all_paid {
            let event = CycleCompletedEvent {
                group_id: circle_id,
                total_volume_distributed: circle.total_volume_distributed,
            };
            env.events().publish((symbol_short!("CYCLE_COMP"),), event);
        }

        write_circle(&env, circle_id, &circle);
    }

    pub fn rollover_group(env: Env, admin: Address, circle_id: u32) {
        admin.require_auth();
        let mut circle = read_circle(&env, circle_id);

        if admin != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        for received in circle.has_received_payout.iter() {
            if !received {
                panic_with_error!(&env, Error::CycleNotComplete);
            }
        }

        circle.cycle_number += 1;
        circle.current_payout_index = 0;
        circle.total_volume_distributed = 0;

        let len = circle.has_received_payout.len();
        circle.has_received_payout = Vec::new(&env);
        for _ in 0..len {
            circle.has_received_payout.push_back(false);
        }

        let event = GroupRolloverEvent {
            group_id: circle_id,
            new_cycle_number: circle.cycle_number,
        };
        env.events().publish((symbol_short!("GROUP_ROLL"),), event);

        write_circle(&env, circle_id, &circle);
    }

    pub fn finalize_circle(env: Env, admin: Address, circle_id: u32) {
        admin.require_auth();
        let mut circle = read_circle(&env, circle_id);

        if admin != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
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
    // VIEW GETTERS & HELPERS
    // ============================================================

    pub fn get_payout_queue(env: Env, circle_id: u32) -> Vec<Address> {
        let circle = read_circle(&env, circle_id);
        circle.payout_queue
    }

    pub fn get_cycle_info(env: Env, circle_id: u32) -> (u32, u32, i128) {
        let circle = read_circle(&env, circle_id);
        (
            circle.cycle_number,
            circle.current_payout_index,
            circle.total_volume_distributed,
        )
    }

    pub fn get_payout_status(env: Env, circle_id: u32) -> Vec<bool> {
        let circle = read_circle(&env, circle_id);
        circle.has_received_payout
    }

    fn require_admin(env: &Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(Error::Unauthorized)?;
        admin.require_auth();
        Ok(())
    }
}

#[cfg(test)]
mod test {
    extern crate std;

    use super::*;
    use soroban_sdk::testutils::{Address as _, AuthorizedFunction, AuthorizedInvocation};
    use soroban_sdk::{vec, IntoVal};

    fn setup() -> (soroban_sdk::Env, SoroSusuClient<'static>) {
        let env = soroban_sdk::Env::default();
        env.mock_all_auths();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        (env, client)
    }

    #[test]
    fn upgrade_requires_admin() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let non_admin = Address::generate(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        env.mock_auths(&[soroban_sdk::testutils::MockAuth {
            address: non_admin.clone(),
            invoke: &soroban_sdk::testutils::MockAuthInvoke {
                contract: &client.address,
                fn_name: "upgrade",
                args: vec![&env, new_wasm_hash.into_val(&env)],
                sub_invokes: &[],
            },
        }]);

        let result = client.try_upgrade(&new_wasm_hash);
        assert!(result.is_err());
    }

    #[test]
    fn upgrade_rejects_empty_hash() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        client.initialize(&admin);

        let empty_hash = BytesN::new(&env);
        let result = client.try_upgrade(&empty_hash);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), Error::InvalidUpgradeHash);
    }

    #[test]
    fn join_circle_enforces_max_members() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &false);

        for _ in 0..MAX_MEMBERS {
            let member = Address::generate(&env);
            client.join_circle(&member, &circle_id);
        }

        let extra = Address::generate(&env);
        let result = std::panic::catch_unwind(|| {
            client.join_circle(&extra, &circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_random_queue_finalization() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &true);

        let members: std::vec::Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(member, &circle_id);
        }

        client.finalize_circle(&admin, &circle_id);
        let queue = client.get_payout_queue(&circle_id);

        assert_eq!(queue.len(), 5);
        for member in &members {
            assert!(queue.contains(member));
        }
    }

    #[test]
    fn test_sequential_queue_finalization() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &false);

        let members: std::vec::Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(member, &circle_id);
        }

        client.finalize_circle(&admin, &circle_id);
        let queue = client.get_payout_queue(&circle_id);

        assert_eq!(queue.len(), 5);
        for (i, member) in members.iter().enumerate() {
            assert_eq!(queue.get(i as u32), Some(member.clone()));
        }
    }

    #[test]
    fn test_process_payout_and_cycle_completion() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &100_i128, &false);

        let members: std::vec::Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(member, &circle_id);
        }

        client.finalize_circle(&admin, &circle_id);

        for member in &members {
            client.process_payout(&admin, &circle_id, member);
        }

        let (cycle_num, payout_index, total_volume) = client.get_cycle_info(&circle_id);
        assert_eq!(cycle_num, 1);
        assert_eq!(payout_index, 3);
        assert_eq!(total_volume, 300_i128);

        let events = env.events().all();
        assert!(!events.is_empty());
    }

    #[test]
    fn test_group_rollover() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &50_i128, &false);

        let members: std::vec::Vec<Address> = (0..2).map(|_| Address::generate(&env)).collect();
        for member in &members {
            client.join_circle(member, &circle_id);
        }

        client.finalize_circle(&admin, &circle_id);

        for member in &members {
            client.process_payout(&admin, &circle_id, member);
        }

        client.rollover_group(&admin, &circle_id);

        let (cycle_num, payout_index, total_volume) = client.get_cycle_info(&circle_id);
        assert_eq!(cycle_num, 2);
        assert_eq!(payout_index, 0);
        assert_eq!(total_volume, 0_i128);
    }

    #[test]
    fn test_payout_unauthorized() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &false);

        let member = Address::generate(&env);
        client.join_circle(&member, &circle_id);
        client.finalize_circle(&admin, &circle_id);

        let unauthorized = Address::generate(&env);
        let result = std::panic::catch_unwind(|| {
            client.process_payout(&unauthorized, &circle_id, &member);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rollover_before_cycle_complete() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &false);

        let member = Address::generate(&env);
        client.join_circle(&member, &circle_id);

        let result = std::panic::catch_unwind(|| {
            client.rollover_group(&admin, &circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_payout() {
        let (env, client) = setup();
        let admin = Address::generate(&env);
        let circle_id = client.create_circle(&admin, &10_i128, &false);

        let member = Address::generate(&env);
        client.join_circle(&member, &circle_id);
        client.finalize_circle(&admin, &circle_id);
        client.process_payout(&admin, &circle_id, &member);

        let result = std::panic::catch_unwind(|| {
            client.process_payout(&admin, &circle_id, &member);
        });
        assert!(result.is_err());
    }
}