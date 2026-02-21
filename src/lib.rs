#![no_std]
#![cfg_attr(test, allow(dead_code))]

use soroban_sdk::{
    contract, contracterror, contractimpl, contractmeta, symbol_short, token, Address, Env, Symbol,
};

const FEE_BASIS_POINTS_KEY: Symbol = symbol_short!("fee_bps");
const TREASURY_KEY: Symbol = symbol_short!("treasury");
const ADMIN_KEY: Symbol = symbol_short!("admin");
const MEMBERS_COUNT_KEY: Symbol = symbol_short!("m_count");
const MAX_MEMBERS: u32 = 50;
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
    MemberLimitExceeded = 1007,
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
        env.storage().instance().set(&FEE_BASIS_POINTS_KEY, &fee_basis_points);
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
        let fee_bps = env.storage().instance().get::<_, u32>(&FEE_BASIS_POINTS_KEY).unwrap_or(0);
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

    /// Join a savings circle. Limited to MAX_MEMBERS.
    pub fn join(env: Env, _user: Address) -> Result<(), Error> {
        let mut count: u32 = env.storage().instance().get(&MEMBERS_COUNT_KEY).unwrap_or(0);
        if count >= MAX_MEMBERS {
            return Err(Error::MemberLimitExceeded);
        }
        count += 1;
        env.storage().instance().set(&MEMBERS_COUNT_KEY, &count);
        Ok(())
    }

    /// Get current member count.
    pub fn member_count(env: Env) -> u32 {
        env.storage().instance().get(&MEMBERS_COUNT_KEY).unwrap_or(0)
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
    fn test_member_limit_boundary() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        // Join 50 members
        for _ in 0..50 {
            let user = Address::generate(&env);
            client.join(&user).unwrap();
        }

        assert_eq!(client.member_count(), 50);

        // 51st member should fail
        let user_51 = Address::generate(&env);
        let result = client.join(&user_51);
        
        assert!(result.is_err());
        // Soroban results in tests are usually Result<Result<(), Error>, Error> or similar if using client
        // But here we've structured it simply. Let's check the error code.
        match result {
            Err(e) => assert_eq!(e, Error::MemberLimitExceeded),
            _ => panic!("Expected Error::MemberLimitExceeded"),
        }
    }
}

