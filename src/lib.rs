use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, panic, Map, Vec, i128, u64, u32};

mod liquidity_buffer;

pub use liquidity_buffer::*;
mod sbt_minter;

pub use lending_market::*;

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
    // Multi-winner batch payout configuration
    pub winners_per_round: u16, // Number of winners per round (1, 2, 5, or 10)
    pub batch_payout_enabled: bool, // Whether batch payout is enabled
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

// Multi-winner batch payout structures
#[contracttype]
#[derive(Clone)]
pub struct BatchPayoutRecord {
    pub circle_id: u64,
    pub round_number: u32,
    pub total_winners: u16,
    pub total_pot: i128,
    pub organizer_fee: i128,
    pub net_payout_per_winner: i128,
    pub dust_amount: i128, // Rounding dust that couldn't be evenly distributed
    pub payout_timestamp: u64,
    pub winners: Vec<Address>, // List of winner addresses for this round
}

#[contracttype]
#[derive(Clone)]
pub struct IndividualPayoutClaim {
    pub recipient: Address,
    pub circle_id: u64,
    pub round_number: u32,
    pub amount_claimed: i128,
    pub claim_timestamp: u64,
    pub batch_payout_id: u64, // Reference to the batch payout record
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
    // Multi-winner batch payout tracking
    BatchPayoutRecord(u64, u32), // (circle_id, round_number) -> batch payout record
    IndividualPayoutClaim(Address, u64, u32), // (recipient, circle_id, round_number) -> individual claim
    BatchPayoutCounter, // Counter for generating batch payout IDs
    DustReserve(u64), // Per-circle dust reserve for rounding errors
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

    // NEW: Multi-winner batch payout functions
    fn configure_batch_payout(env: Env, creator: Address, circle_id: u64, winners_per_round: u16);
    fn distribute_batch_payout(env: Env, caller: Address, circle_id: u64);
    fn get_batch_payout_record(env: Env, circle_id: u64, round_number: u32) -> Option<BatchPayoutRecord>;
    fn get_individual_payout_claim(env: Env, recipient: Address, circle_id: u64, round_number: u32) -> Option<IndividualPayoutClaim>;

    // Helper functions
    fn get_circle(env: Env, circle_id: u64) -> CircleInfo;
    fn get_member(env: Env, member: Address) -> Member;
    fn get_current_recipient(env: Env, circle_id: u64) -> Option<Address>;

    // NEW: DEX Auto-Swap functions for fee stabilization
    fn configure_dex_swap(env: Env, admin: Address, config: DexSwapConfig);
    fn trigger_dex_swap(env: Env, admin: Address, circle_id: u64);
    fn get_dex_swap_config(env: Env, circle_id: u64) -> Option<DexSwapConfig>;
    fn get_dex_swap_record(env: Env, swap_id: u64) -> Option<DexSwapRecord>;
    fn get_gas_reserve(env: Env, circle_id: u64) -> Option<GasReserve>;
    fn emergency_pause_dex_swaps(env: Env, admin: Address);
    fn emergency_refill_gas_reserve(env: Env, admin: Address, amount: i128);

    // Stellar Anchor Direct Deposit API (SEP-24/SEP-31)
    fn register_anchor(env: Env, admin: Address, anchor_info: AnchorInfo);
    fn deposit_for_user(
        env: Env,
        anchor: Address,
        beneficiary_user: Address,
        circle_id: u64,
        amount: i128,
        deposit_memo: String,
        fiat_reference: String,
        sep_type: String,
    );
    fn verify_anchor_deposit(env: Env, deposit_id: u64) -> bool;
    fn get_anchor_info(env: Env, anchor_address: Address) -> AnchorInfo;
    fn get_deposit_record(env: Env, deposit_id: u64) -> AnchorDeposit;
}

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
    InvalidBasketWeights = 15,
    BasketNotEnabled = 16,
    InvalidBasketRatio = 17,
    AnchorNotFound = 18,
    AnchorNotAuthorized = 19,
    InvalidDepositMemo = 20,
    DepositAlreadyProcessed = 21,
    ComplianceCheckFailed = 22,
    DexSwapFailed = 23, // NEW
    DexSwapCooldown = 24, // NEW
    DexSwapThresholdNotMet = 25, // NEW
    DexSwapSlippageExceeded = 26, // NEW
    DexSwapEmergencyCooldown = 27, // NEW
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
    CycleBadge(Address, u64),
    UserStats(Address),
    ProtocolFeeBps,
    ProtocolTreasury,
    CollateralVault(Address, u64),
    ReputationData(Address),
    SocialCapital(Address, u64),
    AuditCount,
    AuditEntry(u64),
    AuditAll,
    AuditByActor(Address),
    AuditByResource(u64),
    LeniencyStats(u64),
    Proposal(u64),
    DefaultedMembers(u64),
    RolloverBonus(u64),
    RolloverVote(u64, Address),
    LeniencyRequest(u64),
    VotingPower(Address, u64),
    DissolutionProposal(u64),
    RefundClaim(u64),
    YieldDelegation(u64),
    YieldVote(u64, Address),
    YieldPoolRegistry,
    GroupTreasury(u64),
    PathPayment(u64),
    PathPaymentVote(u64, Address),
    DexRegistry(Address),
    SupportedTokens(Address),
    // Multi-asset basket storage
    BasketConfig(u64),
    BasketAssetContrib(u64, Address, Address),
    GroupInsuranceFund(u64), // Per-circle insurance fund balance
    InsurancePremium(u64, Address), // Track premiums paid by each member per circle
    PriceOracle(Address), // Price data for each asset
    HardAssetBasket, // Reference hard asset basket
    AssetSwapProposal(u64), // Per-circle asset swap proposals
    AssetSwapVote(u64, Address), // Votes on asset swap proposals
    LateFeeDistribution(u64, u32), // Late fee distribution per circle per round
    LastDepositLedger(Address),
    LastWithdrawalLedger(Address),
    RecursiveOptIn(Address, u64),
    GoldTierCircle,
    PausedPayout(Address, u64), // (user, circle_id) -> is_paused
    LeaseFlowContract,
    GrantStreamContract,
    MilestoneReached(u64),
    PaymentTiming(u64, u32, Address), // Payment timing per circle, round, and member
    PaymentOrderCounter(u64, u32), // Counter to track payment order in each round
    LiquidityBufferConfig,           // Global liquidity buffer configuration
    LiquidityBufferReserve,          // Current reserve balance
    LiquidityAdvance(u64),           // Individual advance records
    LiquidityAdvanceCounter,         // Counter for generating advance IDs
    MemberAdvanceHistory(Address, u64), // Member's advance history
    LiquidityBufferStats,            // Buffer utilization statistics
    PlatformFeeAllocation,           // Platform fee allocation to buffer
    // Stellar Anchor Direct Deposit API (SEP-24/SEP-31)
    AnchorRegistry, // Registry of authorized anchors
    AnchorDeposit(u64), // Track anchor deposits per circle
    DepositMemo(u64), // Track deposit memos for compliance
    // Inter-Susu Lending Market Liquidity Hook
    LendingMarketProposal(u64),       // Lending market proposals
    LendingMarketVote(u64, Address),       // Votes on lending market proposals
    LendingPoolInfo(u64),             // Lending pool information
    LendingPoolParticipant(u64, Address), // Pool participants
    LendingMarketConfig,               // Global lending market configuration
    LendingPosition(u64, Address),        // Individual lending positions
    LendingOffer(u64),                 // Active lending offers
    LiquidityProvider(u64, Address),     // Liquidity provider information
    YieldFarm(u64),                   // Yield farming positions
    EmergencyLoan(u64),                 // Emergency loan requests
    RepaymentSchedule(u64),            // Loan repayment schedules
    LendingMarketStats,               // Lending market statistics
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
    pub consecutive_missed_rounds: u32,
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

/// Group Insurance Fund - Tracks mutual insurance for default protection
#[contracttype]
#[derive(Clone)]
pub struct GroupInsuranceFund {
    pub circle_id: u64,
    pub total_fund_balance: i128,      // Total balance in the insurance fund
    pub total_premiums_collected: i128, // Total premiums collected from all members
    pub total_claims_paid: i128,        // Total claims paid out for defaults
    pub premium_rate_bps: u32,          // Premium rate in basis points (e.g., 50 = 0.5%)
    pub is_active: bool,                // Whether the fund is active
    pub cycle_start_time: u64,          // When the current cycle started
    pub last_claim_time: Option<u64>,   // Timestamp of last claim
}

/// Insurance Premium Record - Track individual member's premium contributions
#[contracttype]
#[derive(Clone)]
pub struct InsurancePremiumRecord {
    pub member: Address,
    pub circle_id: u64,
    pub total_premium_paid: i128,       // Total premium paid by this member
    pub premium_payments: Vec<(u64, i128)>, // List of (round, amount) tuples
    pub claims_made: i128,              // Total claims made by this member
    pub net_contribution: i128,         // Premiums paid minus claims received
}

/// Price Oracle Data - Tracks asset prices for economic circuit breaker
#[contracttype]
#[derive(Clone)]
pub struct PriceOracleData {
    pub asset_address: Address,
    pub current_price: i128,           // Current price in base currency (e.g., USD cents)
    pub last_updated: u64,             // Last update timestamp
    is_stable_asset: bool,             // Whether this is a stable asset
}

/// Hard Asset Basket - Reference basket of hard assets for stability comparison
#[contracttype]
#[derive(Clone)]
pub struct HardAssetBasket {
    pub gold_weight_bps: u32,          // Gold allocation in basis points
    pub btc_weight_bps: u32,           // BTC allocation in basis points  
    pub silver_weight_bps: u32,        // Silver allocation in basis points
    pub total_weight_bps: u32,         // Should equal 10000 (100%)
}

/// Asset Swap Proposal - For voting on swapping treasury assets
#[contracttype]
#[derive(Clone)]
pub struct AssetSwapProposal {
    pub circle_id: u64,
    pub proposer: Address,
    pub current_asset: Address,
    pub target_asset: Address,
    pub swap_percentage_bps: u32,      // Percentage of treasury to swap
    pub price_drop_percentage_bps: u32, // Detected price drop that triggered proposal
    pub created_timestamp: u64,
    pub voting_deadline: u64,
    pub status: ProposalStatus,
    pub for_votes: u32,
    pub against_votes: u32,
    pub total_votes_cast: u32,
    pub executed_timestamp: Option<u64>,
}

/// Late Fee Distribution Record - Tracks priority distribution of late fees
#[contracttype]
#[derive(Clone)]
pub struct LateFeeDistribution {
    pub circle_id: u64,
    pub round_number: u32,
    pub pot_winner: Address,
    pub pot_winner_compensation: i128,      // First priority: compensate pot winner
    pub on_time_payers_bonus: Vec<(Address, i128)>, // Bonus for on-time payers (pro-rated by payment time)
    pub total_late_fees_collected: i128,
    pub distribution_timestamp: u64,
    pub late_payers: Vec<(Address, i128)>,  // List of late payers and their fines
}

/// Payment Timing Record - Track when each member paid in a round
#[contracttype]
#[derive(Clone)]
pub struct PaymentTimingRecord {
    pub member: Address,
    pub circle_id: u64,
    pub round_number: u32,
    pub payment_timestamp: u64,
    pub is_on_time: bool,
    pub payment_order: u32, // Order in which this payment was made (1 = first, 2 = second, etc.)
}

/// Individual Payout Claim - Track individual payout claims
#[contracttype]
#[derive(Clone)]
pub struct IndividualPayoutClaim {
    pub recipient: Address,
    pub circle_id: u64,
    pub round_number: u32,
    pub amount_claimed: i128,
    pub claim_timestamp: u64,
    pub batch_payout_id: u64, // Reference to batch payout record
}

// DEX Auto-Swap structures for fee stabilization
#[contracttype]
#[derive(Clone)]
pub struct DexSwapConfig {
    pub enabled: bool,                    // Whether auto-swap is enabled
    pub swap_threshold_xlm: i128,        // Minimum XLM balance to trigger swap
    pub swap_percentage_bps: u32,         // Percentage of USDC to swap (e.g., 5000 = 50%)
    pub dex_contract: Address,             // Stellar DEX contract address
    pub xlm_token: Address,               // Native XLM token address
    pub slippage_tolerance_bps: u32,      // Maximum slippage tolerance (e.g., 100 = 1%)
    pub minimum_swap_amount: i128,          // Minimum USDC amount to swap
    pub emergency_pause: bool,              // Emergency pause flag
    pub last_swap_timestamp: u64,           // Last successful swap timestamp
    pub total_swapped_xlm: i128,          // Total XLM acquired via swaps
}

#[contracttype]
#[derive(Clone)]
pub struct DexSwapRecord {
    pub swap_id: u64,
    pub circle_id: u64,
    pub trigger_timestamp: u64,
    pub usdc_amount: i128,               // USDC amount swapped
    pub xlm_received: i128,             // XLM amount received
    pub exchange_rate: i128,               // Exchange rate (XLM per USDC * 1M)
    pub slippage_bps: u32,               // Actual slippage experienced
    pub gas_used: i128,                    // Gas cost for swap transaction
    pub execution_timestamp: u64,
    pub success: bool,                      // Whether swap succeeded
}

#[contracttype]
#[derive(Clone)]
pub struct GasReserve {
    pub xlm_balance: i128,                // Current XLM reserve for gas
    pub reserved_for_ttl: i128,            // XLM reserved for next TTL bumps
    pub auto_swap_enabled: bool,            // Whether auto-swap is active
    pub last_refill_timestamp: u64,         // Last manual refill timestamp
    pub consumption_rate: i128,              // XLM consumption per day
}

// --- POT LIQUIDITY BUFFER DATA STRUCTURES ---

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LiquidityAdvanceStatus {
    Pending,        // Advance requested, waiting for deposit
    Active,         // Advance provided, waiting for repayment
    Completed,      // Advance fully repaid
    Defaulted,      // Advance not repaid within grace period
    Cancelled,      // Advance cancelled by member
}

#[contracttype]
#[derive(Clone)]
pub struct LiquidityBufferConfig {
    pub is_enabled: bool,
    pub advance_period: u64,              // 48 hours advance window
    pub min_reputation: u32,               // 100% reputation required
    pub max_advance_bps: u32,             // 100% of contribution can be advanced
    pub platform_fee_allocation: u32,     // 20% of platform fees allocated to buffer
    pub min_reserve: i128,                // Minimum reserve balance
    pub max_reserve: i128,                // Maximum reserve balance
    pub advance_fee_bps: u32,             // 0.5% fee for advance usage
    pub grace_period: u64,                // 24 hours grace period for repayment
    pub max_advances_per_round: u32,      // Maximum advances per member per round
}

#[contracttype]
#[derive(Clone)]
pub struct LiquidityAdvance {
    pub advance_id: u64,
    pub member: Address,
    pub circle_id: u64,
    pub round_number: u32,
    pub contribution_amount: i128,         // Expected contribution amount
    pub advance_amount: i128,             // Amount advanced to member
    pub advance_fee: i128,                // Fee charged for advance
    pub repayment_amount: i128,           // Total amount to be repaid
    pub status: LiquidityAdvanceStatus,
    pub requested_timestamp: u64,         // When advance was requested
    pub provided_timestamp: Option<u64>,  // When advance was provided
    pub repayment_deadline: u64,          // When repayment is due
    pub repaid_timestamp: Option<u64>,    // When repayment was made
    pub reason: String,                   // Reason for advance request
}

#[contracttype]
#[derive(Clone)]
pub struct MemberAdvanceHistory {
    pub member: Address,
    pub total_advances_taken: u32,
    pub total_advance_amount: i128,
    pub total_fees_paid: i128,
    pub current_round_advances: u32,
    pub last_advance_timestamp: Option<u64>,
    pub repayment_history: Vec<u64>,      // List of advance IDs
    pub default_count: u32,               // Number of defaulted advances
    pub reputation_score: u32,            // Current reputation score
}

#[contracttype]
#[derive(Clone)]
pub struct LiquidityBufferStats {
    pub total_reserve_balance: i128,
    pub total_platform_fees_allocated: i128,
    pub total_advances_provided: u64,
    pub total_advances_completed: u64,
    pub total_advances_defaulted: u64,
    pub total_advance_amount: i128,
    pub total_fees_collected: i128,
    pub active_advances_count: u64,
    pub average_advance_size: i128,
    pub buffer_utilization_rate: u32,     // Current utilization as percentage
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct PlatformFeeAllocation {
    pub total_fees_collected: i128,
    pub buffer_allocation_amount: i128,
    pub treasury_allocation_amount: i128,
    pub last_allocation_timestamp: u64,
    pub allocation_frequency: u64,         // How often fees are allocated
}

/// Stellar Anchor Information - SEP-24/SEP-31 compliant anchor registry
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorInfo {
    pub anchor_address: Address,
    pub anchor_name: String,
    pub sep_version: String, // "SEP-24" or "SEP-31"
    pub authorization_level: u32, // 1=Basic, 2=Enhanced, 3=Full
    pub compliance_level: u32, // 1=Basic KYC, 2=Enhanced KYC, 3=Full KYC+AML
    pub is_active: bool,
    pub registration_timestamp: u64,
    pub last_activity: u64,
    pub supported_countries: Vec<String>, // ISO country codes
    pub max_deposit_amount: i128,
    pub daily_deposit_limit: i128,
}

/// Anchor Deposit Record - Track deposits made by anchors on behalf of users
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct AnchorDeposit {
    pub deposit_id: u64,
    pub anchor_address: Address,
    pub beneficiary_user: Address,
    pub circle_id: u64,
    pub amount: i128,
    pub deposit_memo: String, // Unique identifier for compliance
    pub fiat_reference: String, // Reference to fiat transaction
    pub timestamp: u64,
    pub compliance_verified: bool,
    pub processed: bool,
    pub sep_type: String, // "SEP-24" or "SEP-31"
}

/// Deposit Memo Structure - Standardized format for compliance
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct DepositMemo {
    pub memo_type: String, // "text", "hash", or "return"
    pub memo_value: String,
    pub anchor_id: String,
    pub transaction_hash: Option<String>, // For blockchain reference
    pub compliance_data: String, // Encrypted compliance information
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

/// Master Credential NFT Badge - Enhanced metadata for 12-month cycle completion
/// This represents a "Stellar-Native Financial Identity" badge of honor
#[contracttype]
#[derive(Clone)]
pub struct MasterCredentialMetadata {
    pub volume_tier: u32,              // 1=Bronze, 2=Silver, 3=Gold, 4=Platinum
    pub perfect_attendance: bool,       // true if zero late contributions
    pub group_lead_status: bool,        // true if member is the circle creator
    pub total_cycles_completed: u32,    // Total number of full cycles completed
    pub total_volume_saved: i128,       // Lifetime volume saved across all circles
    pub reliability_score: u32,         // 0-10000 bps (0-100%)
    pub social_capital_score: u32,      // 0-10000 bps (0-100%)
    pub badges_earned: Vec<Symbol>,     // List of achievement badges
    pub ecosystem_participation: u32,   // Number of different JerryIdoko projects participated in
    pub mint_timestamp: u64,            // Timestamp when badge was minted
    pub circle_id: u64,                 // The circle that triggered this badge
    pub version: u32,                   // Metadata version for future upgrades
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
    fn mint_badge(env: Env, to: Address, token_id: u128, metadata: NftBadgeMetadata);
    fn mint_master_credential(env: Env, to: Address, token_id: u128, metadata: MasterCredentialMetadata);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: i128);
    fn withdraw(env: Env, token: Address, to: Address, amount: i128);
}

pub trait SoroSusuTrait {
    // Initialize contract
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    fn set_protocol_fee(env: Env, admin: Address, fee_basis_points: u32, treasury: Address);

    // --- POT LIQUIDITY BUFFER FOR BANK HOLIDAYS ---
    fn init_liquidity_buffer(env: Env, admin: Address);
    fn signal_advance_request(
        env: Env,
        member: Address,
        circle_id: u64,
        contribution_amount: i128,
        reason: String,
    ) -> u64;
    fn provide_advance(env: Env, advance_id: u64);
    fn cancel_advance_request(env: Env, advance_id: u64);
    fn process_advance_refill(env: Env, member: Address, circle_id: u64, deposit_amount: i128);
    fn get_liquidity_advance(env: Env, advance_id: u64) -> LiquidityAdvance;
    fn get_member_advance_history(env: Env, member: Address) -> MemberAdvanceHistory;
    fn get_liquidity_buffer_stats(env: Env) -> LiquidityBufferStats;
    fn allocate_platform_fees_to_buffer(env: Env, fee_amount: i128);
    fn check_advance_eligibility(env: Env, member: Address, circle_id: u64) -> bool;

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
        organizer_fee_bps: u32, // New parameter for commission
    ) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64, guarantor: Option<Address>);

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
    fn get_circle(env: Env, circle_id: u64) -> CircleInfo;
    fn get_member(env: Env, member: Address) -> Member;
    fn get_current_recipient(env: Env, circle_id: u64) -> Option<Address>;

    // --- SBT CREDENTIAL SYSTEM FUNCTIONS ---
    fn init_sbt_minter(env: Env, admin: Address);
    fn set_sbt_minter_admin(env: Env, admin: Address, new_admin: Address);
    fn issue_credential(
        env: Env,
        user: Address,
        milestone_id: u64,
        metadata_uri: String,
    ) -> u128;
    fn update_credential_status(
        env: Env,
        token_id: u128,
        new_status: SbtStatus,
    );
    fn revoke_credential(env: Env, token_id: u128, reason: String);
    fn get_credential(env: Env, token_id: u128) -> SoroSusuCredential;
    fn get_user_credential(env: Env, user: Address) -> Option<SoroSusuCredential>;
    fn get_reputation_milestone(env: Env, milestone_id: u64) -> ReputationMilestone;
    fn create_reputation_milestone(
        env: Env,
        user: Address,
        cycles_required: u32,
        description: String,
        reward_tier: ReputationTier,
    ) -> u64;
    fn update_user_reputation(env: Env, user: Address);
    fn get_user_reputation_score(env: Env, user: Address) -> (u32, u32, u32);

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

    // Group Insurance Fund Management
    fn get_insurance_fund(env: Env, circle_id: u64) -> GroupInsuranceFund;
    fn get_premium_record(env: Env, member: Address, circle_id: u64) -> InsurancePremiumRecord;
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);
    fn distribute_remaining_insurance_fund(env: Env, circle_id: u64);

    // Price Oracle and Asset Swap (Economic Circuit Breaker)
    fn update_price_oracle(env: Env, oracle_provider: Address, asset: Address, price: i128);
    fn get_asset_price(env: Env, asset: Address) -> PriceOracleData;
    fn propose_asset_swap(env: Env, user: Address, circle_id: u64, target_asset: Address, swap_percentage_bps: u32);
    fn vote_asset_swap(env: Env, user: Address, circle_id: u64, vote_choice: QuadraticVoteChoice);
    fn execute_asset_swap(env: Env, circle_id: u64);
    fn check_price_drop_and_trigger_swap(env: Env, circle_id: u64) -> bool;
    fn set_hard_asset_basket(env: Env, admin: Address, gold_weight_bps: u32, btc_weight_bps: u32, silver_weight_bps: u32);
    fn get_hard_asset_basket(env: Env) -> HardAssetBasket;

    // Late Fee Priority Distribution
    fn get_late_fee_distribution(env: Env, circle_id: u64, round_number: u32) -> LateFeeDistribution;
    fn get_payment_timing_record(env: Env, circle_id: u64, round_number: u32, member: Address) -> PaymentTimingRecord;
    fn distribute_late_fees_with_priority(env: Env, circle_id: u64, round_number: u32);
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

    // Recursive Susu Cycles (Auto-Compounding)
    fn toggle_recursive_opt_in(env: Env, user: Address, circle_id: u64, enabled: bool);
    /// Set up a "Gold Tier" circle for recursive transitions
    fn recursive_init(env: Env, admin: Address, amount: i128, token: Address, circle_id: u64);

    // Cross-Contract Bridge for LeaseFlow
    fn is_cycle_healthy(env: Env, user: Address, circle_id: u64) -> bool;
    fn handle_leaseflow_default(env: Env, leaseflow_contract: Address, user: Address, circle_id: u64);
    fn set_leaseflow_contract(env: Env, admin: Address, leaseflow: Address);

    // Grant-Stream Matching Logic
    fn handle_grant_stream_match(env: Env, grant_stream_contract: Address, circle_id: u64, amount: i128);
    fn set_grant_stream_contract(env: Env, admin: Address, grant_stream: Address);
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
            circle.grace_period_end = new_deadline;

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
        // Initialize the circle counter to 0 if it doesn't exist
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }

        // Set the admin
        env.storage().instance().set(&DataKey::Admin, &admin);
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
        contribution_amount: u64,
        max_members: u16,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
        organizer_fee_bps: u32,
    ) -> u64 {
        // Validate organizer fee (cannot exceed 100%)
        if organizer_fee_bps > 10_000 {
            panic!("Organizer fee cannot exceed 100%");
        }

        // Validate insurance fee (cannot exceed 100%)
        if insurance_fee_bps > 10_000 {
            panic!("Insurance fee cannot exceed 100%");
        }

        // Get the current Circle Count
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        // Increment for the new circle
        circle_count += 1;
        
        // Create the new circle
        let circle = CircleInfo {
            creator: creator.clone(),
            contribution_amount,
            max_members,
            current_members: 0,
            token: token.clone(),
            cycle_duration,
            insurance_fee_bps,
            organizer_fee_bps,
            nft_contract,
            arbitrator,
            members: Vec::new(&env),
            contributions: Map::new(&env),
            current_round: 0,
            round_start_time: env.ledger().timestamp(),
            is_round_finalized: false,
            current_pot_recipient: None,
            gas_buffer_balance: 0i128,
            gas_buffer_enabled: true, // Enable by default for reliability
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

        // Calculate payout amounts
        let gross_payout = (circle.contribution_amount as i128) * (circle.current_members as i128);
        let organizer_fee = (gross_payout * circle.organizer_fee_bps as i128) / 10_000;
        let net_payout = gross_payout - organizer_fee;

        // Check gas buffer and ensure sufficient funds for transaction
        Self::ensure_gas_buffer(&env, circle_id);

        // Execute the payout with gas buffer protection
        Self::execute_payout_with_gas_protection(
            &env,
            &circle,
            &recipient,
            &circle.creator,
            net_payout,
            organizer_fee,
        ).expect("Payout execution failed");

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
            (recipient, net_payout),
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

    fn calculate_contribution_cap(env: &Env, user: &Address, requested_contribution: i128) -> i128 {
        // Get user's historical contribution volume from UserStats
        let user_stats_key = DataKey::UserStats(user.clone());
        let user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        
        // Get reputation metrics
        let (reliability_score, social_capital_score, total_cycles) = crate::sbt_minter::SoroSusuSbtMinter::get_user_reputation_score(env.clone(), user.clone());
        
        // Calculate maximum allowed contribution based on reputation history
        let max_allowed_contribution = if user_stats.total_volume_saved == 0 {
            // New user: can only join circles with contribution <= 1000 (minimum threshold)
            1000
        } else {
            // Existing user: max 3x step-up from their highest previous contribution
            // This prevents pump-and-default schemes where users build small trust then join massive groups
            let step_up_multiplier = 3i128;
            user_stats.total_volume_saved * step_up_multiplier
        };
        
        // Additional reputation-based modifiers
        let reputation_modifier = if total_cycles >= 10 {
            // Diamond tier users (10+ cycles) get 2x additional buffer
            2i128
        } else if total_cycles >= 6 {
            // Gold tier users (6-9 cycles) get 1.5x additional buffer
            15000i128 / 10000i128  // 1.5x in basis points
        } else if total_cycles >= 3 {
            // Silver tier users (3-5 cycles) get 1.25x additional buffer
            12500i128 / 10000i128  // 1.25x in basis points
        } else {
            // Bronze tier users (0-2 cycles) get no additional buffer
            1i128
        };
        
        max_allowed_contribution * reputation_modifier
    }

    fn get_user_historical_max_contribution(env: &Env, user: &Address) -> i128 {
        // Get user's historical contribution volume from UserStats
        let user_stats_key = DataKey::UserStats(user.clone());
        let user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        
        user_stats.total_volume_saved
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

        // Deduct gas cost from buffer
        let mut updated_circle = circle.clone();
        updated_circle.gas_buffer_balance -= estimated_gas_cost;
        env.storage::instance().set(&DataKey::Circle(circle.id), &updated_circle);

        Ok(())
    }

    fn check_and_finalize_round(env: &Env, circle_id: u64) {
        if Self::all_members_contributed(env, circle_id) {
            // ... (rest of the code remains the same)
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
            has_contributed_current_round: false,
            consecutive_missed_rounds: 0,
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

        // Flash-loan prevention: Ledger-Lock mechanism
        let current_ledger = env.ledger().sequence();
        if let Some(last_withdrawal) = env.storage().instance().get::<DataKey, u32>(&DataKey::LastWithdrawalLedger(user.clone())) {
            if last_withdrawal == current_ledger {
                panic!("Flash-loan prevention: Cannot deposit and withdraw in same ledger");
            }
        }
        env.storage().instance().set(&DataKey::LastDepositLedger(user.clone()), &current_ledger);

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

        // Flash-loan prevention: Ledger-Lock mechanism
        let current_ledger = env.ledger().sequence();
        if let Some(last_deposit) = env.storage().instance().get::<DataKey, u32>(&DataKey::LastDepositLedger(user.clone())) {
            if last_deposit == current_ledger {
                panic!("Flash-loan prevention: Cannot withdraw and deposit in same ledger");
            }
        }
        env.storage().instance().set(&DataKey::LastWithdrawalLedger(user.clone()), &current_ledger);

        // Inter-protocol security: Check if payout is paused due to external default (e.g., LeaseFlow)
        let is_paused = env.storage().instance().get::<DataKey, bool>(&DataKey::PausedPayout(user.clone(), circle_id)).unwrap_or(false);
        if is_paused {
            panic!("Your payout is currently locked due to a default in a connected protocol (LeaseFlow). Please resolve the default to unlock.");
        }
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

        let mut total_payout = circle.contribution_amount * (circle.member_count as i128);
        
        // Recursive Susu (Auto-Compounding) Opt-In Check
        let opt_in = env.storage().instance().get::<DataKey, bool>(&DataKey::RecursiveOptIn(user.clone(), circle_id)).unwrap_or(false);
        if opt_in {
            let recursive_amount = (total_payout * 2000) / 10000; // 20%
            total_payout -= recursive_amount;
            
            // "Wealth Escalator": Move funds to Gold Tier
            if let Some(gold_circle_id) = env.storage().instance().get::<DataKey, u64>(&DataKey::GoldTierCircle) {
                // Record the transition for recursive wealth building
                env.events().publish(
                    (Symbol::new(&env, "WEALTH_ESCALATOR"), user.clone(), gold_circle_id),
                    (recursive_amount, "Automated transition to Gold Tier"),
                );
            }
        }

        // Check for rollover bonus and add to first pot of new cycles
        let rollover_key = DataKey::RolloverBonus(circle_id);
        if let Some(rollover_bonus) = env.storage().instance().get::<DataKey, RolloverBonus>(&rollover_key) {
            if rollover_bonus.status == RolloverStatus::Applied {
                if let Some(applied_cycle) = rollover_bonus.applied_cycle {
                    if applied_cycle == circle.current_recipient_index {
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
                    token_client.transfer(&env.current_contract_address(), &user, &net);
                } else {
                    token_client.transfer(&env.current_contract_address(), &user, &asset_pot);
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
                token_client.transfer(&env.current_contract_address(), &user, &net_payout);
            } else {
                token_client.transfer(&env.current_contract_address(), &user, &total_payout);
            }
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
                
                // Get user's reputation data for comprehensive scoring
                let reputation_key = DataKey::ReputationData(user.clone());
                let reputation: ReputationData = env.storage().instance().get(&reputation_key).unwrap_or(ReputationData {
                    user_address: user.clone(),
                    susu_score: 0,
                    reliability_score: 0,
                    total_contributions: 0,
                    on_time_rate: 0,
                    volume_saved: 0,
                    social_capital: 0,
                    last_updated: 0,
                    is_active: false,
                });
                
                // Count total cycles completed by this user
                let mut total_cycles: u32 = 0;
                for cycle_key_bytes in env.storage().all_keys() {
                    if let Ok(badge_token_id) = env.storage().instance().get::<DataKey, u128>(&DataKey::CycleBadge(user.clone(), 0)) {
                        // This is a simplified check - in production would iterate through all circles
                        total_cycles += 1;
                    }
                }
                
                // Enhanced volume tier with Platinum level
                let volume_tier: u32 = if stats.total_volume_saved >= 100_000_000_000 { 4 } // Platinum
                    else if stats.total_volume_saved >= 10_000_000_000 { 3 } // Gold
                    else if stats.total_volume_saved >= 1_000_000_000 { 2 } // Silver
                    else { 1 }; // Bronze
                
                // Build list of badges earned
                let mut badges_earned = Vec::new(&env);
                if stats.late_contributions == 0 {
                    badges_earned.push_back(symbol_short!("PERFECT"));
                }
                if member_info.address == circle.creator {
                    badges_earned.push_back(symbol_short!("LEADER"));
                }
                if total_cycles > 1 {
                    badges_earned.push_back(symbol_short!("VETERAN"));
                }
                if volume_tier >= 3 {
                    badges_earned.push_back(symbol_short!("ELITE"));
                }
                
                // Calculate ecosystem participation (simplified - would query other contracts)
                let ecosystem_participation: u32 = 1; // Minimum participation in this contract
                
                // Create Master Credential metadata
                let metadata = MasterCredentialMetadata {
                    volume_tier,
                    perfect_attendance: stats.late_contributions == 0,
                    group_lead_status: member_info.address == circle.creator,
                    total_cycles_completed: total_cycles + 1,
                    total_volume_saved: stats.total_volume_saved,
                    reliability_score: reputation.reliability_score,
                    social_capital_score: reputation.social_capital,
                    badges_earned,
                    ecosystem_participation,
                    mint_timestamp: env.ledger().timestamp(),
                    circle_id,
                    version: 1,
                };
                
                // token_id: circle_id in upper 64 bits, member index in lower 64 bits
                let token_id: u128 = ((circle_id as u128) << 64) | (member_info.index as u128);
                let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
                nft_client.mint_master_credential(&user, &token_id, &metadata);
                env.storage().instance().set(&DataKey::CycleBadge(user.clone(), circle_id), &token_id);
                env.events().publish(
                    (symbol_short!("BADGE"), symbol_short!("MASTER")),
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

        // Get the Group Insurance Fund
        let mut insurance_fund: GroupInsuranceFund = env.storage().instance()
            .get(&DataKey::GroupInsuranceFund(circle_id))
            .expect("Group Insurance Fund not found");
        
        if !insurance_fund.is_active {
            panic!("Insurance fund is not active");
        }
        
        if insurance_fund.total_fund_balance <= 0 {
            panic!("Insufficient insurance fund balance");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        // Calculate amount needed to cover the default (contribution for remaining rounds)
        let rounds_remaining = circle.max_members - circle.current_recipient_index;
        let amount_needed = circle.contribution_amount * (rounds_remaining as i128);
        
        if insurance_fund.total_fund_balance < amount_needed {
            panic!("Insufficient insurance fund balance to cover default");
        }

        // Deduct from insurance fund
        insurance_fund.total_fund_balance -= amount_needed;
        insurance_fund.total_claims_paid += amount_needed;
        insurance_fund.last_claim_time = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::GroupInsuranceFund(circle_id), &insurance_fund);

        // Update member's premium record to track claims
        let mut premium_record: InsurancePremiumRecord = env.storage().instance()
            .get(&DataKey::InsurancePremium(circle_id, member.clone()))
            .unwrap_or(InsurancePremiumRecord {
                member: member.clone(),
                circle_id,
                total_premium_paid: 0,
                premium_payments: Vec::new(&env),
                claims_made: 0,
                net_contribution: 0,
            });
        
        premium_record.claims_made += amount_needed;
        premium_record.net_contribution = premium_record.total_premium_paid - premium_record.claims_made;
        env.storage().instance().set(&DataKey::InsurancePremium(circle_id, member.clone()), &premium_record);

        // Mark the member as defaulted
        let mut member_status = member_info.status;
        member_status = MemberStatus::Defaulted;
        env.storage().instance().set(&DataKey::Member(member.clone()), &member_info);

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
            (Symbol::new(&env, "INSURANCE_CLAIM"), circle_id, member.clone()),
            (amount_needed, insurance_fund.total_fund_balance),
        );

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn get_insurance_fund(env: Env, circle_id: u64) -> GroupInsuranceFund {
        env.storage().instance()
            .get(&DataKey::GroupInsuranceFund(circle_id))
            .expect("Group Insurance Fund not found")
    }

    fn get_premium_record(env: Env, member: Address, circle_id: u64) -> InsurancePremiumRecord {
        env.storage().instance()
            .get(&DataKey::InsurancePremium(circle_id, member))
            .expect("Premium record not found")
    }

    fn distribute_remaining_insurance_fund(env: Env, circle_id: u64) {
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let mut insurance_fund: GroupInsuranceFund = env.storage().instance()
            .get(&DataKey::GroupInsuranceFund(circle_id))
            .expect("Group Insurance Fund not found");

        // Check if cycle is complete (all members have received pot)
        if circle.current_recipient_index < circle.max_members - 1 {
            panic!("Cycle not complete - cannot distribute insurance fund yet");
        }

        if insurance_fund.total_fund_balance <= 0 {
            panic!("No remaining insurance fund to distribute");
        }

        // Calculate pro-rata distribution based on premiums paid
        let total_fund = insurance_fund.total_fund_balance;
        let token_client = token::Client::new(&env, &circle.token);

        for i in 0..circle.member_count {
            let member_address = circle.member_addresses.get(i).unwrap();
            
            // Get member's premium record
            if let Some(premium_record) = env.storage().instance()
                .get::<DataKey, InsurancePremiumRecord>(&DataKey::InsurancePremium(circle_id, member_address.clone()))
            {
                // Calculate refund percentage based on premium paid
                let refund_percentage = if insurance_fund.total_premiums_collected > 0 {
                    (premium_record.total_premium_paid * 10_000) / insurance_fund.total_premiums_collected
                } else {
                    0
                };
                
                let refund_amount = (total_fund * refund_percentage) / 10_000;
                
                if refund_amount > 0 {
                    token_client.transfer(&env.current_contract_address(), &member_address, &refund_amount);
                    
                    env.events().publish(
                        (Symbol::new(&env, "INSURANCE_REFUND"), circle_id, member_address.clone()),
                        (refund_amount, premium_record.total_premium_paid),
                    );
                }
            }
        }

        // Reset insurance fund for next cycle or mark as inactive
        insurance_fund.total_fund_balance = 0;
        insurance_fund.is_active = false;
        env.storage().instance().set(&DataKey::GroupInsuranceFund(circle_id), &insurance_fund);

        env.events().publish(
            (Symbol::new(&env, "INSURANCE_FUND_DISTRIBUTED"), circle_id),
            (total_fund, circle.member_count),
        );
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
            let volume_bonus = ((user_stats.total_volume_saved / 1_000_000_0) * 100).min(2000); // Max 20% bonus
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

        // Apply the bonus to the group reserve (will be used in next cycle's first pot)
        let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        reserve += rollover_bonus.bonus_amount;
        env.storage().instance().set(&DataKey::GroupReserve, &reserve);

        // Mark as applied and track the cycle
        rollover_bonus.status = RolloverStatus::Applied;
        rollover_bonus.applied_cycle = Some(circle.current_recipient_index + 1);
        env.storage().instance().set(&rollover_key, &rollover_bonus);

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_APPLIED"), circle_id),
            (rollover_bonus.bonus_amount, rollover_bonus.applied_cycle.unwrap()),
        );
    }

    // Price Oracle and Asset Swap Implementation
    
    fn update_price_oracle(env: Env, oracle_provider: Address, asset: Address, price: i128) {
        // Only authorized oracle providers can update prices (in production, use multi-sig or trusted oracles)
        oracle_provider.require_auth();
        
        if price <= 0 {
            panic!("Invalid price");
        }
        
        let oracle_data = PriceOracleData {
            asset_address: asset.clone(),
            current_price: price,
            last_updated: env.ledger().timestamp(),
            is_stable_asset: false, // Would be determined by oracle provider
        };
        
        env.storage().instance().set(&DataKey::PriceOracle(asset), &oracle_data);
        
        env.events().publish(
            (Symbol::new(&env, "PRICE_UPDATED"), asset),
            (price, env.ledger().timestamp()),
        );
    }
    
    fn get_asset_price(env: Env, asset: Address) -> PriceOracleData {
        env.storage().instance()
            .get(&DataKey::PriceOracle(asset))
            .expect("Asset price not found")
    }
    
    fn set_hard_asset_basket(env: Env, admin: Address, gold_weight_bps: u32, btc_weight_bps: u32, silver_weight_bps: u32) {
        // Verify admin authorization
        let stored_admin: Address = env.storage().instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        if admin != stored_admin {
            panic!("Unauthorized: only admin can set hard asset basket");
        }
        
        let total_weight = gold_weight_bps + btc_weight_bps + silver_weight_bps;
        if total_weight != 10000 {
            panic!("Basket weights must sum to 10000 (100%)");
        }
        
        let basket = HardAssetBasket {
            gold_weight_bps,
            btc_weight_bps,
            silver_weight_bps,
            total_weight_bps: total_weight,
        };
        
        env.storage().instance().set(&DataKey::HardAssetBasket, &basket);
        
        env.events().publish(
            (Symbol::new(&env, "HARD_ASSET_BASKET_SET")),
            (gold_weight_bps, btc_weight_bps, silver_weight_bps),
        );
    }
    
    fn get_hard_asset_basket(env: Env) -> HardAssetBasket {
        env.storage().instance()
            .get(&DataKey::HardAssetBasket)
            .unwrap_or(HardAssetBasket {
                gold_weight_bps: DEFAULT_HARD_ASSET_GOLD_WEIGHT,
                btc_weight_bps: DEFAULT_HARD_ASSET_BTC_WEIGHT,
                silver_weight_bps: DEFAULT_HARD_ASSET_SILVER_WEIGHT,
                total_weight_bps: 10000,
            })
    }
    
    fn check_price_drop_and_trigger_swap(env: Env, circle_id: u64) -> bool {
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        // Get current asset price
        let current_price_data: PriceOracleData = match env.storage().instance().get(&DataKey::PriceOracle(circle.token.clone())) {
            Some(data) => data,
            None => return false, // No oracle data available
        };
        
        // Calculate hard asset basket weighted price
        let basket = Self::get_hard_asset_basket(env.clone());
        
        // Get prices for hard assets (simplified - would need actual oracle feeds)
        // In production, this would query multiple oracle sources
        let gold_price: PriceOracleData = match env.storage().instance().get(&DataKey::PriceOracle(Address::from_str("GOLD_ASSET_ADDRESS")?)) {
            Some(data) => data,
            None => return false,
        };
        
        // Calculate if current asset dropped more than 20% against hard asset basket
        // Simplified calculation: compare current price to a baseline
        let price_drop_threshold = (current_price_data.current_price * PRICE_DROP_THRESHOLD_BPS as i128) / 10000;
        
        // This is simplified - in production would compare against historical baseline
        let current_price = current_price_data.current_price;
        let threshold_price = price_drop_threshold; // 20% drop from some baseline
        
        if current_price < threshold_price {
            // Price drop detected - auto-trigger a swap proposal
            let target_asset = Address::from_str("STABLE_ASSET_ADDRESS").expect("Invalid address"); // Would be configurable
            Self::propose_asset_swap(env.clone(), circle.creator.clone(), circle_id, target_asset, 10000);
            return true;
        }
        
        false
    }
    
    fn propose_asset_swap(env: Env, user: Address, circle_id: u64, target_asset: Address, swap_percentage_bps: u32) {
        user.require_auth();
        
        if swap_percentage_bps > 10000 {
            panic!("Swap percentage cannot exceed 100%");
        }
        
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        // Only circle creator or members can propose
        let mut is_member = false;
        for i in 0..circle.member_count {
            if circle.member_addresses.get(i).unwrap() == user {
                is_member = true;
                break;
            }
        }
        
        if !is_member && user != circle.creator {
            panic!("Unauthorized: only circle members can propose asset swap");
        }
        
        // Get current asset price to calculate price drop
        let current_price_data: PriceOracleData = match env.storage().instance().get(&DataKey::PriceOracle(circle.token.clone())) {
            Some(data) => data,
            None => panic!("Current asset price not found"),
        };
        
        // Calculate price drop percentage (simplified)
        let price_drop_bps = 2000; // Would be calculated from historical data
        
        let proposal = AssetSwapProposal {
            circle_id,
            proposer: user.clone(),
            current_asset: circle.token.clone(),
            target_asset,
            swap_percentage_bps,
            price_drop_percentage_bps: price_drop_bps,
            created_timestamp: env.ledger().timestamp(),
            voting_deadline: env.ledger().timestamp() + ASSET_SWAP_VOTING_PERIOD,
            status: ProposalStatus::Active,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            executed_timestamp: None,
        };
        
        env.storage().instance().set(&DataKey::AssetSwapProposal(circle_id), &proposal);
        
        env.events().publish(
            (Symbol::new(&env, "ASSET_SWAP_PROPOSED"), circle_id),
            (user, target_asset, swap_percentage_bps),
        );
    }
    
    fn vote_asset_swap(env: Env, user: Address, circle_id: u64, vote_choice: QuadraticVoteChoice) {
        user.require_auth();
        
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let mut proposal: AssetSwapProposal = env.storage().instance()
            .get(&DataKey::AssetSwapProposal(circle_id))
            .expect("Asset swap proposal not found");
        
        if proposal.status != ProposalStatus::Active {
            panic!("Proposal is not active");
        }
        
        if env.ledger().timestamp() > proposal.voting_deadline {
            panic!("Voting period has ended");
        }
        
        // Check if user is a circle member
        let mut is_member = false;
        for i in 0..circle.member_count {
            if circle.member_addresses.get(i).unwrap() == user {
                is_member = true;
                break;
            }
        }
        
        if !is_member {
            panic!("Only circle members can vote");
        }
        
        // Prevent duplicate voting
        let vote_key = DataKey::AssetSwapVote(circle_id, user.clone());
        if env.storage().instance().contains(&vote_key) {
            panic!("Already voted on this proposal");
        }
        
        // Record vote (simple 1 member = 1 vote for now, could use quadratic voting)
        let vote_weight = 1u32;
        
        match vote_choice {
            QuadraticVoteChoice::For => proposal.for_votes += vote_weight,
            QuadraticVoteChoice::Against => proposal.against_votes += vote_weight,
            QuadraticVoteChoice::Abstain => { /* Abstain doesn't count */ () },
        }
        
        proposal.total_votes_cast += vote_weight;
        
        // Store vote record
        let vote_record = (vote_choice, env.ledger().timestamp());
        env.storage().instance().set(&vote_key, &vote_record);
        env.storage().instance().set(&DataKey::AssetSwapProposal(circle_id), &proposal);
        
        env.events().publish(
            (Symbol::new(&env, "ASSET_SWAP_VOTE"), circle_id),
            (user, vote_choice),
        );
    }
    
    fn execute_asset_swap(env: Env, circle_id: u64) {
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let mut proposal: AssetSwapProposal = env.storage().instance()
            .get(&DataKey::AssetSwapProposal(circle_id))
            .expect("Asset swap proposal not found");
        
        if proposal.status != ProposalStatus::Active {
            panic!("Proposal is not active");
        }
        
        if env.ledger().timestamp() <= proposal.voting_deadline {
            panic!("Voting period has not ended");
        }
        
        // Check quorum
        let participation_bps = (proposal.total_votes_cast as u32 * 10_000) / circle.member_count;
        if participation_bps < ASSET_SWAP_QUORUM {
            proposal.status = ProposalStatus::Rejected;
            env.storage().instance().set(&DataKey::AssetSwapProposal(circle_id), &proposal);
            panic!("Quorum not met");
        }
        
        // Check majority
        let approval_bps = if proposal.total_votes_cast > 0 {
            (proposal.for_votes * 10_000) / proposal.total_votes_cast
        } else {
            0
        };
        
        if approval_bps < ASSET_SWAP_MAJORITY {
            proposal.status = ProposalStatus::Rejected;
            env.storage().instance().set(&DataKey::AssetSwapProposal(circle_id), &proposal);
            panic!("Majority not reached");
        }
        
        // Execute the swap
        proposal.status = ProposalStatus::Executed;
        proposal.executed_timestamp = Some(env.ledger().timestamp());
        
        // Update circle's token to the new asset
        let mut updated_circle = circle;
        updated_circle.token = proposal.target_asset.clone();
        env.storage().instance().set(&DataKey::Circle(circle_id), &updated_circle);
        
        // In production, would actually perform the token swap via DEX
        // For now, we just update the accounting
        
        env.storage().instance().set(&DataKey::AssetSwapProposal(circle_id), &proposal);
        
        env.events().publish(
            (Symbol::new(&env, "ASSET_SWAP_EXECUTED"), circle_id),
            (proposal.current_asset, proposal.target_asset, proposal.swap_percentage_bps),
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
        distribute_yield_earnings(env, circle_id);

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

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);
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
            source_token: Address::generate(&env), // Will be set during execution
            target_token: circle.token.clone(),
            source_amount: 0, // Will be set during execution
            target_amount: 0, // Will be calculated during execution
            exchange_rate: 0,
            slippage_bps: 0,
            dex_address: Address::generate(&env), // Will be set during execution
            path_payment: Address::generate(&env), // Will be set during execution
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

        // Flash-loan prevention: Ledger-Lock mechanism
        let current_ledger = env.ledger().sequence();
        if let Some(last_withdrawal) = env.storage().instance().get::<DataKey, u32>(&DataKey::LastWithdrawalLedger(user.clone())) {
            if last_withdrawal == current_ledger {
                panic!("Flash-loan prevention: Cannot deposit and withdraw in same ledger");
            }
        }
        env.storage().instance().set(&DataKey::LastDepositLedger(user.clone()), &current_ledger);

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
            let prev_amount: i128 = env
                .storage()
                .instance()
                .get(&contrib_key)
                .unwrap_or(0);
            env.storage()
                .instance()
                .set(&contrib_key, &(prev_amount + asset_amount));
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

    fn toggle_recursive_opt_in(env: Env, user: Address, circle_id: u64, enabled: bool) {
        user.require_auth();
        env.storage().instance().set(&DataKey::RecursiveOptIn(user.clone(), circle_id), &enabled);
        
        env.events().publish(
            (Symbol::new(&env, "RECURSIVE_OPT_IN"), circle_id, user),
            enabled,
        );
    }

    fn recursive_init(env: Env, admin: Address, amount: i128, token: Address, circle_id: u64) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can initialize Gold Tier Susu");
        }

        // Store this circle as the target Gold Tier Susu
        env.storage().instance().set(&DataKey::GoldTierCircle, &circle_id);
        
        env.events().publish(
            (Symbol::new(&env, "GOLD_TIER_INITIALIZED"), circle_id),
            (amount, token),
        );
    }

    fn is_cycle_healthy(env: Env, user: Address, circle_id: u64) -> bool {
        let member_key = DataKey::Member(user.clone());
        if !env.storage().instance().has(&member_key) {
            return false;
        }
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        // A cycle is healthy if the member is active and has no recently missed payments
        member.status == MemberStatus::Active && member.consecutive_missed_rounds == 0
    }

    fn handle_leaseflow_default(env: Env, leaseflow_contract: Address, user: Address, circle_id: u64) {
        leaseflow_contract.require_auth();
        
        let trusted_leaseflow: Address = env.storage().instance().get(&DataKey::LeaseFlowContract)
            .expect("LeaseFlow contract not trusted yet");
        
        if leaseflow_contract != trusted_leaseflow {
            panic!("Unauthorized: Only trusted LeaseFlow contract can signal defaults");
        }

        // Lock the user's next payout
        env.storage().instance().set(&DataKey::PausedPayout(user.clone(), circle_id), &true);
        
        env.events().publish(
            (Symbol::new(&env, "INTER_PROTOCOL_LOCK"), circle_id, user.clone()),
            (leaseflow_contract, "Payout paused due to external default"),
        );
    }

    fn set_leaseflow_contract(env: Env, admin: Address, leaseflow: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can set bridge targets");
        }
        env.storage().instance().set(&DataKey::LeaseFlowContract, &leaseflow);
    }

    fn handle_grant_stream_match(env: Env, grant_stream_contract: Address, circle_id: u64, amount: i128) {
        grant_stream_contract.require_auth();
        
        let trusted_grant_stream: Address = env.storage().instance().get(&DataKey::GrantStreamContract)
            .expect("Grant-Stream contract not trusted yet");
        if grant_stream_contract != trusted_grant_stream {
            panic!("Unauthorized: Only trusted Grant-Stream contract can match savings");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).expect("Circle not found");
        
        // Identify members with 100% on-time record within this circle
        let mut perfect_members: Vec<Address> = Vec::new(&env);
        for i in 0..circle.member_count {
            let addr = get_member_address_by_index(&circle, i);
            let user_stats_key = DataKey::UserStats(addr.clone());
            let stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });
            
            if stats.late_contributions == 0 && stats.on_time_contributions > 0 {
                perfect_members.push_back(addr);
            }
        }

        if perfect_members.len() == 0 {
            panic!("No eligible members with 100% on-time record for matching bonus");
        }

        // Receive the "Incentive Drip" from Grant-Stream
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&grant_stream_contract, &env.current_contract_address(), &amount);

        // Distribute equally among perfect savers
        let share = amount / (perfect_members.len() as i128);
        for member in perfect_members.iter() {
            token_client.transfer(&env.current_contract_address(), &member, &share);
        }

        env.events().publish(
            (Symbol::new(&env, "GRANT_MATCH_DISTRIBUTED"), circle_id),
            (amount, perfect_members.len()),
        );
    }

    fn set_grant_stream_contract(env: Env, admin: Address, grant_stream: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not set");
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can set grant distribution source");
        }
        env.storage().instance().set(&DataKey::GrantStreamContract, &grant_stream);
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

fn calculate_yield_from_pool(env: &Env, delegation: &YieldDelegation, time_elapsed: u64) -> i128 {
    // Simplified yield calculation - in real implementation would query actual pool
    let apy_bps = 500; // 5% APY
    let seconds_in_year = 365 * 24 * 60 * 60;
    let time_fraction = time_elapsed as i128 * 10000 / seconds_in_year as i128;
    (delegation.delegation_amount * apy_bps as i128 * time_fraction) / (10000 * 10000)
}

    #[test]
    fn test_get_reputation() {
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
        
        // Test reputation for new user (should be zero/low)
        let reputation = client.get_reputation(&user);
        assert_eq!(reputation.susu_score, 0);
        assert_eq!(reputation.reliability_score, 0);
        assert_eq!(reputation.total_contributions, 0);
        assert_eq!(reputation.on_time_rate, 0);
        assert_eq!(reputation.volume_saved, 0);
        assert_eq!(reputation.is_active, false);
        
        // Create circle and add user
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
        
        // Test reputation after contribution
        let reputation = client.get_reputation(&user);
        assert!(reputation.susu_score > 0);
        assert!(reputation.reliability_score > 0);
        assert_eq!(reputation.total_contributions, 1);
        assert_eq!(reputation.on_time_rate, 10000); // 100% on-time rate
        assert_eq!(reputation.volume_saved, 1_000_000_000_000);
        assert_eq!(reputation.is_active, true);
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

    #[test]
    fn test_rollover_bonus_proposal_and_voting() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Set up protocol fee for rollover bonus calculation
        client.set_protocol_fee(&admin, &100, &admin); // 1% fee
        
        // Create circle with 2 members
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000, // 1000 tokens
            &2,
            &token_contract,
            &86400,
            &100, // 1% insurance
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&user1, &circle_id);
        
        // Now propose rollover bonus (50% of platform fee)
        client.propose_rollover_bonus(&creator, &circle_id, &5000);
        
        // Second member votes for the rollover
        client.vote_rollover_bonus(&user1, &circle_id, &RolloverVoteChoice::For);
        
        // Apply the rollover bonus
        client.apply_rollover_bonus(&circle_id);
        
        // Start third cycle - first recipient should get rollover bonus
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Check that rollover bonus is applied to payout
        let initial_balance = token_contract.mock_balance(&creator);
        client.claim_pot(&creator, &circle_id);
        let final_balance = token_contract.mock_balance(&creator);
        
        // Should receive regular pot (2000) minus fee (1% = 20) plus rollover bonus (50% of fee = 10)
        let expected_payout = 2000 - 20 + 10; // 1990
        assert_eq!(final_balance - initial_balance, expected_payout);
    }

    #[test]
    fn test_rollover_bonus_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        client.set_protocol_fee(&admin, &100, &admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Propose rollover bonus
        client.propose_rollover_bonus(&creator, &circle_id, &5000);
        
        // Second member votes against - should not meet majority threshold
        client.vote_rollover_bonus(&user1, &circle_id, &RolloverVoteChoice::Against);
        
        // Try to apply should fail since not approved
        std::panic::catch_unwind(|| {
            client.apply_rollover_bonus(&circle_id);
        }).expect_err("Should panic when trying to apply unapproved rollover");
    }

    #[test]
    fn test_yield_delegation_proposal_and_voting() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with 3 members for higher quorum requirements
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000, // 1000 tokens
            &3,
            &token_contract,
            &86400,
            &100, // 1% insurance
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        client.join_circle(&user2, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and finalize again
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose yield delegation (50% of pot)
        let pool_address = Address::generate(&env);
        client.propose_yield_delegation(
            &creator, 
            &circle_id, 
            &5000, // 50%
            &pool_address,
            &YieldPoolType::StellarLiquidityPool
        );
        
        // Other members vote for the delegation
        client.vote_yield_delegation(&user1, &circle_id, &YieldVoteChoice::For);
        client.vote_yield_delegation(&user2, &circle_id, &YieldVoteChoice::For);
        
        // Approve and execute delegation
        client.approve_yield_delegation(&circle_id);
        client.execute_yield_delegation(&circle_id);
        
        // Test compounding
        env.ledger().set_timestamp(env.ledger().timestamp() + YIELD_COMPOUNDING_FREQUENCY + 1);
        client.compound_yield(&circle_id);
        
        // Test withdrawal and distribution
        client.withdraw_yield_delegation(&circle_id);
    }

    #[test]
    fn test_yield_delegation_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose yield delegation
        let pool_address = Address::generate(&env);
        client.propose_yield_delegation(
            &creator, 
            &circle_id, 
            &5000,
            &pool_address,
            &YieldPoolType::StellarLiquidityPool
        );
        
        // Second member votes against - should not meet 80% majority
        client.vote_yield_delegation(&user1, &circle_id, &YieldVoteChoice::Against);
        
        // Try to approve should fail since not approved
        std::panic::catch_unwind(|| {
            client.approve_yield_delegation(&circle_id);
        }).expect_err("Should panic when trying to approve rejected delegation");
    }

    #[test]
    fn test_path_payment_support_proposal_and_execution() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with USDC as target token
        let usdc_address = Address::generate(&env);
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000, // 1000 tokens
            &3,
            &usdc_address, // USDC as target token
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Register XLM as supported token
        client.register_supported_token(
            &creator,
            &token_contract, // XLM token address
            &String::from_str(&env, "XLM"),
            &7,
            &true
        );
        
        // Register USDC as supported token
        client.register_supported_token(
            &creator,
            &usdc_address, // USDC token address
            &String::from_str(&env, "USDC"),
            &6,
            &true
        );
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and propose path payment support
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose path payment support (XLM to USDC)
        client.propose_path_payment_support(&creator, &circle_id);
        
        // Vote for path payment support
        client.vote_path_payment_support(&user1, &circle_id, &PathPaymentVoteChoice::For);
        
        // Approve and execute path payment
        client.approve_path_payment_support(&circle_id);
        
        // Execute path payment (user sends XLM, gets USDC in circle)
        let xlm_address = token_contract;
        client.execute_path_payment(
            &user1,
            &circle_id,
            &xlm_address,
            &500_000_000 // 500 XLM
        );
    }

    #[test]
    fn test_path_payment_support_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let usdc_address = Address::generate(&env);
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000,
            &2,
            &usdc_address, // USDC as target token
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and propose path payment support
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose path payment support
        client.propose_path_payment_support(&creator, &circle_id);
        
        // Second member votes against - should not meet 66% majority
        client.vote_path_payment_support(&user1, &circle_id, &PathPaymentVoteChoice::Against);
        
        // Try to approve should fail since not approved
        std::panic::catch_unwind(|| {
            client.approve_path_payment_support(&circle_id);
        }).expect_err("Should panic when trying to approve rejected path payment");
    }
}
