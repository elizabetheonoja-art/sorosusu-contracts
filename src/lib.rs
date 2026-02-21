#![no_std]
#![cfg_attr(test, allow(dead_code))]
use soroban_sdk::{contract, contractimpl, contracttype, token, Address, Env};

const SEVEN_DAYS: u64 = 7 * 24 * 60 * 60;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    LastActiveTimestamp,
    UserBalance(Address),
    Admin,
}

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusu {
    pub fn init_legacy(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::LastActiveTimestamp, &env.ledger().timestamp());
    }

    pub fn admin_action(env: Env) {
        let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        admin.require_auth();
        env.storage().instance().set(&DataKey::LastActiveTimestamp, &env.ledger().timestamp());
    }

    pub fn deposit(env: Env, user: Address, token_address: Address, amount: i128) {
        user.require_auth();
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&user, &env.current_contract_address(), &amount);
        
        let current_balance: i128 = env.storage().persistent()
            .get(&DataKey::UserBalance(user.clone()))
            .unwrap_or(0);
        env.storage().persistent().set(&DataKey::UserBalance(user), &(current_balance + amount));
    }

    pub fn emergency_withdraw(env: Env, user: Address, token_address: Address) {
        user.require_auth();
        
        let last_active: u64 = env.storage().instance()
            .get(&DataKey::LastActiveTimestamp)
            .unwrap_or(0);
        let current_time = env.ledger().timestamp();
        
        if current_time <= last_active + SEVEN_DAYS {
            panic!("Emergency withdrawal not available yet");
        }
        
        let balance: i128 = env.storage().persistent()
            .get(&DataKey::UserBalance(user.clone()))
            .unwrap_or(0);
        
        if balance > 0 {
            let token_client = token::Client::new(&env, &token_address);
            token_client.transfer(&env.current_contract_address(), &user, &balance);
            env.storage().persistent().remove(&DataKey::UserBalance(user));
        }
    }

    pub fn get_user_balance(env: Env, user: Address) -> i128 {
        env.storage().persistent()
            .get(&DataKey::UserBalance(user))
            .unwrap_or(0)
    }

    pub fn get_last_active_timestamp(env: Env) -> u64 {
        env.storage().instance()
            .get(&DataKey::LastActiveTimestamp)
            .unwrap_or(0)
    }
}

use soroban_sdk::{
    contracterror, contractmeta, symbol_short, Map, Symbol, Vec,
};

const FEE_BASIS_POINTS_KEY: Symbol = symbol_short!("fee_bps");
const TREASURY_KEY: Symbol = symbol_short!("treasury");
const ADMIN_KEY: Symbol = symbol_short!("admin");
const MEMBERS_KEY: Symbol = symbol_short!("members");
const CONTRIBS_KEY: Symbol = symbol_short!("contribs");
const IS_PUBLIC_KEY: Symbol = symbol_short!("is_pub");
const INVITE_CODE_KEY: Symbol = symbol_short!("inv_code");
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
    AlreadyJoined = 1003,
    Unauthorized = 1005,
    InvalidFeeConfig = 1006,
    MemberNotFound = 1007,
    PenaltyExceedsContribution = 1008,
    InvalidInvite = 1009,
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
        env.storage().instance().set(&IS_PUBLIC_KEY, &true);
        
        // Initialize empty members list and contributions map
        let empty_members: Vec<Address> = Vec::new(&env);
        let empty_contribs: Map<Address, i128> = Map::new(&env);
        env.storage().instance().set(&MEMBERS_KEY, &empty_members);
        env.storage().instance().set(&CONTRIBS_KEY, &empty_contribs);
        
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
        env.storage().instance().set(&FEE_BASIS_POINTS_KEY, &fee_basis_points);
        env.storage().instance().set(&TREASURY_KEY, &treasury);
        Ok(())
    }

    /// Get current fee basis points.
    pub fn fee_basis_points(env: Env) -> u32 {
        env.storage()
            .instance()
            .get::<_, u32>(&FEE_BASIS_POINTS_KEY)
            .unwrap_or(0)
    }

    /// Get treasury address.
    pub fn treasury_address(env: Env) -> Option<Address> {
        env.storage().instance().get::<_, Address>(&TREASURY_KEY)
    }

    /// Compute fee from gross amount and perform transfers.
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

    /// [Feature] "Kick Member" (Admin Only)
    /// Removes a member, refunds their contributions minus a penalty, and emits MemberKicked.
    pub fn kick_member(
        env: Env,
        token: Address,
        member: Address,
        penalty: i128,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;

        // 1. Load members and find the member to kick
        let mut members: Vec<Address> = env.storage().instance().get(&MEMBERS_KEY).unwrap_or(Vec::new(&env));
        let mut member_index = None;
        
        for (i, m) in members.iter().enumerate() {
            if m == member {
                member_index = Some(i as u32);
                break;
            }
        }

        let index = member_index.ok_or(Error::MemberNotFound)?;

        // 2. Load contributions
        let mut contribs: Map<Address, i128> = env.storage().instance().get(&CONTRIBS_KEY).unwrap_or(Map::new(&env));
        let total_contributed = contribs.get(member.clone()).unwrap_or(0);

        if total_contributed < penalty {
            return Err(Error::PenaltyExceedsContribution);
        }

        // 3. Remove member from state
        members.remove(index);
        contribs.remove(member.clone());
        
        env.storage().instance().set(&MEMBERS_KEY, &members);
        env.storage().instance().set(&CONTRIBS_KEY, &contribs);

        // 4. Calculate refund and perform transfers
        let refund = total_contributed - penalty;
        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);

        if refund > 0 {
            token_client.transfer(&contract_address, &member, &refund);
        }

        // If there's a penalty, route it to the treasury
        if penalty > 0 {
            if let Some(treasury) = Self::treasury_address(env.clone()) {
                token_client.transfer(&contract_address, &treasury, &penalty);
            }
        }

        // 5. Emit MemberKicked event
        // Topics: ["MemberKicked", member_address], Data: [refund_amount, penalty_amount]
        env.events().publish(
            (symbol_short!("Kicked"), member.clone()),
            (refund, penalty),
        );

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
    
    // NOTE: You will need to build an `add_member` or `deposit` function to populate 
    // the `MEMBERS_KEY` vector and `CONTRIBS_KEY` map.

    /// Set privacy configuration for the circle (Admin only).
    pub fn set_privacy_config(
        env: Env,
        is_public: bool,
        invite_code: Option<u64>,
    ) -> Result<(), Error> {
        Self::require_admin(&env)?;
        env.storage().instance().set(&IS_PUBLIC_KEY, &is_public);
        if let Some(code) = invite_code {
            env.storage().instance().set(&INVITE_CODE_KEY, &code);
        } else {
            env.storage().instance().remove(&INVITE_CODE_KEY);
        }
        Ok(())
    }

    /// [Feature] "Public" vs "Private" Groups
    /// Join the circle
    pub fn join(env: Env, member: Address, invite_code: Option<u64>) -> Result<(), Error> {
        member.require_auth();

        let mut members: Vec<Address> = env.storage().instance().get(&MEMBERS_KEY).unwrap_or(Vec::new(&env));
        
        if members.contains(&member) {
            return Err(Error::AlreadyJoined);
        }

        let is_public: bool = env.storage().instance().get(&IS_PUBLIC_KEY).unwrap_or(true);

        if !is_public {
            let mut authorized_by_code = false;
            if let Some(code) = invite_code {
                if let Some(expected_code) = env.storage().instance().get::<_, u64>(&INVITE_CODE_KEY) {
                    if code == expected_code {
                        authorized_by_code = true;
                    }
                }
            }
            if !authorized_by_code {
                Self::require_admin(&env)?;
            }
        }

        members.push_back(member.clone());
        env.storage().instance().set(&MEMBERS_KEY, &members);

        let mut contribs: Map<Address, i128> = env.storage().instance().get(&CONTRIBS_KEY).unwrap_or(Map::new(&env));
        contribs.set(member, 0);
        env.storage().instance().set(&CONTRIBS_KEY, &contribs);

        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{testutils::{Address as _, Ledger}, token, Address, Env};

    fn create_token_contract<'a>(env: &Env, admin: &Address) -> (token::Client<'a>, token::StellarAssetClient<'a>) {
        let contract_address = env.register_stellar_asset_contract(admin.clone());
        (
            token::Client::new(env, &contract_address),
            token::StellarAssetClient::new(env, &contract_address),
        )
    }

    #[test]
    fn test_emergency_withdraw_after_seven_days() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        token_admin.mint(&user, &1000);

        client.init_legacy(&admin);
        client.deposit(&user, &token_client.address, &500);

        assert_eq!(client.get_user_balance(&user), 500);

        env.ledger().with_mut(|li| {
            li.timestamp = li.timestamp + SEVEN_DAYS + 1;
        });

        client.emergency_withdraw(&user, &token_client.address);

        assert_eq!(client.get_user_balance(&user), 0);
        assert_eq!(token_client.balance(&user), 1000);
    }

    #[test]
    #[should_panic(expected = "Emergency withdrawal not available yet")]
    fn test_emergency_withdraw_before_seven_days() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        let (token_client, token_admin) = create_token_contract(&env, &admin);
        token_admin.mint(&user, &1000);

        client.init_legacy(&admin);
        client.deposit(&user, &token_client.address, &500);

        client.emergency_withdraw(&user, &token_client.address);
    }

    #[test]
    fn test_admin_action_updates_timestamp() {
        let env = Env::default();
        env.mock_all_auths();

        let admin = Address::generate(&env);
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        client.init_legacy(&admin);
        let initial_timestamp = client.get_last_active_timestamp();

        env.ledger().with_mut(|li| {
            li.timestamp = li.timestamp + 100;
        });

        client.admin_action();
        let updated_timestamp = client.get_last_active_timestamp();

        assert!(updated_timestamp > initial_timestamp);
    }

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
        let result = client.try_set_protocol_fee(&10_001, &treasury);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), Error::InvalidFeeConfig);
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
        client.set_protocol_fee(&50, &treasury);
        assert_eq!(client.fee_basis_points(), 50);
        assert_eq!(client.treasury_address(), Some(treasury));
    }

    #[test]
    fn kick_member_fails_if_not_found() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);
        
        let dummy_token = Address::generate(&env);
        let dummy_member = Address::generate(&env);
        
        env.mock_all_auths();
        let result = client.try_kick_member(&dummy_token, &dummy_member, &0);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().unwrap(), Error::MemberNotFound);
    }

    #[test]
    fn test_join_public_circle() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        client.initialize(&admin);

        let member = Address::generate(&env);
        env.mock_all_auths();

        client.join(&member, &None);

        let res = client.try_join(&member, &None);
        assert_eq!(res.unwrap_err().unwrap(), Error::AlreadyJoined);
    }

    #[test]
    fn test_join_private_circle_with_invite() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        env.mock_all_auths();
        client.initialize(&admin);

        client.set_privacy_config(&false, &Some(12345));

        let member = Address::generate(&env);
        client.join(&member, &Some(12345));
    }

    #[test]
    fn test_join_private_circle_with_admin_auth() {
        let env = Env::default();
        let (client, admin) = setup(&env);
        env.mock_all_auths(); 
        client.initialize(&admin);
        client.set_privacy_config(&false, &Some(12345));

        let member = Address::generate(&env);
        client.join(&member, &None);
    }
}
