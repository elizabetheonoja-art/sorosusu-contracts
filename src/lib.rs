#![no_std]
#![cfg_attr(test, allow(dead_code))]

use soroban_sdk::{
    Address, BytesN, Env, Symbol, Vec, contract, contracterror, contractimpl, contractmeta,
    symbol_short, token,
};

const FEE_BASIS_POINTS_KEY: Symbol = symbol_short!("fee_bps");
const TREASURY_KEY: Symbol = symbol_short!("treasury");
const ADMIN_KEY: Symbol = symbol_short!("admin");
const CIRCLES_KEY: Symbol = symbol_short!("circles");
const MEMBER_DATA_KEY: Symbol = symbol_short!("mem_data");
const MAX_BASIS_POINTS: u32 = 10_000;

contractmeta!(
    key = "Description",
    val = "SoroSusu ROSCA protocol with protocol payout fee"
);

#[contract]
pub struct SorosusuContracts;

#[contracterror]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1005,
    InvalidFeeConfig = 1006,
    CircleNotFound = 1004,
    CycleNotComplete = 1001,
    AlreadyJoined = 1003,
    InsufficientAllowance = 1002,
    PayoutAlreadyReceived = 1007,
    InvalidCircleState = 1008,
    InvalidUpgradeHash = 1009,
}

#[contractimpl]
impl SorosusuContracts {
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
    /// fee_basis_points must be <= 10_000. If fee_basis_points > 0, treasury must be set.
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

    /// Get current fee basis points (e.g. 50 = 0.5%).
    pub fn fee_basis_points(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<_, u32>(&FEE_BASIS_POINTS_KEY)
            .unwrap_or(0)
    }

    /// Get treasury address (recipient of protocol fee).
    pub fn treasury_address(env: Env) -> Option<Address> {
        env.storage().instance().get::<_, Address>(&TREASURY_KEY)
    }

    /// Compute fee from gross amount and perform transfers: net to recipient, fee to treasury.
    /// Call this from the payout flow. `from` is the address holding the tokens (e.g. contract).
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
            (gross_payout as i128 * fee_bps as i128) / MAX_BASIS_POINTS as i128
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

    /// Create a new savings circle
    pub fn create_circle(
        env: Env,
        id: u32,
        members: Vec<Address>,
        contribution_amount: i128,
        total_rounds: u32,
        token_address: Address,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;

        // Store circle data
        let circle_key: (u32, Symbol) = (id, symbol_short!("circle"));
        env.storage().instance().set(&circle_key, &members);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("contrib_amt")), &contribution_amount);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("tot_rounds")), &total_rounds);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("cur_round")), &0u32);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("token")), &token_address);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("active")), &true);

        // Initialize member data
        for member_addr in members.iter() {
            env.storage()
                .instance()
                .set(&(id, member_addr.clone(), symbol_short!("contrib")), &0u32);
            env.storage()
                .instance()
                .set(&(id, member_addr.clone(), symbol_short!("paid")), &false);
        }

        Ok(())
    }

    /// Payout to the current round's recipient following Checks-Effects-Interactions pattern
    pub fn payout(env: Env, circle_id: u32, recipient: Address) -> Result<(), Error> {
        // CHECKS: Validate all conditions before making any state changes
        let members_key: (u32, Symbol) = (circle_id, symbol_short!("circle"));
        let members: Vec<Address> = env
            .storage()
            .instance()
            .get(&members_key)
            .ok_or(Error::CircleNotFound)?;

        let is_active: bool = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("active")))
            .ok_or(Error::CircleNotFound)?;
        if !is_active {
            return Err(Error::InvalidCircleState);
        }

        let has_received: bool = env
            .storage()
            .instance()
            .get(&(circle_id, recipient.clone(), symbol_short!("paid")))
            .ok_or(Error::Unauthorized)?;

        if has_received {
            return Err(Error::PayoutAlreadyReceived);
        }

        // Check if all members have contributed for this round
        let current_round: u32 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("cur_round")))
            .ok_or(Error::CircleNotFound)?;

        for member_addr in members.iter() {
            let contributions: u32 = env
                .storage()
                .instance()
                .get(&(circle_id, member_addr.clone(), symbol_short!("contrib")))
                .ok_or(Error::CircleNotFound)?;
            if contributions <= current_round {
                return Err(Error::CycleNotComplete);
            }
        }

        // EFFECTS: Update all state before external calls
        let contribution_amount: i128 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("contrib_amt")))
            .ok_or(Error::CircleNotFound)?;

        let total_payout = contribution_amount * members.len() as i128;

        // Mark payout as received
        env.storage().instance().set(
            &(circle_id, recipient.clone(), symbol_short!("paid")),
            &true,
        );

        // Increment round
        let new_round = current_round + 1;
        env.storage()
            .instance()
            .set(&(circle_id, symbol_short!("cur_round")), &new_round);

        // Check if circle should be deactivated
        let total_rounds: u32 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("tot_rounds")))
            .ok_or(Error::CircleNotFound)?;

        if new_round >= total_rounds {
            env.storage()
                .instance()
                .set(&(circle_id, symbol_short!("active")), &false);
        } else {
            // Reset payout flags for next round
            for member_addr in members.iter() {
                env.storage().instance().set(
                    &(circle_id, member_addr.clone(), symbol_short!("paid")),
                    &false,
                );
            }
        }

        // INTERACTIONS: Perform external calls only after all state changes
        let token_address: Address = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("token")))
            .ok_or(Error::CircleNotFound)?;

        Self::compute_and_transfer_payout(
            env,
            token_address,
            env.current_contract_address(),
            recipient,
            total_payout,
        )?;

        Ok(())
    }

    /// Upgrade contract to new WASM binary. Admin only.
    /// This allows patching logic without losing storage state.
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>) -> Result<(), Error> {
        Self::require_admin(&env)?;

        // Validate new WASM hash is not empty
        if new_wasm_hash.is_empty() {
            return Err(Error::InvalidUpgradeHash);
        }

        // Perform the upgrade
        env.deployer().update_current_contract_wasm(new_wasm_hash);

        Ok(())
    }

    fn require_admin(env: &Env) -> Result<(), Error> {
        let admin: Address = env
            .storage()
            .instance()
            .get(&ADMIN_KEY)
            .ok_or(Error::Unauthorized)?;
        admin.require_auth();
        Ok(())

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
}

#[derive(Clone)]
#[contracttype]
pub struct CycleCompletedEvent {
    group_id: u32,
    total_volume_distributed: i128,
}

#[derive(Clone)]
#[contracttype]
pub struct GroupRolloverEvent {
    group_id: u32,
    new_cycle_number: u32,
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
        let circle = Circle {
            admin,
            contribution,
            members,
            is_random_queue,
            payout_queue,
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

    pub fn process_payout(env: Env, circle_id: u32, recipient: Address) {
        let mut circle = read_circle(&env, circle_id);

        // Only admin can process payouts
        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Check if recipient is a member
        let mut member_index = None;
        for (i, member) in circle.members.iter().enumerate() {
            if member == recipient {
                member_index = Some(i);
                break;
            }
        }

        if member_index.is_none() {
            panic_with_error!(&env, Error::Unauthorized);
        }

        let index = member_index.unwrap();

        // Check if member has already received payout for current cycle
        if circle.has_received_payout.get(index).unwrap_or(&false) == &true {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Mark as received
        circle.has_received_payout.set(index, true);
        circle.current_payout_index += 1;

        // Add to total volume distributed
        circle.total_volume_distributed += circle.contribution;

        // Check if this was the last payout for the cycle
        let all_paid = circle.has_received_payout.iter().all(|&paid| paid);

        if all_paid {
            // Emit CycleCompleted event
            let event = CycleCompletedEvent {
                group_id: circle_id,
                total_volume_distributed: circle.total_volume_distributed,
            };
            event::publish(&env, symbol_short!("CYCLE_COMP"), &event);
        }

        write_circle(&env, circle_id, &circle);
    }

    pub fn rollover_group(env: Env, circle_id: u32) {
        let mut circle = read_circle(&env, circle_id);

        // Only admin can rollover the group
        if env.invoker() != circle.admin {
            panic_with_error!(&env, Error::Unauthorized);
        }

        // Check if all members have received payout for current cycle
        for received in circle.has_received_payout.iter() {
            if !received {
                panic_with_error!(&env, Error::CycleNotComplete);
            }
        }

        // Reset for next cycle
        circle.cycle_number += 1;
        circle.current_payout_index = 0;

        // Reset payout flags
        for i in 0..circle.has_received_payout.len() {
            circle.has_received_payout.set(i, false);
        }

        // Reset volume for new cycle
        circle.total_volume_distributed = 0;

        // Emit GroupRollover event
        let event = GroupRolloverEvent {
            group_id: circle_id,
            new_cycle_number: circle.cycle_number,
        };
        event::publish(&env, symbol_short!("GROUP_ROLL"), &event);

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

    pub fn get_payout_queue(env: Env, circle_id: u32) -> Vec<Address> {
        let circle = read_circle(&env, circle_id);
        circle.payout_queue
    pub fn get_cycle_info(env: Env, circle_id: u32) -> (u32, u32, i128) {
        let circle = read_circle(&env, circle_id);
        (
            circle.cycle_number,
            circle.current_payout_index,
            circle.total_volume_distributed,
        )
    }

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    fn setup(env: &Env) -> (SorosusuContractsClient, Address) {
        let contract_id = env.register_contract(None, SorosusuContracts);
        let admin = Address::generate(env);
        let client = SorosusuContractsClient::new(env, &contract_id);
        (client, admin)
    }

    #[test]
    fn set_protocol_fee_rejects_over_max() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);
        let treasury = Address::generate(&env);
        env.mock_all_auths();
        let result = client.set_protocol_fee(&10_001, &treasury);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidFeeConfig);
    }

    #[test]
    fn fee_basis_points_and_treasury_getters() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);
        assert_eq!(client.fee_basis_points(), 0);
        assert!(client.treasury_address().is_none());

        let treasury = Address::generate(&env);
        env.mock_all_auths();
        client.set_protocol_fee(&50, &treasury).unwrap();
        assert_eq!(client.fee_basis_points(), 50);
        assert_eq!(client.treasury_address(), Some(treasury));
    }

    #[test]
    fn fee_zero_accepted_and_getter_returns_zero() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);
        let treasury = Address::generate(&env);
        env.mock_all_auths();
        client.set_protocol_fee(&0, &treasury).unwrap();
        assert_eq!(client.fee_basis_points(), 0);
    }

    #[test]
    fn create_circle_works() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let member3 = Address::generate(&env);
        let members = vec![&env, member1.clone(), member2.clone(), member3.clone()];
        let token_address = Address::generate(&env);

        env.mock_all_auths();
        client
            .create_circle(&1, &members, &100, &3, &token_address)
            .unwrap();

        // Verify circle was created
        let stored_members: Vec<Address> = env
            .storage()
            .instance()
            .get(&(1, symbol_short!("circle")))
            .unwrap();
        assert_eq!(stored_members.len(), 3);
    }

    #[test]
    fn payout_follows_checks_effects_interactions_pattern() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let member3 = Address::generate(&env);
        let members = vec![&env, member1.clone(), member2.clone(), member3.clone()];
        let token_address = Address::generate(&env);

        env.mock_all_auths();
        client
            .create_circle(&1, &members, &100, &3, &token_address)
            .unwrap();

        // Simulate all members contributing
        for member_addr in members.iter() {
            env.storage()
                .instance()
                .set(&(1, member_addr.clone(), symbol_short!("contrib")), &1u32);
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
    fn test_process_payout_and_cycle_completion() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 100_i128;

        // Create circle and add members
        let circle_id = client.create_circle(&contribution);
        let members: Vec<Address> = (0..3).map(|_| Address::generate(&env)).collect();

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

        // Now test payout - this should work
        env.mock_all_auths();
        let _result = client.payout(&1, &member1);

        // The payout should succeed (though token transfer might fail in test env)
        // What's important is that state changes happen before the transfer
        let updated_round: u32 = env
            .storage()
            .instance()
            .get(&(1, symbol_short!("cur_round")))
            .unwrap();
        assert_eq!(updated_round, 1); // State was updated
    }

    #[test]
    fn test_reentrancy_protection_during_payout() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let malicious_member = Address::generate(&env);
        let member2 = Address::generate(&env);
        let member3 = Address::generate(&env);
        let members = vec![
            &env,
            malicious_member.clone(),
            member2.clone(),
            member3.clone(),
        ];
        let token_address = Address::generate(&env);

        env.mock_all_auths();
        client
            .create_circle(&1, &members, &100, &3, &token_address)
            .unwrap();

        // Simulate all members contributing
        for member_addr in members.iter() {
            env.storage()
                .instance()
                .set(&(1, member_addr.clone(), symbol_short!("contrib")), &1u32);
        }

        // Check initial state
        let initial_round: u32 = env
            .storage()
            .instance()
            .get(&(1, symbol_short!("cur_round")))
            .unwrap();
        let initial_received: bool = env
            .storage()
            .instance()
            .get(&(1, malicious_member.clone(), symbol_short!("paid")))
            .unwrap();
        assert_eq!(initial_round, 0);
        assert!(!initial_received);

        // Attempt payout - state changes should happen before any external calls
        env.mock_all_auths();

        // Even if the token transfer fails, state should already be updated
        // This prevents re-entrancy attacks because the contract state
        // reflects the completed operation before any external interaction
        let _result = client.payout(&1, &malicious_member);

        // Verify state was updated before external call
        let final_round: u32 = env
            .storage()
            .instance()
            .get(&(1, symbol_short!("cur_round")))
            .unwrap();
        assert_eq!(final_round, 1); // Round incremented

        // The member should be marked as having received payout
        let final_received: bool = env
            .storage()
            .instance()
            .get(&(1, malicious_member.clone(), symbol_short!("paid")))
            .unwrap();
        assert!(final_received);

        // Even if a re-entrant call were made during the token transfer,
        // state already reflects the completed payout, preventing double-payout
        let reentrant_result = client.payout(&1, &malicious_member);
        assert!(reentrant_result.is_err()); // Should fail - already received payout
        assert_eq!(reentrant_result.unwrap_err(), Error::PayoutAlreadyReceived);
    }

    #[test]
    fn payout_fails_when_cycle_not_complete() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let members = vec![&env, member1.clone(), member2.clone()];
        let token_address = Address::generate(&env);

        env.mock_all_auths();
        client
            .create_circle(&1, &members, &100, &2, &token_address)
            .unwrap();

        // Don't simulate contributions - cycle is incomplete
        env.mock_all_auths();
        let result = client.payout(&1, &member1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::CycleNotComplete);
    }

    #[test]
    fn payout_fails_when_already_received() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let members = vec![&env, member1.clone(), member2.clone()];
        let token_address = Address::generate(&env);

        env.mock_all_auths();
        client
            .create_circle(&1, &members, &100, &2, &token_address)
            .unwrap();

        // Simulate contributions and mark member1 as already received payout
        for member_addr in members.iter() {
            env.storage()
                .instance()
                .set(&(1, member_addr.clone(), symbol_short!("contrib")), &1u32);
        }
        env.storage()
            .instance()
            .set(&(1, member1.clone(), symbol_short!("paid")), &true);

        env.mock_all_auths();
        let result = client.payout(&1, &member1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::PayoutAlreadyReceived);
    }

    #[test]
    fn upgrade_requires_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let non_admin = Address::generate(&env);
        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // Try upgrade with non-admin - should fail
        env.mock_auths(&non_admin, &[]);
        let result = client.upgrade(&new_wasm_hash);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::Unauthorized);
    }

    #[test]
    fn upgrade_rejects_empty_hash() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let empty_hash = BytesN::new(&env);

        // Try upgrade with empty hash - should fail
        env.mock_all_auths();
        let result = client.upgrade(&empty_hash);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::InvalidUpgradeHash);
    }

    #[test]
    fn upgrade_works_with_admin() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let new_wasm_hash = BytesN::from_array(&env, &[1u8; 32]);

        // Try upgrade with admin - should work
        env.mock_all_auths();
        let result = client.upgrade(&new_wasm_hash);
        assert!(result.is_ok());
    fn test_sequential_queue_finalization() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        // Create circle with random queue disabled
        let circle_id = client.create_circle(&contribution, &false);

        // Add some members in a specific order
        let members: Vec<Address> = (0..5).map(|_| Address::generate(&env)).collect();
        // Process payouts for all members
        for member in &members {
            client.process_payout(&circle_id, member);
        }

        // Verify cycle info
        let (cycle_num, payout_index, total_volume) = client.get_cycle_info(&circle_id);
        assert_eq!(cycle_num, 1);
        assert_eq!(payout_index, 3);
        assert_eq!(total_volume, 300_i128);

        // Check that events were emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1); // One CycleCompleted event

        let event = &events[0];
        assert_eq!(event.0, symbol_short!("CYCLE_COMP"));
    }

    #[test]
    fn test_group_rollover() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 50_i128;

        // Create circle and add members
        let circle_id = client.create_circle(&contribution);
        let members: Vec<Address> = (0..2).map(|_| Address::generate(&env)).collect();

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
    fn test_finalize_circle_unauthorized() {
        // Process all payouts
        for member in &members {
            client.process_payout(&circle_id, member);
        }

        // Clear events to test rollover event
        env.events().all();

        // Perform rollover
        client.rollover_group(&circle_id);

        // Verify new cycle info
        let (cycle_num, payout_index, total_volume) = client.get_cycle_info(&circle_id);
        assert_eq!(cycle_num, 2);
        assert_eq!(payout_index, 0);
        assert_eq!(total_volume, 0_i128);

        // Check that rollover event was emitted
        let events = env.events().all();
        assert_eq!(events.len(), 1);

        let event = &events[0];
        assert_eq!(event.0, symbol_short!("GROUP_ROLL"));
    }

    #[test]
    fn test_payout_unauthorized() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution, &true);

        // Try to finalize with non-admin
        let circle_id = client.create_circle(&contribution);
        let member = Address::generate(&env);
        client.join_circle(&circle_id);

        // Try to process payout with non-admin
        let unauthorized_user = Address::generate(&env);
        env.set_source_account(&unauthorized_user);

        let result = std::panic::catch_unwind(|| {
            client.finalize_circle(&circle_id);
            client.process_payout(&circle_id, &member);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_rollover_before_cycle_complete() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution);
        let member = Address::generate(&env);
        client.join_circle(&circle_id);

        // Try to rollover without completing payouts
        let result = std::panic::catch_unwind(|| {
            client.rollover_group(&circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_duplicate_payout() {
        let env = Env::default();
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        let contribution = 10_i128;

        let circle_id = client.create_circle(&contribution);
        let member = Address::generate(&env);
        client.join_circle(&circle_id);

        // Process payout once
        client.process_payout(&circle_id, &member);

        // Try to process payout again for same member
        let result = std::panic::catch_unwind(|| {
            client.process_payout(&circle_id, &member);
        });
        assert!(result.is_err());
    }
}
