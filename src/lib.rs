#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, contracterror, Address, Env, Symbol, String, symbol_short, token};

// --- ERROR CODES ---

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
    MemberNotFound = 2,
    CircleFull = 3,
    AlreadyMember = 4,
}

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleCount,
    Deposit(u64, Address),
    GroupReserve,
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub has_contributed: bool,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: i128,
    pub max_members: u32,
    pub member_count: u32,
    pub current_recipient_index: u32,
    pub is_active: bool,
    pub token: Address,
    pub deadline_timestamp: u64,
    pub cycle_duration: u64,
}

// --- EVENT STRUCTURES ---

/// Event emitted when a member is forcibly removed from a circle
/// Frontend should listen for "MEM_KICKED" events to update membership lists in real-time
/// Event payload includes: circle_id, member_address, and reason
#[contracttype]
#[derive(Clone, Debug)]
pub struct MemberKickedEvent {
    pub circle_id: u64,
    pub member_address: Address,
    pub reason: String,
}

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn create_circle(env: Env, creator: Address, amount: i128, max_members: u32, token: Address, cycle_duration: u64) -> u64;
    fn join_circle(env: Env, user: Address, circle_id: u64);
    fn deposit(env: Env, user: Address, circle_id: u64);
    fn kick_member(env: Env, admin: Address, member: Address, circle_id: u64, reason: String) -> Result<(), Error>;
}

// --- IMPLEMENTATION ---

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    fn create_circle(env: Env, creator: Address, amount: i128, max_members: u32, token: Address, cycle_duration: u64) -> u64 {
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        circle_count += 1;

        let current_time = env.ledger().timestamp();
        let new_circle = CircleInfo {
            id: circle_count,
            creator: creator.clone(),
            contribution_amount: amount,
            max_members,
            member_count: 0,
            current_recipient_index: 0,
            is_active: true,
            token,
            deadline_timestamp: current_time + cycle_duration,
            cycle_duration,
        };

        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        if !env.storage().instance().has(&DataKey::GroupReserve) {
            env.storage().instance().set(&DataKey::GroupReserve, &0i128);
        }

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("User is already a member");
        }

        let new_member = Member {
            address: user.clone(),
            has_contributed: false,
            contribution_count: 0,
            last_contribution_time: 0,
        };
        
        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        let client = token::Client::new(&env, &circle.token);

        let current_time = env.ledger().timestamp();
        let mut penalty_amount = 0i128;

        if current_time > circle.deadline_timestamp {
            penalty_amount = circle.contribution_amount / 100;
            
            let mut reserve_balance: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
        }

        client.transfer(
            &user, 
            &env.current_contract_address(), 
            &circle.contribution_amount
        );

        member.has_contributed = true;
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        
        env.storage().instance().set(&member_key, &member);

        circle.deadline_timestamp = current_time + circle.cycle_duration;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        env.storage().instance().set(&DataKey::Deposit(circle_id, user), &true);
    }

    fn kick_member(env: Env, admin: Address, member: Address, circle_id: u64, reason: String) -> Result<(), Error> {
        // 1. Authorization: Only admin can kick members
        admin.require_auth();

        // 2. Verify the caller is the admin
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .ok_or(Error::Unauthorized)?;
        
        if admin != stored_admin {
            return Err(Error::Unauthorized);
        }

        // 3. Check if member exists
        let member_key = DataKey::Member(member.clone());
        if !env.storage().instance().has(&member_key) {
            return Err(Error::MemberNotFound);
        }

        // 4. Remove the member from storage
        env.storage().instance().remove(&member_key);

        // 5. Update circle member count
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        if circle.member_count > 0 {
            circle.member_count -= 1;
        }
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // 6. Emit MemberKicked event
        // Frontend should listen for "MEM_KICKED" events to update membership lists
        // Event payload includes: circle_id, member_address, and reason
        let event = MemberKickedEvent {
            circle_id,
            member_address: member,
            reason,
        };
        
        env.events().publish((symbol_short!("MEM_KICK"), circle_id), event);

        Ok(())
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::{Events, Address as _}, Env, TryIntoVal};

    #[test]
    fn test_kick_member_emits_event() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let member = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);

        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);

        client.join_circle(&member, &circle_id);

        let reason = String::from_str(&env, "Violation of terms");
        let result = client.try_kick_member(&admin, &member, &circle_id, &reason);

        assert!(result.is_ok());

        // Verify event was emitted with correct payload
        let events = env.events().all();
        assert!(events.len() > 0);
        
        // Find the MemberKicked event
        let mut found_event = false;
        for event in events.iter() {
            // Event structure: (contract_address, topics_vec, data)
            let topics = &event.1;
            let data = &event.2;
            
            if topics.len() >= 2 {
                // Check if this is our MEM_KICK event
                if let Ok(symbol) = topics.get(0).unwrap().try_into_val(&env) {
                    let sym: Symbol = symbol;
                    if sym == symbol_short!("MEM_KICK") {
                        // Verify the event data contains the expected values
                        let event_data: MemberKickedEvent = data.clone().try_into_val(&env).unwrap();
                        assert_eq!(event_data.circle_id, circle_id);
                        assert_eq!(event_data.member_address, member);
                        assert_eq!(event_data.reason, reason);
                        found_event = true;
                        break;
                    }
                }
            }
        }
        assert!(found_event, "MemberKicked event was not found");
    }

    #[test]
    fn test_kick_member_with_reason() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let member = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);
        client.join_circle(&member, &circle_id);

        let reason = String::from_str(&env, "Missed 3 consecutive payments");
        let result = client.try_kick_member(&admin, &member, &circle_id, &reason);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kick_member_empty_reason() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let member = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);
        client.join_circle(&member, &circle_id);

        let reason = String::from_str(&env, "");
        let result = client.try_kick_member(&admin, &member, &circle_id, &reason);

        assert!(result.is_ok());
    }

    #[test]
    fn test_kick_member_unauthorized() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let non_admin = Address::generate(&env);
        let member = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);
        client.join_circle(&member, &circle_id);

        let reason = String::from_str(&env, "Unauthorized attempt");
        let result = client.try_kick_member(&non_admin, &member, &circle_id, &reason);

        assert_eq!(result, Err(Ok(Error::Unauthorized)));
    }

    #[test]
    fn test_kick_member_not_found() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let non_member = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);

        let reason = String::from_str(&env, "Not a member");
        let result = client.try_kick_member(&admin, &non_member, &circle_id, &reason);

        assert_eq!(result, Err(Ok(Error::MemberNotFound)));
    }

    #[test]
    fn test_kick_member_updates_member_count() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let member1 = Address::generate(&env);
        let member2 = Address::generate(&env);
        let token = Address::generate(&env);

        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);

        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&admin, &1000, &10, &token, &604800);

        client.join_circle(&member1, &circle_id);
        client.join_circle(&member2, &circle_id);

        let reason = String::from_str(&env, "Test removal");
        let result = client.try_kick_member(&admin, &member1, &circle_id, &reason);

        assert!(result.is_ok());
    }
}
