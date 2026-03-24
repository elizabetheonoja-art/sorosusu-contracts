#![no_std]
use soroban_sdk::{

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
const DISSOLUTION_VOTING_PERIOD: u64 = 1209600; // 14 days for dissolution voting
const DISSOLUTION_SUPERMAJORITY: u32 = 75; // 75% supermajority for dissolution
const DISSOLUTION_REFUND_PERIOD: u64 = 2592000; // 30 days for refund claims after dissolution
const DEFAULT_COLLATERAL_BPS: u32 = 2000; // 20%
const HIGH_VALUE_THRESHOLD: i128 = 1_000_000_0; // 1000 XLM (assuming 7 decimals)
const MAX_QUERY_LIMIT: u32 = 100;

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleMember(u64, u32),
    CircleCount,
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    GroupReserve,
    LendingPool,

}

#[contracttype]
#[derive(Clone)]

}

#[contracttype]
#[derive(Clone)]
pub struct UserStats {
    pub total_volume_saved: i128,
    pub on_time_contributions: u32,
    pub late_contributions: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct CreditAdvance {
    pub principal: i128,
    pub fee: i128,
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
pub enum DissolutionVoteChoice {
    Approve,
    Reject,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum DissolutionStatus {
    NotInitiated,
    Voting,
    Approved,
    Refunding,
    Completed,
    Failed,
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
pub struct DissolutionProposal {
    pub circle_id: u64,
    pub initiator: Address,
    pub reason: String,
    pub created_timestamp: u64,
    pub voting_deadline: u64,
    pub status: DissolutionStatus,
    pub approve_votes: u32,
    pub reject_votes: u32,
    pub total_votes_cast: u32,
    pub dissolution_timestamp: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct NetPosition {
    pub member: Address,
    pub circle_id: u64,
    pub total_contributions: i128,
    pub total_received: i128,
    pub net_position: i128, // Positive = owed money, Negative = owed to group
    pub collateral_staked: i128,
    pub collateral_status: CollateralStatus,
    pub has_received_pot: bool,
    pub refund_claimed: bool,
    pub default_marked: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct RefundClaim {
    pub member: Address,
    pub circle_id: u64,
    pub claim_timestamp: u64,
    pub refund_amount: i128,
    pub collateral_refunded: i128,
    pub status: RefundStatus,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum RefundStatus {
    Pending,
    Processed,
    Failed,
}

#[contracttype]
#[derive(Clone)]
pub struct DissolvedCircle {
    pub circle_id: u64,
    pub dissolution_timestamp: u64,
    pub total_contributions: i128,
    pub total_distributed: i128,
    pub remaining_funds: i128,
    pub total_members: u32,
    pub refunded_members: u32,
    pub defaulted_members: u32,
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

}

// --- CONTRACT CLIENTS ---

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AuditAction {
    DisputeSubmission,
    GovernanceVote,
    EvidenceAccess,
    AdminAction,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditEntry {
    pub id: u64,
    pub actor: Address,
    pub action: AuditAction,
    pub timestamp: u64,
    pub resource_id: u64,
}

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

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);

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

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32);
    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64);
    fn vote_penalty_change(env: Env, user: Address, circle_id: u64);

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    );
    fn vote_for_recovery(env: Env, user: Address, circle_id: u64);

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);

    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);

}

// --- IMPLEMENTATION ---


}

fn write_audit(env: &Env, actor: &Address, action: AuditAction, resource_id: u64) {
    let mut audit_count: u64 = env.storage().instance().get(&DataKey::AuditCount).unwrap_or(0);
    audit_count += 1;

    let entry = AuditEntry {
        id: audit_count,
        actor: actor.clone(),
        action,
        timestamp: env.ledger().timestamp(),
        resource_id,
    };

    env.storage()
        .instance()
        .set(&DataKey::AuditEntry(audit_count), &entry);
    env.storage().instance().set(&DataKey::AuditCount, &audit_count);

    append_audit_index(env, DataKey::AuditAll, audit_count);
    append_audit_index(env, DataKey::AuditByActor(actor.clone()), audit_count);
    append_audit_index(env, DataKey::AuditByResource(resource_id), audit_count);
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
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            dissolution_status: DissolutionStatus::NotInitiated,
            dissolution_deadline: None,
        };
        // Check if member is defaulted
        let defaulted_key = DataKey::DefaultedMembers(circle_id);
        let defaulted_members: Vec<Address> = env.storage().instance().get(&defaulted_key).unwrap_or(Vec::new(&env));
        
        if !defaulted_members.contains(&member) {
            panic!("Member not defaulted");
        }

    env.events().publish(
        (symbol_short!("AUDIT"), actor.clone(), resource_id),
        (audit_count, entry.timestamp),
    );
}

fn get_member_address_by_index(circle: &CircleInfo, index: u32) -> Address {
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    circle.member_addresses.get(index).unwrap()
}

fn count_active_members(env: &Env, circle: &CircleInfo) -> u32 {
    let mut active_count = 0u32;
    for i in 0..circle.member_count {
        let member_address = circle.member_addresses.get(i).unwrap();
        let key = DataKey::Member(member_address);
        if let Some(member) = env.storage().instance().get::<DataKey, Member>(&key) {
            if member.status == MemberStatus::Active {
                active_count += 1;
            }
        }
    }
    active_count
}

fn apply_recovery_if_consensus(env: &Env, actor: &Address, circle_id: u64, circle: &mut CircleInfo) {
    let active_members = count_active_members(env, circle);
    if active_members == 0 {
        panic!("No active members");
    }

    let votes = circle.recovery_votes_bitmap.count_ones();
    if votes * 100 <= active_members * 70 {
        return;
    }

    let old_address = circle
        .recovery_old_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));
    let new_address = circle
        .recovery_new_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));

    let old_member_key = DataKey::Member(old_address);
    let mut old_member: Member = env
        .storage()
        .instance()
        .get(&old_member_key)
        .unwrap_or_else(|| panic!("Old member not found"));

    if old_member.status != MemberStatus::Active {
        panic!("Only active members can be recovered");
    }

    let new_member_key = DataKey::Member(new_address.clone());
    if env.storage().instance().has(&new_member_key) {
        panic!("New address is already a member");
    }

    old_member.address = new_address.clone();
    env.storage().instance().set(&new_member_key, &old_member);
    env.storage().instance().remove(&old_member_key);

    circle
        .member_addresses
        .set(old_member.index, new_address);
    circle.recovery_old_address = None;
    circle.recovery_new_address = None;
    circle.recovery_votes_bitmap = 0;

    write_audit(env, actor, AuditAction::AdminAction, circle_id);
}

fn query_from_indexed_ids(
    env: &Env,
    ids: Vec<u64>,
    start_time: u64,
    end_time: u64,
    offset: u32,
    limit: u32,
) -> Vec<AuditEntry> {
    let mut output = Vec::new(env);
    if limit == 0 || start_time > end_time {
        return output;
    }

    let bounded_limit = if limit > MAX_QUERY_LIMIT {
        MAX_QUERY_LIMIT
    } else {
        limit
    };

    let mut skipped = 0u32;
    for i in 0..ids.len() {
        let id = ids.get(i).unwrap();
        let entry: AuditEntry = env
            .storage()
            .instance()
            .get(&DataKey::AuditEntry(id))
            .unwrap_or_else(|| panic!("Audit entry missing"));

        if entry.timestamp < start_time || entry.timestamp > end_time {
            continue;
        }

        if skipped < offset {
            skipped += 1;
            continue;
        }

        if output.len() >= bounded_limit {
            break;
        }

        output.push_back(entry);
    }

    output
}

fn finalize_leniency_vote_internal(
    env: &Env,
    circle_id: u64,
    requester: &Address,
    request: &mut LeniencyRequest,
) {
    let total_possible_votes = request.total_votes_cast;
    let minimum_participation = (total_possible_votes * MINIMUM_VOTING_PARTICIPATION) / 100;

    let mut final_status = LeniencyRequestStatus::Rejected;

    if request.total_votes_cast >= minimum_participation && request.total_votes_cast > 0 {
        let approval_percentage = (request.approve_votes * 100) / request.total_votes_cast;
        if approval_percentage >= SIMPLE_MAJORITY_THRESHOLD {
            final_status = LeniencyRequestStatus::Approved;

            let circle_key = DataKey::Circle(circle_id);
            let mut circle: CircleInfo = env
                .storage()
                .instance()
                .get(&circle_key)
                .expect("Circle not found");

            let extension_seconds = request.extension_hours * 3600;
            let new_deadline = circle.deadline_timestamp + extension_seconds;
            circle.grace_period_end = Some(new_deadline);

            env.storage().instance().set(&circle_key, &circle);

            let social_capital_key = DataKey::SocialCapital(requester.clone(), circle_id);
            let mut social_capital: SocialCapital = env
                .storage()
                .instance()
                .get(&social_capital_key)
                .unwrap_or(SocialCapital {
                    member: requester.clone(),
                    circle_id,
                    leniency_given: 0,
                    leniency_received: 0,
                    voting_participation: 0,
                    trust_score: 50,
                });
            social_capital.leniency_received += 1;
            social_capital.trust_score = (social_capital.trust_score + 5).min(100);
            env.storage().instance().set(&social_capital_key, &social_capital);
        }
    }

    request.status = final_status.clone();

    let stats_key = DataKey::LeniencyStats(circle_id);
    let mut stats: LeniencyStats = env
        .storage()
        .instance()
        .get(&stats_key)
        .unwrap_or(LeniencyStats {
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

    if stats.total_requests > 0 {
        let total_participation =
            stats.average_participation * (stats.total_requests - 1) + request.total_votes_cast;
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

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CircleCount, &0u64);
        env.storage().instance().set(&DataKey::AuditCount, &0u64);
    }

    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
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
        write_audit(&env, &admin, AuditAction::AdminAction, 0);
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
        if max_members == 0 {
            panic!("Max members must be greater than zero");
        }

        let current_time = env.ledger().timestamp();
        let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());
        if let Some(last_created) = env.storage().instance().get::<DataKey, u64>(&rate_limit_key) {
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
            creator,
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
            late_fee_bps: 100,
            nft_contract,
            is_round_finalized: false,
            current_pot_recipient: None,

        };

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);
        circle_count
    }

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        referrer: Option<Address>,
    ) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
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
        circle.member_addresses.push_back(user.clone());
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;
        let mut penalty_amount = 0i128;


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
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &total_amount);
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            if let Some(buddy_addr) = member.buddy.clone() {
                let safety_key = DataKey::SafetyDeposit(buddy_addr, circle_id);
                let safety_balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
                if safety_balance < total_amount {
                    panic!("Insufficient funds and buddy deposit");
                }
                env.storage()
                    .instance()
                    .set(&safety_key, &(safety_balance - total_amount));
            } else {
                panic!("Insufficient funds");
            }
        }

        if insurance_fee > 0 {
            circle.insurance_balance += insurance_fee;
        }

        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn finalize_round(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
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


    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        if !circle.is_round_finalized {
            panic!("Round not finalized");
        }

        let recipient = circle
            .current_pot_recipient
            .clone()
            .unwrap_or_else(|| panic!("No recipient set"));
        if user != recipient {
            panic!("Unauthorized recipient");
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

        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;


        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        env.storage().instance().remove(&DataKey::ScheduledPayoutTime(circle_id));
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        if circle.is_insurance_used {
            panic!("Insurance already used");
        }

        let member_key = DataKey::Member(member);
        let member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        let amount_needed = circle.contribution_amount * member_info.tier_multiplier as i128;
        if circle.insurance_balance < amount_needed {
            panic!("Insufficient insurance");
        }

        circle.contribution_bitmap |= 1u64 << member_info.index;
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
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }
        if new_bps > 10000 {
            panic!("Penalty cannot exceed 100%");
        }

        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64) {
        user.require_auth();
        if new_duration == 0 {
            panic!("Duration must be greater than zero");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let protocol_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if user != circle.creator && user != protocol_admin {
            panic!("Unauthorized");
        }

        circle.cycle_duration = new_duration;
        circle.deadline_timestamp = env.ledger().timestamp() + new_duration;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::AdminAction, circle_id);
    }

    fn vote_penalty_change(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if circle.proposed_late_fee_bps == 0 {
            panic!("No active proposal");
        }

        circle.proposal_votes_bitmap |= 1u64 << member.index;

        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    ) {
        user.require_auth();

        if old_address == new_address {
            panic!("Old and new addresses must differ");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let proposer_key = DataKey::Member(user.clone());
        let proposer: Member = env
            .storage()
            .instance()
            .get(&proposer_key)
            .expect("User is not a member");
        if proposer.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        let old_member_key = DataKey::Member(old_address.clone());
        let old_member: Member = env
            .storage()
            .instance()
            .get(&old_member_key)
            .expect("Old address is not a member");
        if old_member.status != MemberStatus::Active {
            panic!("Old address member is not active");
        }

        let new_member_key = DataKey::Member(new_address.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("New address is already a member");
        }

        circle.recovery_old_address = Some(old_address);
        circle.recovery_new_address = Some(new_address);
        circle.recovery_votes_bitmap = 1u64 << proposer.index;

        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn vote_for_recovery(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if circle.recovery_old_address.is_none() || circle.recovery_new_address.is_none() {
            panic!("No active recovery proposal");
        }



        circle.recovery_votes_bitmap |= 1u64 << member.index;
        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }


        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        nft_client.burn(&member, &token_id);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();
        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env
            .storage()
            .instance()
            .get(&user_key)
            .expect("Member not found");

        user_info.buddy = Some(buddy_address);
        env.storage().instance().set(&user_key, &user_info);
        write_audit(&env, &user, AuditAction::AdminAction, 0);
    }

    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let safety_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        let mut balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&safety_key, &balance);
    }


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

    #[test]
    fn test_sub_susu_credit_line() {
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
        
        let circle_id = client.create_circle(&creator, &1000, &2, &token_contract, &86400, &100, &nft_contract, &arbitrator);
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Payout to creator first to establish history and boost user score
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Now user asks for credit advance. Expected payout = 2000. Limit is 1000.
        client.approve_credit_advance(&creator, &circle_id, &user, &1000);
        
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&user, &circle_id); // debt is deducted seamlessly!
    }
}
