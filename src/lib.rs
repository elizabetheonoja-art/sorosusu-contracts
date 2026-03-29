use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, panic, Map, Vec, i128, u64, u32};

mod liquidity_buffer;

pub use liquidity_buffer::*;
mod sbt_minter;

pub use lending_market::*;
mod tranche_system;

pub use tranche_system::*;

// --- DATA STRUCTURES ---

#[derive(Clone)]
pub struct CircleInfo {
    pub creator: Address,
    pub contribution_amount: u64,
    pub max_members: u16,
    pub current_members: u16,
    pub token: Address,
    pub cycle_duration: u64,
    pub insurance_fee_bps: u32, // basis points (100 = 1%)
    pub organizer_fee_bps: u32,  // basis points (100 = 1%)
    pub nft_contract: Address,
    pub arbitrator: Address,
    pub members: Vec<Address>,
    pub contributions: Map<Address, bool>,
    pub current_round: u32,
    pub round_start_time: u64,
    pub is_round_finalized: bool,
    pub current_pot_recipient: Option<Address>,
    pub gas_buffer_balance: i128, // XLM buffer for gas fees
    pub gas_buffer_enabled: bool,
}

#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub join_time: u64,
    pub total_contributions: i128,
    pub total_received: i128,
    pub has_contributed_current_round: bool,
    pub consecutive_missed_rounds: u32,
}

#[derive(Clone)]
pub struct GasBufferConfig {
    pub min_buffer_amount: i128,     // Minimum XLM to maintain as buffer
    pub max_buffer_amount: i128,     // Maximum XLM that can be buffered
    pub auto_refill_threshold: i128, // When to auto-refill the buffer
    pub emergency_buffer: i128,      // Emergency buffer for extreme network conditions
}

// --- STORAGE KEYS ---

#[derive(Clone)]
pub enum DataKey {
    Admin,
    CircleCount,
    Circle(u64),
    Member(Address),
    MemberByIndex(u64, u32), // For efficient recipient lookup
    GasBufferConfig(u64),  // Per-circle gas buffer config
    ProtocolConfig,
    ScheduledPayoutTime(u64),
    // SBT Credential System Storage
    SoroSusuCredential(u128),    // Token ID -> Credential mapping
    UserCredential(Address),        // User -> Their SBT
    ReputationMilestone(u64),      // Milestone ID -> Milestone data
    MilestoneCounter,              // Counter for generating milestone IDs
    UserReputationScore(Address),    // User -> Reputation metrics
    SbtMinterAdmin,              // Admin address for SBT operations
    // Stellar Anchor Direct Deposit API (SEP-24/SEP-31)
    AnchorRegistry, // Registry of authorized anchors
    AnchorDeposit(u64), // Track anchor deposits per circle
    DepositMemo(u64), // Track deposit memos for compliance
}

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    // Initialize the contract
    fn init(env: Env, admin: Address);
    
    // Create a new savings circle
    fn create_circle(
        env: Env,
        creator: Address,
        contribution_amount: u64,
        max_members: u16,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
        organizer_fee_bps: u32,
    ) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64);

    // Make a deposit (Pay your weekly/monthly due)
    fn deposit(env: Env, user: Address, circle_id: u64);

    // NEW: Gas buffer management functions
    fn fund_gas_buffer(env: Env, circle_id: u64, amount: i128);
    fn set_gas_buffer_config(env: Env, circle_id: u64, config: GasBufferConfig);
    fn get_gas_buffer_balance(env: Env, circle_id: u64) -> i128;

    // NEW: Payout functions with gas buffer support
    fn distribute_payout(env: Env, caller: Address, circle_id: u64);
    fn trigger_payout(env: Env, admin: Address, circle_id: u64);
    fn finalize_round(env: Env, creator: Address, circle_id: u64);

    // Helper functions
    // ... (rest of the code remains the same)
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
    InvalidBasketWeights = 15,
    BasketNotEnabled = 16,
    InvalidBasketRatio = 17,
    AnchorNotFound = 18,
    AnchorNotAuthorized = 19,
    InvalidDepositMemo = 20,
    DepositAlreadyProcessed = 21,
    ComplianceCheckFailed = 22,
    // Tranche-based payout errors
    TrancheNotFound = 23,
    TrancheNotUnlocked = 24,
    TrancheAlreadyClaimed = 25,
    MemberDefaulted = 26,
    TrancheClawbackFailed = 27,
}

// --- CONSTANTS ---
// Batch payout configuration
const MIN_WINNERS_PER_ROUND: u16 = 1;
const MAX_WINNERS_PER_ROUND: u16 = 10;
const VALID_WINNER_COUNTS: [u16; 4] = [1, 2, 5, 10]; // Allowed winner counts
const STROOP_PRECISION: i128 = 10_000_000; // 7 decimal places for Stellar

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
const ROLLOVER_VOTING_PERIOD: u64 = 172800; // 48 hours for rollover voting
const ROLLOVER_QUORUM: u32 = 60; // 60% quorum for rollover voting
const ROLLOVER_MAJORITY: u32 = 66; // 66% supermajority for rollover approval
const DEFAULT_ROLLOVER_BONUS_BPS: u32 = 5000; // 50% of platform fee refunded as bonus

// Yield Delegation Constants
const YIELD_VOTING_PERIOD: u64 = 86400; // 24 hours for yield delegation voting
const YIELD_QUORUM: u32 = 75; // 75% quorum for yield delegation (higher stakes)
const YIELD_MAJORITY: u32 = 80; // 80% supermajority for yield delegation approval
const MIN_DELEGATION_AMOUNT: i128 = 100_000_000; // Minimum 10 tokens to delegate
const MAX_DELEGATION_PERCENTAGE: u32 = 8000; // Maximum 80% of pot can be delegated
const YIELD_DISTRIBUTION_RECIPIENT_BPS: u32 = 5000; // 50% to current round winner
const YIELD_DISTRIBUTION_TREASURY_BPS: u32 = 5000; // 50% to group treasury
const YIELD_COMPOUNDING_FREQUENCY: u64 = 604800; // Weekly compounding (7 days)

// Path Payment Constants
const PATH_PAYMENT_VOTING_PERIOD: u64 = 43200; // 12 hours for path payment voting
const PATH_PAYMENT_QUORUM: u32 = 50; // 50% quorum for path payment approval
const PATH_PAYMENT_MAJORITY: u32 = 66; // 66% majority for path payment approval
const MAX_SLIPPAGE_TOLERANCE_BPS: u32 = 500; // 5% maximum slippage tolerance
const MIN_PATH_PAYMENT_AMOUNT: i128 = 50_000_000; // Minimum 5 tokens for path payment
const PATH_PAYMENT_TIMEOUT: u64 = 300; // 5 minutes timeout for path payment execution

// --- POT LIQUIDITY BUFFER FOR BANK HOLIDAYS ---

const LIQUIDITY_BUFFER_ADVANCE_PERIOD: u64 = 172800; // 48 hours advance window
const LIQUIDITY_BUFFER_MIN_REPUTATION: u32 = 10000; // 100% reputation required
const LIQUIDITY_BUFFER_MAX_ADVANCE_BPS: u32 = 10000; // 100% of contribution can be advanced
const LIQUIDITY_BUFFER_PLATFORM_FEE_ALLOCATION: u32 = 2000; // 20% of platform fees allocated to buffer
const LIQUIDITY_BUFFER_MIN_RESERVE: i128 = 1_000_000_000; // Minimum 100 tokens in reserve
const LIQUIDITY_BUFFER_MAX_RESERVE: i128 = 10_000_000_000; // Maximum 10,000 tokens in reserve
const LIQUIDITY_BUFFER_ADVANCE_FEE_BPS: u32 = 50; // 0.5% fee for advance usage
const LIQUIDITY_BUFFER_GRACE_PERIOD: u64 = 86400; // 24 hours grace period for repayment
const LIQUIDITY_BUFFER_MAX_ADVANCES_PER_ROUND: u32 = 3; // Maximum advances per member per round

// Asset Swap / Economic Circuit Breaker Constants
const PRICE_DROP_THRESHOLD_BPS: u32 = 2000; // 20% price drop triggers circuit breaker
const ASSET_SWAP_VOTING_PERIOD: u64 = 86400; // 24 hours for asset swap voting
const ASSET_SWAP_QUORUM: u32 = 60; // 60% quorum for asset swap approval
const ASSET_SWAP_MAJORITY: u32 = 66; // 66% majority for asset swap approval
const MAX_SLIPPAGE_TOLERANCE_BPS: u32 = 500; // 5% maximum slippage tolerance
const MIN_PATH_PAYMENT_AMOUNT: i128 = 50_000_000; // Minimum 5 tokens for path payment
const PATH_PAYMENT_TIMEOUT: u64 = 300; // 5 minutes timeout for path payment execution

// Tranche-Based Payout Constants
const TRANCHE_IMMEDIATE_PAYOUT_BPS: u32 = 7000; // 70% paid immediately
const TRANCHE_LOCKED_PERCENTAGE_BPS: u32 = 3000; // 30% locked in tranches
const TRANCHE_COUNT: u32 = 2; // Number of tranches for locked amount (2 rounds)
const TRANCHE_CLAIM_GRACE_PERIOD: u64 = 2592000; // 30 days grace period to claim unlocked tranches

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


/// Individual tranche information
#[contracttype]
#[derive(Clone)]
pub struct TrancheInfo {
    pub amount: i128,              // Amount locked in this tranche
    pub unlock_round: u32,         // Round number when this tranche unlocks
    pub unlock_timestamp: u64,     // Timestamp when this tranche unlocks
    pub status: TrancheStatus,     // Current status of this tranche
    pub created_at: u64,           // When this tranche was created
    pub claimed_at: Option<u64>,   // When this tranche was claimed
}

/// Complete tranche schedule for a winner's pot
#[contracttype]
#[derive(Clone)]
pub struct TrancheSchedule {
    pub winner: Address,           // The member who won the pot
    pub circle_id: u64,            // Circle identifier
    pub total_pot: i128,           // Total pot amount
    pub immediate_payout: i128,    // 70% paid immediately
    pub tranches: Vec<TrancheInfo>, // Remaining 30% distributed in tranches
    pub created_at: u64,           // When schedule was created
    pub is_complete: bool,         // Whether all tranches have been claimed or clawed back
}

/// Member contribution tracking for tranche eligibility
#[contracttype]
#[derive(Clone)]
pub struct MemberContributionRecord {
    pub member: Address,
    pub circle_id: u64,
    pub round: u32,
    pub contributed_on_time: bool,
    pub contribution_timestamp: u64,
    pub is_defaulted: bool,
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
pub struct ReputationData {
    pub user_address: Address,
    pub susu_score: u32,        // 0-10000 bps (0-100%)
    pub reliability_score: u32,  // 0-10000 bps (0-100%)
    pub total_contributions: u32,
    pub on_time_rate: u32,      // 0-10000 bps (0-100%)
    pub volume_saved: i128,
    pub social_capital: u32,    // 0-10000 bps (0-100%)
    pub last_updated: u64,
    pub is_active: bool,

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
#[derive(Clone, Debug, PartialEq)]
pub enum RolloverVoteChoice {
    For,
    Against,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum RolloverStatus {
    NotInitiated,
    Voting,
    Approved,
    Rejected,
    Applied,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum YieldVoteChoice {
    For,
    Against,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum YieldDelegationStatus {
    NotInitiated,
    Voting,
    Approved,
    Rejected,
    Active,
    Completed,
    Withdrawn,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum YieldPoolType {
    StellarLiquidityPool,
    RegulatedMoneyMarket,
    StableYieldVault,
}

#[contracttype]
#[derive(Clone)]
pub struct YieldDelegation {
    pub circle_id: u64,
    pub delegation_amount: i128,
    pub pool_address: Address,
    pub pool_type: YieldPoolType,
    pub delegation_percentage: u32, // Percentage of pot to delegate
    pub created_timestamp: u64,
    pub status: YieldDelegationStatus,
    pub voting_deadline: u64,
    pub for_votes: u32,
    pub against_votes: u32,
    pub total_votes_cast: u32,
    pub start_time: Option<u64>,
    pub end_time: Option<u64>,
    pub total_yield_earned: i128,
    pub yield_distributed: i128,
    pub last_compound_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct YieldVote {
    pub voter: Address,
    pub circle_id: u64,
    pub vote_choice: YieldVoteChoice,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct YieldPoolInfo {
    pub pool_address: Address,
    pub pool_type: YieldPoolType,
    pub is_active: bool,
    pub total_delegated: i128,
    pub apy_bps: u32, // Annual Percentage Yield in basis points
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct YieldDistribution {
    pub circle_id: u64,
    pub recipient_share: i128,
    pub treasury_share: i128,
    pub total_yield: i128,
    pub distribution_time: u64,
    pub round_number: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PathPaymentStatus {
    Proposed,
    Approved,
    Executing,
    Completed,
    Failed,
    Refunded,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum PathPaymentVoteChoice {
    For,
    Against,
}

#[contracttype]
#[derive(Clone)]
pub struct PathPayment {
    pub circle_id: u64,
    pub source_token: Address, // Token user sends (e.g., XLM)
    pub target_token: Address, // Token circle requires (e.g., USDC)
    pub source_amount: i128,
    pub target_amount: i128, // Amount after swap
    pub exchange_rate: i128, // Rate used (target_amount / source_amount * 1M)
    pub slippage_bps: u32, // Actual slippage experienced
    pub dex_address: Address, // DEX used for swap
    pub path_payment: Address, // Stellar path payment used
    pub created_timestamp: u64,
    pub status: PathPaymentStatus,
    pub voting_deadline: u64,
    pub for_votes: u32,
    pub against_votes: u32,
    pub total_votes_cast: u32,
    pub execution_timestamp: Option<u64>,
    pub completion_timestamp: Option<u64>,
    pub refund_amount: Option<i128>,
}

#[contracttype]
#[derive(Clone)]
pub struct PathPaymentVote {
    pub voter: Address,
    pub circle_id: u64,
    pub vote_choice: PathPaymentVoteChoice,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct LeaseFlowPayoutAuthorization {
    pub user: Address,
    pub circle_id: u64,
    pub lease_instance: Address,
    pub authorized_at: u64,
    pub is_active: bool,
}

#[contracttype]
#[derive(Clone)]
pub struct SupportedToken {
    pub token_address: Address,
    pub token_symbol: String, // e.g., "XLM", "USDC", "USDT"
    pub decimals: u32,
    pub is_stable: bool,
    pub is_active: bool,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct DexInfo {
    pub dex_address: Address,
    pub dex_name: String,
    pub supported_pairs: Vec<(Address, Address)>, // (source, target) pairs
    pub is_trusted: bool,
    pub is_active: bool,
    pub last_updated: u64,
}

/// A single asset slot in a multi-asset basket with its allocation weight.
#[contracttype]
#[derive(Clone)]
pub struct AssetWeight {
    pub token: Address,
    pub weight_bps: u32, // Allocation in basis points (e.g., 5000 = 50%)
}

#[contracttype]
#[derive(Clone)]
pub struct RolloverBonus {
    pub circle_id: u64,
    pub bonus_amount: i128,
    pub fee_percentage: u32, // Percentage of platform fee to refund
    pub created_timestamp: u64,
    pub status: RolloverStatus,
    pub voting_deadline: u64,
    pub for_votes: u32,
    pub against_votes: u32,
    pub total_votes_cast: u32,
    pub applied_cycle: Option<u64>,
}

#[contracttype]
#[derive(Clone)]
pub struct RolloverVote {
    pub voter: Address,
    pub circle_id: u64,
    pub vote_choice: RolloverVoteChoice,
    pub timestamp: u64,
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
    pub requires_collateral: bool,
    pub collateral_bps: u32,
    pub member_addresses: Vec<Address>,
    pub leniency_enabled: bool,
    pub grace_period_end: Option<u64>,
    pub quadratic_voting_enabled: bool,
    pub proposal_count: u64,
    pub dissolution_status: DissolutionStatus,
    pub dissolution_deadline: Option<u64>,
    pub proposed_late_fee_bps: u32,
    pub proposal_votes_bitmap: u64,
    pub recovery_old_address: Option<Address>,
    pub recovery_new_address: Option<Address>,
    pub recovery_votes_bitmap: u64,
    pub arbitrator: Address,
    /// Multi-asset basket: None for single-token circles, Some(...) for basket circles.
    /// Each AssetWeight specifies a token address and its allocation in basis points.
    pub basket: Option<Vec<AssetWeight>>,
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

// --- CYCLE COMPLETION NFT BADGE ---

#[contracttype]
#[derive(Clone)]
pub struct NftBadgeMetadata {
    pub volume_tier: u32,        // 1=Bronze, 2=Silver, 3=Gold based on total_volume_saved
    pub perfect_attendance: bool, // true if zero late contributions
    pub group_lead_status: bool,  // true if member is the circle creator
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
    fn mint_badge(env: Env, to: Address, token_id: u128, metadata: NftBadgeMetadata);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: i128);
    fn withdraw(env: Env, token: Address, to: Address, amount: i128);
}

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

    // Rollover Bonus Incentive Logic
    fn propose_rollover_bonus(env: Env, user: Address, circle_id: u64, fee_percentage_bps: u32);
    fn vote_rollover_bonus(env: Env, user: Address, circle_id: u64, vote_choice: RolloverVoteChoice);
    fn apply_rollover_bonus(env: Env, circle_id: u64);

    // Idle Pot Yield Delegation to Stellar Pools
    fn propose_yield_delegation(env: Env, user: Address, circle_id: u64, delegation_percentage: u32, pool_address: Address, pool_type: YieldPoolType);
    fn vote_yield_delegation(env: Env, user: Address, circle_id: u64, vote_choice: YieldVoteChoice);
    fn approve_yield_delegation(env: Env, circle_id: u64);
    fn execute_yield_delegation(env: Env, circle_id: u64);
    fn compound_yield(env: Env, circle_id: u64);
    fn withdraw_yield_delegation(env: Env, circle_id: u64);
    fn distribute_yield_earnings(env: Env, circle_id: u64);

    // Path Payment Contribution Support
    fn propose_path_payment_support(env: Env, user: Address, circle_id: u64);
    fn vote_path_payment_support(env: Env, user: Address, circle_id: u64, vote_choice: PathPaymentVoteChoice);
    fn approve_path_payment_support(env: Env, circle_id: u64);
    fn execute_path_payment(env: Env, user: Address, circle_id: u64, source_token: Address, source_amount: i128);
    fn register_supported_token(env: Env, user: Address, token_address: Address, token_symbol: String, decimals: u32, is_stable: bool);
    fn register_dex(env: Env, user: Address, dex_address: Address, dex_name: String, is_trusted: bool);

    // Inter-contract reputation query interface
    fn get_reputation(env: Env, user: Address) -> ReputationData;

    // LeaseFlow landlord-tenant escrow integration
    fn set_leaseflow_contract(env: Env, admin: Address, leaseflow: Address);
    fn authorize_leaseflow_payout(env: Env, user: Address, circle_id: u64, lease_instance: Address);
    fn revoke_leaseflow_payout(env: Env, user: Address, circle_id: u64);
    fn get_leaseflow_payout(env: Env, user: Address, circle_id: u64) -> Option<LeaseFlowPayoutAuthorization>;
    fn handle_leaseflow_default(env: Env, leaseflow_contract: Address, user: Address, circle_id: u64);

    // Multi-Asset Reserve Currency Basket
    fn create_basket_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        basket_assets: Vec<Address>,
        basket_weights: Vec<u32>,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
    ) -> u64;

    fn deposit_basket(env: Env, user: Address, circle_id: u64);
    fn get_basket_config(env: Env, circle_id: u64) -> Vec<AssetWeight>;
}

// --- IMPLEMENTATION ---

fn append_audit_index(env: &Env, key: DataKey, audit_id: u64) {
    let mut ids: Vec<u64> = env.storage().instance().get(&key).unwrap_or(Vec::new(env));
    ids.push_back(audit_id);
    env.storage().instance().set(&key, &ids);
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

    env.events().publish(
        (symbol_short!("AUDIT"), actor.clone(), resource_id),
        (audit_count, entry.timestamp),
    );
}

fn calculate_rollover_bonus(env: &Env, circle_id: u64, fee_percentage_bps: u32) -> i128 {
    // Get the protocol fee settings
    let fee_bps: u32 = env.storage().instance().get(&DataKey::ProtocolFeeBps).unwrap_or(0);
    if fee_bps == 0 {
        return 0; // No protocol fee, no bonus
    }

    // Calculate the total pot amount for this circle
    let circle_key = DataKey::Circle(circle_id);
    let circle: CircleInfo = env.storage().instance().get(&circle_key)
        .expect("Circle not found");
    
    let total_pot = circle.contribution_amount * (circle.member_count as i128);
    
    // Calculate the platform fee that would be charged
    let platform_fee = (total_pot * fee_bps as i128) / 10000;
    
    // Calculate the rollover bonus (percentage of platform fee to refund)
    let bonus_amount = (platform_fee * fee_percentage_bps as i128) / 10000;
    
    bonus_amount
}

fn get_member_address_by_index(circle: &CircleInfo, index: u32) -> Address {
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    circle.member_addresses.get(index).unwrap()
}

fn execute_stellar_path_payment(env: &Env, source_token: &Address, target_token: &Address, source_amount: i128, max_slippage_bps: u32) -> (i128, i128, u32) {
    // This is a simplified implementation - in production would call actual Stellar Path Payment
    // For now, we'll simulate the swap with a basic exchange rate
    
    // Get token info for decimals
    let source_token_key = DataKey::SupportedTokens(source_token.clone());
    let source_token_info: SupportedToken = env.storage().instance().get(&source_token_key)
        .expect("Source token not supported");
    
    let target_token_key = DataKey::SupportedTokens(target_token.clone());
    let target_token_info: SupportedToken = env.storage().instance().get(&target_token_key)
        .expect("Target token not supported");

    // Calculate exchange rate (simplified - would use actual DEX rates)
    // Assume 1:1 rate for demonstration, adjust based on token types
    let rate_adjustment = if source_token_info.is_stable && !target_token_info.is_stable {
        10000 // Stable to volatile might need premium
    } else if !source_token_info.is_stable && target_token_info.is_stable {
        9800 // Stable to stable might have small discount
    } else {
        10000 // Default 1:1 rate
    };

    let exchange_rate = rate_adjustment;
    let target_amount = (source_amount * exchange_rate) / 10000;
    
    // Calculate slippage (0 for this simplified implementation)
    let slippage_bps = 0;
    
    // In real implementation, this would:
    // 1. Call Stellar Path Payment contract
    // 2. Handle slippage protection
    // 3. Handle partial fills
    // 4. Handle failed transactions
    
    (target_amount, exchange_rate, slippage_bps)
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
            requires_collateral,
            collateral_bps,
            member_addresses: Vec::new(&env),
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            dissolution_status: DissolutionStatus::NotInitiated,
            dissolution_deadline: None,
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            recovery_old_address: None,
            recovery_new_address: None,
            recovery_votes_bitmap: 0,
            arbitrator,
            basket: None,
        };

        // Store the circle
        env.storage().instance().set(&DataKey::Circle(circle_count), &circle);
        
        // Update the circle count
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        // Set default gas buffer configuration for this circle
        let default_config = GasBufferConfig {
            min_buffer_amount: 10000000, // 0.01 XLM minimum
            max_buffer_amount: 1000000000, // 10 XLM maximum
            auto_refill_threshold: 5000000, // 0.005 XLM threshold
            emergency_buffer: 50000000, // 0.5 XLM emergency buffer
        };
        env.storage().instance().set(&DataKey::GasBufferConfig(circle_count), &default_config);

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64, guarantor: Option<Address>) {
        // Authorization: The user MUST sign this transaction
        user.require_auth();

        // Check if the circle exists
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check if the circle is full
        if circle.current_members >= circle.max_members {
            panic!("Circle is full");
        }

        // Check if the user is already a member
        if circle.members.contains(&user) {
            panic!("Already a member");
        }

        // Add the user to the members list
        circle.members.push_back(user.clone());
        circle.current_members += 1;

        // Store member by index for efficient lookup during payouts
        let member_index = circle.current_members - 1;
        env.storage().instance().set(&DataKey::MemberByIndex(circle_id, member_index as u32), &user);

        // Create member record
        let member = Member {
            address: user.clone(),
            join_time: env.ledger().timestamp(),
            total_contributions: 0i128,
            total_received: 0i128,
            has_contributed_current_round: false,
            consecutive_missed_rounds: 0,
        };

        // Store the member
        env.storage().instance().set(&DataKey::Member(user.clone()), &member);

        // Update the circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        // Authorization: The user must sign this!
        user.require_auth();

        // Flash-loan prevention: Ledger-Lock mechanism
        let current_ledger = env.ledger().sequence();
        if let Some(last_withdrawal) = env.storage().instance().get::<DataKey, u32>(&DataKey::LastWithdrawalLedger(user.clone())) {
            if last_withdrawal == current_ledger {
                panic!("Flash-loan prevention: Cannot deposit and withdraw in same ledger");
            }
        }
        env.storage().instance().set(&DataKey::LastDepositLedger(user.clone()), &current_ledger);

        // Get the circle
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Get the member
        let mut member: Member = env.storage().instance()
            .get(&DataKey::Member(user.clone()))
            .unwrap_or_else(|| panic!("Member not found"));

        // Check if already contributed this round
        if member.has_contributed_current_round {
            panic!("Already contributed this round");
        }

        // Calculate the total amount needed (contribution + insurance fee + group insurance premium)
        let insurance_fee = (circle.contribution_amount as i128 * circle.insurance_fee_bps as i128) / 10_000;
        
        // Group Insurance Fund premium (0.5% = 50 basis points)
        let group_insurance_premium = (circle.contribution_amount as i128 * 50i128) / 10_000;
        
        let total_amount = circle.contribution_amount as i128 + insurance_fee + group_insurance_premium;

        // Transfer the tokens from user to contract
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &total_amount);

        // Update member record
        member.has_contributed_current_round = true;
        member.total_contributions += total_amount;
        member.consecutive_missed_rounds = 0; // Reset missed rounds counter

        // Update circle contributions
        circle.contributions.set(user.clone(), true);

        // Update Group Insurance Fund
        let mut insurance_fund: GroupInsuranceFund = env.storage().instance()
            .get(&DataKey::GroupInsuranceFund(circle_id))
            .unwrap_or(GroupInsuranceFund {
                circle_id,
                total_fund_balance: 0,
                total_premiums_collected: 0,
                total_claims_paid: 0,
                premium_rate_bps: 50, // 0.5%
                is_active: true,
                cycle_start_time: env.ledger().timestamp(),
                last_claim_time: None,
            });
        
        insurance_fund.total_fund_balance += group_insurance_premium;
        insurance_fund.total_premiums_collected += group_insurance_premium;
        env.storage().instance().set(&DataKey::GroupInsuranceFund(circle_id), &insurance_fund);

        // Update individual premium record
        let mut premium_record: InsurancePremiumRecord = env.storage().instance()
            .get(&DataKey::InsurancePremium(circle_id, user.clone()))
            .unwrap_or(InsurancePremiumRecord {
                member: user.clone(),
                circle_id,
                total_premium_paid: 0,
                premium_payments: Vec::new(&env),
                claims_made: 0,
                net_contribution: 0,
            });
        
        premium_record.total_premium_paid += group_insurance_premium;
        let current_round = circle.current_recipient_index + 1;
        premium_record.premium_payments.push_back((current_round, group_insurance_premium));
        premium_record.net_contribution = premium_record.total_premium_paid - premium_record.claims_made;
        env.storage().instance().set(&DataKey::InsurancePremium(circle_id, user.clone()), &premium_record);

        // Track payment timing for priority distribution
        let current_time = env.ledger().timestamp();
        let is_on_time = current_time <= circle.deadline_timestamp;
        
        // Record contribution for tranche eligibility tracking
        tranche_system::record_contribution(&env, circle_id, &user, is_on_time);
        
        // Get or initialize payment order counter for this round
        let mut payment_order_counter: u32 = env.storage().instance()
            .get(&DataKey::PaymentOrderCounter(circle_id, current_round))
            .unwrap_or(0);
        payment_order_counter += 1;
        env.storage().instance().set(&DataKey::PaymentOrderCounter(circle_id, current_round), &payment_order_counter);
        
        let payment_timing = PaymentTimingRecord {
            member: user.clone(),
            circle_id,
            round_number: current_round,
            payment_timestamp: current_time,
            is_on_time,
            payment_order: payment_order_counter,
        };
        env.storage().instance().set(&DataKey::PaymentTiming(circle_id, current_round, user.clone()), &payment_timing);

        // Store updated records
        env.storage().instance().set(&DataKey::Member(user), &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Check if all members have contributed and auto-finalize if so
        Self::check_and_finalize_round(&env, circle_id);
    }

    // --- GAS BUFFER MANAGEMENT ---

    fn fund_gas_buffer(env: Env, circle_id: u64, amount: i128) {
        // Get the circle
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Get gas buffer config
        let config: GasBufferConfig = env.storage().instance()
            .get(&DataKey::GasBufferConfig(circle_id))
            .unwrap_or_else(|| panic!("Gas buffer config not found"));

        // Validate amount doesn't exceed maximum
        if circle.gas_buffer_balance + amount > config.max_buffer_amount {
            panic!("Amount exceeds maximum gas buffer limit");
        }

        // Transfer XLM from caller to contract
        let xlm_token = env.native_token();
        let token_client = token::Client::new(&env, &xlm_token);
        
        // Get caller address - in a real implementation, this would be extracted from auth
        let caller = env.current_contract_address(); 
        
        token_client.transfer(&caller, &env.current_contract_address(), &amount);

        // Update gas buffer balance
        circle.gas_buffer_balance += amount;

        // Store updated circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Emit event for gas buffer funding
        env.events().publish(
            (Symbol::new(&env, "gas_buffer_funded"), circle_id),
            (amount, circle.gas_buffer_balance),
        );
    }

    fn set_gas_buffer_config(env: Env, circle_id: u64, config: GasBufferConfig) {
        // Only circle creator can set config
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check authorization
        circle.creator.require_auth();

        // Validate config parameters
        if config.min_buffer_amount < 0 || config.max_buffer_amount <= config.min_buffer_amount {
            panic!("Invalid buffer configuration");
        }

        // Store the configuration
        env.storage().instance().set(&DataKey::GasBufferConfig(circle_id), &config);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "gas_buffer_config_updated"), circle_id),
            (config.min_buffer_amount, config.max_buffer_amount),
        );
    }

    fn get_gas_buffer_balance(env: Env, circle_id: u64) -> i128 {
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));
        
        circle.gas_buffer_balance
    }

    // --- PAYOUT FUNCTIONS WITH GAS BUFFER ---

    fn distribute_payout(env: Env, caller: Address, circle_id: u64) {
        // Authorization check
        caller.require_auth();

        // Get the circle
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check if all members have contributed
        if !Self::all_members_contributed(&env, circle_id) {
            panic!("Not all members have contributed this cycle");
        }

        // Get the current recipient
        let recipient = Self::get_current_recipient(&env, circle_id)
            .unwrap_or_else(|| panic!("No recipient found"));

        // Inter-protocol security: Check if payout is paused due to external default (e.g., LeaseFlow)
        let is_paused = env.storage().instance().get::<DataKey, bool>(&DataKey::PausedPayout(recipient.clone(), circle_id)).unwrap_or(false);
        if is_paused {
            panic!("Recipient's payout is currently locked due to a default in a connected protocol (LeaseFlow).");
        }

        // Flash-loan prevention: Ledger-Lock mechanism for recipient
        let current_ledger = env.ledger().sequence();
        if let Some(last_deposit) = env.storage().instance().get::<DataKey, u32>(&DataKey::LastDepositLedger(recipient.clone())) {
            if last_deposit == current_ledger {
                panic!("Flash-loan prevention: Recipient cannot receive payout and deposit in same ledger");
            }
        }
        env.storage().instance().set(&DataKey::LastWithdrawalLedger(recipient.clone()), &current_ledger);

        // Calculate total pot
        let gross_payout = (circle.contribution_amount as i128) * (circle.current_members as i128);
        let organizer_fee = (gross_payout * circle.organizer_fee_bps as i128) / 10_000;
        let net_payout = gross_payout - organizer_fee;

        // TRANCHE-BASED PAYOUT: Split into immediate (70%) and locked tranches (30%)
        let immediate_amount = (net_payout * TRANCHE_IMMEDIATE_PAYOUT_BPS as i128) / 10000;
        
        // Check gas buffer and ensure sufficient funds for transaction
        Self::ensure_gas_buffer(&env, circle_id);

        // Execute immediate payout (70%)
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(
            &env.current_contract_address(),
            &recipient,
            &immediate_amount,
        );

        // Transfer organizer fee
        if organizer_fee > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &circle.creator,
                &organizer_fee,
            );
        }

        // Create tranche schedule for remaining 30%
        let locked_amount = net_payout - immediate_amount;
        if locked_amount > 0 {
            // Store locked amount in contract (will be claimed via tranches)
            // In production, track this separately to prevent double-spending
            tranche_system::create_tranche_schedule(&env, &circle, &recipient, locked_amount);
        }

        // Record contribution for this round (for tranche eligibility)
        tranche_system::record_contribution(&env, circle_id, &recipient, true);

        // Update circle state
        circle.current_round += 1;
        circle.round_start_time = env.ledger().timestamp();
        circle.is_round_finalized = false;
        circle.current_pot_recipient = None;

        // Reset contribution status for all members
        Self::reset_contributions(&env, circle_id);

        // Store updated circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Emit events
        env.events().publish(
            (Symbol::new(&env, "payout_distributed"), circle_id),
            (recipient.clone(), immediate_amount),
        );

        env.events().publish(
            (Symbol::new(&env, "TRANCHE_SCHEDULE_CREATED"), circle_id, recipient.clone()),
            (locked_amount, TRANCHE_COUNT, env.ledger().timestamp()),
        );

        if organizer_fee > 0 {
            env.events().publish(
                (Symbol::new(&env, "commission_paid"), circle_id),
                (circle.creator, organizer_fee),
            );
        }
    }

    fn trigger_payout(env: Env, admin: Address, circle_id: u64) {
        // Admin-only function
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can trigger payout");
        }

        // Call distribute_payout with admin as caller
        Self::distribute_payout(env, admin, circle_id);
    }

    fn finalize_round(env: Env, creator: Address, circle_id: u64) {
        // Check authorization (only creator can finalize)
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        if creator != circle.creator {
            panic!("Only creator can finalize round");
        }

        // Check if all members have contributed
        if !Self::all_members_contributed(&env, circle_id) {
            panic!("Not all members have contributed this cycle");
        }

        // Determine next recipient (simple round-robin for now)
        let next_recipient_index = circle.current_round % (circle.current_members as u32);
        let next_recipient = env.storage().instance()
            .get(&DataKey::MemberByIndex(circle_id, next_recipient_index))
            .unwrap_or_else(|| panic!("Member not found for next round"));

        // Update circle state
        let mut updated_circle = circle;
        updated_circle.is_round_finalized = true;
        updated_circle.current_pot_recipient = Some(next_recipient);
        updated_circle.round_start_time = env.ledger().timestamp();

        // Store updated circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &updated_circle);

        // Schedule payout time
        let scheduled_time = env.ledger().timestamp() + updated_circle.cycle_duration;
        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &scheduled_time);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "round_finalized"), circle_id),
            (next_recipient, scheduled_time),
        );
    }

    // --- HELPER FUNCTIONS ---

    fn get_circle(env: Env, circle_id: u64) -> CircleInfo {
        env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"))
    }

    fn get_member(env: Env, member: Address) -> Member {
        env.storage().instance()
            .get(&DataKey::Member(member))
            .unwrap_or_else(|| panic!("Member not found"))
    }

    fn get_current_recipient(env: Env, circle_id: u64) -> Option<Address> {
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // If circle is finalized and has a designated recipient, use that
        if circle.is_round_finalized {
            return circle.current_pot_recipient;
        }

        // Otherwise, determine based on round number (round-robin)
        if circle.current_members == 0 {
            return None;
        }

        let recipient_index = circle.current_round % (circle.current_members as u32);
        env.storage().instance()
            .get(&DataKey::MemberByIndex(circle_id, recipient_index))
    }

    // --- TRANCHE-BASED PAYOUT SYSTEM ---

    fn claim_tranche(env: Env, member: Address, circle_id: u64, tranche_index: u32) {
        member.require_auth();
        
        let result = tranche_system::claim_tranche(&env, &member, circle_id, tranche_index);
        
        match result {
            Ok(amount) => {
                env.events().publish(
                    (Symbol::new(&env, "TRANCHE_CLAIM_SUCCESS"), circle_id, member.clone()),
                    (amount, tranche_index),
                );
            }
            Err(e) => {
                panic!("Tranche claim failed: {:?}", e);
            }
        }
    }

    fn get_tranche_schedule(env: Env, circle_id: u64, winner: Address) -> Option<TrancheSchedule> {
        env.storage().instance().get(&DataKey::TrancheSchedule(circle_id, winner))
    }

    fn mark_member_defaulted(env: Env, admin: Address, circle_id: u64, member: Address) {
        admin.require_auth();
        
        let result = tranche_system::mark_member_defaulted(&env, &admin, circle_id, &member);
        
        match result {
            Ok(_) => {
                env.events().publish(
                    (Symbol::new(&env, "MEMBER_DEFAULT_MARKED"), circle_id, member.clone()),
                    (env.ledger().timestamp(), admin),
                );
            }
            Err(e) => {
                panic!("Failed to mark member defaulted: {:?}", e);
            }
        }
    }

    fn execute_tranche_clawback(env: Env, admin: Address, circle_id: u64, member: Address) {
        admin.require_auth();
        
        let result = tranche_system::execute_tranche_clawback(&env, &admin, circle_id, &member);
        
        match result {
            Ok(clawed_amount) => {
                env.events().publish(
                    (Symbol::new(&env, "TRANCHE_CLAWBACK_SUCCESS"), circle_id, member.clone()),
                    (clawed_amount, env.ledger().timestamp()),
                );
            }
            Err(e) => {
                panic!("Tranche clawback failed: {:?}", e);
            }
        }
    }

    // --- STELLAR ANCHOR DIRECT DEPOSIT API (SEP-24/SEP-31) ---

    fn register_anchor(env: Env, admin: Address, anchor_info: AnchorInfo) {
        // Only admin can register anchors
        admin.require_auth();
        
        // Verify admin is contract admin
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not found"));
        
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can register anchors");
        }

        // Store anchor info in registry
        let mut anchor_registry: Map<Address, AnchorInfo> = env.storage().instance()
            .get(&DataKey::AnchorRegistry)
            .unwrap_or_else(|| Map::new(&env));
        
        anchor_registry.set(anchor_info.anchor_address.clone(), anchor_info.clone());
        env.storage().instance().set(&DataKey::AnchorRegistry, &anchor_registry);

        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: admin,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: 0, // Use 0 for anchor registration
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    fn deposit_for_user(
        env: Env,
        anchor: Address,
        beneficiary_user: Address,
        circle_id: u64,
        amount: i128,
        deposit_memo: String,
        fiat_reference: String,
        sep_type: String,
    ) {
        // Authorization: The anchor must sign this!
        anchor.require_auth();

        // Verify anchor is registered and authorized
        let anchor_registry: Map<Address, AnchorInfo> = env.storage::instance()
            .get(&DataKey::AnchorRegistry)
            .unwrap_or_else(|| panic!("Anchor registry not found"));
        
        let anchor_info: AnchorInfo = anchor_registry.get(anchor.clone())
            .unwrap_or_else(|| panic!("Anchor not found"));

        if !anchor_info.is_active {
            panic!("Anchor not active");
        }

        // Verify SEP type is supported
        if sep_type != "SEP-24" && sep_type != "SEP-31" {
            panic!("Unsupported SEP type");
        }

        // Compliance checks
        if amount > anchor_info.max_deposit_amount {
            panic!("Amount exceeds anchor's maximum deposit limit");
        }

        // Check if deposit memo already processed (prevent double processing)
        let memo_key = DataKey::DepositMemo(circle_id);
        let mut processed_memos: Vec<String> = env.storage::instance()
            .get(&memo_key)
            .unwrap_or_else(|| Vec::new(&env));
        
        if processed_memos.contains(&deposit_memo) {
            panic!("Deposit already processed");
        }

        // Get the circle
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Get the member
        let mut member: Member = env.storage().instance()
            .get(&DataKey::Member(beneficiary_user.clone()))
            .unwrap_or_else(|| panic!("Member not found"));

        // Check if already contributed this round
        if member.has_contributed_current_round {
            panic!("Already contributed this round");
        }

        // Calculate the total amount needed (contribution + insurance fee + group insurance premium)
        let insurance_fee = (circle.contribution_amount as i128 * circle.insurance_fee_bps as i128) / 10_000;
        let group_insurance_premium = (circle.contribution_amount as i128 * 50i128) / 10_000;
        let total_amount = circle.contribution_amount as i128 + insurance_fee + group_insurance_premium;

        // Verify amount matches expected contribution
        if amount != total_amount {
            panic!("Amount does not match required contribution");
        }

        // Create deposit record
        let deposit_id = env.ledger().sequence(); // Use ledger sequence as unique ID
        let deposit_record = AnchorDeposit {
            deposit_id,
            anchor_address: anchor.clone(),
            beneficiary_user: beneficiary_user.clone(),
            circle_id,
            amount,
            deposit_memo: deposit_memo.clone(),
            fiat_reference,
            timestamp: env.ledger().timestamp(),
            compliance_verified: true,
            processed: false,
            sep_type,
        };

        // Store deposit record
        env.storage().instance().set(&DataKey::AnchorDeposit(deposit_id), &deposit_record);

        // Mark memo as processed
        processed_memos.push_back(deposit_memo);
        env.storage().instance().set(&memo_key, &processed_memos);

        // Transfer the tokens from anchor to contract
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&anchor, &env.current_contract_address(), &total_amount);

        // Update member record (similar to regular deposit)
        member.has_contributed_current_round = true;
        member.last_contribution_time = env.ledger().timestamp();
        member.contribution_count += 1;
        member.total_contributions += total_amount;

        // Update user stats
        let mut user_stats: UserStats = env.storage().instance()
            .get(&DataKey::UserStats(beneficiary_user.clone()))
            .unwrap_or_else(|| UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });
        
        user_stats.total_volume_saved += total_amount;
        user_stats.on_time_contributions += 1;
        env.storage().instance().set(&DataKey::UserStats(beneficiary_user.clone()), &user_stats);

        // Store the updated member
        env.storage().instance().set(&DataKey::Member(beneficiary_user.clone()), &member);

        // Update circle contribution bitmap
        let member_index = member.index;
        circle.contribution_bitmap |= 1u64 << member_index;

        // Store the updated circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Update anchor's last activity
        let mut updated_anchor_info = anchor_info.clone();
        updated_anchor_info.last_activity = env.ledger().timestamp();
        anchor_registry.set(anchor.clone(), updated_anchor_info);
        env.storage().instance().set(&DataKey::AnchorRegistry, &anchor_registry);

        // Mark deposit as processed
        let mut updated_deposit = deposit_record;
        updated_deposit.processed = true;
        env.storage().instance().set(&DataKey::AnchorDeposit(deposit_id), &updated_deposit);

        // Log audit entry
        let audit_count: u64 = env.storage::instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: anchor,
            action: AuditAction::AdminAction, // Use AdminAction for anchor deposits
            timestamp: env.ledger().timestamp(),
            resource_id: circle_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    fn verify_anchor_deposit(env: Env, deposit_id: u64) -> bool {
        let deposit: AnchorDeposit = env.storage().instance()
            .get(&DataKey::AnchorDeposit(deposit_id))
            .unwrap_or_else(|| panic!("Deposit not found"));
        
        deposit.processed && deposit.compliance_verified
    }

    fn get_anchor_info(env: Env, anchor_address: Address) -> AnchorInfo {
        let anchor_registry: Map<Address, AnchorInfo> = env.storage::instance()
            .get(&DataKey::AnchorRegistry)
            .unwrap_or_else(|| panic!("Anchor registry not found"));
        
        anchor_registry.get(anchor_address)
            .unwrap_or_else(|| panic!("Anchor not found"))
    }

    fn get_deposit_record(env: Env, deposit_id: u64) -> AnchorDeposit {
        env.storage().instance()
            .get(&DataKey::AnchorDeposit(deposit_id))
            .unwrap_or_else(|| panic!("Deposit not found"))
    }

    // --- INTERNAL HELPER FUNCTIONS ---

    fn all_members_contributed(env: &Env, circle_id: u64) -> bool {
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        if circle.current_members == 0 {
            return false;
        }

        // Check if every member has contributed
        for member in circle.members.iter() {
            if !circle.contributions.get(member).unwrap_or(false) {
                return false;
            }
        }

        true
    }

    fn ensure_gas_buffer(env: &Env, circle_id: u64) {
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        let config: GasBufferConfig = env.storage::instance()
            .get(&DataKey::GasBufferConfig(circle_id))
            .unwrap_or_else(|| panic!("Gas buffer config not found"));

        // Check if gas buffer is enabled
        if !circle.gas_buffer_enabled {
            return;
        }

        // Check if buffer needs refilling
        if circle.gas_buffer_balance < config.auto_refill_threshold {
            // Use emergency buffer if available
            if circle.gas_buffer_balance >= config.emergency_buffer {
                // Allow payout but emit warning
                env.events().publish(
                    (Symbol::new(&env, "gas_buffer_warning"), circle_id),
                    ("Low gas buffer", circle.gas_buffer_balance),
                );
            } else {
                // Critical: buffer too low, attempt auto-refill from emergency funds
                if config.emergency_buffer > 0 {
                    env.events().publish(
                        (Symbol::new(&env, "emergency_gas_usage"), circle_id),
                        ("Using emergency buffer", config.emergency_buffer),
                    );
                    circle.gas_buffer_balance += config.emergency_buffer;
                    env.storage::instance().set(&DataKey::Circle(circle_id), &circle);
                } else {
                    panic!("Insufficient gas buffer for payout. Please fund the gas buffer.");
                }
            }
        }
    }

    fn execute_payout_with_gas_protection(
        env: &Env,
        circle: &CircleInfo,
        recipient: &Address,
        organizer: &Address,
        net_payout: i128,
        organizer_fee: i128,
    ) -> Result<(), ()> {
        let token_client = token::Client::new(env, &circle.token);

        // Calculate estimated gas cost (conservative estimate)
        let estimated_gas_cost = 2000000i128; // 2 XLM conservative estimate
        
        // Check if we have enough gas buffer
        if circle.gas_buffer_balance < estimated_gas_cost {
            return Err(());
        }

        // Execute transfers
        token_client.transfer(
            &env.current_contract_address(),
            recipient,
            &net_payout,
        );

        if organizer_fee > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                organizer,
                &organizer_fee,
            );
        }

        // Deduct gas cost from buffer (in a real implementation, this would be the actual gas used)
        let mut updated_circle = circle.clone();
        updated_circle.gas_buffer_balance -= estimated_gas_cost;
        env.storage::instance().set(&DataKey::Circle(circle_id), &updated_circle);

        Ok(())
    }

    fn check_and_finalize_round(env: &Env, circle_id: u64) {
        if Self::all_members_contributed(env, circle_id) {
            let circle: CircleInfo = env.storage::instance()
                .get(&DataKey::Circle(circle_id))
                .unwrap_or_else(|| panic!("Circle not found"));

            if !circle.is_round_finalized {
                // Auto-finalize the round
                Self::finalize_round(env.clone(), circle.creator.clone(), circle_id);
            }
        }
    }

    fn reset_contributions(env: &Env, circle_id: u64) {
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Clear all contribution statuses
        circle.contributions = Map::new(env);

        // Reset member contribution flags
        for member in circle.members.iter() {
            let mut member_info: Member = env.storage::instance()
                .get(&DataKey::Member(member))
                .unwrap_or_else(|| panic!("Member not found"));
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
            requires_collateral,
            collateral_bps,
            member_addresses: Vec::new(&env),
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            dissolution_status: DissolutionStatus::NotInitiated,
            dissolution_deadline: None,
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            recovery_old_address: None,
            recovery_new_address: None,
            recovery_votes_bitmap: 0,
            arbitrator,
            basket: None,
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

        // Member-Specific Contribution Cap via Reputation (Sybil-Resistance)
        // Prevent "Whales" from over-leveraging small groups and "Pump and Default" schemes
        let requested_contribution = circle.contribution_amount * tier_multiplier as i128;
        let max_allowed_contribution = Self::calculate_contribution_cap(&env, &user, requested_contribution);
        
        // Enforce contribution cap
        if requested_contribution > max_allowed_contribution {
            panic!(
                "Contribution cap exceeded: Requested {}, Maximum allowed based on reputation history {}. 
                Build trust gradually by starting with smaller contributions and completing cycles.",
                requested_contribution, max_allowed_contribution
            );
        }
        
        // Get user stats for event emission
        let user_stats_key = DataKey::UserStats(user.clone());
        let user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        
        let (_, _, total_cycles) = crate::sbt_minter::SoroSusuSbtMinter::get_user_reputation_score(env.clone(), user.clone());
        
        // Emit event for reputation-based contribution validation
        env.events().publish(
            (Symbol::new(&env, "CONTRIBUTION_CAP_VALIDATION"), user.clone(), circle_id),
            (requested_contribution, max_allowed_contribution, total_cycles, user_stats.total_volume_saved),
        );

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
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        // Check if contribution is late
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

        // Update user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        if penalty_amount > 0 {
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

        // Set round as finalized and determine next recipient
        circle.is_round_finalized = true;
        
        // Set next recipient (round-robin)
        let next_recipient_index = (circle.current_recipient_index + 1) % circle.member_count;
        let next_recipient = get_member_address_by_index(&circle, next_recipient_index);
        
        circle.current_recipient_index = next_recipient_index;
        circle.current_pot_recipient = Some(next_recipient.clone());

        // Schedule payout time (end of month from now)
        let current_time = env.ledger().timestamp();
        let payout_time = current_time + (30 * 24 * 60 * 60); // 30 days from now
        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &payout_time);

        // Reset for next round
        circle.contribution_bitmap = 0;
        circle.deadline_timestamp = current_time + circle.cycle_duration;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Publish round finalization event
        env.events().publish(
            (Symbol::new(&env, "ROUND_FINALIZED"), circle_id),
            (next_recipient, payout_time, next_recipient_index),
        );


    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let is_paused = env
            .storage()
            .instance()
            .get::<DataKey, bool>(&DataKey::PausedPayout(user.clone(), circle_id))
            .unwrap_or(false);
        if is_paused {
            panic!("Your payout is currently locked due to a default in a connected LeaseFlow agreement.");
        }

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
        
        // Check for rollover bonus and add to first pot of new cycles
        let mut total_payout = pot_amount;
        let rollover_key = DataKey::RolloverBonus(circle_id);
        if let Some(rollover_bonus) = env.storage().instance().get::<DataKey, RolloverBonus>(&rollover_key) {
            if rollover_bonus.status == RolloverStatus::Applied {
                if let Some(applied_cycle) = rollover_bonus.applied_cycle {
                    if applied_cycle == circle.current_recipient_index as u64 {
                        total_payout += rollover_bonus.bonus_amount;
                        
                        env.events().publish(
                            (Symbol::new(&env, "ROLLOVER_BONUS_APPLIED"), circle_id, user.clone()),
                            (rollover_bonus.bonus_amount, applied_cycle),
                        );
                    }
                }
            }
        }
        
        let fee_bps: u32 = env.storage().instance().get(&DataKey::ProtocolFeeBps).unwrap_or(0);
        let payout_destination = env
            .storage()
            .instance()
            .get::<DataKey, LeaseFlowPayoutAuthorization>(&DataKey::LeaseFlowPayoutAuthorization(user.clone(), circle_id))
            .filter(|authorization| authorization.is_active)
            .map(|authorization| authorization.lease_instance)
            .unwrap_or_else(|| user.clone());
        let payout_redirected = payout_destination != user;

        if let Some(ref basket) = circle.basket.clone() {
            // Basket circle: distribute each asset to the winner proportionally
            let maybe_treasury: Option<Address> = if fee_bps > 0 {
                env.storage().instance().get(&DataKey::ProtocolTreasury)
            } else {
                None
            };

            for i in 0..basket.len() {
                let asset_weight = basket.get(i).unwrap();
                // Total pot for this asset = contribution_amount * member_count * weight / 10000
                let asset_pot = (circle.contribution_amount
                    * (circle.member_count as i128)
                    * asset_weight.weight_bps as i128)
                    / 10000;

                let token_client = token::Client::new(&env, &asset_weight.token);

                if fee_bps > 0 {
                    let treasury = maybe_treasury
                        .clone()
                        .expect("Treasury not set");
                    let fee = (asset_pot * fee_bps as i128) / 10000;
                    let net = asset_pot - fee;
                    token_client.transfer(&env.current_contract_address(), &treasury, &fee);
                    token_client.transfer(&env.current_contract_address(), &payout_destination, &net);
                } else {
                    token_client.transfer(&env.current_contract_address(), &payout_destination, &asset_pot);
                }
            }

            env.events().publish(
                (Symbol::new(&env, "BASKET_POT_CLAIMED"), circle_id, user.clone()),
                (basket.len(), circle.member_count),
            );
        } else {
            // Single-token circle (original logic)
            let token_client = token::Client::new(&env, &circle.token);

            if fee_bps > 0 {
                let treasury: Address = env
                    .storage()
                    .instance()
                    .get(&DataKey::ProtocolTreasury)
                    .expect("Treasury not set");
                let fee = (total_payout * fee_bps as i128) / 10000;
                let net_payout = total_payout - fee;
                token_client.transfer(&env.current_contract_address(), &treasury, &fee);
                token_client.transfer(&env.current_contract_address(), &payout_destination, &net_payout);
            } else {
                token_client.transfer(&env.current_contract_address(), &payout_destination, &total_payout);
            }
        }

        if payout_redirected {
            env.events().publish(
                (Symbol::new(&env, "LEASEFLOW_RENT_DRIP"), circle_id, user.clone()),
                (payout_destination.clone(), total_payout),
            );
        }

        // Auto-release collateral if member has completed all contributions
        if circle.requires_collateral {
            let token_client = token::Client::new(&env, &circle.token);
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

        // Mint soulbound "Susu Master" badge when the full cycle completes
        let next_index = circle.current_recipient_index + 1;
        if next_index >= circle.max_members {
            let member_key = DataKey::Member(user.clone());
            if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
                let stats_key = DataKey::UserStats(user.clone());
                let stats: UserStats = env.storage().instance().get(&stats_key).unwrap_or(UserStats {
                    total_volume_saved: 0,
                    on_time_contributions: 0,
                    late_contributions: 0,
                });
                let volume_tier: u32 = if stats.total_volume_saved >= 10_000_000_000 { 3 }
                    else if stats.total_volume_saved >= 1_000_000_000 { 2 }
                    else { 1 };
                let metadata = NftBadgeMetadata {
                    volume_tier,
                    perfect_attendance: stats.late_contributions == 0,
                    group_lead_status: member_info.address == circle.creator,
                };
                // token_id: circle_id in upper 64 bits, member index in lower 64 bits
                let token_id: u128 = ((circle_id as u128) << 64) | (member_info.index as u128);
                let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
                nft_client.mint_badge(&user, &token_id, &metadata);
                env.storage().instance().set(&DataKey::CycleBadge(user.clone(), circle_id), &token_id);
                env.events().publish(
                    (symbol_short!("BADGE"), symbol_short!("MINT")),
                    (user.clone(), circle_id, token_id, metadata),
                );
            }
        }

        circle.current_recipient_index = next_index;
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

        let member_key = DataKey::Member(member.clone());
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

        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("Not a member");

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

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

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

    fn get_reputation(env: Env, user: Address) -> ReputationData {
        let current_time = env.ledger().timestamp();
        
        // Get user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        // Get member information to check if user is active
        let member_key = DataKey::Member(user.clone());
        let is_active = if let Some(member) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            member.status == MemberStatus::Active
        } else {
            false
        };

        // Calculate total contributions
        let total_contributions = user_stats.on_time_contributions + user_stats.late_contributions;
        
        // Calculate on-time rate (in basis points)
        let on_time_rate = if total_contributions > 0 {
            (user_stats.on_time_contributions * 10000) / total_contributions
        } else {
            0
        };

        // Calculate reliability score based on on-time rate and volume
        let mut reliability_score = on_time_rate;
        
        // Boost reliability based on volume saved (higher volume = higher reliability)
        if user_stats.total_volume_saved > 0 {
            let volume_bonus = (((user_stats.total_volume_saved / 1_000_000_0) * 100).min(2000)) as u32; // Max 20% bonus
            reliability_score = (reliability_score + volume_bonus).min(10000);
        }

        // Calculate social capital (sum of trust scores across all circles)
        let mut social_capital = 0u32;
        let mut circle_count = 0u32;
        
        // Get all circles the user is part of by checking member data
        // For now, we'll use a simplified approach - in a full implementation,
        // you might want to maintain an index of user's circles
        for circle_id in 1..=1000 { // Reasonable limit for iteration
            let circle_key = DataKey::Circle(circle_id);
            if let Some(_circle) = env.storage().instance().get::<DataKey, CircleInfo>(&circle_key) {
                let social_capital_key = DataKey::SocialCapital(user.clone(), circle_id);
                if let Some(soc_cap) = env.storage().instance().get::<DataKey, SocialCapital>(&social_capital_key) {
                    social_capital += soc_cap.trust_score;
                    circle_count += 1;
                }
            }
        }

        // Average social capital across circles
        let avg_social_capital = if circle_count > 0 {
            (social_capital / circle_count) * 100 // Convert to basis points
        } else {
            0
        };

        // Calculate final Susu Score (weighted combination)
        // Weight: 50% reliability, 30% social capital, 20% activity
        let activity_score = if total_contributions > 0 {
            ((total_contributions as u32).min(50) * 200) // Max 10% from activity
        } else {
            0
        };

        let susu_score = (
            (reliability_score * 50) / 100 +  // 50% weight
            (avg_social_capital * 30) / 100 +  // 30% weight  
            (activity_score * 20) / 100         // 20% weight
        ).min(10000);

        ReputationData {
            user_address: user.clone(),
            susu_score,
            reliability_score,
            total_contributions,
            on_time_rate,
            volume_saved: user_stats.total_volume_saved,
            social_capital: avg_social_capital,
            last_updated: current_time,
            is_active,
        }
    }

    fn propose_rollover_bonus(env: Env, user: Address, circle_id: u64, fee_percentage_bps: u32) {
        user.require_auth();

        if fee_percentage_bps > 10000 {
            panic!("Fee percentage cannot exceed 100%");
        }

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active rollover proposal
        let rollover_key = DataKey::RolloverBonus(circle_id);
        if let Some(existing_rollover) = env.storage().instance().get::<DataKey, RolloverBonus>(&rollover_key) {
            if existing_rollover.status == RolloverStatus::Voting {
                panic!("Rollover bonus proposal already active");
            }
        }

        // Only allow rollover proposals after the first round is complete
        if !circle.is_round_finalized || circle.current_recipient_index == 0 {
            panic!("Rollover can only be proposed after first complete cycle");
        }

        let current_time = env.ledger().timestamp();
        let bonus_amount = calculate_rollover_bonus(&env, circle_id, fee_percentage_bps);

        let rollover_bonus = RolloverBonus {
            circle_id,
            bonus_amount,
            fee_percentage: fee_percentage_bps,
            created_timestamp: current_time,
            status: RolloverStatus::Voting,
            voting_deadline: current_time + ROLLOVER_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            applied_cycle: None,
        };

        env.storage().instance().set(&rollover_key, &rollover_bonus);
        
        // The proposer automatically votes for
        let vote_key = DataKey::RolloverVote(circle_id, user.clone());
        let vote = RolloverVote {
            voter: user.clone(),
            circle_id,
            vote_choice: RolloverVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_rollover = rollover_bonus;
        updated_rollover.for_votes = 1;
        updated_rollover.total_votes_cast = 1;
        env.storage().instance().set(&rollover_key, &updated_rollover);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_PROPOSED"), circle_id, user.clone()),
            (bonus_amount, fee_percentage_bps, updated_rollover.voting_deadline),
        );
    }

    fn vote_rollover_bonus(env: Env, user: Address, circle_id: u64, vote_choice: RolloverVoteChoice) {
        user.require_auth();

        let rollover_key = DataKey::RolloverBonus(circle_id);
        let mut rollover_bonus: RolloverBonus = env.storage().instance().get(&rollover_key)
            .expect("No active rollover proposal");

        if rollover_bonus.status != RolloverStatus::Voting {
            panic!("Rollover proposal is not in voting period");
        }

        if env.ledger().timestamp() > rollover_bonus.voting_deadline {
            rollover_bonus.status = RolloverStatus::Rejected;
            env.storage().instance().set(&rollover_key, &rollover_bonus);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::RolloverVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = RolloverVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            RolloverVoteChoice::For => rollover_bonus.for_votes += 1,
            RolloverVoteChoice::Against => rollover_bonus.against_votes += 1,
        }
        rollover_bonus.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (rollover_bonus.total_votes_cast * 100) >= (active_members * ROLLOVER_QUORUM);
        
        if quorum_met && rollover_bonus.total_votes_cast > 0 {
            let approval_percentage = (rollover_bonus.for_votes * 100) / rollover_bonus.total_votes_cast;
            if approval_percentage >= ROLLOVER_MAJORITY {
                rollover_bonus.status = RolloverStatus::Approved;
            }
        }

        env.storage().instance().set(&rollover_key, &rollover_bonus);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_VOTE"), circle_id, user.clone()),
            (vote_choice, rollover_bonus.for_votes, rollover_bonus.against_votes),
        );
    }

    fn apply_rollover_bonus(env: Env, circle_id: u64) {
        let rollover_key = DataKey::RolloverBonus(circle_id);
        let mut rollover_bonus: RolloverBonus = env.storage().instance().get(&rollover_key)
            .expect("No rollover bonus proposal found");

        if rollover_bonus.status != RolloverStatus::Approved {
            panic!("Rollover bonus is not approved");
        }

        let circle_key = DataKey::Circle(circle_id);
        let mut circle: CircleInfo = env.storage().instance().get(&circle_key)
            .expect("Circle not found");



        // Mark as applied and track the cycle
        rollover_bonus.status = RolloverStatus::Applied;
        rollover_bonus.applied_cycle = Some((circle.current_recipient_index + 1) as u64);
        env.storage().instance().set(&rollover_key, &rollover_bonus);

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_APPLIED"), circle_id),
            (rollover_bonus.bonus_amount, rollover_bonus.applied_cycle.unwrap()),
        );
    }

    fn propose_yield_delegation(env: Env, user: Address, circle_id: u64, delegation_percentage: u32, pool_address: Address, pool_type: YieldPoolType) {
        user.require_auth();

        if delegation_percentage > MAX_DELEGATION_PERCENTAGE {
            panic!("Delegation percentage exceeds maximum");
        }

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active yield delegation proposal
        let delegation_key = DataKey::YieldDelegation(circle_id);
        if let Some(existing_delegation) = env.storage().instance().get::<DataKey, YieldDelegation>(&delegation_key) {
            if existing_delegation.status == YieldDelegationStatus::Voting || 
               existing_delegation.status == YieldDelegationStatus::Active {
                panic!("Yield delegation already active");
            }
        }

        // Only allow yield delegation after round is finalized but before payout
        if !circle.is_round_finalized {
            panic!("Round must be finalized before yield delegation");
        }

        let current_time = env.ledger().timestamp();
        let pot_amount = circle.contribution_amount * (circle.member_count as i128);
        let delegation_amount = (pot_amount * delegation_percentage as i128) / 10000;

        if delegation_amount < MIN_DELEGATION_AMOUNT {
            panic!("Delegation amount below minimum");
        }

        let yield_delegation = YieldDelegation {
            circle_id,
            delegation_amount,
            pool_address: pool_address.clone(),
            pool_type: pool_type.clone(),
            delegation_percentage,
            created_timestamp: current_time,
            status: YieldDelegationStatus::Voting,
            voting_deadline: current_time + YIELD_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            start_time: None,
            end_time: None,
            total_yield_earned: 0,
            yield_distributed: 0,
            last_compound_time: current_time,
        };

        env.storage().instance().set(&delegation_key, &yield_delegation);
        
        // The proposer automatically votes for
        let vote_key = DataKey::YieldVote(circle_id, user.clone());
        let vote = YieldVote {
            voter: user.clone(),
            circle_id,
            vote_choice: YieldVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_delegation = yield_delegation;
        updated_delegation.for_votes = 1;
        updated_delegation.total_votes_cast = 1;
        env.storage().instance().set(&delegation_key, &updated_delegation);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_PROPOSED"), circle_id, user.clone()),
            (delegation_amount, delegation_percentage, pool_address, updated_delegation.voting_deadline),
        );
    }

    fn vote_yield_delegation(env: Env, user: Address, circle_id: u64, vote_choice: YieldVoteChoice) {
        user.require_auth();

        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No active yield delegation proposal");

        if delegation.status != YieldDelegationStatus::Voting {
            panic!("Yield delegation is not in voting period");
        }

        if env.ledger().timestamp() > delegation.voting_deadline {
            delegation.status = YieldDelegationStatus::Rejected;
            env.storage().instance().set(&delegation_key, &delegation);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::YieldVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = YieldVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            YieldVoteChoice::For => delegation.for_votes += 1,
            YieldVoteChoice::Against => delegation.against_votes += 1,
        }
        delegation.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (delegation.total_votes_cast * 100) >= (active_members * YIELD_QUORUM);
        
        if quorum_met && delegation.total_votes_cast > 0 {
            let approval_percentage = (delegation.for_votes * 100) / delegation.total_votes_cast;
            if approval_percentage >= YIELD_MAJORITY {
                delegation.status = YieldDelegationStatus::Approved;
            }
        }

        env.storage().instance().set(&delegation_key, &delegation);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_VOTE"), circle_id, user.clone()),
            (vote_choice, delegation.for_votes, delegation.against_votes),
        );
    }

    fn approve_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation proposal found");

        if delegation.status != YieldDelegationStatus::Approved {
            panic!("Yield delegation is not approved");
        }

        // Register the yield pool if not already registered
        let pool_registry_key = DataKey::YieldPoolRegistry;
        let mut pool_registry: Vec<Address> = env.storage().instance().get(&pool_registry_key).unwrap_or(Vec::new(&env));
        
        if !pool_registry.contains(&delegation.pool_address) {
            pool_registry.push_back(delegation.pool_address.clone());
            env.storage().instance().set(&pool_registry_key, &pool_registry);
        }

        // Update pool info
        let pool_info = YieldPoolInfo {
            pool_address: delegation.pool_address.clone(),
            pool_type: delegation.pool_type.clone(),
            is_active: true,
            total_delegated: delegation.delegation_amount,
            apy_bps: 500, // Default 5% APY (would be fetched from pool)
            last_updated: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::YieldDelegation(circle_id), &pool_info);

        // Execute the delegation
        execute_yield_delegation_internal(&env, circle_id, &mut delegation);

        env.storage().instance().set(&delegation_key, &delegation);
        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_APPROVED"), circle_id),
            (delegation.delegation_amount, delegation.pool_address),
        );
    }

    fn execute_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Approved && delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not approved");
        }

        execute_yield_delegation_internal(&env, circle_id, &mut delegation);
        env.storage().instance().set(&delegation_key, &delegation);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_EXECUTED"), circle_id),
            (delegation.delegation_amount, delegation.pool_address),
        );
    }

    fn compound_yield(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not active");
        }

        let current_time = env.ledger().timestamp();
        if current_time < delegation.last_compound_time + YIELD_COMPOUNDING_FREQUENCY {
            panic!("Too early to compound");
        }

        // Calculate yield (simplified - would query actual yield from pool)
        let time_elapsed = current_time - delegation.last_compound_time;
        let yield_earned = calculate_yield_from_pool(&env, &delegation, time_elapsed);

        delegation.total_yield_earned += yield_earned;
        delegation.last_compound_time = current_time;

        env.storage().instance().set(&delegation_key, &delegation);

        env.events().publish(
            (Symbol::new(&env, "YIELD_COMPOUNDED"), circle_id),
            (yield_earned, delegation.total_yield_earned),
        );
    }

    fn withdraw_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not active");
        }

        // Final compound before withdrawal
        let current_time = env.ledger().timestamp();
        let time_elapsed = current_time - delegation.last_compound_time;
        let final_yield = calculate_yield_from_pool(&env, &delegation, time_elapsed);
        delegation.total_yield_earned += final_yield;

        // Withdraw from pool (simplified - would call actual pool contract)
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let token_client = token::Client::new(&env, &circle.token);
        
        // In real implementation, this would withdraw from the actual yield pool
        let total_withdrawn = delegation.delegation_amount + delegation.total_yield_earned;
        
        // Return funds to contract
        // token_client.transfer(&delegation.pool_address, &env.current_contract_address(), &total_withdrawn);

        delegation.status = YieldDelegationStatus::Completed;
        delegation.end_time = Some(current_time);

        env.storage().instance().set(&delegation_key, &delegation);

        // Distribute earnings
        Self::distribute_yield_earnings(env.clone(), circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_WITHDRAWN"), circle_id),
            (total_withdrawn, delegation.total_yield_earned),
        );
    }

    fn distribute_yield_earnings(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.total_yield_earned <= delegation.yield_distributed {
            panic!("No new yield to distribute");
        }

        let new_yield = delegation.total_yield_earned - delegation.yield_distributed;
        
        // Calculate 50/50 split
        let recipient_share = (new_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = (new_yield * YIELD_DISTRIBUTION_TREASURY_BPS as i128) / 10000;

        // Get current round recipient
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        if let Some(recipient) = &circle.current_pot_recipient {
            // Transfer to current recipient
            let token_client = token::Client::new(&env, &circle.token);
            // token_client.transfer(&env.current_contract_address(), recipient, &recipient_share);
        }

        // Add to group treasury
        let treasury_key = DataKey::GroupTreasury(circle_id);
        let mut treasury: i128 = env.storage().instance().get(&treasury_key).unwrap_or(0);
        treasury += treasury_share;
        env.storage().instance().set(&treasury_key, &treasury);

        // Update delegation record
        let mut updated_delegation = delegation;
        updated_delegation.yield_distributed += new_yield;
        env.storage().instance().set(&delegation_key, &updated_delegation);

        // Create distribution record
        let distribution = YieldDistribution {
            circle_id,
            recipient_share,
            treasury_share,
            total_yield: new_yield,
            distribution_time: env.ledger().timestamp(),
            round_number: circle.current_recipient_index,
        };

        env.events().publish(
            (Symbol::new(&env, "YIELD_DISTRIBUTED"), circle_id),
            (recipient_share, treasury_share, new_yield),
        );


    }

    fn propose_path_payment_support(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active path payment proposal
        let path_payment_key = DataKey::PathPayment(circle_id);
        if let Some(existing_payment) = env.storage().instance().get::<DataKey, PathPayment>(&path_payment_key) {
            if existing_payment.status == PathPaymentStatus::Proposed || 
               existing_payment.status == PathPaymentStatus::Executing ||
               existing_payment.status == PathPaymentStatus::Completed {
                panic!("Path payment already active");
            }
        }

        let current_time = env.ledger().timestamp();
        
        let path_payment = PathPayment {
            circle_id,
            source_token: circle.token.clone(), // Updated during execution
            target_token: circle.token.clone(),
            source_amount: 0, // Will be set during execution
            target_amount: 0, // Will be calculated during execution
            exchange_rate: 0,
            slippage_bps: 0,
            dex_address: env.current_contract_address(), // Updated during execution
            path_payment: env.current_contract_address(), // Updated during execution
            created_timestamp: current_time,
            status: PathPaymentStatus::Proposed,
            voting_deadline: current_time + PATH_PAYMENT_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            execution_timestamp: None,
            completion_timestamp: None,
            refund_amount: None,
        };

        env.storage().instance().set(&path_payment_key, &path_payment);
        
        // The proposer automatically votes for
        let vote_key = DataKey::PathPaymentVote(circle_id, user.clone());
        let vote = PathPaymentVote {
            voter: user.clone(),
            circle_id,
            vote_choice: PathPaymentVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_payment = path_payment;
        updated_payment.for_votes = 1;
        updated_payment.total_votes_cast = 1;
        env.storage().instance().set(&path_payment_key, &updated_payment);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_PROPOSED"), circle_id, user.clone()),
            (circle.token.clone(), updated_payment.voting_deadline),
        );
    }

    fn vote_path_payment_support(env: Env, user: Address, circle_id: u64, vote_choice: PathPaymentVoteChoice) {
        user.require_auth();

        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No active path payment proposal");

        if payment.status != PathPaymentStatus::Proposed {
            panic!("Path payment is not in voting period");
        }

        if env.ledger().timestamp() > payment.voting_deadline {
            payment.status = PathPaymentStatus::Failed;
            env.storage().instance().set(&payment_key, &payment);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::PathPaymentVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = PathPaymentVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            PathPaymentVoteChoice::For => payment.for_votes += 1,
            PathPaymentVoteChoice::Against => payment.against_votes += 1,
        }
        payment.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (payment.total_votes_cast * 100) >= (active_members * PATH_PAYMENT_QUORUM);
        
        if quorum_met && payment.total_votes_cast > 0 {
            let approval_percentage = (payment.for_votes * 100) / payment.total_votes_cast;
            if approval_percentage >= PATH_PAYMENT_MAJORITY {
                payment.status = PathPaymentStatus::Approved;
            }
        }

        env.storage().instance().set(&payment_key, &payment);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_VOTE"), circle_id, user.clone()),
            (vote_choice, payment.for_votes, payment.against_votes),
        );
    }

    fn approve_path_payment_support(env: Env, circle_id: u64) {
        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No path payment proposal found");

        if payment.status != PathPaymentStatus::Approved {
            panic!("Path payment is not approved");
        }

        payment.status = PathPaymentStatus::Executing;
        env.storage().instance().set(&payment_key, &payment);

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_APPROVED"), circle_id),
            (payment.source_token, payment.target_token),
        );
    }

    fn execute_path_payment(env: Env, user: Address, circle_id: u64, source_token: Address, source_amount: i128) {
        user.require_auth();

        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No path payment proposal found");

        if payment.status != PathPaymentStatus::Approved && payment.status != PathPaymentStatus::Executing {
            panic!("Path payment is not approved for execution");
        }

        // Validate source token is supported
        let source_token_key = DataKey::SupportedTokens(source_token.clone());
        let source_token_info: SupportedToken = env.storage().instance().get(&source_token_key)
            .expect("Source token not supported");

        if !source_token_info.is_active {
            panic!("Source token is not active");
        }

        // Validate minimum amount
        if source_amount < MIN_PATH_PAYMENT_AMOUNT {
            panic!("Amount below minimum path payment");
        }

        // Get target token info (circle's token)
        let target_token_key = DataKey::SupportedTokens(payment.target_token.clone());
        let target_token_info: SupportedToken = env.storage().instance().get(&target_token_key)
            .expect("Target token not supported");

        if !target_token_info.is_active {
            panic!("Target token is not supported");
        }

        let current_time = env.ledger().timestamp();
        
        // Update payment details
        payment.source_token = source_token.clone();
        payment.source_amount = source_amount;
        payment.execution_timestamp = Some(current_time);
        payment.status = PathPaymentStatus::Executing;

        // Execute the swap via Stellar Path Payments
        let (target_amount, exchange_rate, slippage_bps) = execute_stellar_path_payment(
            &env, 
            &source_token, 
            &payment.target_token, 
            source_amount,
            MAX_SLIPPAGE_TOLERANCE_BPS
        );

        // Update payment with execution results
        payment.target_amount = target_amount;
        payment.exchange_rate = exchange_rate;
        payment.slippage_bps = slippage_bps;

        // Deposit target tokens to circle
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let token_client = token::Client::new(&env, &payment.target_token);
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &target_amount);
        
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            payment.status = PathPaymentStatus::Failed;
            payment.refund_amount = Some(source_amount);
            env.storage().instance().set(&payment_key, &payment);
            panic!("Token transfer failed");
        }

        // Update member contribution
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .expect("Member not found");

        let contribution_amount = circle.contribution_amount * member.tier_multiplier as i128;
        
        // Update user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        user_stats.total_volume_saved += contribution_amount;
        user_stats.on_time_contributions += 1;
        env.storage().instance().set(&user_stats_key, &user_stats);

        // Update member and circle
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Mark as completed
        payment.status = PathPaymentStatus::Completed;
        payment.completion_timestamp = Some(current_time);
        env.storage().instance().set(&payment_key, &payment);

        write_audit(&env, &user, AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_EXECUTED"), circle_id, user.clone()),
            (source_amount, target_amount, exchange_rate, slippage_bps),
        );
    }

    fn register_supported_token(env: Env, user: Address, token_address: Address, token_symbol: String, decimals: u32, is_stable: bool) {
        user.require_auth();

        let token_key = DataKey::SupportedTokens(token_address.clone());
        if env.storage().instance().has(&token_key) {
            panic!("Token already registered");
        }

        let current_time = env.ledger().timestamp();
        let supported_token = SupportedToken {
            token_address: token_address.clone(),
            token_symbol: token_symbol.clone(),
            decimals,
            is_stable,
            is_active: true,
            last_updated: current_time,
        };

        env.storage().instance().set(&token_key, &supported_token);

        write_audit(&env, &user, AuditAction::AdminAction, 0);

        env.events().publish(
            (Symbol::new(&env, "TOKEN_REGISTERED"), token_address),
            (token_symbol, decimals, is_stable),
        );
    }

    fn register_dex(env: Env, user: Address, dex_address: Address, dex_name: String, is_trusted: bool) {
        user.require_auth();

        let dex_key = DataKey::DexRegistry(dex_address.clone());
        if env.storage().instance().has(&dex_key) {
            panic!("DEX already registered");
        }

        let current_time = env.ledger().timestamp();
        let dex_info = DexInfo {
            dex_address: dex_address.clone(),
            dex_name: dex_name.clone(),
            supported_pairs: Vec::new(&env),
            is_trusted,
            is_active: true,
            last_updated: current_time,
        };

        env.storage().instance().set(&dex_key, &dex_info);

        write_audit(&env, &user, AuditAction::AdminAction, 0);

        env.events().publish(
            (Symbol::new(&env, "DEX_REGISTERED"), dex_address),
            (dex_name, is_trusted),
        );
    }

    fn set_leaseflow_contract(env: Env, admin: Address, leaseflow: Address) {
        admin.require_auth();

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }

        env.storage().instance().set(&DataKey::LeaseFlowContract, &leaseflow);
        write_audit(&env, &admin, AuditAction::AdminAction, 0);
    }

    fn authorize_leaseflow_payout(env: Env, user: Address, circle_id: u64, lease_instance: Address) {
        user.require_auth();

        if !env.storage().instance().has(&DataKey::LeaseFlowContract) {
            panic!("LeaseFlow contract not configured");
        }
        if !env.storage().instance().has(&DataKey::Circle(circle_id)) {
            panic!("Circle not found");
        }
        if !env.storage().instance().has(&DataKey::Member(user.clone())) {
            panic!("Member not found");
        }

        let authorization = LeaseFlowPayoutAuthorization {
            user: user.clone(),
            circle_id,
            lease_instance: lease_instance.clone(),
            authorized_at: env.ledger().timestamp(),
            is_active: true,
        };

        env.storage().instance().set(
            &DataKey::LeaseFlowPayoutAuthorization(user.clone(), circle_id),
            &authorization,
        );

        env.events().publish(
            (Symbol::new(&env, "LEASEFLOW_PAYOUT_AUTHORIZED"), circle_id, user.clone()),
            (lease_instance, authorization.authorized_at),
        );
    }

    fn revoke_leaseflow_payout(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let auth_key = DataKey::LeaseFlowPayoutAuthorization(user, circle_id);
        if let Some(mut authorization) = env
            .storage()
            .instance()
            .get::<DataKey, LeaseFlowPayoutAuthorization>(&auth_key)
        {
            authorization.is_active = false;
            env.storage().instance().set(&auth_key, &authorization);
        }
    }

    fn get_leaseflow_payout(env: Env, user: Address, circle_id: u64) -> Option<LeaseFlowPayoutAuthorization> {
        env.storage()
            .instance()
            .get(&DataKey::LeaseFlowPayoutAuthorization(user, circle_id))
    }

    fn handle_leaseflow_default(env: Env, leaseflow_contract: Address, user: Address, circle_id: u64) {
        leaseflow_contract.require_auth();

        let trusted_leaseflow: Address = env
            .storage()
            .instance()
            .get(&DataKey::LeaseFlowContract)
            .expect("LeaseFlow contract not configured");
        if trusted_leaseflow != leaseflow_contract {
            panic!("Unauthorized");
        }

        env.storage()
            .instance()
            .set(&DataKey::PausedPayout(user.clone(), circle_id), &true);

        env.events().publish(
            (Symbol::new(&env, "LEASEFLOW_DEFAULT_LOCK"), circle_id, user.clone()),
            (leaseflow_contract, true),
        );
    }

    fn create_basket_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        basket_assets: Vec<Address>,
        basket_weights: Vec<u32>,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
    ) -> u64 {
        creator.require_auth();

        if max_members == 0 {
            panic!("Max members must be greater than zero");
        }

        // Validate basket has at least 2 assets
        if basket_assets.len() < 2 {
            panic!("Basket must contain at least 2 assets");
        }
        if basket_assets.len() != basket_weights.len() {
            panic!("basket_assets and basket_weights must have the same length");
        }

        // Validate weights sum to exactly 10000 bps
        let mut total_weight: u32 = 0;
        for i in 0..basket_weights.len() {
            total_weight = total_weight
                .checked_add(basket_weights.get(i).unwrap())
                .expect("Weight overflow");
        }
        if total_weight != 10000 {
            panic!("Basket weights must sum to exactly 10000 bps (100%)");
        }

        // Rate limit check
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

        // Build AssetWeight basket vec
        let mut basket: Vec<AssetWeight> = Vec::new(&env);
        for i in 0..basket_assets.len() {
            basket.push_back(AssetWeight {
                token: basket_assets.get(i).unwrap(),
                weight_bps: basket_weights.get(i).unwrap(),
            });
        }

        // Determine collateral requirements based on total cycle value
        let total_cycle_value = amount * (max_members as i128);
        let requires_collateral = total_cycle_value >= HIGH_VALUE_THRESHOLD;
        let collateral_bps = if requires_collateral { DEFAULT_COLLATERAL_BPS } else { 0 };

        // Primary token is the first basket asset (for legacy single-token compatibility)
        let primary_token = basket_assets.get(0).unwrap();

        let new_circle = CircleInfo {
            id: circle_count,
            creator,
            contribution_amount: amount,
            max_members,
            member_count: 0,
            current_recipient_index: 0,
            is_active: true,
            token: primary_token,
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
            requires_collateral,
            collateral_bps,
            member_addresses: Vec::new(&env),
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            dissolution_status: DissolutionStatus::NotInitiated,
            dissolution_deadline: None,
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            recovery_old_address: None,
            recovery_new_address: None,
            recovery_votes_bitmap: 0,
            arbitrator,
            basket: Some(basket),
        };

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_count), &new_circle);
        env.storage()
            .instance()
            .set(&DataKey::CircleCount, &circle_count);

        env.events().publish(
            (Symbol::new(&env, "BASKET_CIRCLE_CREATED"), circle_count),
            (amount, max_members),
        );

        circle_count
    }

    fn deposit_basket(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        // Ensure this is a basket circle
        let basket = match circle.basket.clone() {
            Some(b) => b,
            None => panic!("Not a basket circle; use deposit() for single-asset circles"),
        };

        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;

        // Determine whether contribution is late (for stats tracking)
        let is_late = current_time > circle.deadline_timestamp;

        // Transfer the correct ratio of each basket asset from the user
        for i in 0..basket.len() {
            let asset_weight = basket.get(i).unwrap();

            // Required amount for this asset = base_amount * weight / 10000
            let asset_amount = (base_amount * asset_weight.weight_bps as i128) / 10000;
            if asset_amount <= 0 {
                panic!("Asset amount too small for current basket weight");
            }

            // Add insurance fee proportionally
            let insurance_fee = (asset_amount * circle.insurance_fee_bps as i128) / 10000;
            let total_asset_amount = asset_amount + insurance_fee;

            // Transfer asset from user to contract
            let token_client = token::Client::new(&env, &asset_weight.token);
            token_client.transfer(&user, &env.current_contract_address(), &total_asset_amount);

            // Accumulate insurance balance (tracked in primary token equivalent)
            if insurance_fee > 0 {
                circle.insurance_balance += insurance_fee;
            }

            // Track per-asset contribution for payout distribution
            let contrib_key = DataKey::BasketAssetContrib(
                circle_id,
                user.clone(),
                asset_weight.token.clone(),
            );

        }

        // Update user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env
            .storage()
            .instance()
            .get(&user_stats_key)
            .unwrap_or(UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });

        if is_late {
            user_stats.late_contributions += 1;
        } else {
            user_stats.on_time_contributions += 1;
        }
        user_stats.total_volume_saved += base_amount;
        env.storage().instance().set(&user_stats_key, &user_stats);

        // Mark member as having contributed this round via bitmap
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        env.events().publish(
            (Symbol::new(&env, "BASKET_DEPOSIT"), circle_id, user.clone()),
            (basket.len(), base_amount),
        );
    }

    fn get_basket_config(env: Env, circle_id: u64) -> Vec<AssetWeight> {
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        match circle.basket {
            Some(b) => b,
            None => panic!("Circle does not have a basket configuration"),
        }
    }

    // Emergency Manual Revert for Oracle Failure (#205)
    fn update_oracle_heartbeat(env: Env, oracle: Address) {
        oracle.require_auth();
        let current_time = env.ledger().timestamp();
        let heartbeat = OracleHeartbeat {
            last_heartbeat: current_time,
            oracle_address: oracle,
        };
        env.storage().instance().set(&DataKey::OracleHeartbeat, &heartbeat);
    }

    fn activate_trust_mode(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        let heartbeat: OracleHeartbeat = env.storage().instance().get(&DataKey::OracleHeartbeat)
            .unwrap_or_else(|| panic!("No oracle heartbeat found"));
        
        let current_time = env.ledger().timestamp();
        let hours_since_heartbeat = (current_time - heartbeat.last_heartbeat) / 3600;
        
        if hours_since_heartbeat < 72 {
            panic!("Trust mode can only be activated after 72 hours of oracle silence");
        }

        env.storage().instance().set(&DataKey::TrustMode, &true);
    }

    fn set_emergency_price(env: Env, circle_id: u64, price: i128, setter: Address) {
        setter.require_auth();
        
        let trust_mode: bool = env.storage().instance().get(&DataKey::TrustMode).unwrap_or(false);
        if !trust_mode {
            panic!("Emergency pricing only available in trust mode");
        }

        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let current_time = env.ledger().timestamp();
        let emergency_price = EmergencyPrice {
            circle_id,
            price,
            set_by: setter,
            timestamp: current_time,
        };
        env.storage().instance().set(&DataKey::ManualPrice(circle_id), &emergency_price);
    }

    // Cross-Group Liquidity Sharing Vault (#204)
    fn create_liquidity_vault(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        if !env.storage().instance().has(&DataKey::LiquidityVault) {
            env.storage().instance().set(&DataKey::LiquidityVault, &0i128);
        }
    }

    fn lend_to_circle(env: Env, from_circle_id: u64, to_circle_id: u64, amount: i128, interest_rate: u32, lead: Address) -> u64 {
        lead.require_auth();
        
        let from_circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(from_circle_id))
            .unwrap_or_else(|| panic!("Source circle does not exist"));
        let _to_circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(to_circle_id))
            .unwrap_or_else(|| panic!("Target circle does not exist"));

        let current_time = env.ledger().timestamp();
        let loan_duration = 30 * 24 * 3600; // 30 days
        let due_at = current_time + loan_duration;

        let mut loan_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        loan_count += 1;

        let loan = CircleLoan {
            loan_id: loan_count,
            from_circle_id,
            to_circle_id,
            amount,
            interest_rate,
            created_at: current_time,
            due_at,
            is_repaid: false,
        };

        env.storage().instance().set(&DataKey::CircleLoan(loan_count), &loan);

        let client = token::Client::new(&env, &from_circle.token);
        client.transfer(&env.current_contract_address(), &env.current_contract_address(), &amount);

        loan_count
    }

    fn repay_circle_loan(env: Env, circle_id: u64, loan_id: u64, lead: Address) {
        lead.require_auth();
        
        let mut loan: CircleLoan = env.storage().instance().get(&DataKey::CircleLoan(loan_id))
            .unwrap_or_else(|| panic!("Loan does not exist"));

        if loan.is_repaid {
            panic!("Loan already repaid");
        }

        if loan.to_circle_id != circle_id {
            panic!("This loan does not belong to your circle");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let interest = (loan.amount * loan.interest_rate as i128) / 10000;
        let total_repayment = loan.amount + interest;

        let client = token::Client::new(&env, &circle.token);
        client.transfer(&env.current_contract_address(), &env.current_contract_address(), &total_repayment);

        loan.is_repaid = true;
        env.storage().instance().set(&DataKey::CircleLoan(loan_id), &loan);
    }

    // Variable Interest Rate for Internal Susu Lending (#203)
    fn update_circle_risk_level(env: Env, admin: Address, circle_id: u64, late_payments: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("No admin set"));
        
        if stored_admin != admin {
            panic!("Caller is not the admin");
        }

        let _circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let current_time = env.ledger().timestamp();
        let risk_score = if late_payments == 0 { 0 } else if late_payments <= 2 { 25 } else if late_payments <= 5 { 50 } else if late_payments <= 10 { 75 } else { 100 };

        let risk_level = CircleRiskLevel {
            circle_id,
            risk_score,
            late_payments,
            last_updated: current_time,
        };

        env.storage().instance().set(&DataKey::CircleRiskLevel(circle_id), &risk_level);
    }

    fn get_dynamic_interest_rate(env: Env, circle_id: u64) -> u32 {
        let risk_level: CircleRiskLevel = env.storage().instance().get(&DataKey::CircleRiskLevel(circle_id))
            .unwrap_or_else(|| CircleRiskLevel {
                circle_id,
                risk_score: 0,
                late_payments: 0,
                last_updated: 0,
            });

        let base_rate = 200u32; // 2% base rate
        let max_rate = 1000u32; // 10% max rate
        
        let additional_rate = (risk_level.risk_score * (max_rate - base_rate)) / 100;
        base_rate + additional_rate
    }

    // Group Lead Performance Bond Slashing (#202)
    fn post_lead_bond(env: Env, circle_id: u64, lead: Address, bond_amount: i128) {
        lead.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        if circle.creator != lead {
            panic!("Only circle creator can post bond");
        }

        let current_time = env.ledger().timestamp();
        let bond = GroupLeadBond {
            circle_id,
            lead_address: lead.clone(),
            bond_amount,
            posted_at: current_time,
            is_slashed: false,
        };

        env.storage().instance().set(&DataKey::GroupLeadBond(circle_id), &bond);

        let client = token::Client::new(&env, &circle.token);
        client.transfer(&lead, &env.current_contract_address(), &bond_amount);
    }


        let member_key = DataKey::Member(proposer.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        let bond: GroupLeadBond = env.storage().instance().get(&DataKey::GroupLeadBond(circle_id))
            .unwrap_or_else(|| panic!("No bond found for this circle"));

        if bond.lead_address != target_lead {
            panic!("Target is not the lead of this circle");
        }

        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        proposal_count += 1;

        let current_time = env.ledger().timestamp();
        let voting_deadline = current_time + (7 * 24 * 3600); // 7 days

        let proposal = SlashingProposal {
            proposal_id: proposal_count,
            circle_id,
            target_lead,
            reason,
            proposed_by: proposer,
            created_at: current_time,
            voting_deadline,
            yes_votes: 0,
            no_votes: 0,
            total_members: circle.member_count,
            is_executed: false,
        };

        env.storage().instance().set(&DataKey::SlashingProposal(proposal_count), &proposal);
        proposal_count
    }

    fn vote_on_slashing(env: Env, voter: Address, proposal_id: u64, vote: bool) {
        voter.require_auth();
        
        let mut proposal: SlashingProposal = env.storage().instance().get(&DataKey::SlashingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Slashing proposal does not exist"));

        let current_time = env.ledger().timestamp();
        if current_time > proposal.voting_deadline {
            panic!("Voting period has ended");
        }

        let member_key = DataKey::Member(voter.clone());
        let _member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        let vote_key = DataKey::Vote(proposal_id, voter.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("User has already voted on this proposal");
        }

        let vote_record = VoteCast {
            proposal_id,
            voter: voter.clone(),
            vote,
            voting_power: 1,
        };
        env.storage().instance().set(&vote_key, &vote_record);

        if vote {
            proposal.yes_votes += 1;
        } else {
            proposal.no_votes += 1;
        }

        env.storage().instance().set(&DataKey::SlashingProposal(proposal_id), &proposal);
    }

    fn execute_slashing(env: Env, executor: Address, proposal_id: u64) {
        executor.require_auth();
        
        let mut proposal: SlashingProposal = env.storage().instance().get(&DataKey::SlashingProposal(proposal_id))
            .unwrap_or_else(|| panic!("Slashing proposal does not exist"));

        if proposal.is_executed {
            panic!("Proposal has already been executed");
        }

        let current_time = env.ledger().timestamp();
        if current_time <= proposal.voting_deadline {
            panic!("Voting period has not ended yet");
        }

        let required_votes = (proposal.total_members * 90) / 100; // 90% threshold
        if proposal.yes_votes < required_votes {
            panic!("Insufficient votes for slashing (90% required)");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(proposal.circle_id))
            .unwrap_or_else(|| panic!("Circle does not exist"));

        let mut bond: GroupLeadBond = env.storage().instance().get(&DataKey::GroupLeadBond(proposal.circle_id))
            .unwrap_or_else(|| panic!("No bond found for this circle"));

        if bond.is_slashed {
            panic!("Bond already slashed");
        }

        let _client = token::Client::new(&env, &circle.token);
        
        let _slash_per_member = bond.bond_amount / proposal.total_members as i128;
        
        env.events().publish((Symbol::new(&env, "bond_slashed"),), (proposal.circle_id, bond.bond_amount, proposal.total_members));
        
        bond.is_slashed = true;
        env.storage().instance().set(&DataKey::GroupLeadBond(proposal.circle_id), &bond);

        proposal.is_executed = true;
        env.storage().instance().set(&DataKey::SlashingProposal(proposal_id), &proposal);
    }

    // --- SBT HELPER FUNCTIONS ---

    fn calculate_user_reputation_score(env: Env, user: Address) -> u32 {
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| Member {
                address: user,
                index: 0,
                contribution_count: 0,
                last_contribution_time: 0,
                is_active: false,
            });

        if !member.is_active {
            return 0;
        }

        // Base score from contribution count (max 50 points)
        let contribution_score = (member.contribution_count.min(50) as u32) * 1;

        // Bonus for timely payments (max 30 points)
        let timely_bonus = Self::calculate_timely_payment_bonus(env.clone(), user.clone());

        // Bonus for active participation (max 20 points)
        let participation_bonus = Self::calculate_participation_bonus(env.clone(), user.clone());

        let total_score = contribution_score + timely_bonus + participation_bonus;
        total_score.min(100) // Cap at 100
    }

    fn calculate_timely_payment_bonus(env: Env, user: Address) -> u32 {
        // Check if user has any late fees in their history
        // This is a simplified implementation - in practice you'd track payment history
        let member_key = DataKey::Member(user);
        if let Ok(member) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            if member.contribution_count > 0 {
                // Assume most payments are timely for this example
                return 25;
            }
        }
        0
    }

    fn calculate_participation_bonus(env: Env, user: Address) -> u32 {
        // Bonus for being active in multiple circles or long-term participation
        let member_key = DataKey::Member(user);
        if let Ok(member) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            if member.contribution_count >= 10 {
                return 20;
            } else if member.contribution_count >= 5 {
                return 10;
            } else if member.contribution_count >= 2 {
                return 5;
            }
        }
        0
    }

    fn get_user_total_cycles_completed(env: Env, user: Address) -> u32 {
        let member_key = DataKey::Member(user);
        if let Ok(member) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            return member.contribution_count;
        }
        0
    }

    fn check_and_issue_sbt_credential(env: Env, user: Address) {
        // Check if SBT contract is set
        let sbt_contract: Option<Address> = env.storage().instance().get(&DataKey::SbtContract);
        if sbt_contract.is_none() {
            return; // SBT system not initialized
        }

        // Check if user already has an SBT
        let user_sbt_key = DataKey::UserSbt(user.clone());
        if env.storage().instance().has(&user_sbt_key) {
            return; // User already has SBT
        }

        // Calculate user's reputation score and cycles
        let reputation_score = Self::calculate_user_reputation_score(env.clone(), user.clone());
        let total_cycles = Self::get_user_total_cycles_completed(env.clone(), user.clone());

        // Check for milestone 1 (5 cycles, 80 reputation)
        if total_cycles >= 5 && reputation_score >= 80 {
            let milestone_id = 1u32;
            if let Ok(milestone) = env.storage().instance().get::<DataKey, ReputationMilestone>(&DataKey::ReputationMilestone(milestone_id)) {
                if milestone.is_active {
                    // Auto-issue SBT credential
                    let token_id = (milestone_id as u128) << 96 | (env.ledger().timestamp() as u128);
                    
                    let sbt = SoulboundToken {
                        token_id,
                        owner: user.clone(),
                        milestone_id,
                        issued_at: env.ledger().timestamp(),
                        status: SbtStatus::Active,
                        reputation_score,
                        cycles_completed: total_cycles,
                        metadata: milestone.name,
                    };

                    // Store SBT
                    env.storage().instance().set(&user_sbt_key, &sbt);

                    // Mint SBT on external contract
                    let sbt_client = SbtTokenClient::new(&env, &sbt_contract.unwrap());
                    sbt_client.mint_sbt(&user, &token_id, &milestone.name);

                    // Emit event
                    env.events().publish(
                        (Symbol::short("sbt_auto_issued"), user, milestone_id),
                        token_id,
                    );
                }
            }
        }
    }

    fn handle_sbt_clawback_impact(env: Env, circle_id: u64) {
        // Get all members of the circle and check if any have SBTs
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        let sbt_contract: Option<Address> = env.storage().instance().get(&DataKey::SbtContract);
        if sbt_contract.is_none() {
            return; // SBT system not initialized
        }

        // Check each member for SBT and potentially mark as dishonored
        for i in 0..circle.member_count {
            let member_by_index_key = DataKey::MemberByIndex(circle_id, i as u32);
            if let Ok(member_address) = env.storage().instance().get::<DataKey, Address>(&member_by_index_key) {
                let user_sbt_key = DataKey::UserSbt(member_address.clone());
                if let Ok(mut sbt) = env.storage().instance().get::<DataKey, SoulboundToken>(&user_sbt_key) {
                    // Mark SBT as dishonored due to clawback involvement
                    if sbt.status == SbtStatus::Active {
                        sbt.status = SbtStatus::Dishonored;
                        env.storage().instance().set(&user_sbt_key, &sbt);

                        // Update metadata on external contract
                        let sbt_client = SbtTokenClient::new(&env, &sbt_contract.unwrap());
                        sbt_client.update_metadata(&sbt.token_id, &Symbol::short("Dishonored"));

                        // Emit event
                        env.events().publish(
                            (Symbol::short("sbt_dishonored"), member_address, circle_id),
                            sbt.token_id,
                        );
                    }
                }
            }
        }
    }

    fn generate_financial_statement(env: Env, user: Address) -> FinancialStatement {
        let stats_key = DataKey::UserStats(user.clone());
        let stats: UserStats = env.storage().instance().get(&stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
            total_won: 0,
        });

        let reputation = Self::get_reputation(env.clone(), user.clone());

        FinancialStatement {
            user: user.clone(),
            total_contributed: stats.total_volume_saved,
            total_won: stats.total_won,
            average_reputation: reputation.susu_score,
            generated_at: env.ledger().timestamp(),
        }
    }

    fn detect_and_handle_clawback(env: Env, circle_id: u64) {
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let client = token::Client::new(&env, &circle.token);
        let actual_balance = client.balance(&env.current_contract_address());
        
        if actual_balance < circle.expected_balance {
            let deficit = circle.expected_balance - actual_balance;
            circle.is_paused = true;
            env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
            env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &deficit);
            
            env.events().publish((Symbol::new(&env, "CLAWBACK_DETECTED"), circle_id), deficit);
        }
    }

    fn pause_round(env: Env, admin: Address, circle_id: u64, reason: String) {
        admin.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        if admin != circle.creator { panic!("Unauthorized"); }
        circle.is_paused = true;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn propose_recovery_plan(env: Env, user: Address, circle_id: u64, recovery_type: RecoveryType) {
        user.require_auth();
        let deficit: i128 = env.storage().instance().get(&DataKey::ClawbackDeficit(circle_id)).unwrap_or(0);
        if deficit == 0 { panic!("No deficit to recover"); }
        
        let plan = RecoveryPlan {
            circle_id,
            total_deficit: deficit,
            recovery_type,
            proposed_by: user,
            votes_for: 0,
            votes_against: 0,
            is_active: true,
        };
        env.storage().instance().set(&DataKey::RecoveryPlanInfo(circle_id), &plan);
    }

    fn vote_recovery_plan(env: Env, user: Address, circle_id: u64, vote: bool) {
        user.require_auth();
        let mut plan: RecoveryPlan = env.storage().instance().get(&DataKey::RecoveryPlanInfo(circle_id)).unwrap();
        if vote { plan.votes_for += 1; } else { plan.votes_against += 1; }
        env.storage().instance().set(&DataKey::RecoveryPlanInfo(circle_id), &plan);
    }

    fn contribute_to_recovery(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let client = token::Client::new(&env, &circle.token);
        client.transfer(&user, &env.current_contract_address(), &amount);
        
        let mut contributions: i128 = env.storage().instance().get(&DataKey::RecoveryContributions(circle_id, user.clone())).unwrap_or(0);
        contributions += amount;
        env.storage().instance().set(&DataKey::RecoveryContributions(circle_id, user), &contributions);
    }

    fn execute_recovery_plan(env: Env, admin: Address, circle_id: u64) {
        admin.require_auth();
        let mut plan: RecoveryPlan = env.storage().instance().get(&DataKey::RecoveryPlanInfo(circle_id)).unwrap();
        if plan.votes_for <= plan.votes_against { panic!("Plan not approved"); }
        plan.is_active = false;
        env.storage().instance().set(&DataKey::RecoveryPlanInfo(circle_id), &plan);
        
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        circle.expected_balance -= plan.total_deficit;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        env.storage().instance().set(&DataKey::ClawbackDeficit(circle_id), &0i128);
    }

    fn resume_round(env: Env, admin: Address, circle_id: u64) {
        admin.require_auth();
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        if admin != circle.creator { panic!("Unauthorized"); }
        circle.is_paused = false;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn raise_dispute(env: Env, user: Address, circle_id: u64, target: Address, amount: i128) -> u64 {
        user.require_auth();
        let mut dispute_count: u64 = env.storage().instance().get(&DataKey::DisputeCount).unwrap_or(0);
        dispute_count += 1;
        
        let mut arbitrators = Vec::new(&env);
        let circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        if circle_count > 1 {
            let mut seed = env.ledger().timestamp().wrapping_add(env.ledger().sequence() as u64);
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            
            let start_circle = (seed % circle_count) + 1;
            
            for i in 0..circle_count {
                let current_circle_id = ((start_circle + i - 1) % circle_count) + 1;
                
                if current_circle_id == circle_id {
                    continue; // Must be outside the group
                }
                
                if let Some(rand_circle) = env.storage().instance().get::<DataKey, CircleInfo>(&DataKey::Circle(current_circle_id)) {
                    for j in 0..rand_circle.member_count {
                        if let Some(member_addr) = rand_circle.member_addresses.get(j) {
                            if !arbitrators.contains(&member_addr) {
                                let rep = Self::get_reputation(env.clone(), member_addr.clone());
                                if rep.susu_score >= 9000 {
                                    arbitrators.push_back(member_addr);
                                    if arbitrators.len() == 3 {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
                
                if arbitrators.len() == 3 {
                    break;
                }
            }
        }
        
        let dispute = Dispute {
            dispute_id: dispute_count,
            circle_id,
            raised_by: user.clone(),
            target,
            amount,
            arbitrators,
            votes_release: 0,
            votes_refund: 0,
            is_resolved: false,
        };
        
        env.storage().instance().set(&DataKey::Dispute(dispute_count), &dispute);
        env.storage().instance().set(&DataKey::DisputeCount, &dispute_count);
        
        dispute_count
    }

    fn vote_on_dispute(env: Env, arbitrator: Address, dispute_id: u64, release_funds: bool) {
        arbitrator.require_auth();
        
        let mut dispute: Dispute = env.storage().instance().get(&DataKey::Dispute(dispute_id)).unwrap();
        if dispute.is_resolved { panic!("Dispute already resolved"); }
        
        if !dispute.arbitrators.contains(&arbitrator) {
            panic!("Not an assigned arbitrator for this dispute");
        }
        
        let rep = Self::get_reputation(env.clone(), arbitrator.clone());
        if rep.susu_score < 9000 {
            panic!("Arbitrator must have near perfect reputation");
        }
        
        let vote_key = DataKey::DisputeArbitratorVote(dispute_id, arbitrator.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }
        env.storage().instance().set(&vote_key, &true);
        
        if release_funds { dispute.votes_release += 1; } else { dispute.votes_refund += 1; }
        
        if dispute.votes_release >= 2 || dispute.votes_refund >= 2 {
            dispute.is_resolved = true;
            let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(dispute.circle_id)).unwrap();
            let client = token::Client::new(&env, &circle.token);
            if dispute.votes_release >= 2 {
                client.transfer(&env.current_contract_address(), &dispute.target, &dispute.amount);
            } else {
                client.transfer(&env.current_contract_address(), &dispute.raised_by, &dispute.amount);
            }
            circle.expected_balance -= dispute.amount;
            env.storage().instance().set(&DataKey::Circle(dispute.circle_id), &circle);
        }
        
        env.storage().instance().set(&DataKey::Dispute(dispute_id), &dispute);
    }
}

fn execute_yield_delegation_internal(env: &Env, circle_id: u64, delegation: &mut YieldDelegation) {
    let current_time = env.ledger().timestamp();
    
    // Transfer funds to yield pool
    let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
        .expect("Circle not found");
    let token_client = token::Client::new(env, &circle.token);
    
    // In real implementation, this would call the actual yield pool contract
    // token_client.transfer(&env.current_contract_address(), &delegation.pool_address, &delegation.delegation_amount);
    
    delegation.status = YieldDelegationStatus::Active;
    delegation.start_time = Some(current_time);
    delegation.last_compound_time = current_time;
}
