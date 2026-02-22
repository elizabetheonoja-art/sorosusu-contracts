#![no_std]
#![cfg_attr(test, allow(dead_code))]

use soroban_sdk::{
    Address, Env, Map, Symbol, Vec, contract, contracterror, contractimpl, contractmeta,
    symbol_short, token,
};

const FEE_BASIS_POINTS_KEY: Symbol = symbol_short!("fee_bps");
const TREASURY_KEY: Symbol = symbol_short!("treasury");
const ADMIN_KEY: Symbol = symbol_short!("admin");
const CIRCLES_KEY: Symbol = symbol_short!("circles");
const MEMBER_DATA_KEY: Symbol = symbol_short!("member_data");
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
            .set(&(id, symbol_short!("contribution")), &contribution_amount);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("total_rounds")), &total_rounds);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("token")), &token_address);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("current_round")), &0u32);
        env.storage()
            .instance()
            .set(&(id, symbol_short!("active")), &true);

        // Initialize member data
        for (i, member_addr) in members.iter().enumerate() {
            let member_key: Symbol = symbol_short!("member_").concat(&i.to_string().into_val(&env));
            env.storage().instance().set(&(id, member_key), &member_addr);
            env.storage().instance().set(&(id, member_addr.clone(), symbol_short!("contributions")), &0u32);
            env.storage().instance().set(&(id, member_addr.clone(), symbol_short!("payout_received")), &false);
                    member_addr.clone(),
                    symbol_short!("payout_received"),
                ),
                &false,
            );
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
            .get(&(
                circle_id,
                recipient.clone(),
                symbol_short!("payout_received"),
            ))
            .ok_or(Error::Unauthorized)?;

        if has_received {
            return Err(Error::PayoutAlreadyReceived);
        }

        // Check if all members have contributed for this round
        let current_round: u32 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("current_round")))
            .ok_or(Error::CircleNotFound)?;

        for member_addr in members.iter() {
            let contributions: u32 = env
                .storage()
                .instance()
                .get(&(
                    circle_id,
                    member_addr.clone(),
                    symbol_short!("contributions"),
                ))
                .ok_or(Error::CircleNotFound)?;
            if contributions <= current_round {
                return Err(Error::CycleNotComplete);
            }
        }

        // EFFECTS: Update all state before external calls
        let contribution_amount: i128 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("contribution")))
            .ok_or(Error::CircleNotFound)?;

        let total_payout = contribution_amount * members.len() as i128;

        // Mark payout as received
        env.storage().instance().set(
            &(
                circle_id,
                recipient.clone(),
                symbol_short!("payout_received"),
            ),
            &true,
        );

        // Increment round
        let new_round = current_round + 1;
        env.storage()
            .instance()
            .set(&(circle_id, symbol_short!("current_round")), &new_round);

        // Check if circle should be deactivated
        let total_rounds: u32 = env
            .storage()
            .instance()
            .get(&(circle_id, symbol_short!("total_rounds")))
            .ok_or(Error::CircleNotFound)?;

        if new_round >= total_rounds {
            env.storage()
                .instance()
                .set(&(circle_id, symbol_short!("active")), &false);
        } else {
            // Reset payout flags for next round
            for member_addr in members.iter() {
                env.storage().instance().set(
                    &(
                        circle_id,
                        member_addr.clone(),
                        symbol_short!("payout_received"),
                    ),
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

        let circle = client.get_circle(&1).unwrap();
        assert_eq!(circle.id, 1);
        assert_eq!(circle.contribution_amount, 100);
        assert_eq!(circle.total_rounds, 3);
        assert_eq!(circle.current_round, 0);
        assert!(circle.is_active);
        assert_eq!(circle.member_addresses.len(), 3);
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
        let mut circle = client.get_circle(&1).unwrap();
        for member_addr in circle.member_addresses.iter() {
            let mut member = circle.member_data.get(member_addr).unwrap();
            member.contribution_count = 1; // All contributed for round 0
            circle.member_data.set(member_addr.clone(), member);
        }

        // Update the circle with contributions
        let mut circles: Map<u32, SavingsCircle> = env
            .storage()
            .instance()
            .get(&CIRCLES_KEY)
            .unwrap_or(Map::new(&env));
        circles.set(1, circle);
        env.storage().instance().set(&CIRCLES_KEY, &circles);

        // Now test payout - this should work
        env.mock_all_auths();
        let _result = client.payout(&1, &member1);

        // The payout should succeed (though token transfer might fail in test env)
        // What's important is that state changes happen before the transfer
        let updated_circle = client.get_circle(&1).unwrap();
        assert_eq!(updated_circle.current_round, 1); // State was updated
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
        let mut circle = client.get_circle(&1).unwrap();
        for member_addr in circle.member_addresses.iter() {
            let mut member = circle.member_data.get(member_addr).unwrap();
            member.contribution_count = 1; // All contributed for round 0
            circle.member_data.set(member_addr.clone(), member);
        }

        // Update the circle with contributions
        let mut circles: Map<u32, SavingsCircle> = env
            .storage()
            .instance()
            .get(&CIRCLES_KEY)
            .unwrap_or(Map::new(&env));
        circles.set(1, circle);
        env.storage().instance().set(&CIRCLES_KEY, &circles);

        // Check initial state
        let initial_circle = client.get_circle(&1).unwrap();
        assert_eq!(initial_circle.current_round, 0);
        let initial_member = initial_circle.member_data.get(&malicious_member).unwrap();
        assert!(!initial_member.has_received_payout);

        // Attempt payout - state changes should happen before any external calls
        env.mock_all_auths();

        // Even if the token transfer fails, the state should already be updated
        // This prevents re-entrancy attacks because the contract state
        // reflects the completed operation before any external interaction
        let _result = client.payout(&1, &malicious_member);

        // Verify state was updated before the external call
        let final_circle = client.get_circle(&1).unwrap();
        assert_eq!(final_circle.current_round, 1); // Round incremented

        // The member should be marked as having received payout
        let member = final_circle.member_data.get(&malicious_member).unwrap();
        assert!(member.has_received_payout);

        // Even if a re-entrant call were made during the token transfer,
        // the state already reflects the completed payout, preventing double-payout
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
        let mut circle = client.get_circle(&1).unwrap();
        for member_addr in circle.member_addresses.iter() {
            let mut member = circle.member_data.get(member_addr).unwrap();
            member.contribution_count = 1;
            if member_addr == &member1 {
                member.has_received_payout = true; // Already received
            }
            circle.member_data.set(member_addr.clone(), member);
        }

        let mut circles: Map<u32, SavingsCircle> = env
            .storage()
            .instance()
            .get(&CIRCLES_KEY)
            .unwrap_or(Map::new(&env));
        circles.set(1, circle);
        env.storage().instance().set(&CIRCLES_KEY, &circles);

        env.mock_all_auths();
        let result = client.payout(&1, &member1);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), Error::PayoutAlreadyReceived);
    }
}
