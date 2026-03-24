#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token, Address, Env, Symbol,
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
    DisputeAlreadyExists = 15,
    DisputeNotFound = 16,
    DisputeAlreadyResolved = 17,
    InvalidFeeConfig = 18,
}

// --- CONSTANTS ---
const REFERRAL_DISCOUNT_BPS: u32 = 500; // 5%
const RATE_LIMIT_SECONDS: u64 = 300; // 5 minutes

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleMember(u64, u32),
    CircleCount,
    Deposit(u64, Address),
    GroupReserve,
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    LendingPool,
    Dispute(u64, Address),
    ProtocolFeeBps,
    ProtocolTreasury,
    UserStats(Address),
}

#[contracttype]
#[derive(Clone)]
pub struct Dispute {
    pub circle_id: u64,
    pub user: Address,
    pub amount: i128,
    pub reason_hash: u64,
    pub is_open: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct UserStats {
    pub total_volume_saved: i128,
    pub on_time_contributions: u32,
    pub late_contributions: u32,
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
    pub index: u32,
    pub contribution_count: u32,
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
    pub arbitrator: Address,
    pub proposed_arbitrator: Option<Address>,
    pub arbitrator_votes_bitmap: u64,
}

// --- CONTRACT CLIENTS ---

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

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    fn set_protocol_fee(env: Env, admin: Address, fee_basis_points: u32, treasury: Address);
    
    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
    ) -> u64;

    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>);
    fn deposit(env: Env, user: Address, circle_id: u64);
    
    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);
    
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);
    
    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);

    fn raise_dispute(env: Env, user: Address, circle_id: u64, amount: i128, reason_hash: u64);
    fn resolve_dispute(env: Env, caller: Address, circle_id: u64, user: Address, release_to_user: bool);

    fn propose_arbitrator(env: Env, user: Address, circle_id: u64, new_arbitrator: Address);
    fn vote_arbitrator(env: Env, user: Address, circle_id: u64);

    fn transfer_membership(env: Env, old_user: Address, new_user: Address, circle_id: u64);

    fn slash_user_credit(env: Env, admin: Address, user: Address, late_penalty_count: u32);

    fn get_user_reliability_score(env: Env, user: Address) -> u32;
    fn get_user_stats(env: Env, user: Address) -> UserStats;
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

    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::LendingPool, &pool);
    }

    fn set_protocol_fee(env: Env, admin: Address, fee_basis_points: u32, treasury: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        if fee_basis_points > 10000 {
            panic!("InvalidFeeConfig");
        }
        env.storage().instance().set(&DataKey::ProtocolFeeBps, &fee_basis_points);
        env.storage().instance().set(&DataKey::ProtocolTreasury, &treasury);
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
        arbitrator: Address,
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
            arbitrator,
            proposed_arbitrator: None,
            arbitrator_votes_bitmap: 0,
        };

        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("Already member");
        }

        let new_member = Member {
            address: user.clone(),
            index: circle.member_count,
            contribution_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
            tier_multiplier,
            referrer,
            buddy: None,
        };

        env.storage().instance().set(&member_key, &new_member);
        env.storage().instance().set(&DataKey::CircleMember(circle_id, circle.member_count), &user);
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

        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

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
            
            user_stats.late_contributions += 1;
        } else {
            user_stats.on_time_contributions += 1;
        }

        user_stats.total_volume_saved += base_amount;
        env.storage().instance().set(&user_stats_key, &user_stats);

        env.events().publish(
            (Symbol::new(&env, "USER_STATS"), user.clone()),
            (user_stats.on_time_contributions, user_stats.late_contributions, user_stats.total_volume_saved)
        );

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

        let recipient_addr: Address = env.storage().instance()
            .get(&DataKey::CircleMember(circle_id, circle.current_recipient_index))
            .expect("Member not found");

        let mut updated_circle = circle;
        updated_circle.is_round_finalized = true;
        updated_circle.current_pot_recipient = Some(recipient_addr);
        
        let current_time = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &current_time);
        env.storage().instance().set(&DataKey::Circle(circle_id), &updated_circle);
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
        
        let fee_bps: u32 = env.storage().instance().get(&DataKey::ProtocolFeeBps).unwrap_or(0);
        if fee_bps > 0 {
            let treasury: Address = env.storage().instance().get(&DataKey::ProtocolTreasury).expect("Treasury not set");
            let fee = (pot_amount * fee_bps as i128) / 10000;
            let net_payout = pot_amount - fee;
            token_client.transfer(&env.current_contract_address(), &treasury, &fee);
            token_client.transfer(&env.current_contract_address(), &user, &net_payout);
        } else {
            token_client.transfer(&env.current_contract_address(), &user, &pot_amount);
        }

        // Reset for next round
        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;
        
        let new_index = (circle.current_recipient_index + 1) % circle.member_count;
        if new_index == 0 {
            let total_volume = pot_amount * (circle.member_count as i128);
            env.events().publish((Symbol::new(&env, "CYCLE_COMP"), circle_id), total_volume);
            env.events().publish((Symbol::new(&env, "GROUP_ROLL"), circle_id), 0u32);
        }
        circle.current_recipient_index = new_index;
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

        // The member defaulted and needed an insurance bailout, increment late count
        let user_stats_key = DataKey::UserStats(member.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        user_stats.late_contributions += 1;
        env.storage().instance().set(&user_stats_key, &user_stats);

        env.events().publish(
            (Symbol::new(&env, "USER_STATS"), member.clone()),
            (user_stats.on_time_contributions, user_stats.late_contributions, user_stats.total_volume_saved)
        );

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

    fn raise_dispute(env: Env, user: Address, circle_id: u64, amount: i128, reason_hash: u64) {
        user.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let dispute_key = DataKey::Dispute(circle_id, user.clone());
        if env.storage().instance().has(&dispute_key) {
            let existing: Dispute = env.storage().instance().get(&dispute_key).unwrap();
            if existing.is_open {
                panic!("Dispute already exists");
            }
        }
        
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);
        
        let new_dispute = Dispute {
            circle_id,
            user: user.clone(),
            amount,
            reason_hash,
            is_open: true,
        };
        env.storage().instance().set(&dispute_key, &new_dispute);
        
        env.events().publish(
            (symbol_short!("DISPUTE"), symbol_short!("RAISED"), circle_id),
            (user, amount, reason_hash)
        );
    }

    fn resolve_dispute(env: Env, caller: Address, circle_id: u64, user: Address, release_to_user: bool) {
        caller.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if caller != circle.arbitrator {
            panic!("Unauthorized");
        }
        
        let dispute_key = DataKey::Dispute(circle_id, user.clone());
        let mut dispute: Dispute = env.storage().instance().get(&dispute_key).expect("Dispute not found");
        
        if !dispute.is_open {
            panic!("Dispute already resolved");
        }
        
        dispute.is_open = false;
        env.storage().instance().set(&dispute_key, &dispute);
        
        let token_client = token::Client::new(&env, &circle.token);
        if release_to_user {
            token_client.transfer(&env.current_contract_address(), &user, &dispute.amount);
        } else {
            // Funds stay in the contract, credit to circle's insurance
            circle.insurance_balance += dispute.amount;
            env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
            
            // Penalize the user for a bad faith dispute
            let user_stats_key = DataKey::UserStats(user.clone());
            let mut stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });
            stats.late_contributions += 3; // Equivalent to 3 late payments
            env.storage().instance().set(&user_stats_key, &stats);
            env.events().publish(
                (Symbol::new(&env, "USER_STATS"), user.clone()),
                (stats.on_time_contributions, stats.late_contributions, stats.total_volume_saved)
            );
        }
        
        env.events().publish(
            (symbol_short!("DISPUTE"), symbol_short!("RESOLVED"), circle_id),
            (user, release_to_user)
        );
    }

    fn propose_arbitrator(env: Env, user: Address, circle_id: u64, new_arbitrator: Address) {
        user.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        circle.proposed_arbitrator = Some(new_arbitrator);
        circle.arbitrator_votes_bitmap = 1 << member.index;

        if circle.arbitrator_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.arbitrator = circle.proposed_arbitrator.clone().unwrap();
            circle.proposed_arbitrator = None;
            circle.arbitrator_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn vote_arbitrator(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        if circle.proposed_arbitrator.is_none() {
            panic!("No active proposal");
        }

        circle.arbitrator_votes_bitmap |= 1 << member.index;

        if circle.arbitrator_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.arbitrator = circle.proposed_arbitrator.clone().unwrap();
            circle.proposed_arbitrator = None;
            circle.arbitrator_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn transfer_membership(env: Env, old_user: Address, new_user: Address, circle_id: u64) {
        old_user.require_auth();
        new_user.require_auth();

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let old_member_key = DataKey::Member(old_user.clone());
        let mut old_member: Member = env.storage().instance().get(&old_member_key).expect("Member not found");

        if old_member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let new_member_key = DataKey::Member(new_user.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("Already member");
        }

        let base_amount = circle.contribution_amount * old_member.tier_multiplier as i128;
        let buyout_amount = base_amount * old_member.contribution_count as i128;

        if buyout_amount > 0 {
            let token_client = token::Client::new(&env, &circle.token);
            token_client.transfer(&new_user, &old_user, &buyout_amount);
        }

        env.storage().instance().set(&DataKey::CircleMember(circle_id, old_member.index), &new_user);

        let new_member = Member {
            address: new_user.clone(),
            index: old_member.index,
            contribution_count: old_member.contribution_count,
            last_contribution_time: old_member.last_contribution_time,
            status: MemberStatus::Active,
            tier_multiplier: old_member.tier_multiplier,
            referrer: old_member.referrer.clone(),
            buddy: None,
        };
        env.storage().instance().set(&new_member_key, &new_member);

        old_member.status = MemberStatus::Ejected;
        env.storage().instance().set(&old_member_key, &old_member);

        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (old_member.index as u128);
        nft_client.burn(&old_user, &token_id);
        nft_client.mint(&new_user, &token_id);
    }

    fn slash_user_credit(env: Env, admin: Address, user: Address, late_penalty_count: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }

        let user_stats_key = DataKey::UserStats(user.clone());
        let mut stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        stats.late_contributions += late_penalty_count;
        env.storage().instance().set(&user_stats_key, &stats);
        env.events().publish(
            (Symbol::new(&env, "USER_STATS"), user.clone()),
            (stats.on_time_contributions, stats.late_contributions, stats.total_volume_saved)
        );
    }

    fn get_user_reliability_score(env: Env, user: Address) -> u32 {
        let stats: UserStats = env.storage().instance().get(&DataKey::UserStats(user)).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        let total_contributions = stats.on_time_contributions + stats.late_contributions;
        if total_contributions == 0 {
            return 0; // Unscored
        }

        // 1. Reliability (Max 700 points, weights heavily on successful completion rate)
        let reliability = (stats.on_time_contributions * 700) / total_contributions;

        // 2. Experience (Max 200 points, 20 points per successful contribution)
        let experience = (stats.on_time_contributions * 20).min(200);

        // 3. Volume Score (Max 100 points, linearly scales based on the order of magnitude)
        let mut vol = if stats.total_volume_saved > 0 { stats.total_volume_saved } else { 0 };
        let mut order = 0;
        while vol > 0 {
            vol /= 10;
            order += 1;
        }
        let volume_score = (order * 5).min(100) as u32;

        reliability + experience + volume_score
    }

    fn get_user_stats(env: Env, user: Address) -> UserStats {
        env.storage().instance().get(&DataKey::UserStats(user)).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Env};

    #[contract]
    pub struct MockNft;
    #[contractimpl]
    impl MockNft {
        pub fn mint(_env: Env, _to: Address, _id: u128) {}
        pub fn burn(_env: Env, _from: Address, _id: u128) {}
    }

    #[contract]
    pub struct MockToken;
    #[contractimpl]
    impl MockToken {
        pub fn transfer(_env: Env, _from: Address, _to: Address, _amount: i128) {}
    }

    #[contract]
    pub struct MockLending;
    #[contractimpl]
    impl MockLending {
        pub fn can_borrow(env: Env, oracle: Address, user: Address) -> bool {
            let client = SoroSusuClient::new(&env, &oracle);
            client.get_user_reliability_score(&user) >= 500
        }
    }

    #[test]
    fn test_arbitration_flow() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();

        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Raise dispute
        client.raise_dispute(&user, &circle_id, &500, &12345);
        
        // Resolve dispute - refund to user
        client.resolve_dispute(&arbitrator, &circle_id, &user, &true);
        
        // Raise second dispute
        client.raise_dispute(&user, &circle_id, &500, &12345);
        
        // Resolve dispute - send to insurance pool
        client.resolve_dispute(&arbitrator, &circle_id, &user, &false);
    }

    #[test]
    fn test_arbitrator_voting() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator1 = Address::generate(&env);
        let arbitrator2 = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();

        client.init(&admin);
        let circle_id = client.create_circle(&creator, &1000, &10, &token_contract, &86400, &100, &nft_contract, &arbitrator1);
        
        client.join_circle(&user1, &circle_id, &1, &None);
        client.join_circle(&user2, &circle_id, &1, &None);
        
        client.propose_arbitrator(&user1, &circle_id, &arbitrator2);
        client.vote_arbitrator(&user2, &circle_id);
        
        // Dispute should now be resolvable by arbitrator2
        client.raise_dispute(&user1, &circle_id, &500, &123);
        client.resolve_dispute(&arbitrator2, &circle_id, &user1, &true);
    }

    #[test]
    #[should_panic(expected = "Unauthorized")]
    fn test_resolve_dispute_unauthorized() {
        let env = Env::default();
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();

        client.init(&Address::generate(&env));
        let circle_id = client.create_circle(&Address::generate(&env), &1000, &10, &token_contract, &86400, &100, &nft_contract, &Address::generate(&env));
        
        let user = Address::generate(&env);
        client.join_circle(&user, &circle_id, &1, &None);
        client.raise_dispute(&user, &circle_id, &500, &12345);
        
        // Malicious actor tries to resolve dispute
        client.resolve_dispute(&Address::generate(&env), &circle_id, &user, &true);
    }

    #[test]
    fn test_protocol_fee_config() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        client.set_protocol_fee(&admin, &50, &treasury); // 0.5% fee
    }

    #[test]
    fn test_transfer_membership() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let old_user = Address::generate(&env);
        let new_user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();

        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&old_user, &circle_id, &1, &None);
        client.deposit(&old_user, &circle_id);
        
        client.transfer_membership(&old_user, &new_user, &circle_id);
        
        // The new user should now be able to act on behalf of the old position 
        client.deposit(&new_user, &circle_id);
    }

    #[test]
    fn test_credit_score_oracle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Start out unscored
        assert_eq!(client.get_user_reliability_score(&user), 0);

        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&user, &circle_id, &1, &None);
        client.deposit(&user, &circle_id);

        // Should earn positive reliability
        let score = client.get_user_reliability_score(&user);
        assert!(score > 0);
        
        let stats = client.get_user_stats(&user);
        assert_eq!(stats.on_time_contributions, 1);
        assert_eq!(stats.late_contributions, 0);
        assert_eq!(stats.total_volume_saved, 1_000_000_000_000);
    }

    #[test]
    fn test_slash_user_credit() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        client.slash_user_credit(&admin, &user, &5);
        let stats = client.get_user_stats(&user);
        assert_eq!(stats.late_contributions, 5);
        assert_eq!(client.get_user_reliability_score(&user), 0);
    }

    #[test]
    fn test_cross_contract_oracle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let oracle_id = env.register_contract(None, SoroSusu);
        let oracle_client = SoroSusuClient::new(&env, &oracle_id);
        
        let lending_id = env.register_contract(None, MockLending);
        let lending_client = MockLendingClient::new(&env, &lending_id);
        
        env.mock_all_auths();
        oracle_client.init(&admin);
        
        // Start out unscored, cannot borrow
        assert_eq!(lending_client.can_borrow(&oracle_id, &user), false);

        let circle_id = oracle_client.create_circle(
            &creator,
            &1_000_000_000_000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        oracle_client.join_circle(&user, &circle_id, &1, &None);
        oracle_client.deposit(&user, &circle_id);

        // After a successful on-time deposit, score surges past the 500 threshold
        assert_eq!(lending_client.can_borrow(&oracle_id, &user), true);
    }
}
