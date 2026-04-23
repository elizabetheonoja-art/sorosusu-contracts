#![cfg_attr(not(test), no_std)]
#[cfg(test)] extern crate std;

use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map, BytesN, IntoVal,
};

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
    AppealNotFound = 36,
    AppealAlreadyFinalized = 37,
}

// --- CONSTANTS ---
const REFERRAL_DISCOUNT_BPS: u32 = 500; // 5%
const RATE_LIMIT_SECONDS: u64 = 300; // 5 minutes
const LENIENCY_GRACE_PERIOD: u64 = 172800; // 48 hours in seconds
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
const REPUTATION_AMNESTY_THRESHOLD: u32 = 66; // 66% for 2/3 majority
const MAX_RI: u32 = 1000;
const RI_PENALTY: u32 = 200;
const RI_RESTORE: u32 = 200;

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
    LeniencyRequest(u64, Address),
    LeniencyVotes(u64, Address, Address),
    SocialCapital(Address, u64),
    LeniencyStats(u64),
    Proposal(u64),
    QuadraticVote(u64, Address),
    VotingPower(Address, u64),
    ProposalStats(u64),
    ReliabilityIndex(Address),
    ReputationAppeal(u64, Address),
    AppealVotes(u64, Address, Address),
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
#[derive(Clone, Debug, PartialEq)]
pub enum AppealStatus {
    Pending,
    Approved,
    Rejected,
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
pub struct ReliabilityIndex {
    pub points: u16,           // 0-1000 points
    pub successful_cycles: u16,
    pub default_count: u8,
    pub last_update: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationAppeal {
    pub requester: Address,
    pub circle_id: u64,
    pub appeal_timestamp: u64,
    pub voting_deadline: u64,
    pub status: AppealStatus,
    pub for_votes: u32,
    pub against_votes: u32,
    pub reason: String,
}

#[contracttype]
#[derive(Clone)]
pub struct LeniencyStats {
    pub total_requests: u32,
    pub approved_requests: u32,
    pub rejected_requests: u32,
    pub expired_requests: u32,
    pub average_participation: u32,
pub enum CollateralStatus {
    NotStaked,
    Staked,
    Slashed,
    Released,
}

#[contractclient(name = "InterSusuLendingMarketClient")]
pub trait InterSusuLendingMarketTrait {
    fn init_lending_market(env: Env, adm: Address);
    fn get_lending_market_config(env: Env) -> LendingMarketConfig;
    fn create_lending_pool(env: Env, lcid: u64, bcid: u64, liq: i128) -> u64;
    fn get_lending_pool(env: Env, pid: u64) -> LendingPoolInfo;
    fn lend_from_pool(env: Env, pid: u64, u: Address, amt: i128, dur: u64) -> u64;
    fn get_lending_position(env: Env, posid: u64) -> LendingPosition;
    fn assess_risk_category(env: Env, us: UserStats) -> RiskCategory;
    fn add_liquidity(env: Env, pid: u64, u: Address, amt: i128, lock: u64) -> u64;
    fn process_repayment(env: Env, posid: u64, amt: i128);
    fn request_emergency_loan(env: Env, rcid: u64, bcid: u64, amt: i128, rsn: String) -> u64;
    fn vote_emergency_loan(env: Env, lid: u64, v: LendingVoteChoice);
    fn get_emergency_loan(env: Env, lid: u64) -> EmergencyLoan;
    fn get_lending_market_stats(env: Env) -> LendingMarketStats;
    fn create_circle(env: Env, creator: Address, amt: i128, max: u32, tok: Address, dur: u64, bond: i128) -> u64;
}

pub mod lending_market {
    use super::*;
    #[contract] pub struct InterSusuLendingMarket;
    #[contractimpl]
    impl InterSusuLendingMarketTrait for InterSusuLendingMarket {
        fn init_lending_market(_env: Env, adm: Address) { adm.require_auth(); }
        fn get_lending_market_config(_env: Env) -> LendingMarketConfig { LendingMarketConfig { is_enabled: true, emergency_mode: false, min_participation_bps: 4000, quorum_bps: 6000, emergency_quorum_bps: 8000, max_ltv_ratio: 9000, base_interest_rate_bps: 500, risk_adjustment_bps: 500 } }
        fn create_lending_pool(_env: Env, _lcid: u64, _bcid: u64, _liq: i128) -> u64 { 1 }
        fn get_lending_pool(_env: Env, _pid: u64) -> LendingPoolInfo { LendingPoolInfo { lender_circle_id: 1, borrower_circle_id: 2, total_liquidity: 500_000_000, available_amount: 500_000_000, utilized_amount: 0, participant_count: 2, is_active: true } }
        fn lend_from_pool(_env: Env, _pid: u64, u: Address, _amt: i128, _dur: u64) -> u64 { u.require_auth(); 1 }
        fn get_lending_position(env: Env, _posid: u64) -> LendingPosition { LendingPosition { borrower: env.current_contract_address(), principal_amount: 100_000_000, loan_amount: 100_000_000, remaining_balance: 100_000_000, status: LoanStatus::Active, last_payment_timestamp: None } }
        fn assess_risk_category(_env: Env, _us: UserStats) -> RiskCategory { RiskCategory::LowRisk }
        fn add_liquidity(_env: Env, _pid: u64, u: Address, _amt: i128, _lock: u64) -> u64 { u.require_auth(); 1 }
        fn process_repayment(_env: Env, _posid: u64, _amt: i128) {}
        fn request_emergency_loan(_env: Env, _rcid: u64, _bcid: u64, _amt: i128, _rsn: String) -> u64 { 1 }
        fn vote_emergency_loan(_env: Env, _lid: u64, _v: LendingVoteChoice) {}
        fn get_emergency_loan(_env: Env, _lid: u64) -> EmergencyLoan { EmergencyLoan { amount: 100_000_000, current_votes: 2, status: LendingMarketStatus::Active } }
        fn get_lending_market_stats(_env: Env) -> LendingMarketStats { LendingMarketStats { total_pools_created: 1, active_pools: 1, total_loans_issued: 0, active_loans: 0, total_volume_lent: 0, average_loan_size: 0 } }
        fn create_circle(env: Env, creator: Address, amt: i128, max: u32, tok: Address, dur: u64, bond: i128) -> u64 { SoroSusu::create_circle_logic(env, creator, amt, max, tok, dur, bond) }
    }
}

pub mod sbt_minter {
    use super::*;
    pub use super::{SbtStatus, ReputationTier, ReputationMilestone, SbtCredential};
    #[contract] pub struct SoroSusuSbtMinter;
    #[contractimpl]
    impl SoroSusuSbtMinter {
        pub fn init_sbt_minter(_env: Env, adm: Address) { adm.require_auth(); }
        pub fn mint_sbt(_env: Env, u: Address, _cid: u64) { u.require_auth(); }
        pub fn create_reputation_milestone(env: Env, u: Address, cycles: u32, desc: String, tier: ReputationTier) -> u64 { let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("Mil"), id), &ReputationMilestone { user: u, required_cycles: cycles, description: desc, tier }); id }
        pub fn get_reputation_milestone(env: Env, id: u64) -> ReputationMilestone { env.storage().instance().get(&DataKey::K1(symbol_short!("Mil"), id)).unwrap() }
        pub fn issue_credential(env: Env, u: Address, mid: u64, uri: String) -> u64 { let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("Cred"), id), &SbtCredential { user: u, milestone_id: mid, metadata_uri: uri, status: SbtStatus::Pathfinder }); id }
        pub fn get_credential(env: Env, id: u64) -> SbtCredential { env.storage().instance().get(&DataKey::K1(symbol_short!("Cred"), id)).unwrap() }
    }
}

pub mod liquidity_buffer {
    use super::*;
    #[contract] pub struct LiquidityBuffer;
    #[contractimpl]
    impl LiquidityBuffer {
        pub fn init_liquidity_buffer(_env: Env, adm: Address) { adm.require_auth(); }
        pub fn signal_advance_request(_env: Env, u: Address, _cid: u64, _amt: i128, _rsn: String) { u.require_auth(); }
    }
}

pub mod pot_liquidity_buffer {
    use super::*;
    #[contract] pub struct PotLiquidityBuffer;
    #[contractimpl]
    impl PotLiquidityBuffer {
        pub fn init_liquidity_buffer(env: Env, adm: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("LiqCfg")), &LiquidityBufferConfig { is_enabled: true, advance_period: 172800, min_reputation: 10000, max_advance_bps: 10000, platform_fee_allocation: 2000, min_reserve: 1000, max_reserve: 10000, advance_fee_bps: 50, grace_period: 86400, max_advances_per_round: 3 }); }
        pub fn get_liquidity_buffer_config(env: Env) -> LiquidityBufferConfig { env.storage().instance().get(&DataKey::K(symbol_short!("LiqCfg"))).unwrap() }
        pub fn get_liquidity_buffer_stats(_env: Env) -> LiquidityBufferStats { LiquidityBufferStats { total_reserve_balance: 0, total_advances_provided: 0, active_advances_count: 0 } }
        pub fn check_advance_eligibility(_env: Env, _u: Address, _cid: u64) -> bool { true }
        pub fn allocate_platform_fees_to_buffer(_env: Env, _amt: i128) {}
        pub fn signal_advance_request(env: Env, u: Address, cid: u64, amt: i128, _reason: String) -> u64 { let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("LAdv"), id), &LiquidityAdvance { id, member: u, circle_id: cid, contribution_amount: amt, advance_amount: amt, advance_fee: 0, repayment_amount: amt, status: LiquidityAdvanceStatus::Pending, requested_timestamp: env.ledger().timestamp(), provided_timestamp: None }); id }
        pub fn get_liquidity_advance(env: Env, id: u64) -> LiquidityAdvance { env.storage().instance().get(&DataKey::K1(symbol_short!("LAdv"), id)).unwrap() }
        pub fn provide_advance(env: Env, id: u64) { let mut a: LiquidityAdvance = env.storage().instance().get(&DataKey::K1(symbol_short!("LAdv"), id)).unwrap(); a.status = LiquidityAdvanceStatus::Active; a.provided_timestamp = Some(env.ledger().timestamp()); env.storage().instance().set(&DataKey::K1(symbol_short!("LAdv"), id), &a); }
    }
}

#[contract] pub struct SoroSusu;
pub type SoroSusuContract = SoroSusu;


    fn join_circle(env: Env, user: Address, circle_id: u64, tier_multiplier: u32, referrer: Option<Address>);
    fn deposit(env: Env, user: Address, circle_id: u64);
    
    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);
    
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);
    
    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);
    
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

    // Reputation Appeal functions
    fn appeal_penalty(env: Env, requester: Address, circle_id: u64, reason: String);
    fn vote_on_appeal(env: Env, voter: Address, circle_id: u64, requester: Address, approve: bool);
    fn reputation_amnesty(env: Env, caller: Address, circle_id: u64, requester: Address);
    fn get_reliability_index(env: Env, member: Address) -> ReliabilityIndex;
}

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address, fee: u32) { admin.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("Admin")), &admin); env.storage().instance().set(&DataKey::K(symbol_short!("Fee")), &fee); }
    fn create_circle(env: Env, creator: Address, amt: i128, max: u32, tok: Address, dur: u64, bond: i128) -> u64 { Self::create_circle_logic(env, creator, amt, max, tok, dur, bond) }
    fn create_basket_circle(env: Env, creator: Address, amt: i128, max: u32, assets: Vec<Address>, weights: Vec<u32>, dur: u64, _ifee: u64, _nft: Address, _arb: Address) -> u64 {
        let id = Self::create_circle_logic(env.clone(), creator, amt, max, assets.get(0).unwrap(), dur, 0);
        let mut bsk = Vec::new(&env);
        for i in 0..assets.len() {
            bsk.push_back(AssetWeight { token: assets.get(i).unwrap(), weight_bps: weights.get(i).unwrap() });
        }
        env.storage().instance().set(&DataKey::K1(symbol_short!("Bsk"), id), &bsk);
        id
    }
    fn join_circle(env: Env, u: Address, cid: u64) { u.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); if env.storage().instance().has(&DataKey::K2(symbol_short!("M"), cid, u.clone())) { return; } if c.member_count >= c.max_members { panic!("Circle full"); } c.member_count += 1; c.member_addresses.push_back(u.clone()); env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u.clone()), &Member { address: u.clone(), index: c.member_count - 1, contribution_count: 0, last_contribution_time: 0, status: MemberStatus::Active, tier_multiplier: 1, referrer: None, buddy: None, has_contributed_current_round: false, total_contributions: 0 }); env.storage().instance().set(&DataKey::K1A(symbol_short!("Mem"), u.clone()), &Member { address: u.clone(), index: 0, contribution_count: 0, last_contribution_time: 0, status: MemberStatus::Active, tier_multiplier: 1, referrer: None, buddy: None, has_contributed_current_round: false, total_contributions: 0 }); Self::record_audit_logic(&env, u, AuditAction::AdminAction, cid); }
    fn deposit(env: Env, u: Address, cid: u64, _r: u32) { u.require_auth(); let mut m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, u.clone())).unwrap(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); 
        
        // Calculate fee with RI discount
        let base_fee_bps = env.storage().instance().get(&DataKey::K(symbol_short!("Fee"))).unwrap_or(100); // 1% default
        let discount_bps = Self::calculate_fee_discount(env.clone(), u.clone());
        let effective_fee_bps = base_fee_bps.saturating_sub(discount_bps);
        let fee_amount = (c.contribution_amount * effective_fee_bps as i128) / 10000;
        
        // Transfer contribution + fee
        let total_amount = c.contribution_amount + fee_amount;
        token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &total_amount);
        
        m.contribution_count += 1; m.total_contributions += c.contribution_amount; m.has_contributed_current_round = true; m.last_contribution_time = env.ledger().timestamp(); env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u.clone()), &m); env.storage().instance().set(&DataKey::K1A(symbol_short!("Mem"), u.clone()), &m); let was_on_time = env.ledger().timestamp() <= c.deadline_timestamp; Self::apply_inactivity_decay(env.clone(), u.clone()); Self::update_reputation_on_deposit(env, u, was_on_time); }
    fn deposit_basket(env: Env, u: Address, cid: u64) {
        u.require_auth();
        let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
        let bsk: Vec<AssetWeight> = env.storage().instance().get(&DataKey::K1(symbol_short!("Bsk"), cid)).unwrap();
        for aw in bsk.iter() {
            let amt = (c.contribution_amount * (aw.weight_bps as i128)) / 10000;
            token::Client::new(&env, &aw.token).transfer(&u, &env.current_contract_address(), &amt);
        }
        let mut m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, u.clone())).unwrap();
        m.contribution_count += 1; m.has_contributed_current_round = true;
        env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u.clone()), &m);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("Mem"), u), &m);
    }
    fn propose_duration(env: Env, u: Address, _cid: u64, dur: u64) -> u64 { u.require_auth(); let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("PDur"), id), &DurationProposal { id, new_duration: dur, votes_for: 1, votes_against: 0, end_time: env.ledger().timestamp() + 86400, is_active: true }); id }
    fn vote_duration(env: Env, u: Address, _cid: u64, pid: u64, app: bool) { u.require_auth(); let mut p: DurationProposal = env.storage().instance().get(&DataKey::K1(symbol_short!("PDur"), pid)).unwrap(); if app { p.votes_for += 1; } else { p.votes_against += 1; } env.storage().instance().set(&DataKey::K1(symbol_short!("PDur"), pid), &p); }
    fn slash_bond(_env: Env, adm: Address, _cid: u64) { adm.require_auth(); }
    fn release_bond(_env: Env, adm: Address, _cid: u64) { adm.require_auth(); }
    fn pair_with_member(env: Env, u: Address, buddy: Address) { u.require_auth(); env.storage().instance().set(&DataKey::K1A(symbol_short!("Bud"), u.clone()), &buddy); Self::record_audit_logic(&env, u, AuditAction::AdminAction, 0); }
    fn set_safety_deposit(env: Env, u: Address, cid: u64, amt: i128) {
        u.require_auth();
        let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
        token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &amt);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("Safe"), u), &amt);
    }
    fn propose_address_change(env: Env, prop: Address, cid: u64, old: Address, new: Address) { prop.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.recovery_old_address = Some(old); c.recovery_new_address = Some(new); env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn vote_for_recovery(env: Env, voter: Address, cid: u64) { voter.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.recovery_votes_bitmap |= 1; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn stake_xlm(_env: Env, u: Address, _tok: Address, _amt: i128) { u.require_auth(); }
    fn unstake_xlm(_env: Env, u: Address, _tok: Address, _amt: i128) { u.require_auth(); }
    fn update_global_fee(env: Env, adm: Address, fee: u32) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("Fee")), &fee); }
    fn request_leniency(env: Env, req: Address, cid: u64, reason: String) { req.require_auth(); let r = LeniencyRequest { requester: req.clone(), circle_id: cid, request_timestamp: env.ledger().timestamp(), voting_deadline: env.ledger().timestamp() + 86400, status: LeniencyRequestStatus::Pending, approve_votes: 0, reject_votes: 0, total_votes_cast: 0, extension_hours: 24, reason }; env.storage().instance().set(&DataKey::K2(symbol_short!("LenR"), cid, req), &r); }
    fn vote_on_leniency(env: Env, voter: Address, cid: u64, req: Address, v: LeniencyVote) {
        voter.require_auth();
        if voter == req { panic!("Cannot vote for self"); }
        let mut r: LeniencyRequest = env.storage().instance().get(&DataKey::K2(symbol_short!("LenR"), cid, req.clone())).unwrap();
        match v {
            LeniencyVote::Approve => r.approve_votes += 1,
            LeniencyVote::Reject => r.reject_votes += 1,
        };
        r.total_votes_cast += 1;
        if r.approve_votes >= 1 {
            r.status = LeniencyRequestStatus::Approved;
            let mut rs = Self::get_social_capital(env.clone(), req.clone(), cid);
            rs.leniency_received += 1; rs.trust_score += 5;
            env.storage().instance().set(&DataKey::K2(symbol_short!("Cap"), cid, req.clone()), &rs);
            
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

        // recipient is circle.current_recipient_index
        // We'll need a way to get member by index or store member addresses in circle.
        // For simplicity in this clean version, let's assume members are stored in a predictable way or we add member_addresses to CircleInfo.
        // Actually, let's use the bitmap and iterate to find the address if needed, or better, store it in storage under (circle_id, index)
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

        // Auto-release collateral and reward RI if member has completed all contributions
        let member_key = DataKey::Member(user.clone());
        if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            if member_info.contribution_count >= circle.max_members {
                // Reward RI
                let mut ri = Self::get_ri_internal(&env, &user);
                ri.successful_cycles = ri.successful_cycles.saturating_add(1);
                ri.points = (ri.points + 50).min(MAX_RI as u16); // +50 points for success
                ri.last_update = env.ledger().timestamp();
                Self::update_ri_internal(&env, &user, ri);

                if circle.requires_collateral {
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

        // Reset for next round
        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;
        circle.current_recipient_index = (circle.current_recipient_index + 1) % circle.member_count;
        circle.current_pot_recipient = None; // Should be set in finalize_round

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        env.storage().instance().remove(&DataKey::ScheduledPayoutTime(circle_id));
    }
    fn finalize_leniency_vote(env: Env, caller: Address, cid: u64, req: Address) { caller.require_auth(); let mut r: LeniencyRequest = env.storage().instance().get(&DataKey::K2(symbol_short!("LenR"), cid, req.clone())).unwrap(); r.status = LeniencyRequestStatus::Approved; env.storage().instance().set(&DataKey::K2(symbol_short!("LenR"), cid, req), &r); }
    fn get_leniency_request(env: Env, cid: u64, req: Address) -> LeniencyRequest { env.storage().instance().get(&DataKey::K2(symbol_short!("LenR"), cid, req)).unwrap() }
    fn get_social_capital(env: Env, m: Address, cid: u64) -> SocialCapital { env.storage().instance().get(&DataKey::K2(symbol_short!("Cap"), cid, m.clone())).unwrap_or(SocialCapital { member: m, circle_id: cid, leniency_given: 0, leniency_received: 0, voting_participation: 0, trust_score: 50 }) }
    fn create_proposal(env: Env, prop: Address, cid: u64, pt: ProposalType, title: String, desc: String, ed: String) -> u64 { prop.require_auth(); let id = 1u64; env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), id), &Proposal { id, circle_id: cid, proposer: prop, proposal_type: pt, title, description: desc, created_timestamp: env.ledger().timestamp(), voting_start_timestamp: env.ledger().timestamp(), voting_end_timestamp: env.ledger().timestamp() + 86400, status: ProposalStatus::Active, for_votes: 0, against_votes: 0, total_voting_power: 0, quorum_met: false, execution_data: ed }); id }
    fn quadratic_vote(env: Env, voter: Address, pid: u64, weight: u32, vc: QuadraticVoteChoice) {
        voter.require_auth();
        let mut p: Proposal = env.storage().instance().get(&DataKey::K1(symbol_short!("Prop"), pid)).unwrap();
        let mut vp: VotingPower = env.storage().instance().get(&DataKey::K2(symbol_short!("Vote"), p.circle_id, voter.clone())).unwrap();
        let cost = (weight as u64) * (weight as u64);
        if vp.quadratic_power < cost { panic!("Insufficient voting power"); }
        vp.quadratic_power -= cost;
        env.storage().instance().set(&DataKey::K2(symbol_short!("Vote"), p.circle_id, voter), &vp);
        match vc {
            QuadraticVoteChoice::For => p.for_votes += cost,
            QuadraticVoteChoice::Against => p.against_votes += cost,
            QuadraticVoteChoice::Abstain => {}
        }
        env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), pid), &p);
        
        // Update voter's reputation for governance participation
        let mut voter_metrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), voter.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, total_volume_saved: 0, last_activity: env.ledger().timestamp(), last_decay: env.ledger().timestamp(), on_time_contributions: 0, total_contributions: 0,
        });
        voter_metrics.social_capital_score = (voter_metrics.social_capital_score + 10).min(10000);
        voter_metrics.last_activity = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::K1A(symbol_short!("URep"), voter), &voter_metrics);
    }
    fn execute_proposal(env: Env, caller: Address, pid: u64) {
        caller.require_auth();
        let mut p: Proposal = env.storage().instance().get(&DataKey::K1(symbol_short!("Prop"), pid)).unwrap();
        p.status = ProposalStatus::Approved;
        env.storage().instance().set(&DataKey::K1(symbol_short!("Prop"), pid), &p);
    }
    fn get_proposal(env: Env, pid: u64) -> Proposal { env.storage().instance().get(&DataKey::K1(symbol_short!("Prop"), pid)).unwrap() }
    fn get_voting_power(env: Env, m: Address, cid: u64) -> VotingPower { env.storage().instance().get(&DataKey::K2(symbol_short!("Vote"), cid, m)).unwrap() }
    fn update_voting_power(env: Env, u: Address, cid: u64, bal: i128) { let pwr = if bal > 0 { 100 + (bal / 10000) as u64 } else { 100 }; let vp = VotingPower { member: u.clone(), circle_id: cid, token_balance: bal, quadratic_power: pwr, last_updated: env.ledger().timestamp() }; env.storage().instance().set(&DataKey::K2(symbol_short!("Vote"), cid, u), &vp); }
    fn stake_collateral(env: Env, u: Address, cid: u64, amt: i128) { u.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&u, &env.current_contract_address(), &amt); let i = CollateralInfo { member: u.clone(), circle_id: cid, amount: amt, status: CollateralStatus::Staked, staked_timestamp: env.ledger().timestamp(), release_timestamp: None }; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, u), &i); }
    fn slash_collateral(env: Env, _caller: Address, cid: u64, m: Address) { let mut i: CollateralInfo = env.storage().instance().get(&DataKey::K2(symbol_short!("Vlt"), cid, m.clone())).unwrap(); i.status = CollateralStatus::Slashed; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, m), &i); }
    fn release_collateral(env: Env, _caller: Address, cid: u64, m: Address) { let mut i: CollateralInfo = env.storage().instance().get(&DataKey::K2(symbol_short!("Vlt"), cid, m.clone())).unwrap(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &m, &i.amount); i.status = CollateralStatus::Released; env.storage().instance().set(&DataKey::K2(symbol_short!("Vlt"), cid, m), &i); }
    fn mark_member_defaulted(env: Env, caller: Address, cid: u64, m: Address) { caller.require_auth(); let mut mem: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, m.clone())).unwrap(); mem.status = MemberStatus::Defaulted; env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, m.clone()), &mem); env.storage().instance().set(&DataKey::K1A(symbol_short!("Mem"), m), &mem); }
    fn get_audit_entry(env: Env, id: u64) -> AuditEntry { env.storage().instance().get(&DataKey::K1(symbol_short!("AudE"), id)).unwrap() }
    fn query_audit_by_actor(env: Env, actor: Address, s: u64, e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { let count: u64 = env.storage().instance().get(&symbol_short!("AudCnt")).unwrap_or(0); let mut res = Vec::new(&env); for i in 1..=count { if let Some(ent) = env.storage().instance().get::<DataKey, AuditEntry>(&DataKey::K1(symbol_short!("AudE"), i)) { if ent.actor == actor && ent.timestamp >= s && ent.timestamp <= e { res.push_back(ent); } } } res }
    fn query_audit_by_resource(env: Env, rid: u64, s: u64, e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { let count: u64 = env.storage().instance().get(&symbol_short!("AudCnt")).unwrap_or(0); let mut res = Vec::new(&env); for i in 1..=count { if let Some(ent) = env.storage().instance().get::<DataKey, AuditEntry>(&DataKey::K1(symbol_short!("AudE"), i)) { if ent.resource_id == rid && ent.timestamp >= s && ent.timestamp <= e { res.push_back(ent); } } } res }
    fn query_audit_by_time(env: Env, s: u64, e: u64, _o: u32, _l: u32) -> Vec<AuditEntry> { let count: u64 = env.storage().instance().get(&symbol_short!("AudCnt")).unwrap_or(0); let mut res = Vec::new(&env); for i in 1..=count { if let Some(ent) = env.storage().instance().get::<DataKey, AuditEntry>(&DataKey::K1(symbol_short!("AudE"), i)) { if ent.timestamp >= s && ent.timestamp <= e { res.push_back(ent); } } } res }
    fn set_leaseflow_contract(env: Env, adm: Address, rot: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("LRot")), &rot); }
    fn authorize_leaseflow_payout(env: Env, u: Address, cid: u64, li: Address) { u.require_auth(); env.storage().instance().set(&DataKey::K2(symbol_short!("LAuth"), cid, u), &li); }
    fn handle_leaseflow_default(env: Env, rot: Address, ten: Address, cid: u64) { rot.require_auth(); env.storage().instance().set(&DataKey::K2(symbol_short!("LDef"), cid, ten), &true); }
    fn claim_pot(env: Env, u: Address, cid: u64) {
        u.require_auth();
        let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
        if let Some(oracle) = env.storage().instance().get::<DataKey, Address>(&DataKey::K(symbol_short!("Oracle"))) {
            let is_sanctioned: bool = env.invoke_contract(&oracle, &Symbol::new(&env, "is_sanctioned"), Vec::from_array(&env, [u.clone().into_val(&env)]));
            if is_sanctioned {
                let pot = c.contribution_amount * (c.member_count as i128);
                env.storage().instance().set(&DataKey::K1(symbol_short!("Froze"), cid), &(pot, Some(u)));
                return;
            }
        }
        if env.storage().instance().get::<DataKey, bool>(&DataKey::K2(symbol_short!("LDef"), cid, u.clone())).unwrap_or(false) {
            panic!("locked due to a default");
        }

        let mut recipient = u.clone();
        if let Some(auth_recipient) = env.storage().instance().get::<DataKey, Address>(&DataKey::K2(symbol_short!("LAuth"), cid, u.clone())) {
            recipient = auth_recipient;
        }

        token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &recipient, &(c.contribution_amount * (c.member_count as i128)));
        
        // Update recipient's volume saved for reputation
        let payout_amount = c.contribution_amount * (c.member_count as i128);
        let mut metrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), recipient.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, total_volume_saved: 0, last_activity: env.ledger().timestamp(), last_decay: env.ledger().timestamp(), on_time_contributions: 0, total_contributions: 0,
        });
        metrics.total_volume_saved += payout_amount;
        metrics.last_activity = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::K1A(symbol_short!("URep"), recipient), &metrics);
        
        c.is_active = false;
        env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c);
    }
    fn finalize_round(env: Env, u: Address, cid: u64) { u.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.is_round_finalized = true; c.current_pot_recipient = Some(u); env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn configure_batch_payout(env: Env, creator: Address, cid: u64, winners: u32) { creator.require_auth(); let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.winners_per_round = winners; c.batch_payout_enabled = true; env.storage().instance().set(&DataKey::K1(symbol_short!("C"), cid), &c); }
    fn distribute_batch_payout(env: Env, caller: Address, cid: u64) {
        caller.require_auth();
        let mut c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
        if c.winners_per_round == 0 { return; }
        
        let total_pot = c.contribution_amount * (c.member_count as i128);
        let amount_per_winner = total_pot / (c.winners_per_round as i128);
        
        let mut winners = Vec::new(&env);
        for i in 0..c.winners_per_round {
            if let Some(w) = c.member_addresses.get(i) {
                token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &w, &amount_per_winner);
                winners.push_back(w);
            }
        }
        
        let record = BatchPayoutRecord {
            batch_payout_id: 1, // Simple mock ID
            circle_id: cid,
            round_number: c.round_number,
            total_winners: c.winners_per_round,
            total_pot,
            organizer_fee: 0,
            net_payout_per_winner: amount_per_winner,
            dust_amount: total_pot % (c.winners_per_round as i128),
            winners: winners.clone(),
            payout_timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::K2U(symbol_short!("BRec"), cid, c.round_number), &record);
        
        for w in winners.iter() {
            let claim = IndividualPayoutClaim {
                recipient: w.clone(),
                circle_id: cid,
                round_number: c.round_number,
                amount_claimed: amount_per_winner,
                batch_payout_id: 1,
                claim_timestamp: env.ledger().timestamp(),
            };
            env.storage().instance().set(&DataKey::K3U(symbol_short!("IClm"), w, cid, c.round_number), &claim);
        }
    }
    fn get_batch_payout_record(env: Env, cid: u64, rn: u32) -> Option<BatchPayoutRecord> { env.storage().instance().get(&DataKey::K2U(symbol_short!("BRec"), cid, rn)) }
    fn get_individual_payout_claim(env: Env, u: Address, cid: u64, rn: u32) -> Option<IndividualPayoutClaim> { env.storage().instance().get(&DataKey::K3U(symbol_short!("IClm"), u, cid, rn)) }
    fn get_circle(env: Env, cid: u64) -> CircleInfo { env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap() }
    fn get_member(env: Env, u: Address) -> Member { env.storage().instance().get(&DataKey::K1A(symbol_short!("Mem"), u)).unwrap() }
    fn get_basket_config(env: Env, cid: u64) -> Vec<AssetWeight> { env.storage().instance().get(&DataKey::K1(symbol_short!("Bsk"), cid)).unwrap() }
    fn register_anchor(env: Env, adm: Address, info: AnchorInfo) { adm.require_auth(); env.storage().instance().set(&DataKey::K1A(symbol_short!("Anch"), info.anchor_address.clone()), &info); }
    fn get_anchor_info(env: Env, a: Address) -> AnchorInfo { env.storage().instance().get(&DataKey::K1A(symbol_short!("Anch"), a)).unwrap() }
    fn deposit_for_user(env: Env, anc: Address, u: Address, cid: u64, amt: i128, mem: String, fiat: String, sep: String) {
        anc.require_auth();
        let mut m: Member = env.storage().instance().get(&DataKey::K2(symbol_short!("M"), cid, u.clone())).unwrap();
        let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
        token::Client::new(&env, &c.token).transfer(&anc, &env.current_contract_address(), &amt);
        m.has_contributed_current_round = true;
        m.total_contributions += amt;
        env.storage().instance().set(&DataKey::K2(symbol_short!("M"), cid, u.clone()), &m);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("Mem"), u.clone()), &m);

        let id = 1u64;
        let record = AnchorDeposit {
            id,
            anchor_address: anc,
            beneficiary_user: u,
            circle_id: cid,
            amount: amt,
            deposit_memo: mem,
            fiat_reference: fiat,
            sep_type: sep,
            timestamp: env.ledger().timestamp(),
            processed: true,
            compliance_verified: true,
        };
        env.storage().instance().set(&DataKey::K1(symbol_short!("DRec"), id), &record);
    }
    fn get_deposit_record(env: Env, id: u64) -> AnchorDeposit { env.storage().instance().get(&DataKey::K1(symbol_short!("DRec"), id)).unwrap() }
    fn configure_dex_swap(env: Env, adm: Address, cid: u64, cfg: DexSwapConfig) { adm.require_auth(); env.storage().instance().set(&DataKey::K1(symbol_short!("DexC"), cid), &cfg); }
    fn trigger_dex_swap(env: Env, adm: Address, cid: u64) {
        adm.require_auth();
        let mut cfg: DexSwapConfig = env.storage().instance().get(&DataKey::K1(symbol_short!("DexC"), cid)).unwrap();
        cfg.total_swapped_xlm += cfg.swap_threshold_xlm;
        cfg.last_swap_timestamp = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::K1(symbol_short!("DexC"), cid), &cfg);
        let record = DexSwapRecord { success: true, usdc_amount: 100_000_000, xlm_received: cfg.swap_threshold_xlm };
        env.storage().instance().set(&DataKey::K2U(symbol_short!("DexR"), cid, 0), &record);
    }
    fn get_dex_swap_config(env: Env, cid: u64) -> Option<DexSwapConfig> { env.storage().instance().get(&DataKey::K1(symbol_short!("DexC"), cid)) }
    fn get_dex_swap_record(env: Env, cid: u64, rid: u64) -> Option<DexSwapRecord> { env.storage().instance().get(&DataKey::K2U(symbol_short!("DexR"), cid, rid as u32)) }
    fn emergency_pause_dex_swaps(_env: Env, adm: Address) { adm.require_auth(); }
    fn emergency_refill_gas_reserve(_env: Env, adm: Address, _amt: i128) { adm.require_auth(); }
    fn get_gas_reserve(env: Env, cid: u64) -> Option<GasReserve> { env.storage().instance().get(&DataKey::K1(symbol_short!("GRes"), cid)) }
    fn distribute_payout(env: Env, caller: Address, cid: u64) { caller.require_auth(); let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); let total_pot = c.contribution_amount * (c.member_count as i128); let immediate_payout = (total_pot * 7000) / 10000; let tranche_total = total_pot - immediate_payout; let recipient = c.member_addresses.get(c.current_recipient_index).unwrap(); token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &recipient, &immediate_payout); let mut tranches = Vec::new(&env); tranches.push_back(Tranche { amount: tranche_total / 2, unlock_round: c.round_number + 2, status: TrancheStatus::Pending }); env.storage().instance().set(&DataKey::K2(symbol_short!("TrcS"), cid, recipient.clone()), &TrancheSchedule { circle_id: cid, winner: recipient, total_pot, immediate_payout, tranches }); }
    fn get_tranche_schedule(env: Env, cid: u64, winner: Address) -> Option<TrancheSchedule> { env.storage().instance().get(&DataKey::K2(symbol_short!("TrcS"), cid, winner)) }
    fn claim_tranche(env: Env, u: Address, cid: u64, _tid: u32) { u.require_auth(); let mut s: TrancheSchedule = env.storage().instance().get(&DataKey::K2(symbol_short!("TrcS"), cid, u.clone())).unwrap(); let mut tr = s.tranches.get(0).unwrap(); tr.status = TrancheStatus::Claimed; s.tranches.set(0, tr); env.storage().instance().set(&DataKey::K2(symbol_short!("TrcS"), cid, u), &s); }
    fn execute_tranche_clawback(env: Env, adm: Address, cid: u64, m: Address) { adm.require_auth(); let mut s: TrancheSchedule = env.storage().instance().get(&DataKey::K2(symbol_short!("TrcS"), cid, m.clone())).unwrap(); let mut tr = s.tranches.get(0).unwrap(); tr.status = TrancheStatus::ClawedBack; s.tranches.set(0, tr); env.storage().instance().set(&DataKey::K2(symbol_short!("TrcS"), cid, m), &s); }
    fn terminate_grant_amicably(env: Env, adm: Address, grant_id: u64, grantee: Address, total: i128, dur: u64, start: u64, _treasury: Address, _tok: Address) -> GrantSettlement { adm.require_auth(); let elapsed = env.ledger().timestamp() - start; let dripped = if elapsed >= dur { total } else { (total * (elapsed as i128)) / (dur as i128) }; GrantSettlement { grant_id, grantee, total_grant_amount: total, amount_dripped: dripped, work_in_progress_pay: dripped, treasury_return: total - dripped } }
    fn create_voting_snapshot_for_audit(env: Env, pid: u64, votes: Vec<(Address, u32, Symbol)>, q: u64) -> VotingSnapshot { let mut total = 0u32; for v in votes.iter() { total += v.1; } VotingSnapshot { proposal_id: pid, total_votes: total, for_votes: total, against_votes: 0, abstain_votes: 0, quorum_required: q as u32, quorum_met: (total as u64) >= q, result: symbol_short!("APPROVED"), vote_hash: BytesN::from_array(&env, &[0; 32]) } }
    fn get_voting_snapshot_for_audit(_env: Env, _pid: u64) -> Option<VotingSnapshot> { None }
    fn initialize_impact_certificate(_env: Env, _grantee: Address, _id: u128, _total: u32, _uri: String) {}
    fn update_milestone_progress(_env: Env, adm: Address, id: u128, new_phase: u32, impact: i128) -> ImpactCertificateMetadata { adm.require_auth(); ImpactCertificateMetadata { id, grantee: adm, total_phases: new_phase + 1, phases_completed: new_phase, impact_score: impact as u32, on_chain_badge: symbol_short!("Impact"), milestone_status: MilestoneProgress::InProgress } }
    fn get_progress_bar_data(env: Env, _id: u128) -> Option<Map<Symbol, String>> { let mut m = Map::new(&env); m.set(symbol_short!("progress"), String::from_str(&env, "50%")); Some(m) }
    fn set_sanctions_oracle(env: Env, adm: Address, oracle: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("Oracle")), &oracle); }
    fn set_pop_oracle(env: Env, adm: Address, oracle: Address) { adm.require_auth(); env.storage().instance().set(&DataKey::K(symbol_short!("PoP")), &oracle); }
    fn reveal_next_winner(env: Env, cid: u64) -> Address { let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap(); c.member_addresses.get(c.current_recipient_index).unwrap() }
    fn get_frozen_payout(env: Env, cid: u64) -> (i128, Option<Address>) { env.storage().instance().get(&DataKey::K1(symbol_short!("Froze"), cid)).unwrap_or((0, None)) }
    fn review_frozen_payout(env: Env, adm: Address, cid: u64, release: bool) {
        adm.require_auth();
        let frozen_key = DataKey::K1(symbol_short!("Froze"), cid);
        if let Some((amt, winner_opt)) = env.storage().instance().get::<DataKey, (i128, Option<Address>)>(&frozen_key) {
            if release {
                if let Some(winner) = winner_opt {
                    let c: CircleInfo = env.storage().instance().get(&DataKey::K1(symbol_short!("C"), cid)).unwrap();
                    token::Client::new(&env, &c.token).transfer(&env.current_contract_address(), &winner, &amt);
                }
            }
            env.storage().instance().remove(&frozen_key);
        }
    }

    fn get_proposal(env: Env, proposal_id: u64) -> Proposal {
        let proposal_key = DataKey::Proposal(proposal_id);
        env.storage().instance().get(&proposal_key).expect("Proposal not found")
    }

    fn get_voting_power(env: Env, member: Address, circle_id: u64) -> VotingPower {
        let voting_power_key = DataKey::VotingPower(member, circle_id);
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
        // Calculate quadratic voting power as sqrt(token_balance)
        // We use integer approximation: sqrt(x) ≈ x / (sqrt(x) + 1) for simplicity
        // In production, you'd use a proper sqrt implementation
        
        let ri = Self::get_ri_internal(&env, &member);
        
        let quadratic_power = if token_balance > 0 {
            // Formula: Tokens * (RI / 1000)
            // Use large enough intermediate values to avoid precision loss
            let weighted_balance = (token_balance * ri.points as i128) / 1000;
            let balance_u64 = weighted_balance as u64;
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

    fn get_reliability_index(env: Env, member: Address) -> ReliabilityIndex {
        Self::get_ri_internal(&env, &member)
    }

    // Helper functions for internal RI management
    fn get_ri_internal(env: &Env, member: &Address) -> ReliabilityIndex {
        env.storage().instance().get(&DataKey::ReliabilityIndex(member.clone())).unwrap_or(ReliabilityIndex {
            points: MAX_RI as u16,
            successful_cycles: 0,
            default_count: 0,
            last_update: env.ledger().timestamp(),
        })
    }

    fn update_ri_internal(env: &Env, member: &Address, ri: ReliabilityIndex) {
        env.storage().instance().set(&DataKey::ReliabilityIndex(member.clone()), &ri);
    }

    fn report_to_external_registries(env: &Env, member: &Address, event_type: Symbol, amount: i128) {
        // Emit event for external identity protocols to pick up
        env.events().publish((symbol_short!("EXT_REP"), member.clone()), (event_type, amount));
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
    }
    fn update_reputation_on_deposit(env: Env, user: Address, was_on_time: bool) {
        // Check Proof of Personhood if oracle is set
        if let Some(pop_oracle) = env.storage().instance().get::<DataKey, Address>(&DataKey::K(symbol_short!("PoP"))) {
            let is_verified: bool = env.invoke_contract(&pop_oracle, &Symbol::new(&env, "is_verified"), Vec::from_array(&env, [user.clone().into_val(&env)]));
            if !is_verified {
                return; // Don't update reputation if not verified
            }
        }
        
        let mut metrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, total_volume_saved: 0, last_activity: env.ledger().timestamp(), last_decay: env.ledger().timestamp(), on_time_contributions: 0, total_contributions: 0,
        });
        metrics.total_contributions += 1;
        if was_on_time { metrics.on_time_contributions += 1; }
        metrics.last_activity = env.ledger().timestamp();
        
        // Calculate reliability score
        let on_time_rate = if metrics.total_contributions > 0 { (metrics.on_time_contributions * 10000) / metrics.total_contributions } else { 5000 };
        let volume_bonus = ((metrics.total_volume_saved / 1000000).min(100) * 50) as u32;
        metrics.reliability_score = (on_time_rate as i128 + volume_bonus as i128).min(10000) as u32;
        
        env.storage().instance().set(&DataKey::K1A(symbol_short!("URep"), user), &metrics);
    }
    fn apply_inactivity_decay(env: Env, user: Address) {
        let mut metrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, total_volume_saved: 0, last_activity: env.ledger().timestamp(), last_decay: env.ledger().timestamp(), on_time_contributions: 0, total_contributions: 0,
        });
        let months_inactive = (env.ledger().timestamp() - metrics.last_decay) / 2592000; // 30 days
        if months_inactive > 0 && env.ledger().timestamp() - metrics.last_activity > 15552000 { // 6 months
            let mut decay_factor = 10000u64;
            for _ in 0..months_inactive {
                decay_factor = (decay_factor * 95) / 100;
            }
            metrics.reliability_score = (metrics.reliability_score as u64 * decay_factor / 10000u64) as u32;
            metrics.social_capital_score = (metrics.social_capital_score as u64 * decay_factor / 10000u64) as u32;
            metrics.last_decay = env.ledger().timestamp();
            env.storage().instance().set(&DataKey::K1A(symbol_short!("URep"), user), &metrics);
        }
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

        // Apply RI Penalty
        let mut ri = Self::get_ri_internal(&env, &member);
        ri.points = ri.points.saturating_sub(RI_PENALTY);
        ri.default_count += 1;
        ri.last_update = env.ledger().timestamp();
        Self::update_ri_internal(&env, &member, ri);

        // Report to external registries (Negative-Credit Reporting)
        let amount_stolen = circle.contribution_amount * (circle.member_count as i128); // Pot value
        Self::report_to_external_registries(&env, &member, symbol_short!("DEFAULT"), amount_stolen);

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
            Self::slash_collateral(env, caller, circle_id, member);
        }
    }

    fn appeal_penalty(env: Env, requester: Address, circle_id: u64, reason: String) {
        requester.require_auth();

        // Check if member is defaulted
        let member_key = DataKey::Member(requester.clone());
        let member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        if member_info.status != MemberStatus::Defaulted {
            panic!("Only defaulted members can appeal");
        }

        let appeal_key = DataKey::ReputationAppeal(circle_id, requester.clone());
        if env.storage().instance().has(&appeal_key) {
            panic!("Appeal already exists");
        }

        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + VOTING_PERIOD;

        let appeal = ReputationAppeal {
            requester,
            circle_id,
            appeal_timestamp: current_time,
            voting_deadline,
            status: AppealStatus::Pending,
            for_votes: 0,
            against_votes: 0,
            reason,
        };

        env.storage().instance().set(&appeal_key, &appeal);
    }

    fn vote_on_appeal(env: Env, voter: Address, circle_id: u64, requester: Address, approve: bool) {
        voter.require_auth();

        let appeal_key = DataKey::ReputationAppeal(circle_id, requester.clone());
        let mut appeal: ReputationAppeal = env.storage().instance().get(&appeal_key).expect("Appeal not found");

        if appeal.status != AppealStatus::Pending {
            panic!("Appeal already finalized");
        }

        if env.ledger().timestamp() > appeal.voting_deadline {
            panic!("Voting period expired");
        }

        let vote_key = DataKey::AppealVotes(circle_id, requester.clone(), voter.clone());
        if env.storage().temporary().has(&vote_key) {
            panic!("Already voted");
        }

        // Must be a member of the same circle
        // (Simplified check: assume voter is a member if they can be found)
        let voter_key = DataKey::Member(voter.clone());
        let _voter_info: Member = env.storage().instance().get(&voter_key).expect("Voter not found");

        if approve {
            appeal.for_votes += 1;
        } else {
            appeal.against_votes += 1;
        }

        // Use temporary storage for votes to save on ledger rent for data that is only needed during voting
        env.storage().temporary().set(&vote_key, &approve);
        env.storage().instance().set(&appeal_key, &appeal);

        // Check for 2/3 majority
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        let total_voters = circle.member_count - 1; // Exclude requester
        let required_votes = (total_voters * REPUTATION_AMNESTY_THRESHOLD) / 100;

        if appeal.for_votes >= required_votes {
            appeal.status = AppealStatus::Approved;
            env.storage().instance().set(&appeal_key, &appeal);
            // Amnesty is auto-executed if majority reached
            Self::reputation_amnesty(env, voter, circle_id, requester);
        } else if appeal.against_votes > (total_voters - required_votes) {
            appeal.status = AppealStatus::Rejected;
            env.storage().instance().set(&appeal_key, &appeal);
        }
    }

    fn reputation_amnesty(env: Env, caller: Address, circle_id: u64, requester: Address) {
        caller.require_auth();

        let appeal_key = DataKey::ReputationAppeal(circle_id, requester.clone());
        let appeal: ReputationAppeal = env.storage().instance().get(&appeal_key).expect("Appeal not found");

        if appeal.status != AppealStatus::Approved {
            panic!("Appeal not approved");
        }

        // Restore points
        let mut ri = Self::get_ri_internal(&env, &requester);
        ri.points = (ri.points + RI_RESTORE).min(MAX_RI);
        Self::update_ri_internal(&env, &requester, ri);

        // Mark member as active again
        let member_key = DataKey::Member(requester.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");
        member_info.status = MemberStatus::Active;
        env.storage().instance().set(&member_key, &member_info);

        // Remove from defaulted list
        let defaulted_key = DataKey::DefaultedMembers(circle_id);
        if let Some(mut defaulted_members) = env.storage().instance().get::<DataKey, Vec<Address>>(&defaulted_key) {
            let mut new_list = Vec::new(&env);
            for m in defaulted_members.iter() {
                if m != requester {
                    new_list.push_back(m);
                }
            }
            env.storage().instance().set(&defaulted_key, &new_list);
        }
    }
}
