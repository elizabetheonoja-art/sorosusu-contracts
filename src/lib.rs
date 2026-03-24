#![no_std]
use soroban_sdk::xdr::ToXdr;
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, token,
    Address, Env, String, Vec,
    Address, Env, Symbol, Vec,
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Bytes, BytesN, Env, String, Symbol, Vec,
    Address, Env, String, Symbol, Vec,
    contract, contractclient, contracterror, contractimpl, contracttype, token, Address, Env, Vec,
    contract, contractclient, contracterror, contractimpl, contracttype, token, Address, Env,
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
    Blacklisted = 15,
    InsufficientCollateral = 15,
    CollateralAlreadyStaked = 16,
    CollateralNotStaked = 17,
    CollateralLocked = 18,
    MemberNotDefaulted = 19,
    CollateralAlreadyReleased = 20,
    // Guarantor-related errors
    GuarantorNotFound = 21,
    InsufficientReputation = 22,
    GuarantorNotRegistered = 23,
    VoucherAlreadyExists = 24,
    VoucherNotFound = 25,
    GuarantorOverextended = 26,
    SelfGuaranteeNotAllowed = 27,
    GuarantorInsufficientFunds = 28,
    LeniencyRequestNotFound = 21,
    AlreadyVoted = 22,
    VotingPeriodExpired = 23,
    LeniencyAlreadyApproved = 24,
    LeniencyNotRequested = 25,
    CannotVoteForOwnRequest = 26,
    InvalidVote = 27,
    ProposalNotFound = 28,
    ProposalAlreadyExecuted = 29,
    VotingNotActive = 30,
    InsufficientVotingPower = 31,
    QuadraticVoteExceeded = 32,
    InvalidProposalType = 33,
    QuorumNotMet = 34,
    ProposalExpired = 35,
    CannotVouchForSelf = 36,
    InsufficientTrustScore = 37,
    VoucherNotActive = 38,
    VoucheeAlreadyMember = 39,
    VouchAlreadyExists = 40,
    VouchExpired = 41,
    CollateralInsufficientForVouch = 42,
}

// --- CONSTANTS ---
const REFERRAL_DISCOUNT_BPS: u32 = 500; // 5%
const RATE_LIMIT_SECONDS: u64 = 300; // 5 minutes
const VOTING_PERIOD: u64 = 86400; // 24 hours voting period
const MINIMUM_VOTING_PARTICIPATION: u32 = 50; // 50% minimum participation
const SIMPLE_MAJORITY_THRESHOLD: u32 = 51; // 51% simple majority
const QUADRATIC_VOTING_PERIOD: u64 = 604800; // 7 days for rule changes
const QUADRATIC_QUORUM: u32 = 40; // 40% quorum for quadratic voting
const QUADRATIC_MAJORITY: u32 = 60; // 60% supermajority for rule changes
const MAX_VOTE_WEIGHT: u32 = 100; // Maximum quadratic vote weight
const MIN_GROUP_SIZE_FOR_QUADRATIC: u32 = 10; // Enable quadratic voting for groups >= 10 members
const DEFAULT_COLLATERAL_BPS: u32 = 2000; // 20%
const HIGH_VALUE_THRESHOLD: i128 = 1_000_000_0; // 1000 XLM (assuming 7 decimals)
const MIN_TRUST_SCORE_FOR_VOUCH: u32 = 70; // Minimum trust score to vouch
const VOUCH_COLLATERAL_MULTIPLIER: u32 = 1500; // 15% of cycle value as vouch collateral
const VOUCH_EXPIRY_SECONDS: u64 = 2592000; // 30 days
const MAX_VOUCHES_PER_MEMBER: u32 = 3; // Maximum concurrent vouches
// Guarantor-related constants
const MIN_GUARANTOR_REPUTATION: u32 = 100; // Minimum reputation score to become a guarantor
const MAX_VOUCHURES_PER_GUARANTOR: u32 = 5; // Maximum concurrent vouchers per guarantor
const GUARANTOR_COLLATERAL_MULTIPLIER: u32 = 150; // 150% of member's contribution as guarantor collateral
const HIGH_VALUE_THRESHOLD: i128 = 10_000_000_000; // 1000 XLM (assuming 7 decimals)

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    GlobalBlacklist(Address),
    CircleCount,
    Deposit(u64, Address),
    GroupReserve,
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    LendingPool,
    ReputationHash(Address),
    CollateralVault(Address, u64),
    CollateralConfig(u64),
    DefaultedMembers(u64),
    // Guarantor-related storage keys
    Guarantor(Address),
    Voucher(Address, u64), // (guarantor_address, circle_id)
    GuarantorVault(Address), // Guarantor's collateral vault
    ActiveVouchersCount(Address),
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
    CircleMembership(u64, Address),
    CircleMemberIndex(u64, u32),
    SuccessionProposal(u64),
    CollateralVault(Address, u64),
    CollateralConfig(u64),
    DefaultedMembers(u64),
    LeniencyRequest(u64, Address),
    LeniencyVotes(u64, Address, Address),
    SocialCapital(Address, u64),
    LeniencyStats(u64),
    Proposal(u64),
    QuadraticVote(u64, Address),
    VotingPower(Address, u64),
    ProposalStats(u64),
    VouchRecord(Address, Address), // voucher -> vouchee
    VouchCollateral(Address, u64), // vouchee -> vouch_id
    VouchStats(Address), // voucher stats
    VouchReverseMapping(Address, u64), // vouchee -> voucher (for efficient lookup)
    Blacklist(Address),
    Reputation(Address),
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
pub enum LeniencyVote {
    Approve,
    Reject,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LeniencyRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalType {
    ChangeLateFee,
    ChangeInsuranceFee,
    ChangeCycleDuration,
    AddMember,
    RemoveMember,
    ChangeQuorum,
    EmergencyAction,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Active,
    Approved,
    Rejected,
    Executed,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum QuadraticVoteChoice {
    For,
    Against,
    Abstain,
}

#[contracttype]
#[derive(Clone)]
pub struct LeniencyRequest {
    pub requester: Address,
    pub circle_id: u64,
    pub request_timestamp: u64,
    pub voting_deadline: u64,
    pub status: LeniencyRequestStatus,
    pub approve_votes: u32,
    pub reject_votes: u32,
    pub total_votes_cast: u32,
    pub extension_hours: u64,
    pub reason: String,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub circle_id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_timestamp: u64,
    pub voting_start_timestamp: u64,
    pub voting_end_timestamp: u64,
    pub status: ProposalStatus,
    pub for_votes: u64,
    pub against_votes: u64,
    pub total_voting_power: u64,
    pub quorum_met: bool,
    pub execution_data: String, // JSON or structured data for execution
}

#[contracttype]
#[derive(Clone)]
pub struct QuadraticVote {
    pub voter: Address,
    pub proposal_id: u64,
    pub vote_weight: u32,
    pub vote_choice: QuadraticVoteChoice,
    pub voting_power_used: u64,
    pub timestamp: u64,
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
pub struct VotingPower {
    pub member: Address,
    pub circle_id: u64,
    pub token_balance: i128,
    pub quadratic_power: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProposalStats {
    pub total_proposals: u32,
    pub approved_proposals: u32,
    pub rejected_proposals: u32,
    pub executed_proposals: u32,
    pub average_participation: u32,
    pub average_voting_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VouchStatus {
    Active,
    Slashed,
    Completed,
    Expired,
}

#[contracttype]
#[derive(Clone)]
pub struct VouchRecord {
    pub voucher: Address,
    pub vouchee: Address,
    pub circle_id: u64,
    pub collateral_amount: i128,
    pub vouch_timestamp: u64,
    pub expiry_timestamp: u64,
    pub status: VouchStatus,
    pub slash_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct VouchStats {
    pub voucher: Address,
    pub total_vouches_made: u32,
    pub active_vouches: u32,
    pub successful_vouches: u32,
    pub slashed_vouches: u32,
    pub total_collateral_locked: i128,
    pub total_collateral_lost: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct LeniencyStats {
    pub total_requests: u32,
    pub approved_requests: u32,
    pub rejected_requests: u32,
    pub expired_requests: u32,
    pub average_participation: u32,
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
pub struct SocialCapital {
    pub member: Address,
    pub circle_id: u64,
    pub leniency_given: u32,
    pub leniency_received: u32,
    pub voting_participation: u32,
    pub trust_score: u32,
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

// --- GUARANTOR DATA STRUCTURES ---

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GuarantorStatus {
    Registered,
    Active,
    Suspended,
    Blacklisted,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum VoucherStatus {
    Active,
    Claimed, // Guarantor paid for member default
    Expired, // Member completed all obligations
    Cancelled,
}

#[contracttype]
#[derive(Clone)]
pub struct GuarantorInfo {
    pub address: Address,
    pub reputation_score: u32,
    pub total_vouched_amount: i128,
    pub active_vouchers_count: u32,
    pub successful_vouchers: u32,
    pub claimed_vouchers: u32,
    pub status: GuarantorStatus,
    pub registered_timestamp: u64,
    pub vault_balance: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct VoucherInfo {
    pub guarantor: Address,
    pub member: Address,
    pub circle_id: u64,
    pub vouched_amount: i128,
    pub status: VoucherStatus,
    pub created_timestamp: u64,
    pub claimed_timestamp: Option<u64>,
    pub collateral_required: i128,
}

#[contracttype]
#[derive(Clone)]
pub struct Reputation {
    pub cycles_completed: u32,
    pub total_contributions: u32,
    pub on_time_contributions: u32,
    pub total_volume: i128,
}

#[contracttype]
#[derive(Clone)]
#[derive(Clone, Debug, PartialEq)]
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
    pub guarantor: Option<Address>, // Guarantor who co-signed for this member
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub current_lead: Address,
    pub organizer_fee_recipient: Address,
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
    pub leniency_enabled: bool,
    pub grace_period_end: Option<u64>,
    pub quadratic_voting_enabled: bool,
    pub proposal_count: u64,
    pub requires_collateral: bool,
    pub collateral_bps: u32,
    pub total_cycle_value: i128,
    pub min_reputation: u32,
}

pub mod external_clients {
    use super::*;
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct SuccessionProposal {
    pub nominee: Address,
    pub approvals_bitmap: u64,
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
#[contractclient(name = "BadgeClient")]
pub trait BadgeTrait {
    fn mint(env: Env, to: Address, traits: Vec<Symbol>);
}

pub use external_clients::{SusuNftClient, SusuNftTrait, LendingPoolClient, LendingPoolTrait, BadgeClient, BadgeTrait};

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    fn set_badge_contract(env: Env, admin: Address, badge: Address);

    fn flag_member_for_default(env: Env, caller: Address, circle_id: u64, member: Address);
    fn clear_global_blacklist(env: Env, admin: Address, member: Address);
    fn is_globally_blacklisted(env: Env, member: Address) -> bool;
    
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
    
    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        referrer: Option<Address>,
    );
    fn deposit(env: Env, user: Address, circle_id: u64);

    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);

    fn propose_group_lead_succession(env: Env, user: Address, circle_id: u64, nominee: Address);
    fn approve_group_lead_succession(env: Env, user: Address, circle_id: u64);

    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);

    fn compute_reputation_hash(env: Env, user: Address, threshold: u32) -> soroban_sdk::Bytes;
    fn verify_reputation(env: Env, user: Address, threshold: u32) -> bool;
    fn get_circle(env: Env, circle_id: u64) -> CircleInfo;
    fn get_member(env: Env, circle_id: u64, member: Address) -> Member;
    fn get_members(env: Env, circle_id: u64) -> Vec<Address>;
    fn get_succession_proposal(env: Env, circle_id: u64) -> Option<SuccessionProposal>;
}

fn read_circle(env: &Env, circle_id: u64) -> CircleInfo {
    env.storage()
        .instance()
        .get(&DataKey::Circle(circle_id))
        .expect("Circle not found")
}

fn write_circle(env: &Env, circle: &CircleInfo) {
    env.storage()
        .instance()
        .set(&DataKey::Circle(circle.id), circle);
}

fn read_member(env: &Env, member: &Address) -> Member {
    env.storage()
        .instance()
        .get(&DataKey::Member(member.clone()))
        .expect("Member not found")
}

fn require_circle_membership(env: &Env, circle_id: u64, member: &Address) {
    if !env
        .storage()
        .instance()
        .has(&DataKey::CircleMembership(circle_id, member.clone()))
    {
        panic!("Member not found");
    }
}

fn read_circle_member_index(env: &Env, circle_id: u64, member: &Address) -> u32 {
    env.storage()
        .instance()
        .get(&DataKey::CircleMembership(circle_id, member.clone()))
        .expect("Member not found")
}

fn active_member_count(env: &Env, circle: &CircleInfo) -> u32 {
    let mut active_members = 0u32;

    for index in 0..circle.member_count {
        if let Some(member_address) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::CircleMemberIndex(circle.id, index))
        {
            let member = read_member(env, &member_address);
            if member.status == MemberStatus::Active {
                active_members += 1;
            }
        }
    }

    active_members
}

fn active_member_bitmap(env: &Env, circle: &CircleInfo) -> u64 {
    let mut bitmap = 0u64;

    for index in 0..circle.member_count {
        if let Some(member_address) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::CircleMemberIndex(circle.id, index))
        {
            let member = read_member(env, &member_address);
            if member.status == MemberStatus::Active {
                bitmap |= 1u64 << index;
            }
        }
    }

    bitmap
}

fn required_succession_approvals(active_members: u32) -> u32 {
    if active_members == 0 {
        0
    } else {
        ((active_members * 2) + 2) / 3
    }
}

fn maybe_execute_succession(env: &Env, circle: &mut CircleInfo, proposal: SuccessionProposal) {
    let active_members = active_member_count(env, circle);
    let active_votes = (proposal.approvals_bitmap & active_member_bitmap(env, circle)).count_ones();
    let threshold = required_succession_approvals(active_members);

    if active_votes >= threshold {
        circle.current_lead = proposal.nominee.clone();
        circle.organizer_fee_recipient = proposal.nominee;
        write_circle(env, circle);
        env.storage()
            .instance()
            .remove(&DataKey::SuccessionProposal(circle.id));
    } else {
        write_circle(env, circle);
        env.storage()
            .instance()
            .set(&DataKey::SuccessionProposal(circle.id), &proposal);
    }
}

fn find_active_member_address(env: &Env, circle: &CircleInfo, start_index: u32) -> Address {
    if circle.member_count == 0 {
        panic!("Member not found");
    }

    let base_index = start_index % circle.member_count;

    for offset in 0..circle.member_count {
        let index = (base_index + offset) % circle.member_count;
        if let Some(member_address) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::CircleMemberIndex(circle.id, index))
        {
            let member = read_member(env, &member_address);
            if member.status == MemberStatus::Active {
                return member_address;
            }
        }
    }

    panic!("Member not found");
}

fn next_active_member_index(env: &Env, circle: &CircleInfo, start_index: u32) -> u32 {
    if circle.member_count == 0 {
        return 0;
    }

    let base_index = start_index % circle.member_count;

    for offset in 0..circle.member_count {
        let index = (base_index + offset) % circle.member_count;
        if let Some(member_address) = env
            .storage()
            .instance()
            .get::<DataKey, Address>(&DataKey::CircleMemberIndex(circle.id, index))
        {
            let member = read_member(env, &member_address);
            if member.status == MemberStatus::Active {
                return index;
            }
        }
    }

    0
    
    // Leniency voting functions
    fn request_leniency(env: Env, requester: Address, circle_id: u64, reason: String);
    fn vote_on_leniency(env: Env, voter: Address, circle_id: u64, requester: Address, vote: LeniencyVote);
    fn finalize_leniency_vote(env: Env, caller: Address, circle_id: u64, requester: Address);
    fn get_leniency_request(env: Env, circle_id: u64, requester: Address) -> LeniencyRequest;
    fn get_social_capital(env: Env, member: Address, circle_id: u64) -> SocialCapital;
    fn get_leniency_stats(env: Env, circle_id: u64) -> LeniencyStats;
    
    // Quadratic voting functions
    fn create_proposal(
        env: Env,
        proposer: Address,
        circle_id: u64,
        proposal_type: ProposalType,
        title: String,
        description: String,
        execution_data: String,
    ) -> u64;
    
    fn quadratic_vote(env: Env, voter: Address, proposal_id: u64, vote_weight: u32, vote_choice: QuadraticVoteChoice);
    fn execute_proposal(env: Env, caller: Address, proposal_id: u64);
    fn get_proposal(env: Env, proposal_id: u64) -> Proposal;
    fn get_voting_power(env: Env, member: Address, circle_id: u64) -> VotingPower;
    fn get_proposal_stats(env: Env, circle_id: u64) -> ProposalStats;
    fn update_voting_power(env: Env, member: Address, circle_id: u64, token_balance: i128);

    // Collateral functions
    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128);
    fn slash_collateral(env: Env, caller: Address, circle_id: u64, member: Address);
    fn release_collateral(env: Env, caller: Address, circle_id: u64, member: Address);
    fn mark_member_defaulted(env: Env, caller: Address, circle_id: u64, member: Address);
    
    // Social vouching functions
    fn vouch_for_member(env: Env, voucher: Address, vouchee: Address, circle_id: u64, collateral_amount: i128);
    fn slash_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address);
    fn release_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address);
    fn get_vouch_record(env: Env, voucher: Address, vouchee: Address) -> VouchRecord;
    fn get_vouch_stats(env: Env, voucher: Address) -> VouchStats;
    // Guarantor functions
    fn register_guarantor(env: Env, user: Address, initial_collateral: i128);
    fn update_guarantor_reputation(env: Env, admin: Address, guarantor: Address, new_score: u32);
    fn create_voucher(env: Env, guarantor: Address, member: Address, circle_id: u64, vouched_amount: i128);
    fn claim_voucher(env: Env, caller: Address, circle_id: u64, member: Address);
    fn add_guarantor_collateral(env: Env, guarantor: Address, amount: i128);
    fn withdraw_guarantor_collateral(env: Env, guarantor: Address, amount: i128);
    
    // Query functions for guarantor system
    fn get_guarantor_info(env: Env, guarantor: Address) -> GuarantorInfo;
    fn get_voucher_info(env: Env, guarantor: Address, circle_id: u64) -> VoucherInfo;
    fn get_member_guarantor(env: Env, member: Address) -> Option<Address>;
    fn get_guarantor_vault_balance(env: Env, guarantor: Address) -> i128;

    // Privacy functions
    fn get_reputation_hash(env: Env, user: Address) -> BytesN<32>;
    // Oracle & Blacklist
    fn get_reliability_score(env: Env, user: Address) -> u32;
    fn is_blacklisted(env: Env, user: Address) -> bool;
    fn update_blacklist_status(env: Env, admin: Address, user: Address, status: bool);
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

    fn finalize_leniency_vote_internal(env: &Env, circle_id: &u64, requester: &Address, request: &mut LeniencyRequest) {
        // Calculate voting results
        let total_possible_votes = request.total_votes_cast;
        let minimum_participation = (total_possible_votes * MINIMUM_VOTING_PARTICIPATION) / 100;
        
        let mut final_status = LeniencyRequestStatus::Rejected;
        
        if request.total_votes_cast >= minimum_participation {
            let approval_percentage = (request.approve_votes * 100) / request.total_votes_cast;
            if approval_percentage >= SIMPLE_MAJORITY_THRESHOLD {
                final_status = LeniencyRequestStatus::Approved;
                
                // Apply grace period extension
                let circle_key = DataKey::Circle(*circle_id);
                let mut circle: CircleInfo = env.storage().instance().get(&circle_key).expect("Circle not found");
                
                let extension_seconds = request.extension_hours * 3600;
                let new_deadline = circle.deadline_timestamp + extension_seconds;
                circle.deadline_timestamp = new_deadline;
                circle.grace_period_end = Some(new_deadline);
                
                env.storage().instance().set(&circle_key, &circle);
                
                // Update requester's social capital
                let social_capital_key = DataKey::SocialCapital(requester.clone(), *circle_id);
                let mut social_capital: SocialCapital = env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
                    member: requester.clone(),
                    circle_id: *circle_id,
                    leniency_given: 0,
                    leniency_received: 0,
                    voting_participation: 0,
                    trust_score: 50,
                });
                social_capital.leniency_received += 1;
                social_capital.trust_score = (social_capital.trust_score + 5).min(100); // Bonus for receiving leniency
                env.storage().instance().set(&social_capital_key, &social_capital);
            }
        }
        
        request.status = final_status.clone();

        // Update leniency stats
        let stats_key = DataKey::LeniencyStats(*circle_id);
        let mut stats: LeniencyStats = env.storage().instance().get(&stats_key).unwrap_or(LeniencyStats {
            total_requests: 0,
            approved_requests: 0,
            rejected_requests: 0,
            expired_requests: 0,
            average_participation: 0,
        });

        match final_status {
            LeniencyRequestStatus::Approved => stats.approved_requests += 1,
            LeniencyRequestStatus::Rejected => stats.rejected_requests += 1,
            LeniencyRequestStatus::Expired => stats.expired_requests += 1,
            _ => {}
        }

        // Update average participation
        if stats.total_requests > 0 {
            let total_participation = stats.average_participation * (stats.total_requests - 1) + request.total_votes_cast;
            stats.average_participation = total_participation / stats.total_requests;
        }

        env.storage().instance().set(&stats_key, &stats);
    }

    fn execute_proposal_logic(env: &Env, proposal: &Proposal) {
        let proposal_key = DataKey::Proposal(proposal.id);
        let mut updated_proposal = proposal.clone();
        updated_proposal.status = ProposalStatus::Executed;
        env.storage().instance().set(&proposal_key, &updated_proposal);
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

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if admin != stored_admin {
            panic!("Unauthorized");
        }

        env.storage().instance().set(&DataKey::LendingPool, &pool);
    }

    fn flag_member_for_default(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        if !env.storage().instance().has(&member_key) {
            panic!("Member not found");
        }

        env.storage().instance().set(&DataKey::GlobalBlacklist(member), &true);
    }

    fn clear_global_blacklist(env: Env, admin: Address, member: Address) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::BadgeContract, &badge);

        env.storage().instance().remove(&DataKey::GlobalBlacklist(member));
    }

    fn is_globally_blacklisted(env: Env, member: Address) -> bool {
        env.storage()
            .instance()
            .get(&DataKey::GlobalBlacklist(member))
            .unwrap_or(false)
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

        if amount <= 0 {
            panic!("Invalid amount");
        }

        if max_members == 0 || max_members > 64 {
            panic!("Max members must be between 1 and 64");
        }

        if insurance_fee_bps > 10_000 {
            panic!("Insurance fee cannot exceed 100%");
        }

        let current_time = env.ledger().timestamp();
        let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());

        if let Some(last_created) = env
            .storage()
            .instance()
            .get::<DataKey, u64>(&rate_limit_key)
        {
            if current_time < last_created + RATE_LIMIT_SECONDS {
                panic!("Rate limit exceeded");
            }
        }

        env.storage().instance().set(&rate_limit_key, &current_time);

        let mut circle_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CircleCount)
            .unwrap_or(0);
        circle_count += 1;

        // Calculate total cycle value and determine collateral requirements
        let total_cycle_value = amount * (max_members as i128);
        let requires_collateral = total_cycle_value >= HIGH_VALUE_THRESHOLD;
        let collateral_bps = if requires_collateral { DEFAULT_COLLATERAL_BPS } else { 0 };

        let new_circle = CircleInfo {
            id: circle_count,
            creator: creator.clone(),
            current_lead: creator.clone(),
            organizer_fee_recipient: creator,
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
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            requires_collateral,
            collateral_bps,
            total_cycle_value,
            min_reputation,
        };

        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);
        write_circle(&env, &new_circle);
        env.storage()
            .instance()
            .set(&DataKey::CircleCount, &circle_count);

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>) {
        user.require_auth();

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        referrer: Option<Address>,
    ) {
        user.require_auth();

        let mut circle = read_circle(&env, circle_id);
        if env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::GlobalBlacklist(user.clone()))
            .unwrap_or(false)
        {
            panic!("User is globally blacklisted");
        }

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
            // First check if member is vouched for
            let vouch_reverse_key = DataKey::VouchReverseMapping(user.clone(), circle_id);
            if let Some(_voucher) = env.storage().instance().get::<DataKey, Address>(&vouch_reverse_key) {
                // User is vouched for, skip collateral requirement
                let vouch_collateral_key = DataKey::VouchCollateral(user.clone(), circle_id);
                if let Some(_vouch_id) = env.storage().instance().get::<DataKey, u64>(&vouch_collateral_key) {
                    // Vouch exists, proceed without collateral check
                } else {
                    panic!("Vouch not found for this user");
                }
            } else {
                // No vouch found, check regular collateral
                let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
                let collateral_info: Option<CollateralInfo> = env.storage().instance().get(&collateral_key);
                
                match collateral_info {
                    Some(collateral) => {
                        if collateral.status != CollateralStatus::Staked {
                            panic!("Collateral not properly staked");
                        }
            let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
            let collateral_info: Option<CollateralInfo> = env.storage().instance().get(&collateral_key);
            
            // Allow member to join if they have either collateral OR a guarantor
            let has_collateral = match collateral_info {
                Some(collateral) => collateral.status == CollateralStatus::Staked,
                None => false,
            };

            let has_guarantor = {
                // Check if there's an active voucher for this member
                // This would require finding the voucher by member address, which is challenging with current storage structure
                // For now, we'll assume the member has been assigned a guarantor before joining
                false // This will be handled by the create_voucher function
            };

            if !has_collateral && !has_guarantor {
                panic!("Collateral or guarantor required for this circle");
            }
        }

        // Store member by index for the circle
        env.storage().instance().set(&DataKey::MemberAtIndex(circle_id, circle.member_count), &user);

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
                    None => panic!("Collateral required for this circle"),
                }
            }
        }

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
            guarantor: None, // Will be set when voucher is created
        };

        env.storage().instance().set(&member_key, &new_member);
        env.storage().instance().set(
            &DataKey::CircleMembership(circle_id, user.clone()),
            &new_member.index,
        );
        env.storage().instance().set(
            &DataKey::CircleMemberIndex(circle_id, new_member.index),
            &user,
        );

        circle.member_count += 1;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Mint NFT
        write_circle(&env, &circle);

        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        require_circle_membership(&env, circle_id, &user);

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key).expect("Member not found");
        let mut circle = read_circle(&env, circle_id);
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let member_index = read_circle_member_index(&env, circle_id, &user);
        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;
        let mut penalty_amount = 0i128;

        if current_time > circle.deadline_timestamp {
            let base_penalty = (base_amount * circle.late_fee_bps as i128) / 10000;
            // Apply referral discount
            let mut discount = 0i128;
        // Check if late fee applies (considering grace periods)
        let effective_deadline = circle.grace_period_end.unwrap_or(circle.deadline_timestamp);
        
        if current_time > effective_deadline {
            let base_penalty = (base_amount * circle.late_fee_bps as i128) / 10000;
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


            penalty_amount = base_penalty - discount;

            let mut reserve: i128 = env
                .storage()
                .instance()
                .get(&DataKey::GroupReserve)
                .unwrap_or(0);
            reserve += penalty_amount;
            env.storage()
                .instance()
                .set(&DataKey::GroupReserve, &reserve);
        }

        let insurance_fee = (base_amount * circle.insurance_fee_bps as i128) / 10000;
        let total_amount = base_amount + insurance_fee + penalty_amount;
        let token_client = token::Client::new(&env, &circle.token);

        let transfer_result =
            token_client.try_transfer(&user, &env.current_contract_address(), &total_amount);
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            if let Some(buddy_addr) = &member.buddy {
                let safety_key = DataKey::SafetyDeposit(buddy_addr.clone(), circle_id);
                let safety_balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
                if safety_balance >= total_amount {
                    env.storage()
                        .instance()
                        .set(&safety_key, &(safety_balance - total_amount));
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

        // Update Reputation (Issue 71)
        // Update Reputation
        let rep_key = DataKey::Reputation(user.clone());
        let mut rep: Reputation = env.storage().instance().get(&rep_key).unwrap_or(Reputation {
            cycles_completed: 0,
            total_contributions: 0,
            on_time_contributions: 0,
            total_volume: 0,
        });
        rep.total_contributions += 1;
        rep.total_volume += total_amount;
        if current_time <= effective_deadline {
            rep.on_time_contributions += 1;
        }
        env.storage().instance().set(&rep_key, &rep);

        rep.total_contributions += 1;
        rep.total_volume += base_amount;
        if current_time <= circle.deadline_timestamp {
            rep.on_time_contributions += 1;
            member.on_time_count += 1;
        }
        env.storage().instance().set(&rep_key, &rep);
        env.storage().instance().set(&member_key, &member);
        circle.contribution_bitmap |= 1u64 << member_index;

        env.storage().instance().set(&member_key, &member);
        write_circle(&env, &circle);
    }

    fn finalize_round(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if caller != circle.creator && caller != stored_admin {

        let mut circle = read_circle(&env, circle_id);
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if caller != circle.current_lead && caller != stored_admin {
            panic!("Unauthorized");
        }

        if circle.is_round_finalized {
            panic!("Round already finalized");
        }

        let expected_bitmap = (1u64 << circle.member_count) - 1;
        if circle.contribution_bitmap != expected_bitmap {
            panic!("Not all contributed");
        }

        // iterate to find the address if needed, or better, store it in storage under (circle_id, index)
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

        let expected_bitmap = active_member_bitmap(&env, &circle);
        if circle.contribution_bitmap != expected_bitmap {
            panic!("Not all contributed");
        }

        let recipient = find_active_member_address(&env, &circle, circle.current_recipient_index);
        circle.current_pot_recipient = Some(recipient);
        circle.is_round_finalized = true;

        env.storage().instance().set(
            &DataKey::ScheduledPayoutTime(circle_id),
            &env.ledger().timestamp(),
        );
        write_circle(&env, &circle);
    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle = read_circle(&env, circle_id);

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

        let scheduled_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ScheduledPayoutTime(circle_id))
            .expect("Payout not scheduled");

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

                            // Update Reputation completion (Issue 71)
                            let rep_key = DataKey::Reputation(user.clone());
                            let mut rep: Reputation = env.storage().instance().get(&rep_key).unwrap_or(Reputation {
                                cycles_completed: 0,
                                total_contributions: 0,
                                on_time_contributions: 0,
                                total_volume: 0,
                            });
                            rep.cycles_completed += 1;
                            env.storage().instance().set(&rep_key, &rep);

                            // Mint "Susu Master" Badge (Issue 72)
                            // We use the nft_contract as the badge issuer for this implementation
                            let badge_client = BadgeClient::new(&env, &circle.nft_contract);
                            let mut traits = Vec::new(&env);
                            traits.push_back(symbol_short!("Master"));
                            if rep.on_time_contributions == rep.total_contributions {
                                traits.push_back(symbol_short!("Perfect"));
                            }
                            if rep.total_volume >= 100_000_000 {
                                traits.push_back(symbol_short!("Whale"));
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
        circle.current_pot_recipient = None;

        if circle.member_count > 0 {
            let next_index = (circle.current_recipient_index + 1) % circle.member_count;
            circle.current_recipient_index = next_active_member_index(&env, &circle, next_index);
        }

        write_circle(&env, &circle);
        env.storage()
            .instance()
            .remove(&DataKey::ScheduledPayoutTime(circle_id));
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        require_circle_membership(&env, circle_id, &member);

        let mut circle = read_circle(&env, circle_id);
        if caller != circle.current_lead {
            panic!("Unauthorized");
        }

        if circle.is_insurance_used {
            panic!("Insurance already used");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member_info.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let member_index = read_circle_member_index(&env, circle_id, &member);
        if (circle.contribution_bitmap & (1u64 << member_index)) != 0 {
            panic!("Member already contributed");
        }

        let amount_needed = circle.contribution_amount * member_info.tier_multiplier as i128;
        if circle.insurance_balance < amount_needed {
            panic!("Insufficient insurance");
        }

        circle.contribution_bitmap |= 1u64 << member_index;
        circle.insurance_balance -= amount_needed;
        circle.is_insurance_used = true;

        write_circle(&env, &circle);
        env.storage().instance().set(&DataKey::GlobalBlacklist(member), &true);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        require_circle_membership(&env, circle_id, &member);

        let circle = read_circle(&env, circle_id);
        if caller != circle.current_lead {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member_info.status == MemberStatus::Ejected {
            panic!("Already ejected");
        }

        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        let member_index = read_circle_member_index(&env, circle_id, &member);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (member_index as u128);
        nft_client.burn(&member, &token_id);
    }

    fn propose_group_lead_succession(env: Env, user: Address, circle_id: u64, nominee: Address) {
        user.require_auth();
        require_circle_membership(&env, circle_id, &user);

        let member = read_member(&env, &user);
        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let member_index = read_circle_member_index(&env, circle_id, &user);
        let mut circle = read_circle(&env, circle_id);
        let proposal = SuccessionProposal {
            nominee,
            approvals_bitmap: 1u64 << member_index,
        };

        maybe_execute_succession(&env, &mut circle, proposal);
    }

    fn approve_group_lead_succession(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        require_circle_membership(&env, circle_id, &user);

        let member = read_member(&env, &user);
        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let member_index = read_circle_member_index(&env, circle_id, &user);
        let mut circle = read_circle(&env, circle_id);
        let mut proposal: SuccessionProposal = env
            .storage()
            .instance()
            .get(&DataKey::SuccessionProposal(circle_id))
            .expect("No active succession proposal");

        proposal.approvals_bitmap |= 1u64 << member_index;
        maybe_execute_succession(&env, &mut circle, proposal);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();
        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env.storage().instance().get(&user_key).expect("Member not found");
        

        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env
            .storage()
            .instance()
            .get(&user_key)
            .expect("Member not found");

        user_info.buddy = Some(buddy_address);
        env.storage().instance().set(&user_key, &user_info);
    }

    fn compute_reputation_hash(env: Env, user: Address, threshold: u32) -> soroban_sdk::Bytes {
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member.contribution_count < threshold {
            panic!("Reputation threshold not met");
        }

        // Build preimage: user bytes + threshold + contract address bytes
        let mut preimage = soroban_sdk::Bytes::new(&env);
        let user_bytes = user.clone().to_xdr(&env);
        preimage.append(&user_bytes);

        let threshold_arr = [
            ((threshold >> 24) & 0xff) as u8,
            ((threshold >> 16) & 0xff) as u8,
            ((threshold >> 8) & 0xff) as u8,
            (threshold & 0xff) as u8,
        ];
        let threshold_bytes = soroban_sdk::Bytes::from_slice(&env, &threshold_arr);
        preimage.append(&threshold_bytes);

        let contract_bytes = env.current_contract_address().to_xdr(&env);
        preimage.append(&contract_bytes);

        let hash = env.crypto().sha256(&preimage);
        let hash_bytes = soroban_sdk::Bytes::from_slice(&env, hash.to_array().as_slice());

        env.storage().instance().set(&DataKey::ReputationHash(user), &hash_bytes.clone());
        hash_bytes
    }

    fn verify_reputation(env: Env, user: Address, threshold: u32) -> bool {
        let member_key = DataKey::Member(user.clone());
        if !env.storage().instance().has(&member_key) {
            return false;
        }
        let member: Member = env.storage().instance().get(&member_key).expect("Member not found");
        member.contribution_count >= threshold
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
        let _token_client = token::Client::new(&env, &circle.token);
        let slash_amount = collateral_info.amount;
        
        // Get active members (excluding defaulted member)
        // This is a simplified approach - in practice, you'd want to store member addresses more efficiently
        // For now, we'll distribute to group reserve
        
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
            Self::slash_collateral(env.clone(), caller.clone(), circle_id, member.clone());
        }

        // Auto-claim voucher if member has a guarantor
        if let Some(guarantor_addr) = &member_info.guarantor {
            let voucher_key = DataKey::Voucher(guarantor_addr.clone(), circle_id);
            if let Some(voucher_info) = env.storage().instance().get::<DataKey, VoucherInfo>(&voucher_key) {
                if voucher_info.status == VoucherStatus::Active {
                    // Automatically claim voucher to cover default
                    Self::claim_voucher(env.clone(), stored_admin.clone(), circle_id, member.clone());
                }
            }
        }
    }

// --- GUARANTOR FUNCTION IMPLEMENTATIONS ---
    // --- GUARANTOR FUNCTION IMPLEMENTATIONS ---

    fn register_guarantor(env: Env, user: Address, initial_collateral: i128) {
        user.require_auth();

        let guarantor_key = DataKey::Guarantor(user.clone());
        if env.storage().instance().has(&guarantor_key) {
            panic!("Already registered as guarantor");
        }

        if initial_collateral <= 0 {
            panic!("Initial collateral must be positive");
        }

        // Transfer initial collateral to contract
        // Note: In a real implementation, you'd need to get the token address
        // For now, we'll assume a default token or that it's passed as a parameter
        let token_address = Address::from_string(&String::from_str(&env, "TOKEN_ADDRESS_HERE"));
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&user, &env.current_contract_address(), &initial_collateral);

        let current_time = env.ledger().timestamp();
        let guarantor_info = GuarantorInfo {
            address: user.clone(),
            reputation_score: MIN_GUARANTOR_REPUTATION,
            total_vouched_amount: 0,
            active_vouchers_count: 0,
            successful_vouchers: 0,
            claimed_vouchers: 0,
            status: GuarantorStatus::Active,
            registered_timestamp: current_time,
            vault_balance: initial_collateral,
        };

        env.storage().instance().set(&guarantor_key, &guarantor_info);
        env.storage().instance().set(&DataKey::GuarantorVault(user.clone()), &initial_collateral);
        env.storage().instance().set(&DataKey::ActiveVouchersCount(user.clone()), &0u32);
    }

    fn update_guarantor_reputation(env: Env, admin: Address, guarantor: Address, new_score: u32) {
        admin.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }

        let guarantor_key = DataKey::Guarantor(guarantor.clone());
        let mut guarantor_info: GuarantorInfo = env.storage().instance().get(&guarantor_key)
            .expect("Guarantor not found");

        guarantor_info.reputation_score = new_score;
        
        // Update status based on reputation
        if new_score < MIN_GUARANTOR_REPUTATION {
            guarantor_info.status = GuarantorStatus::Suspended;
        } else if guarantor_info.status == GuarantorStatus::Suspended {
            guarantor_info.status = GuarantorStatus::Active;
        }

        env.storage().instance().set(&guarantor_key, &guarantor_info);
    }

    fn create_voucher(env: Env, guarantor: Address, member: Address, circle_id: u64, vouched_amount: i128) {
        guarantor.require_auth();

        // Validate guarantor
        let guarantor_key = DataKey::Guarantor(guarantor.clone());
        let mut guarantor_info: GuarantorInfo = env.storage().instance().get(&guarantor_key)
            .expect("Guarantor not found");

        if guarantor_info.status != GuarantorStatus::Active {
            panic!("Guarantor not active");
        }

        if guarantor_info.reputation_score < MIN_GUARANTOR_REPUTATION {
            panic!("Insufficient reputation");
        }

        if guarantor_info.active_vouchers_count >= MAX_VOUCHURES_PER_GUARANTOR {
            panic!("Guarantor overextended");
        }

        if guarantor == member {
            panic!("Self-guarantee not allowed");
        }

        // Check if voucher already exists
        let voucher_key = DataKey::Voucher(guarantor.clone(), circle_id);
        if env.storage().instance().has(&voucher_key) {
            panic!("Voucher already exists for this circle");
        }

        // Validate circle exists
        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        // Calculate required collateral
        let collateral_required = (vouched_amount * GUARANTOR_COLLATERAL_MULTIPLIER as i128) / 100;
        
        if guarantor_info.vault_balance < collateral_required {
            panic!("Insufficient guarantor collateral");
        }

        let current_time = env.ledger().timestamp();
        let voucher_info = VoucherInfo {
            guarantor: guarantor.clone(),
            member: member.clone(),
            circle_id,
            vouched_amount,
            status: VoucherStatus::Active,
            created_timestamp: current_time,
            claimed_timestamp: None,
            collateral_required,
        };

        // Update guarantor info
        guarantor_info.total_vouched_amount += vouched_amount;
        guarantor_info.active_vouchers_count += 1;
        guarantor_info.vault_balance -= collateral_required;

        // Store voucher and update guarantor
        env.storage().instance().set(&voucher_key, &voucher_info);
        env.storage().instance().set(&guarantor_key, &guarantor_info);
        env.storage().instance().set(&DataKey::GuarantorVault(guarantor.clone()), &guarantor_info.vault_balance);
        env.storage().instance().set(&DataKey::ActiveVouchersCount(guarantor.clone()), &guarantor_info.active_vouchers_count);

        // Update member with guarantor info
        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key)
            .expect("Member not found");
        member_info.guarantor = Some(guarantor.clone());
        env.storage().instance().set(&member_key, &member_info);
    }

    fn claim_voucher(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if caller != stored_admin && caller != member {
            panic!("Unauthorized");
        }

        // Get member info
        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key)
            .expect("Member not found");

        if member_info.status != MemberStatus::Defaulted {
            panic!("Member not defaulted");
        }

        let guarantor = member_info.guarantor.expect("No guarantor found");
        let voucher_key = DataKey::Voucher(guarantor.clone(), circle_id);
        let mut voucher_info: VoucherInfo = env.storage().instance().get(&voucher_key)
            .expect("Voucher not found");

        if voucher_info.status != VoucherStatus::Active {
            panic!("Voucher not active");
        }

        let guarantor_key = DataKey::Guarantor(guarantor.clone());
        let mut guarantor_info: GuarantorInfo = env.storage().instance().get(&guarantor_key)
            .expect("Guarantor not found");

        // Get circle info for token address
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        // Transfer collateral from guarantor vault to cover member's obligation
        let token_client = token::Client::new(&env, &circle.token);
        let amount_to_transfer = voucher_info.vouched_amount.min(voucher_info.collateral_required);

        // Get guarantor vault balance and transfer
        let vault_balance: i128 = env.storage().instance().get(&DataKey::GuarantorVault(guarantor.clone()))
            .expect("Guarantor vault not found");

        if vault_balance < amount_to_transfer {
            panic!("Insufficient funds in guarantor vault");
        }

        // Update voucher status
        voucher_info.status = VoucherStatus::Claimed;
        voucher_info.claimed_timestamp = Some(env.ledger().timestamp());

        // Update guarantor stats
        guarantor_info.active_vouchers_count -= 1;
        guarantor_info.claimed_vouchers += 1;
        guarantor_info.vault_balance -= amount_to_transfer;

        // Transfer funds to cover the defaulted member's obligation
        token_client.transfer(&env.current_contract_address(), &env.current_contract_address(), &amount_to_transfer);

        // Store updated information
        env.storage().instance().set(&voucher_key, &voucher_info);
        env.storage().instance().set(&guarantor_key, &guarantor_info);
        env.storage().instance().set(&DataKey::GuarantorVault(guarantor.clone()), &guarantor_info.vault_balance);
        env.storage().instance().set(&DataKey::ActiveVouchersCount(guarantor.clone()), &guarantor_info.active_vouchers_count);
    }

    fn add_guarantor_collateral(env: Env, guarantor: Address, amount: i128) {
        guarantor.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let guarantor_key = DataKey::Guarantor(guarantor.clone());
        let mut guarantor_info: GuarantorInfo = env.storage().instance().get(&guarantor_key)
            .expect("Guarantor not found");

        // Get token address (this should be stored or passed as parameter)
        let token_address = Address::from_string(&String::from_str(&env, "TOKEN_ADDRESS_HERE"));
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&guarantor, &env.current_contract_address(), &amount);

        guarantor_info.vault_balance += amount;

        env.storage().instance().set(&guarantor_key, &guarantor_info);
        env.storage().instance().set(&DataKey::GuarantorVault(guarantor.clone()), &guarantor_info.vault_balance);
    }

    fn withdraw_guarantor_collateral(env: Env, guarantor: Address, amount: i128) {
        guarantor.require_auth();

        if amount <= 0 {
            panic!("Amount must be positive");
        }

        let guarantor_key = DataKey::Guarantor(guarantor.clone());
        let mut guarantor_info: GuarantorInfo = env.storage().instance().get(&guarantor_key)
            .expect("Guarantor not found");

        // Check if withdrawing would leave insufficient collateral for active vouchers
        let required_collateral = guarantor_info.total_vouched_amount - 
            ((guarantor_info.successful_vouchers as i128) * guarantor_info.total_vouched_amount / 
             ((guarantor_info.successful_vouchers + guarantor_info.claimed_vouchers + 1) as i128));

        if guarantor_info.vault_balance - amount < required_collateral {
            panic!("Insufficient collateral available for withdrawal");
        }

        guarantor_info.vault_balance -= amount;

        // Get token address and transfer
        let token_address = Address::from_string(&String::from_str(&env, "TOKEN_ADDRESS_HERE"));
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&env.current_contract_address(), &guarantor, &amount);

        env.storage().instance().set(&guarantor_key, &guarantor_info);
        env.storage().instance().set(&DataKey::GuarantorVault(guarantor.clone()), &guarantor_info.vault_balance);
    }

    // --- QUERY FUNCTION IMPLEMENTATIONS ---

    fn get_guarantor_info(env: Env, guarantor: Address) -> GuarantorInfo {
        let guarantor_key = DataKey::Guarantor(guarantor);
        env.storage().instance().get(&guarantor_key).expect("Guarantor not found")
    }

    fn get_voucher_info(env: Env, guarantor: Address, circle_id: u64) -> VoucherInfo {
        let voucher_key = DataKey::Voucher(guarantor, circle_id);
        env.storage().instance().get(&voucher_key).expect("Voucher not found")
    }

    fn get_member_guarantor(env: Env, member: Address) -> Option<Address> {
        let member_key = DataKey::Member(member);
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        member_info.guarantor
    }

    fn get_guarantor_vault_balance(env: Env, guarantor: Address) -> i128 {
        let vault_key = DataKey::GuarantorVault(guarantor);
        env.storage().instance().get(&vault_key).unwrap_or(0)
    }

    fn get_reputation_hash(env: Env, user: Address) -> BytesN<32> {
        let member_key = DataKey::Member(user.clone());
        let member_info_opt: Option<Member> = env.storage().instance().get(&member_key);
        
        let has_good_reputation = match member_info_opt {
            Some(member_info) => {
                member_info.contribution_count > 3 && member_info.status == MemberStatus::Active
            },
            None => false,
        };
        
        let mut payload = Bytes::new(&env);
        if has_good_reputation {
            payload.push_back(1u32);
        } else {
            payload.push_back(0u32);
        }
        
        env.crypto().sha256(&payload)
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

        let circle = read_circle(&env, circle_id);
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let safety_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        let mut balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&safety_key, &balance);
    }

    fn get_circle(env: Env, circle_id: u64) -> CircleInfo {
        read_circle(&env, circle_id)
    }

    fn get_member(env: Env, circle_id: u64, member: Address) -> Member {
        require_circle_membership(&env, circle_id, &member);
        read_member(&env, &member)
    }

    fn get_members(env: Env, circle_id: u64) -> Vec<Address> {
        let circle = read_circle(&env, circle_id);
        let mut members = Vec::new(&env);

        for index in 0..circle.member_count {
            if let Some(member_address) = env
                .storage()
                .instance()
                .get::<DataKey, Address>(&DataKey::CircleMemberIndex(circle_id, index))
            {
                members.push_back(member_address);
            }
        }

        members
    }

    fn get_succession_proposal(env: Env, circle_id: u64) -> Option<SuccessionProposal> {
        read_circle(&env, circle_id);
        env.storage()
            .instance()
            .get(&DataKey::SuccessionProposal(circle_id))
    // --- LENIENCY VOTING IMPLEMENTATION ---

    fn request_leniency(env: Env, requester: Address, circle_id: u64, reason: String) {
        requester.require_auth();

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if !circle.leniency_enabled {
            panic!("Leniency not enabled for this circle");
        }

        let member_key = DataKey::Member(requester.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member_info.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let request_key = DataKey::LeniencyRequest(circle_id, requester.clone());
        if let Some(existing_request) = env.storage().instance().get::<DataKey, LeniencyRequest>(&request_key) {
            if existing_request.status == LeniencyRequestStatus::Pending {
                panic!("Leniency request already pending");
            }
        }

        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + VOTING_PERIOD;

        let new_request = LeniencyRequest {
            requester: requester.clone(),
            circle_id,
            request_timestamp: current_time,
            voting_deadline,
            status: LeniencyRequestStatus::Pending,
            approve_votes: 0,
            reject_votes: 0,
            total_votes_cast: 0,
            extension_hours: 48,
            reason,
        };

        env.storage().instance().set(&request_key, &new_request);

        let stats_key = DataKey::LeniencyStats(circle_id);
        let mut stats: LeniencyStats = env.storage().instance().get(&stats_key).unwrap_or(LeniencyStats {
            total_requests: 0,
            approved_requests: 0,
            rejected_requests: 0,
            expired_requests: 0,
            average_participation: 0,
        });
        stats.total_requests += 1;
        env.storage().instance().set(&stats_key, &stats);
    }

    fn vote_on_leniency(env: Env, voter: Address, circle_id: u64, requester: Address, vote: LeniencyVote) {
        voter.require_auth();

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let voter_key = DataKey::Member(voter.clone());
        let voter_info: Member = env.storage().instance().get(&voter_key).expect("Voter not found");

        if voter_info.status != MemberStatus::Active {
            panic!("Voter not active");
        }

        if voter == requester {
            panic!("Cannot vote for own request");
        }

        let request_key = DataKey::LeniencyRequest(circle_id, requester.clone());
        let mut request: LeniencyRequest = env.storage().instance().get(&request_key)
            .expect("Leniency request not found");

        if request.status != LeniencyRequestStatus::Pending {
            panic!("Voting period has ended");
        }

        let current_time = env.ledger().timestamp();
        if current_time > request.voting_deadline {
            request.status = LeniencyRequestStatus::Expired;
            env.storage().instance().set(&request_key, &request);
            panic!("Voting period expired");
        }

        let vote_key = DataKey::LeniencyVotes(circle_id, voter.clone(), requester.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        env.storage().instance().set(&vote_key, &vote);
        request.total_votes_cast += 1;

        match vote {
            LeniencyVote::Approve => request.approve_votes += 1,
            LeniencyVote::Reject => request.reject_votes += 1,
        }

        let social_capital_key = DataKey::SocialCapital(voter.clone(), circle_id);
        let mut social_capital: SocialCapital = env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
            member: voter.clone(),
            circle_id,
            leniency_given: 0,
            leniency_received: 0,
            voting_participation: 0,
            trust_score: 50,
        });
        social_capital.voting_participation += 1;
        
        if vote == LeniencyVote::Approve {
            social_capital.leniency_given += 1;
            social_capital.trust_score = (social_capital.trust_score + 2).min(100);
        } else {
            social_capital.trust_score = (social_capital.trust_score - 1).max(0);
        }
        
        env.storage().instance().set(&social_capital_key, &social_capital);

        let total_possible_votes = (circle.member_count - 1) as u32;
        let votes_needed_for_majority = (total_possible_votes * SIMPLE_MAJORITY_THRESHOLD) / 100;
        
        if request.approve_votes >= votes_needed_for_majority {
            request.status = LeniencyRequestStatus::Approved;
            SoroSusu::finalize_leniency_vote_internal(&env, &circle_id, &requester, &mut request);
        } else if request.reject_votes >= votes_needed_for_majority {
            request.status = LeniencyRequestStatus::Rejected;
        }

        env.storage().instance().set(&request_key, &request);
    }

    fn finalize_leniency_vote(env: Env, caller: Address, circle_id: u64, requester: Address) {
        caller.require_auth();

        let request_key = DataKey::LeniencyRequest(circle_id, requester.clone());
        let mut request: LeniencyRequest = env.storage().instance().get(&request_key)
            .expect("Leniency request not found");

        if request.status != LeniencyRequestStatus::Pending {
            panic!("Request already finalized");
        }

        let current_time = env.ledger().timestamp();
        if current_time <= request.voting_deadline {
            panic!("Voting period not yet expired");
        }

        SoroSusu::finalize_leniency_vote_internal(&env, &circle_id, &requester, &mut request);
        env.storage().instance().set(&request_key, &request);
    }

    fn get_leniency_request(env: Env, circle_id: u64, requester: Address) -> LeniencyRequest {
        let request_key = DataKey::LeniencyRequest(circle_id, requester);
        env.storage().instance().get(&request_key).expect("Leniency request not found")
    }

    fn get_social_capital(env: Env, member: Address, circle_id: u64) -> SocialCapital {
        let social_capital_key = DataKey::SocialCapital(member.clone(), circle_id);
        env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
            member,
            circle_id,
            leniency_given: 0,
            leniency_received: 0,
            voting_participation: 0,
            trust_score: 50,
        })
    }

    fn get_leniency_stats(env: Env, circle_id: u64) -> LeniencyStats {
        let stats_key = DataKey::LeniencyStats(circle_id);
        env.storage().instance().get(&stats_key).unwrap_or(LeniencyStats {
            total_requests: 0,
            approved_requests: 0,
            rejected_requests: 0,
            expired_requests: 0,
            average_participation: 0,
        })
    }

    fn create_proposal(
        env: Env,
        proposer: Address,
        circle_id: u64,
        proposal_type: ProposalType,
        title: String,
        description: String,
        execution_data: String,
    ) -> u64 {
        proposer.require_auth();

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if !circle.quadratic_voting_enabled {
            panic!("Quadratic voting not enabled for this circle");
        }

        let member_key = DataKey::Member(proposer.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member_info.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let current_time = env.ledger().timestamp();
        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        proposal_count += 1;

        let new_proposal = Proposal {
            id: proposal_count,
            circle_id,
            proposer: proposer.clone(),
            proposal_type,
            title,
            description,
            created_timestamp: current_time,
            voting_start_timestamp: current_time,
            voting_end_timestamp: current_time + QUADRATIC_VOTING_PERIOD,
            status: ProposalStatus::Active,
            for_votes: 0,
            against_votes: 0,
            total_voting_power: 0,
            quorum_met: false,
            execution_data,
        };

        env.storage().instance().set(&DataKey::Proposal(proposal_count), &new_proposal);

        let mut circle_info = circle;
        circle_info.proposal_count += 1;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle_info);

        let stats_key = DataKey::ProposalStats(circle_id);
        let mut stats: ProposalStats = env.storage().instance().get(&stats_key).unwrap_or(ProposalStats {
            total_proposals: 0,
            approved_proposals: 0,
            rejected_proposals: 0,
            executed_proposals: 0,
            average_participation: 0,
            average_voting_time: 0,
        });
        stats.total_proposals += 1;
        env.storage().instance().set(&stats_key, &stats);

        proposal_count
    }

    fn quadratic_vote(env: Env, voter: Address, proposal_id: u64, vote_weight: u32, vote_choice: QuadraticVoteChoice) {
        voter.require_auth();

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env.storage().instance().get(&proposal_key)
            .expect("Proposal not found");

        if proposal.status != ProposalStatus::Active {
            panic!("Voting not active for this proposal");
        }

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_end_timestamp {
            proposal.status = ProposalStatus::Expired;
            env.storage().instance().set(&proposal_key, &proposal);
            panic!("Voting period expired");
        }

        let vote_key = DataKey::QuadraticVote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted on this proposal");
        }

        let voting_power_key = DataKey::VotingPower(voter.clone(), proposal.circle_id);
        let voting_power: VotingPower = env.storage().instance().get(&voting_power_key)
            .expect("Voting power not calculated");

        if vote_weight > MAX_VOTE_WEIGHT {
            panic!("Vote weight exceeds maximum");
        }

        let voting_cost = (vote_weight as u64) * (vote_weight as u64);
        
        if voting_cost > voting_power.quadratic_power {
            panic!("Insufficient voting power");
        }

        let quadratic_vote = QuadraticVote {
            voter: voter.clone(),
            proposal_id,
            vote_weight,
            vote_choice: vote_choice.clone(),
            voting_power_used: voting_cost,
            timestamp: current_time,
        };

        env.storage().instance().set(&vote_key, &quadratic_vote);

        match vote_choice {
            QuadraticVoteChoice::For => {
                proposal.for_votes += voting_cost;
            }
            QuadraticVoteChoice::Against => {
                proposal.against_votes += voting_cost;
            }
            QuadraticVoteChoice::Abstain => {}
        }

        proposal.total_voting_power += voting_cost;

        let circle_key = DataKey::Circle(proposal.circle_id);
        let circle: CircleInfo = env.storage().instance().get(&circle_key).expect("Circle not found");
        let required_quorum = (circle.member_count * QUADRATIC_QUORUM) / 100;
        proposal.quorum_met = proposal.total_voting_power >= required_quorum as u64;

        env.storage().instance().set(&proposal_key, &proposal);
    }

    fn execute_proposal(env: Env, caller: Address, proposal_id: u64) {
        caller.require_auth();

        let proposal_key = DataKey::Proposal(proposal_id);
        let mut proposal: Proposal = env.storage().instance().get(&proposal_key)
            .expect("Proposal not found");

        if proposal.status != ProposalStatus::Active {
            panic!("Proposal not active");
        }

        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_end_timestamp {
            panic!("Voting period not yet ended");
        }

        if !proposal.quorum_met {
            proposal.status = ProposalStatus::Rejected;
            env.storage().instance().set(&proposal_key, &proposal);
            panic!("Quorum not met");
        }

        let total_votes = proposal.for_votes + proposal.against_votes;
        if total_votes == 0 {
            proposal.status = ProposalStatus::Rejected;
        } else {
            let approval_percentage = (proposal.for_votes * 100) / total_votes;
            if approval_percentage >= QUADRATIC_MAJORITY as u64 {
                proposal.status = ProposalStatus::Approved;
                SoroSusu::execute_proposal_logic(&env, &proposal);
            } else {
                proposal.status = ProposalStatus::Rejected;
            }
        }

        env.storage().instance().set(&proposal_key, &proposal);

        let stats_key = DataKey::ProposalStats(proposal.circle_id);
        let mut stats: ProposalStats = env.storage().instance().get(&stats_key).unwrap_or(ProposalStats {
            total_proposals: 0,
            approved_proposals: 0,
            rejected_proposals: 0,
            executed_proposals: 0,
            average_participation: 0,
            average_voting_time: 0,
        });

        match proposal.status {
            ProposalStatus::Approved => stats.approved_proposals += 1,
            ProposalStatus::Rejected => stats.rejected_proposals += 1,
            ProposalStatus::Executed => stats.executed_proposals += 1,
            _ => {}
        }

        env.storage().instance().set(&stats_key, &stats);
    }

    fn get_proposal(env: Env, proposal_id: u64) -> Proposal {
        let proposal_key = DataKey::Proposal(proposal_id);
        env.storage().instance().get(&proposal_key).expect("Proposal not found")
    }

    fn get_voting_power(env: Env, member: Address, circle_id: u64) -> VotingPower {
        let voting_power_key = DataKey::VotingPower(member.clone(), circle_id);
        env.storage().instance().get(&voting_power_key).unwrap_or(VotingPower {
            member,
            circle_id,
            token_balance: 0,
            quadratic_power: 0,
            last_updated: 0,
        })
    }

    fn get_proposal_stats(env: Env, circle_id: u64) -> ProposalStats {
        let stats_key = DataKey::ProposalStats(circle_id);
        env.storage().instance().get(&stats_key).unwrap_or(ProposalStats {
            total_proposals: 0,
            approved_proposals: 0,
            rejected_proposals: 0,
            executed_proposals: 0,
            average_participation: 0,
            average_voting_time: 0,
        })
    }

    fn update_voting_power(env: Env, member: Address, circle_id: u64, token_balance: i128) {
        let quadratic_power = if token_balance > 0 {
            let balance_u64 = token_balance as u64;
            (balance_u64 / 1000).max(1) 
        } else {
            0
        };

        let voting_power = VotingPower {
            member: member.clone(),
            circle_id,
            token_balance,
            quadratic_power,
            last_updated: env.ledger().timestamp(),
        };

        env.storage().instance().set(&DataKey::VotingPower(member, circle_id), &voting_power);
    }

    fn stake_collateral(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        if !circle.requires_collateral {
            panic!("Collateral not required for this circle");
        }

        let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
        
        if let Some(_collateral) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
            panic!("Collateral already staked");
        }

        let required_collateral = (circle.total_cycle_value * circle.collateral_bps as i128) / 10000;
        
        if amount < required_collateral {
            panic!("Insufficient collateral amount");
        }

        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

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
        SoroSusu::_slash_collateral(env, caller, circle_id, member);
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

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        
        if member_info.contribution_count < circle.max_members {
            panic!("Member has not completed all contributions");
        }

        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &member, &collateral_info.amount);

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

        member_info.status = MemberStatus::Defaulted;
        env.storage().instance().set(&member_key, &member_info);

        env.storage().instance().set(&DataKey::Blacklist(member.clone()), &true);

        let defaulted_key = DataKey::DefaultedMembers(circle_id);
        let mut defaulted_members: Vec<Address> = env.storage().instance().get(&defaulted_key).unwrap_or(Vec::new(&env));
        
        if !defaulted_members.contains(&member) {
            defaulted_members.push_back(member.clone());
            env.storage().instance().set(&defaulted_key, &defaulted_members);
        }

        let collateral_key = DataKey::CollateralVault(member.clone(), circle_id);
        if let Some(_collateral) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
            SoroSusu::_slash_collateral(env, caller.clone(), circle_id, member);
            // Reuse slash_collateral logic
            Self::_slash_collateral(env, caller, circle_id, member);
        }
        
        // Check and slash vouch collateral if member was vouched for
        let vouch_key = DataKey::VouchCollateral(member.clone(), circle_id);
        if let Some(vouch_id) = env.storage().instance().get::<DataKey, u64>(&vouch_key) {
            // Find the voucher by checking all vouch records
            // In practice, you'd want a more efficient lookup, but this works for demonstration
            // We'll need to iterate through potential vouchers or store a reverse mapping
            // For now, we'll assume we can find the voucher and slash their collateral
            
            // This would require additional storage structure to efficiently find the voucher
            // For implementation, we'll add a reverse mapping
        }
    }
    
    // --- SOCIAL VOUCHING IMPLEMENTATION ---
    
    fn vouch_for_member(env: Env, voucher: Address, vouchee: Address, circle_id: u64, collateral_amount: i128) {
        voucher.require_auth();
        
        // Prevent self-vouching
        if voucher == vouchee {
            panic!("Cannot vouch for self");
        }
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        // Check if voucher is an active member
        let voucher_key = DataKey::Member(voucher.clone());
        let voucher_info: Member = env.storage().instance().get(&voucher_key).expect("Voucher not found");
        
        if voucher_info.status != MemberStatus::Active {
            panic!("Voucher not active");
        }
        
        // Check if vouchee is already a member
        let vouchee_key = DataKey::Member(vouchee.clone());
        if env.storage().instance().has(&vouchee_key) {
            panic!("Vouchee already member");
        }
        
        // Check voucher's trust score
        let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
        let social_capital: SocialCapital = env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
            member: voucher.clone(),
            circle_id,
            leniency_given: 0,
            leniency_received: 0,
            voting_participation: 0,
            trust_score: 50,
        });
        
        if social_capital.trust_score < MIN_TRUST_SCORE_FOR_VOUCH {
            panic!("Insufficient trust score to vouch");
        }
        
        // Check if vouch already exists
        let vouch_record_key = DataKey::VouchRecord(voucher.clone(), vouchee.clone());
        if env.storage().instance().has(&vouch_record_key) {
            panic!("Vouch already exists");
        }
        
        // Check vouch limits
        let vouch_stats_key = DataKey::VouchStats(voucher.clone());
        let vouch_stats: VouchStats = env.storage().instance().get(&vouch_stats_key).unwrap_or(VouchStats {
            voucher: voucher.clone(),
            total_vouches_made: 0,
            active_vouches: 0,
            successful_vouches: 0,
            slashed_vouches: 0,
            total_collateral_locked: 0,
            total_collateral_lost: 0,
        });
        
        if vouch_stats.active_vouches >= MAX_VOUCHES_PER_MEMBER {
            panic!("Maximum active vouches exceeded");
        }
        
        // Calculate minimum required collateral
        let min_collateral = (circle.total_cycle_value * VOUCH_COLLATERAL_MULTIPLIER as i128) / 10000;
        if collateral_amount < min_collateral {
            panic!("Insufficient collateral amount");
        }
        
        // Transfer collateral from voucher to contract
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&voucher, &env.current_contract_address(), &collateral_amount);
        
        let current_time = env.ledger().timestamp();
        let expiry_time = current_time + VOUCH_EXPIRY_SECONDS;
        
        // Create vouch record
        let vouch_record = VouchRecord {
            voucher: voucher.clone(),
            vouchee: vouchee.clone(),
            circle_id,
            collateral_amount,
            vouch_timestamp: current_time,
            expiry_timestamp: expiry_time,
            status: VouchStatus::Active,
            slash_count: 0,
        };
        
        env.storage().instance().set(&vouch_record_key, &vouch_record);
        
        // Store reverse mapping for efficient lookup
        let vouch_collateral_key = DataKey::VouchCollateral(vouchee.clone(), circle_id);
        env.storage().instance().set(&vouch_collateral_key, &circle_id); // Use circle_id as vouch_id for simplicity
        
        // Store reverse mapping to find voucher by vouchee
        let reverse_mapping_key = DataKey::VouchReverseMapping(vouchee.clone(), circle_id);
        env.storage().instance().set(&reverse_mapping_key, &voucher);
        
        // Update voucher stats
        let mut updated_stats = vouch_stats;
        updated_stats.total_vouches_made += 1;
        updated_stats.active_vouches += 1;
        updated_stats.total_collateral_locked += collateral_amount;
        env.storage().instance().set(&vouch_stats_key, &updated_stats);
        
        // Update voucher's social capital (vouching increases trust score)
        let mut updated_social_capital = social_capital;
        updated_social_capital.trust_score = (updated_social_capital.trust_score + 3).min(100);
        env.storage().instance().set(&social_capital_key, &updated_social_capital);
    }
    
    fn slash_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }
        
        // Find the voucher using reverse mapping
        let reverse_mapping_key = DataKey::VouchReverseMapping(vouchee.clone(), circle_id);
        let voucher: Address = env.storage().instance().get(&reverse_mapping_key).expect("No vouch found");
        
        // Get the vouch record
        let vouch_record_key = DataKey::VouchRecord(voucher.clone(), vouchee.clone());
        let mut vouch_record: VouchRecord = env.storage().instance().get(&vouch_record_key).expect("Vouch record not found");
        
        if vouch_record.status != VouchStatus::Active {
            panic!("Vouch not active");
        }
        
        // Check if vouchee is defaulted
        let vouchee_key = DataKey::Member(vouchee.clone());
        if let Some(vouchee_info) = env.storage().instance().get::<DataKey, Member>(&vouchee_key) {
            if vouchee_info.status != MemberStatus::Defaulted {
                panic!("Vouchee not defaulted");
            }
        } else {
            panic!("Vouchee not found");
        }
        
        // Transfer collateral to group reserve
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &env.current_contract_address(), &vouch_record.collateral_amount);
        
        // Update group reserve
        let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        reserve += vouch_record.collateral_amount;
        env.storage().instance().set(&DataKey::GroupReserve, &reserve);
        
        // Update vouch record
        vouch_record.status = VouchStatus::Slashed;
        vouch_record.slash_count += 1;
        env.storage().instance().set(&vouch_record_key, &vouch_record);
        
        // Update voucher stats
        let vouch_stats_key = DataKey::VouchStats(voucher.clone());
        let mut vouch_stats: VouchStats = env.storage().instance().get(&vouch_stats_key).expect("Vouch stats not found");
        vouch_stats.active_vouches -= 1;
        vouch_stats.slashed_vouches += 1;
        vouch_stats.total_collateral_lost += vouch_record.collateral_amount;
        env.storage().instance().set(&vouch_stats_key, &vouch_stats);
        
        // Decrease voucher's trust score due to slash
        let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
        let mut social_capital: SocialCapital = env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
            member: voucher.clone(),
            circle_id,
            leniency_given: 0,
            leniency_received: 0,
            voting_participation: 0,
            trust_score: 50,
        });
        social_capital.trust_score = (social_capital.trust_score - 10).max(0); // Significant penalty for slash
        env.storage().instance().set(&social_capital_key, &social_capital);
        
        // Clean up reverse mapping
        env.storage().instance().remove(&reverse_mapping_key);
        let vouch_collateral_key = DataKey::VouchCollateral(vouchee.clone(), circle_id);
        env.storage().instance().remove(&vouch_collateral_key);
    }
    
    fn release_vouch_collateral(env: Env, caller: Address, circle_id: u64, vouchee: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }
        
        // Find the voucher using reverse mapping
        let reverse_mapping_key = DataKey::VouchReverseMapping(vouchee.clone(), circle_id);
        let voucher: Address = env.storage().instance().get(&reverse_mapping_key).expect("No vouch found");
        
        // Get the vouch record
        let vouch_record_key = DataKey::VouchRecord(voucher.clone(), vouchee.clone());
        let mut vouch_record: VouchRecord = env.storage().instance().get(&vouch_record_key).expect("Vouch record not found");
        
        if vouch_record.status != VouchStatus::Active {
            panic!("Vouch not active");
        }
        
        // Check if vouchee has completed all contributions
        let vouchee_key = DataKey::Member(vouchee.clone());
        let vouchee_info: Member = env.storage().instance().get(&vouchee_key).expect("Vouchee not found");
        
        if vouchee_info.contribution_count < circle.max_members {
            panic!("Vouchee has not completed all contributions");
        }
        
        // Return collateral to voucher
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&env.current_contract_address(), &voucher, &vouch_record.collateral_amount);
        
        // Update vouch record
        vouch_record.status = VouchStatus::Completed;
        env.storage().instance().set(&vouch_record_key, &vouch_record);
        
        // Update voucher stats
        let vouch_stats_key = DataKey::VouchStats(voucher.clone());
        let mut vouch_stats: VouchStats = env.storage().instance().get(&vouch_stats_key).expect("Vouch stats not found");
        vouch_stats.active_vouches -= 1;
        vouch_stats.successful_vouches += 1;
        env.storage().instance().set(&vouch_stats_key, &vouch_stats);
        
        // Increase voucher's trust score due to successful vouch
        let social_capital_key = DataKey::SocialCapital(voucher.clone(), circle_id);
        let mut social_capital: SocialCapital = env.storage().instance().get(&social_capital_key).unwrap_or(SocialCapital {
            member: voucher.clone(),
            circle_id,
            leniency_given: 0,
            leniency_received: 0,
            voting_participation: 0,
            trust_score: 50,
        });
        social_capital.trust_score = (social_capital.trust_score + 5).min(100); // Bonus for successful vouch
        env.storage().instance().set(&social_capital_key, &social_capital);
        
        // Clean up reverse mapping
        env.storage().instance().remove(&reverse_mapping_key);
        let vouch_collateral_key = DataKey::VouchCollateral(vouchee.clone(), circle_id);
        env.storage().instance().remove(&vouch_collateral_key);
    }
    
    fn get_vouch_record(env: Env, voucher: Address, vouchee: Address) -> VouchRecord {
        let vouch_record_key = DataKey::VouchRecord(voucher, vouchee);
        env.storage().instance().get(&vouch_record_key).expect("Vouch record not found")
    }
    
    fn get_vouch_stats(env: Env, voucher: Address) -> VouchStats {
        let vouch_stats_key = DataKey::VouchStats(voucher);
        env.storage().instance().get(&vouch_stats_key).unwrap_or(VouchStats {
            voucher,
            total_vouches_made: 0,
            active_vouches: 0,
            successful_vouches: 0,
            slashed_vouches: 0,
            total_collateral_locked: 0,
            total_collateral_lost: 0,
        })
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

        let on_time_ratio = (rep.on_time_contributions * 400) / rep.total_contributions;
        
        let cycles_score = if rep.cycles_completed >= 10 {
            300
        } else {
            rep.cycles_completed * 30
        };

        let normalized_volume = (rep.total_volume / 10_000_000) as u32; // Normalizing to 1 XLM units
        let volume_score = if normalized_volume >= 10000 {
            300
        } else {
            (normalized_volume * 300) / 10000
        };

        on_time_ratio + cycles_score + volume_score
    }

    fn is_blacklisted(env: Env, user: Address) -> bool {
        env.storage().instance().get(&DataKey::Blacklist(user)).unwrap_or(false)
    }

    fn update_blacklist_status(env: Env, admin: Address, user: Address, status: bool) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::Blacklist(user), &status);
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
