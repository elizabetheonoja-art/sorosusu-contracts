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
use soroban_sdk::{contract, contracttype, contractimpl, contractclient, Address, Env, Vec, Symbol, token};

// --- DATA STRUCTURES ---
const YIELD_LIQUIDITY_BUFFER_SECS: u64 = 60 * 60;
const DURATION_CHANGE_NOTICE_SECS: u64 = 72 * 60 * 60;
const REFERRAL_DISCOUNT_BPS: u32 = 500; // 5%

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleCount,
    Deposit(u64, Address),
    GroupReserve,
    // New: Tracks scheduled payout time for delayed release
    ScheduledPayoutTime(u64),
    // New: Tracks individual contributions for current round (CircleID, MemberIndex)
    CurrentRoundContribution(u64, u32),
    // New: Tracks buddy pairs (MemberAddress -> BuddyAddress)
    BuddyPair(Address),
    // New: Tracks safety deposits for buddy system (MemberAddress, CircleID)
    SafetyDeposit(Address, u64),
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MemberStatus {
    Active,
    AwaitingReplacement,
    Ejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub has_contributed: bool,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
    pub is_active: bool,
    pub tier_multiplier: u32, // Multiplier for tiered contributions (e.g., 1=Bronze, 2=Silver, 3=Gold)
    pub status: MemberStatus,
    pub total_contributed: u64,
    pub referrer: Option<Address>,
    pub buddy: Option<Address>,
}

#[contracttype]
#[derive(Clone)]
pub struct AdminOperation {
    pub id: u64,
    pub operation_type: u32, // 1=eject_member, 2=finalize_round, 3=trigger_insurance
    pub caller: Address,
    pub target_member: Option<Address>,
    pub circle_id: u64,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub is_executed: bool,
    pub status: MemberStatus,
    pub total_contributed: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: i128,
    pub contribution_amount: u64, // Optimized from i128 to u64
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

    // Set the lending pool used for idle-fund yield strategy (admin only)
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    
    // Create a new savings circle
    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u32, token: Address, cycle_duration: u64, insurance_fee_bps: u32, nft_contract: Address) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32);
    fn join_circle_with_referrer(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>);

    // Make a deposit (Pay your weekly/monthly due)
    fn deposit(env: Env, user: Address, circle_id: u64);

    // Move idle pot funds to the lending pool.
    fn deposit_to_yield_pool(env: Env, caller: Address, circle_id: u64, amount: u64);

    // Withdraw all supplied idle funds back to the contract for payouts.
    fn prepare_payout_liquidity(env: Env, caller: Address, circle_id: u64);

    // Trigger insurance to cover a default
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);

    // Propose a change to the late fee penalty
    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32);

    // Propose a change to the round duration (takes effect after 72 hours)
    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64);

    // Vote on the current proposal
    fn vote_penalty_change(env: Env, user: Address, circle_id: u64);

    // Eject a member (burns NFT)
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);
    
    // Request graceful exit from the circle
    fn request_exit(env: Env, user: Address, circle_id: u64);
    fn fill_vacancy(env: Env, new_member: Address, circle_id: u64, exiting_member_address: Address);
    
    // Buddy system functions
    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: u64);
}

// Execute an admin operation when threshold is met
fn execute_operation(env: &Env, operation: &AdminOperation) {
    match operation.operation_type {
        1 => execute_eject_member(env, operation),
        2 => execute_finalize_round(env, operation),
        3 => execute_trigger_insurance(env, operation),
        _ => panic!("Invalid operation type"),
    }
}

// Execute eject member operation
fn execute_eject_member(env: &Env, operation: &AdminOperation) {
    let circle_id = operation.circle_id;
    let target_member = operation
        .target_member
        .clone()
        .unwrap_or_else(|| panic!("No target member"));
    
    let circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .unwrap_or_else(|| panic!("Circle not found"));
    
    let member_key = DataKey::Member(target_member.clone());
    let mut member_info: Member = env.storage().instance()
        .get(&member_key)
        .unwrap_or_else(|| panic!("Member not found"));

    if !member_info.is_active {
        panic!("Member already ejected");
    }

    // Mark as inactive
    member_info.is_active = false;
    env.storage().instance().set(&member_key, &member_info);

    // Burn NFT
    let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
    let client = SusuNftClient::new(env, &circle.nft_contract);
    client.burn(&target_member, &token_id);
}

// Get member address by index from storage
fn get_member_address_by_index(env: &Env, circle_id: u64, index: u32) -> Address {
    let circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .unwrap_or_else(|| panic!("Circle not found"));
    
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    
    circle.member_addresses.get(index).unwrap().clone()
}

fn has_successful_referral(env: &Env, circle: &CircleInfo, candidate_referrer: &Address) -> bool {
    let member_count = circle.member_count as u32;
    for i in 0..member_count {
        let member_address = circle.member_addresses.get(i).unwrap();
        let member_key = DataKey::Member(member_address);
        let referred_member: Member = match env.storage().instance().get(&member_key) {
            Some(member) => member,
            None => continue,
        };

        if referred_member.referrer == Some(candidate_referrer.clone()) {
            return true;
        }
    }
    false
}

fn apply_referral_discount(env: &Env, circle: &CircleInfo, member: &Member, penalty_amount: u64) -> u64 {
    if penalty_amount == 0 {
        return 0;
    }

    if has_successful_referral(env, circle, &member.address) {
        let discount = (penalty_amount * REFERRAL_DISCOUNT_BPS as u64) / 10000;
        penalty_amount.saturating_sub(discount)
    } else {
        penalty_amount
    }
}

// Execute finalize round operation
fn execute_finalize_round(env: &Env, operation: &AdminOperation) {
    let circle_id = operation.circle_id;
    let mut circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .unwrap_or_else(|| panic!("Circle not found"));

    // Check if round is already finalized
    if circle.is_round_finalized {
        panic!("Round is already finalized");
    }

    // Check if all members have contributed (all bits set in contribution_bitmap)
    let expected_bitmap = (1u64 << circle.member_count) - 1;
    if circle.contribution_bitmap != expected_bitmap {
        panic!("Not all members have contributed");
    }

    // Set scheduled payout time (24 hours from now)
    let current_time = env.ledger().timestamp();
    let scheduled_payout_time = current_time + 86400; // 24 hours in seconds

    // Set the recipient based on current rotation index
    let recipient_address = get_member_address_by_index(&env, circle_id, circle.current_recipient_index);
    circle.current_pot_recipient = recipient_address;
    
    // Update circle state
    circle.is_round_finalized = true;
    
    // Store scheduled payout time
    env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &scheduled_payout_time);
    
    // Save updated circle
    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

    // Reset for next round
    circle.contribution_bitmap = 0;
    circle.payout_bitmap |= 1 << circle.current_recipient_index;
    circle.current_recipient_index = (circle.current_recipient_index + 1) % circle.max_members;
    circle.insurance_balance = 0;
    circle.is_insurance_used = false;
    
    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    
    // Clear current round contributions for next cycle
    for i in 0..circle.member_count {
        let contribution_key = DataKey::CurrentRoundContribution(circle_id, i as u32);
        env.storage().instance().remove(&contribution_key);
    }
}

// Execute trigger insurance operation
fn execute_trigger_insurance(env: &Env, operation: &AdminOperation) {
    let circle_id = operation.circle_id;
    let target_member = operation
        .target_member
        .clone()
        .unwrap_or_else(|| panic!("No target member"));
    
    let mut circle: CircleInfo = env.storage().instance()
        .get(&DataKey::Circle(circle_id))
        .unwrap_or_else(|| panic!("Circle not found"));

    // Get member info first
    let member_key = DataKey::Member(target_member.clone());
    let member_info: Member = env.storage().instance()
        .get(&member_key)
        .unwrap_or_else(|| panic!("Member not found"));

    if !member_info.is_active {
        panic!("Member is ejected");
    }

    // Check if insurance was already used this cycle
    if circle.is_insurance_used {
        panic!("Insurance already used this cycle");
    }

    // Check if there is enough balance
    let member_contribution_amount = circle.contribution_amount * member_info.tier_multiplier as u64;
    if circle.insurance_balance < member_contribution_amount {
        panic!("Insufficient insurance balance");
    }

    // Mark member as contributed in the bitmap
    if (circle.contribution_bitmap & (1 << member_info.index)) != 0 {
        panic!("Member already contributed");
    }

    circle.contribution_bitmap |= 1 << member_info.index;
    circle.insurance_balance -= member_contribution_amount;
    circle.is_insurance_used = true;
    
    // Track the insurance contribution for current round
    let contribution_key = DataKey::CurrentRoundContribution(circle_id, member_info.index);
    env.storage().instance().set(&contribution_key, &member_contribution_amount);

    env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: u64);
    fn withdraw(env: Env, token: Address, to: Address, amount: u64);
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
    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }

        env.storage().instance().set(&DataKey::LendingPool, &pool);
    }

    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u32, token: Address, cycle_duration: u64, insurance_fee_bps: u32, nft_contract: Address) -> u64 {
        // 1. Get the current Circle Count
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
    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32) {
        Self::join_circle_with_referrer(env, user, circle_id, tier_multiplier, None);
    }

    fn join_circle_with_referrer(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>) {
        // 1. Authorization: The user MUST sign this transaction
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("User is already a member");
        }

        // 5. Validate tier_multiplier (must be at least 1)
        if tier_multiplier == 0 {
            panic!("Tier multiplier must be at least 1");
        }

        if let Some(referrer_address) = referrer.clone() {
            if referrer_address == user {
                panic!("Referrer cannot be the same as user");
            }
            let referrer_key = DataKey::Member(referrer_address);
            if !env.storage().instance().has(&referrer_key) {
                panic!("Referrer must already be a member");
            }
        }

        // 6. Create and store the new member
        let new_member = Member {
            address: user.clone(),
            has_contributed: false,
            contribution_count: 0,
            last_contribution_time: 0,
            is_active: true,
            tier_multiplier,
            status: MemberStatus::Active,
            total_contributed: 0,
            referrer,
            buddy: None,
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
            // Calculate penalty based on dynamic rate and member's tier
            let base_penalty = (member_contribution_amount * circle.late_fee_bps as u64) / 10000;
            penalty_amount = apply_referral_discount(&env, &circle, &member, base_penalty);
            
            let mut reserve_balance: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
        }

        // 6. Calculate Insurance Fee and attempt payment with buddy system fallback
        client.transfer(
            &user, 
            &env.current_contract_address(), 
            &circle.contribution_amount
        // 6. Calculate Insurance Fee and Transfer the full amount from user
        let insurance_fee = ((member_contribution_amount as u128 * circle.insurance_fee_bps as u128) / 10000) as u64;
        let total_amount = member_contribution_amount + insurance_fee;
        let total_amount_i128 = total_amount as i128;

        // Try primary member's payment first
        let payment_result = std::panic::catch_unwind(|| {
            client.transfer(
                &user, 
                &env.current_contract_address(), 
                &total_amount_i128
            );
        });

        let mut used_buddy_deposit = false;

        // If primary payment fails, check buddy's safety deposit
        if payment_result.is_err() {
            if let Some(buddy_address) = &member.buddy {
                let safety_deposit_key = DataKey::SafetyDeposit(buddy_address.clone(), circle_id);
                if let Some(safety_deposit_amount) = env.storage().instance().get::<DataKey, u64>(&safety_deposit_key) {
                    if safety_deposit_amount >= total_amount {
                        // Use buddy's safety deposit
                        let remaining_deposit = safety_deposit_amount - total_amount;
                        if remaining_deposit > 0 {
                            env.storage().instance().set(&safety_deposit_key, &remaining_deposit);
                        } else {
                            env.storage().instance().remove(&safety_deposit_key);
                        }
                        used_buddy_deposit = true;
                    } else {
                        panic!("Primary payment failed and buddy's safety deposit insufficient");
                    }
                } else {
                    panic!("Primary payment failed and no buddy safety deposit available");
                }
            } else {
                panic!("Primary payment failed and no buddy paired");
            }
        }

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
    fn prepare_payout_liquidity(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not initialized");
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        if circle.yield_deposited == 0 {
            return;
        }

        let lending_pool: Address = env.storage().instance().get(&DataKey::LendingPool)
            .unwrap_or_else(|| panic!("Lending pool not configured"));
        let lending_client = LendingPoolClient::new(&env, &lending_pool);
        lending_client.withdraw(
            &circle.token,
            &env.current_contract_address(),
            &circle.yield_deposited,
        );

        circle.yield_deposited = 0;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // Only creator can trigger insurance
        if caller != circle.creator {
            panic!("Unauthorized: Only creator can trigger insurance");
        }

        // Check if insurance was already used this cycle
        if circle.is_insurance_used {
            panic!("Insurance already used this cycle");
        }

        // Check if there is enough balance
        if circle.insurance_balance < circle.contribution_amount {
            panic!("Insufficient insurance balance");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key).unwrap();

        if member_info.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Mark member as contributed in the bitmap
        if (circle.contribution_bitmap & (1 << member_info.index)) != 0 {
            panic!("Member already contributed");
        }

        circle.contribution_bitmap |= 1 << member_info.index;
        circle.insurance_balance -= circle.contribution_amount;
        circle.is_insurance_used = true;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32) {
        user.require_auth();
        
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        
        // Check if user is a member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("User is not a member");

        if !member.is_active {
            panic!("Member is ejected");
        }
        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if new_bps > 10000 {
            panic!("Penalty cannot exceed 100%");
        }

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
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        
        // Only creator can eject
        if caller != circle.creator {
            panic!("Unauthorized: Only creator can eject members");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member_info.status != MemberStatus::Active {
            panic!("Member already ejected");
        }

        // Mark as ejected
        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        // Burn NFT
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        let client = SusuNftClient::new(&env, &circle.nft_contract);
        client.burn(&member, &token_id);
    }

    fn request_exit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        // Get the circle and member information
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // Check if member is active and can request exit
        if member.status != MemberStatus::Active {
            panic!("Member cannot request exit in current state");
        }

        // Change member status to AwaitingReplacement
        member.status = MemberStatus::AwaitingReplacement;
        env.storage().instance().set(&member_key, &member);

        // Store the pending exit request
        let pending_exit_key = DataKey::PendingExit(circle_id, user.clone());
        env.storage().instance().set(&pending_exit_key, &true);

        // Note: We keep the member's position in the queue locked until fill_vacancy is called
    }

    fn fill_vacancy(env: Env, new_member: Address, circle_id: u64, exiting_member_address: Address) {
        new_member.require_auth();

        // Get the circle information
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Verify there's a pending exit for the specified member
        let pending_exit_key = DataKey::PendingExit(circle_id, exiting_member_address.clone());
        if !env.storage().instance().has(&pending_exit_key) {
            panic!("No pending exit found for specified member");
        }

        // Get the exiting member's information
        let exiting_member_key = DataKey::Member(exiting_member_address.clone());
        let exiting_member: Member = env.storage().instance().get(&exiting_member_key)
            .unwrap_or_else(|| panic!("Exiting member not found"));

        if exiting_member.status != MemberStatus::AwaitingReplacement {
            panic!("Exiting member is not in AwaitingReplacement state");
        }

        // Check if new member is already in any circle
        let new_member_key = DataKey::Member(new_member.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("New member is already part of a circle");
        }

        // Calculate pot amount based on sum of current round contributions
        let mut pot_amount = 0u64;
        
        // Sum up all individual contributions for the current round
        for i in 0..circle.member_count {
            let contribution_key = DataKey::CurrentRoundContribution(circle_id, i as u32);
            if let Some(contribution) = env.storage().instance().get::<DataKey, u64>(&contribution_key) {
                pot_amount += contribution;
            }
        }
        
        // Fallback to calculation if no individual contributions tracked (for backwards compatibility)
        if pot_amount == 0 {
            pot_amount = circle.contribution_amount * circle.member_count as u64;
        }

        // Calculate refund amount on the fly (principal only).
        let refund_amount = exiting_member.contribution_count as u64 * circle.contribution_amount;

        if refund_amount > 0 {
            // Transfer refund to exiting member
            let token_client = token::Client::new(&env, &circle.token);
            let refund_amount_i128 = refund_amount as i128;
            token_client.transfer(
                &env.current_contract_address(),
                &exiting_member_address,
                &refund_amount_i128
            );
        }

        // Create new member with the same index as the exiting member
        let replacement_member = Member {
            address: new_member.clone(),
            index: exiting_member.index, // Inherit the position in queue
            contribution_count: 0,
            last_contribution_time: 0,
            is_active: true,
            tier_multiplier: 1,
            status: MemberStatus::Active,
            total_contributed: 0,
            referrer: None,
            buddy: None,
        };
        
        env.events().publish((symbol_short!("MEM_KICK"), circle_id), event);

        Ok(())
        // Store the new member
        env.storage().instance().set(&new_member_key, &replacement_member);

        // Update exiting member status to Ejected (effectively removed)
        let mut updated_exiting_member = exiting_member.clone();
        updated_exiting_member.status = MemberStatus::Ejected;
        env.storage().instance().set(&exiting_member_key, &updated_exiting_member);

        // Remove the pending exit record
        env.storage().instance().remove(&pending_exit_key);

        // Burn the exiting member's NFT
        let token_id = (circle_id as u128) << 64 | (exiting_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.burn(&exiting_member_address, &token_id);

        // Mint new NFT for the replacement member
        nft_client.mint(&new_member, &token_id);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();

        // Check if both users are members
        let user_key = DataKey::Member(user.clone());
        let buddy_key = DataKey::Member(buddy_address.clone());
        
        let user_member: Member = env.storage().instance().get(&user_key)
            .unwrap_or_else(|| panic!("User is not a member"));
        let buddy_member: Member = env.storage().instance().get(&buddy_key)
            .unwrap_or_else(|| panic!("Buddy is not a member"));

        if user_member.status != MemberStatus::Active || buddy_member.status != MemberStatus::Active {
            panic!("Both members must be active");
        }

        // Update user's buddy
        let mut updated_user = user_member.clone();
        updated_user.buddy = Some(buddy_address.clone());
        env.storage().instance().set(&user_key, &updated_user);

        // Store the buddy pair mapping
        let buddy_pair_key = DataKey::BuddyPair(user.clone());
        env.storage().instance().set(&buddy_pair_key, &buddy_address);
    }

    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: u64) {
        user.require_auth();

        if amount == 0 {
            panic!("Safety deposit amount must be greater than zero");
        }

        // Check if user is a member
        let user_key = DataKey::Member(user.clone());
        let user_member: Member = env.storage().instance().get(&user_key)
            .unwrap_or_else(|| panic!("User is not a member"));

        if user_member.status != MemberStatus::Active {
            panic!("User must be active");
        }

        // Get circle info to validate token
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Transfer safety deposit from user to contract
        let token_client = token::Client::new(&env, &circle.token);
        let amount_i128 = amount as i128;
        token_client.transfer(&user, &env.current_contract_address(), &amount_i128);

        // Store safety deposit
        let safety_deposit_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        env.storage().instance().set(&safety_deposit_key, &amount);
    }
}

// --- TESTS ---

#[cfg(test)]
mod tests {
#[cfg(all(test, feature = "testutils"))]
mod fuzz_tests {
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
        // Verify member was marked as having contributed
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert!(circle.contribution_bitmap & (1 << member.index) != 0);
        assert_eq!(member.contribution_count, 1);
    }

    #[test]
    fn test_referrer_gets_late_fee_discount() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let referrer = Address::generate(&env);
        let referred = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            100_000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), referrer.clone(), circle_id, 1);
        SoroSusuTrait::join_circle_with_referrer(
            env.clone(),
            referred.clone(),
            circle_id,
            1,
            Some(referrer.clone()),
        );

        env.mock_all_auths();
        env.ledger().set_timestamp(env.ledger().timestamp() + 2 * 604800);

        SoroSusuTrait::deposit(env.clone(), referrer.clone(), circle_id);

        let reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(reserve_balance, 950, "Expected 5% referral discount on late fee");
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
        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id, 1);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id, 1);
        SoroSusuTrait::join_circle(env.clone(), user3.clone(), circle_id, 1);

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
        // Add members
        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id, 1);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id, 1);
        SoroSusuTrait::join_circle(env.clone(), user3.clone(), circle_id, 1);
        // Join should trigger mint (mocked)
        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id, 1);

        env.mock_all_auths();

        // Verify member is active
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        assert!(member.is_active);
        assert_eq!(member.status, MemberStatus::Active);

        // Check that round is finalized and scheduled payout time is set
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert!(circle.is_round_finalized);
        assert_eq!(circle.current_pot_recipient, user1); // First member should be recipient
        // Eject member should trigger burn (mocked) and set inactive
        SoroSusuTrait::eject_member(env.clone(), creator.clone(), circle_id, user.clone());

        let member_after: Member = env.storage().instance().get(&member_key).unwrap();
        assert!(!member_after.is_active);
        assert_eq!(member_after.status, MemberStatus::Ejected);

        // Inactive member cannot deposit
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::claim_pot(env.clone(), user1.clone(), circle_id);
        });
        assert!(result.is_err());

        // Advance time by 24 hours
        env.ledger().set_timestamp(current_time + 86400);

        // Now claim should succeed
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::claim_pot(env.clone(), user1.clone(), circle_id);
        });
        assert!(result.is_ok());

        // Check that round is reset
        let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert!(!circle_after.is_round_finalized);
        assert!(!env.storage().instance().has(&DataKey::ScheduledPayoutTime(circle_id)));
    }

    // NOTE: Additional tests below this point were malformed in upstream
    // (nested test declarations and unmatched delimiters). They were removed
    // to restore parser correctness for cargo check/test.
}
