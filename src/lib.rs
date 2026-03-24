#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, token,
    Address, Env, Symbol, Vec,
};

// --- ERROR CODES ---

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    Unauthorized = 1,
    MemberNotFound = 2,
    CircleFull = 3,
    AlreadyMember = 4,
    CircleNotFound = 5,
    InvalidAmount = 6,
    RoundAlreadyFinalized = 7,
    RoundNotFinalized = 8,
    NotAllContributed = 9,
    PayoutNotScheduled = 10,
    PayoutTooEarly = 11,
    InsufficientInsurance = 12,
    InsuranceAlreadyUsed = 13,
    RateLimitExceeded = 14,
    InsufficientCollateral = 15,
    CollateralAlreadyStaked = 16,
    CollateralNotStaked = 17,
    CollateralLocked = 18,
    MemberNotDefaulted = 19,
    CollateralAlreadyReleased = 20,
}

// --- CONSTANTS ---
const REFERRAL_DISCOUNT_BPS: u32 = 500; // 5%
const RATE_LIMIT_SECONDS: u64 = 300; // 5 minutes
const DEFAULT_COLLATERAL_BPS: u32 = 2000; // 20%
const HIGH_VALUE_THRESHOLD: i128 = 10_000_000_000; // 1000 XLM (assuming 7 decimals)

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
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    LendingPool,
    CollateralVault(Address, u64),
    CollateralConfig(u64),
    DefaultedMembers(u64),
    MemberAtIndex(u64, u32),
    Reputation(Address),
    BadgeContract,
}

#[contracttype]
#[derive(Clone, Debug)]
pub struct Reputation {
    pub cycles_completed: u32,
    pub total_contributions: u32,
    pub on_time_contributions: u32,
    pub total_volume: i128,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MemberStatus {
    Active,
    AwaitingReplacement,
    Ejected,
    Defaulted,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum CollateralStatus {
    NotStaked,
    Staked,
    Slashed,
    Released,
}

#[contracttype]
#[derive(Clone)]
pub struct CollateralInfo {
    pub member: Address,
    pub circle_id: u64,
    pub amount: i128,
    pub status: CollateralStatus,
    pub staked_timestamp: u64,
    pub release_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub index: u32,
    pub contribution_count: u32,
    pub on_time_count: u32,
    pub last_contribution_time: u64,
    pub status: MemberStatus,
    pub tier_multiplier: u32,
    pub referrer: Option<Address>,
    pub buddy: Option<Address>,
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
    pub contribution_bitmap: u64,
    pub insurance_balance: i128,
    pub insurance_fee_bps: u32,
    pub is_insurance_used: bool,
    pub late_fee_bps: u32,
    pub nft_contract: Address,
    pub is_round_finalized: bool,
    pub current_pot_recipient: Option<Address>,
    pub requires_collateral: bool,
    pub collateral_bps: u32,
    pub total_cycle_value: i128,
    pub min_reputation: u32,
}

pub mod external_clients {
    use super::*;

    #[contractclient(name = "SusuNftClient")]
    pub trait SusuNftTrait {
        fn mint(env: Env, to: Address, token_id: u128);
        fn burn(env: Env, from: Address, token_id: u128);
    }

    #[contractclient(name = "LendingPoolClient")]
    pub trait LendingPoolTrait {
        fn supply(env: Env, token: Address, from: Address, amount: i128);
        fn withdraw(env: Env, token: Address, to: Address, amount: i128);
    }

    #[contractclient(name = "BadgeClient")]
    pub trait BadgeTrait {
        fn mint(env: Env, to: Address, traits: Vec<Symbol>);
    }
}

pub use external_clients::{SusuNftClient, SusuNftTrait, LendingPoolClient, LendingPoolTrait, BadgeClient, BadgeTrait};

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    fn set_badge_contract(env: Env, admin: Address, badge: Address);
    
    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        min_reputation: u32,
    ) -> u64;

    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>);
    fn deposit(env: Env, user: Address, circle_id: u64);
    
    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);
    
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);
    
    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);
    
    // Collateral functions
    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128);
    fn slash_collateral(env: Env, caller: Address, circle_id: u64, member: Address);
    fn release_collateral(env: Env, caller: Address, circle_id: u64, member: Address);
    fn mark_member_defaulted(env: Env, caller: Address, circle_id: u64, member: Address);

    // Oracle function
    fn get_reliability_score(env: Env, user: Address) -> u32;
}

// --- IMPLEMENTATION ---

#[contract]
pub struct SoroSusu;

impl SoroSusu {
    // Internal helper for slashing collateral
    fn _slash_collateral(env: Env, caller: Address, circle_id: u64, member: Address) {
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        let collateral_key = DataKey::CollateralVault(member.clone(), circle_id);
        let mut collateral_info: CollateralInfo = env.storage().instance().get(&collateral_key)
            .expect("Collateral not staked");

        if collateral_info.status != CollateralStatus::Staked {
            panic!("Collateral not available for slashing");
        }

        // Check if member is defaulted
        let defaulted_key = DataKey::DefaultedMembers(circle_id);
        let defaulted_members: Vec<Address> = env.storage().instance().get(&defaulted_key).unwrap_or(Vec::new(&env));
        
        if !defaulted_members.contains(&member) {
            panic!("Member not defaulted");
        }

        // Slash the collateral - distribute to remaining active members
        
        let slash_amount = collateral_info.amount;
        
        // Transfer to group reserve for distribution
        let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        reserve += slash_amount;
        env.storage().instance().set(&DataKey::GroupReserve, &reserve);

        // Notify badge contract if necessary (optional future task)
        // ...
        
        // Update collateral status
        collateral_info.status = CollateralStatus::Slashed;
        env.storage().instance().set(&collateral_key, &collateral_info);
    }
}

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::LendingPool, &pool);
    }

    fn set_badge_contract(env: Env, admin: Address, badge: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::BadgeContract, &badge);
    }

    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        min_reputation: u32,
    ) -> u64 {
        creator.require_auth();

        // Rate limiting
        let current_time = env.ledger().timestamp();
        let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());
        if let Some(last_created) = env.storage().instance().get::<DataKey, u64>(&rate_limit_key) {
            if current_time < last_created + RATE_LIMIT_SECONDS {
                panic!("Rate limit exceeded");
            }
        }
        env.storage().instance().set(&rate_limit_key, &current_time);

        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        circle_count += 1;

        // Calculate total cycle value and determine collateral requirements
        let total_cycle_value = amount * (max_members as i128);
        let requires_collateral = total_cycle_value >= HIGH_VALUE_THRESHOLD;
        let collateral_bps = if requires_collateral { DEFAULT_COLLATERAL_BPS } else { 0 };

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
            contribution_bitmap: 0,
            insurance_balance: 0,
            insurance_fee_bps,
            is_insurance_used: false,
            late_fee_bps: 100, // 1%
            nft_contract,
            is_round_finalized: false,
            current_pot_recipient: None,
            requires_collateral,
            collateral_bps,
            total_cycle_value,
            min_reputation,
        };

        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        // Reputation Gate check
        let user_score = Self::get_reliability_score(env.clone(), user.clone());
        if user_score < circle.min_reputation {
            panic!("Insufficient reliability score");
        }

        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("Already member");
        }

        // Check collateral requirement for high-value circles
        if circle.requires_collateral {
            let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
            let collateral_info: Option<CollateralInfo> = env.storage().instance().get(&collateral_key);
            
            match collateral_info {
                Some(collateral) => {
                    if collateral.status != CollateralStatus::Staked {
                        panic!("Collateral not properly staked");
                    }
                }
                None => panic!("Collateral required for this circle"),
            }
        }

        // Store member by index for the circle
        env.storage().instance().set(&DataKey::MemberAtIndex(circle_id, circle.member_count), &user);

        let new_member = Member {
            address: user.clone(),
            index: circle.member_count,
            contribution_count: 0,
            on_time_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
            tier_multiplier,
            referrer,
            buddy: None,
        };

        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Mint NFT
        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;
        let mut penalty_amount = 0i128;

        if current_time > circle.deadline_timestamp {
            let base_penalty = (base_amount * circle.late_fee_bps as i128) / 10000;
            // Apply referral discount
            let mut discount = 0i128;
            if let Some(ref_addr) = &member.referrer {
                let ref_key = DataKey::Member(ref_addr.clone());
                if env.storage().instance().has(&ref_key) {
                    discount = (base_penalty * REFERRAL_DISCOUNT_BPS as i128) / 10000;
                }
            }
            penalty_amount = base_penalty - discount;
            
            let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve);
        }

        let insurance_fee = (base_amount * circle.insurance_fee_bps as i128) / 10000;
        let total_amount = base_amount + insurance_fee + penalty_amount;

        let token_client = token::Client::new(&env, &circle.token);

        // Try transfer from user
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &total_amount);
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            // Buddy fallback
            if let Some(buddy_addr) = &member.buddy {
                let safety_key = DataKey::SafetyDeposit(buddy_addr.clone(), circle_id);
                let safety_balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
                if safety_balance >= total_amount {
                    env.storage().instance().set(&safety_key, &(safety_balance - total_amount));
                } else {
                    panic!("Insufficient funds and buddy deposit");
                }
            } else {
                panic!("Insufficient funds");
            }
        }

        if insurance_fee > 0 {
            circle.insurance_balance += insurance_fee;
        }

        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1 << member.index;
        
        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Update Reputation
        let rep_key = DataKey::Reputation(user.clone());
        let mut rep: Reputation = env.storage().instance().get(&rep_key).unwrap_or(Reputation {
            cycles_completed: 0,
            total_contributions: 0,
            on_time_contributions: 0,
            total_volume: 0,
        });

        rep.total_contributions += 1;
        rep.total_volume += base_amount;
        if current_time <= circle.deadline_timestamp {
            rep.on_time_contributions += 1;
            member.on_time_count += 1;
        }
        env.storage().instance().set(&rep_key, &rep);
        env.storage().instance().set(&member_key, &member);
    }

    fn finalize_round(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        if circle.is_round_finalized {
            panic!("Round already finalized");
        }

        let expected_bitmap = (1u64 << circle.member_count) - 1;
        if circle.contribution_bitmap != expected_bitmap {
            panic!("Not all contributed");
        }

        let mut circle = circle;
        circle.is_round_finalized = true;
        
        // Set recipient for this round
        let recipient: Address = env.storage().instance().get(&DataKey::MemberAtIndex(circle_id, circle.current_recipient_index)).expect("Recipient not found");
        circle.current_pot_recipient = Some(recipient);

        // Schedule payout (for simplicity, now)
        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &env.ledger().timestamp());
        
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if !circle.is_round_finalized {
            panic!("Round not finalized");
        }

        if let Some(recipient) = &circle.current_pot_recipient {
            if user != *recipient {
                panic!("Unauthorized recipient");
            }
        } else {
            panic!("No recipient set");
        }

        let scheduled_time: u64 = env.storage().instance().get(&DataKey::ScheduledPayoutTime(circle_id)).expect("Payout not scheduled");
        if env.ledger().timestamp() < scheduled_time {
            panic!("Payout too early");
        }

        let pot_amount = circle.contribution_amount * (circle.member_count as i128);
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &user, &pot_amount);

        // Auto-release collateral if member has completed all contributions
        if circle.requires_collateral {
            let member_key = DataKey::Member(user.clone());
            if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
                if member_info.contribution_count >= circle.max_members {
                    let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
                    if let Some(mut collateral_info) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
                        if collateral_info.status == CollateralStatus::Staked {
                            // Release collateral back to member
                            token_client.transfer(&env.current_contract_address(), &user, &collateral_info.amount);
                            
                            // Update collateral status
                            collateral_info.status = CollateralStatus::Released;
                            collateral_info.release_timestamp = Some(env.ledger().timestamp());
                            env.storage().instance().set(&collateral_key, &collateral_info);
                        }
                    }
                }
            }
        }

        // Update Reputation: Increment cycles completed if member has finished their course
        let member_key = DataKey::Member(user.clone());
        if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            if member_info.contribution_count >= circle.max_members {
                let rep_key = DataKey::Reputation(user.clone());
                if let Some(mut rep) = env.storage().instance().get::<DataKey, Reputation>(&rep_key) {
                    // Only increment once per cycle.
                    rep.cycles_completed += 1;
                    env.storage().instance().set(&rep_key, &rep);

                    // Trigger NFT Badge if they finish a 12-month cycle with zero defaults
                    let cycle_duration_total = (circle.max_members as u64) * circle.cycle_duration;
                    let one_year_seconds: u64 = 12 * 30 * 86400; // 360 days (approx "12-month cycle")

                    if cycle_duration_total >= one_year_seconds && member_info.status == MemberStatus::Active {
                        if let Some(badge_contract) = env.storage().instance().get::<DataKey, Address>(&DataKey::BadgeContract) {
                            let badge_client = BadgeClient::new(&env, &badge_contract);
                            let mut traits: Vec<Symbol> = Vec::new(&env);
                            
                            // Volume Tier
                            if circle.total_cycle_value >= 100_000_000_0 { // 1000 units
                                traits.push_back(Symbol::new(&env, "Volume_High"));
                            } else if circle.total_cycle_value >= 20_000_000_0 { // 200 units
                                traits.push_back(Symbol::new(&env, "Volume_Med"));
                            } else {
                                traits.push_back(Symbol::new(&env, "Volume_Low"));
                            }
                            
                            // Perfect Attendance
                            if member_info.on_time_count == circle.max_members {
                                traits.push_back(Symbol::new(&env, "PerfectAttendance"));
                            }
                            
                            // Group Lead
                            if user == circle.creator {
                                traits.push_back(Symbol::new(&env, "GroupLead"));
                            }
                            
                            badge_client.mint(&user, &traits);
                        }
                    }
                }
            }
        }

        // Reset for next round
        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;
        circle.current_recipient_index = (circle.current_recipient_index + 1) % circle.member_count;
        circle.current_pot_recipient = None; // Should be set in finalize_round

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        env.storage().instance().remove(&DataKey::ScheduledPayoutTime(circle_id));
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        if circle.is_insurance_used {
            panic!("Insurance already used");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        
        let amount_needed = circle.contribution_amount * member_info.tier_multiplier as i128;
        if circle.insurance_balance < amount_needed {
            panic!("Insufficient insurance");
        }

        circle.contribution_bitmap |= 1 << member_info.index;
        circle.insurance_balance -= amount_needed;
        circle.is_insurance_used = true;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        
        if member_info.status == MemberStatus::Ejected {
            panic!("Already ejected");
        }

        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        nft_client.burn(&member, &token_id);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();
        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env.storage().instance().get(&user_key).expect("Member not found");
        
        user_info.buddy = Some(buddy_address);
        env.storage().instance().set(&user_key, &user_info);
    }

    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let safety_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        let mut balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&safety_key, &balance);
    }

    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if !circle.requires_collateral {
            panic!("Collateral not required for this circle");
        }

        let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
        
        // Check if collateral already staked
        if let Some(_collateral) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
            panic!("Collateral already staked");
        }

        // Calculate required collateral amount
        let required_collateral = (circle.total_cycle_value * circle.collateral_bps as i128) / 10000;
        
        if amount < required_collateral {
            panic!("Insufficient collateral amount");
        }

        // Transfer collateral to contract
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        // Create collateral record
        let collateral_info = CollateralInfo {
            member: user.clone(),
            circle_id,
            amount,
            status: CollateralStatus::Staked,
            staked_timestamp: env.ledger().timestamp(),
            release_timestamp: None,
        };

        env.storage().instance().set(&collateral_key, &collateral_info);
    }

    fn slash_collateral(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        Self::_slash_collateral(env, caller, circle_id, member);
    }

    fn release_collateral(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        
        if caller != circle.creator && caller != stored_admin && caller != member {
            panic!("Unauthorized");
        }

        let collateral_key = DataKey::CollateralVault(member.clone(), circle_id);
        let mut collateral_info: CollateralInfo = env.storage().instance().get(&collateral_key)
            .expect("Collateral not staked");

        if collateral_info.status != CollateralStatus::Staked {
            panic!("Collateral not available for release");
        }

        // Check if member has completed all contributions
        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        
        if member_info.contribution_count < circle.max_members {
            panic!("Member has not completed all contributions");
        }

        // Release collateral back to member
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &member, &collateral_info.amount);

        // Update collateral status
        collateral_info.status = CollateralStatus::Released;
        collateral_info.release_timestamp = Some(env.ledger().timestamp());
        env.storage().instance().set(&collateral_key, &collateral_info);
    }

    fn mark_member_defaulted(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        
        if member_info.status == MemberStatus::Defaulted {
            panic!("Member already defaulted");
        }

        // Mark member as defaulted
        member_info.status = MemberStatus::Defaulted;
        env.storage().instance().set(&member_key, &member_info);

        // Add to defaulted members list
        let defaulted_key = DataKey::DefaultedMembers(circle_id);
        let mut defaulted_members: Vec<Address> = env.storage().instance().get(&defaulted_key).unwrap_or(Vec::new(&env));
        
        if !defaulted_members.contains(&member) {
            defaulted_members.push_back(member.clone());
            env.storage().instance().set(&defaulted_key, &defaulted_members);
        }

        // Auto-slash collateral if staked
        let collateral_key = DataKey::CollateralVault(member.clone(), circle_id);
        if let Some(_collateral) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
            // Reuse slash_collateral logic
            Self::_slash_collateral(env, caller, circle_id, member);
        }
    }

    fn get_reliability_score(env: Env, user: Address) -> u32 {
        let rep_key = DataKey::Reputation(user.clone());
        let rep: Reputation = env.storage().instance().get(&rep_key).unwrap_or(Reputation {
            cycles_completed: 0,
            total_contributions: 0,
            on_time_contributions: 0,
            total_volume: 0,
        });

        if rep.total_contributions == 0 {
            return 0;
        }

        // Weights:
        // 40% - On-time contribution ratio
        // 30% - Total cycles completed (capped at 10 cycles for max points)
        // 30% - Total volume rotated (capped at 10,000 units for max points)

        let on_time_ratio = (rep.on_time_contributions * 400) / rep.total_contributions;
        
        let cycles_score = if rep.cycles_completed >= 10 {
            300
        } else {
            rep.cycles_completed * 30
        };

        // Assuming 7 decimals for volume normalization (e.g. 1000 XLM = 10,000,000,0 units)
        // Max volume points at 10,000 XLM
        let normalized_volume = (rep.total_volume / 1_000_000_0) as u32;
        let volume_score = if normalized_volume >= 10000 {
            300
        } else {
            (normalized_volume * 300) / 10000
        };

        on_time_ratio + cycles_score + volume_score
    }
}
