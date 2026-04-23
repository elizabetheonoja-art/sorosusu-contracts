// --- DEFAULT RECOVERY REPAYMENT MODULE ---
//
// Implements the repayment logic for members who have previously defaulted.
// A defaulted member must repay the missing contribution amount plus a
// reinstatement penalty. Their Reliability Index (RI) recovers gradually
// over several rounds rather than being instantly restored.

#![no_std]

use soroban_sdk::{contracttype, Address, Env};

// --- CONSTANTS ---

/// Reinstatement penalty on top of owed amount (basis points)
const REINSTATEMENT_PENALTY_BPS: u32 = 1500; // 15%

/// RI points recovered per successful on-time contribution after default
const RI_RECOVERY_PER_ROUND: u32 = 25;

/// Maximum RI score a member can hold (cap)
const MAX_RI_SCORE: u32 = 1000;

/// Minimum rounds of good standing required before full reinstatement
const MIN_RECOVERY_ROUNDS: u32 = 3;

// --- DATA KEYS ---

#[contracttype]
#[derive(Clone)]
pub enum DefaultRecoveryKey {
    RepaymentRecord(Address, u64),  // (member, circle_id) -> RepaymentRecord
    RecoveryProgress(Address, u64), // (member, circle_id) -> RecoveryProgress
}

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RepaymentStatus {
    Pending,
    Repaid,
    Overdue,
    Waived,
}

/// Tracks what a defaulted member owes to regain standing
#[contracttype]
#[derive(Clone)]
pub struct RepaymentRecord {
    pub member: Address,
    pub circle_id: u64,
    pub missed_amount: i128,
    pub penalty_amount: i128,
    pub total_owed: i128,
    pub amount_paid: i128,
    pub status: RepaymentStatus,
    pub defaulted_at: u64,
    pub repaid_at: Option<u64>,
}

/// Tracks the gradual RI recovery process post-repayment
#[contracttype]
#[derive(Clone)]
pub struct RecoveryProgress {
    pub member: Address,
    pub circle_id: u64,
    pub ri_score_at_default: u32,
    pub current_ri_score: u32,
    pub recovery_rounds_completed: u32,
    pub fully_reinstated: bool,
}

// --- FUNCTIONS ---

/// Initiate a repayment record for a member who has defaulted.
/// Calculates total owed = missed_amount + reinstatement penalty.
pub fn record_default(
    env: &Env,
    member: Address,
    circle_id: u64,
    missed_amount: i128,
    ri_score_at_default: u32,
) -> RepaymentRecord {
    assert!(missed_amount > 0, "Missed amount must be positive");

    let penalty = (missed_amount * REINSTATEMENT_PENALTY_BPS as i128) / 10_000;
    let total_owed = missed_amount + penalty;
    let now = env.ledger().timestamp();

    let record = RepaymentRecord {
        member: member.clone(),
        circle_id,
        missed_amount,
        penalty_amount: penalty,
        total_owed,
        amount_paid: 0,
        status: RepaymentStatus::Pending,
        defaulted_at: now,
        repaid_at: None,
    };

    let progress = RecoveryProgress {
        member: member.clone(),
        circle_id,
        ri_score_at_default,
        current_ri_score: ri_score_at_default,
        recovery_rounds_completed: 0,
        fully_reinstated: false,
    };

    env.storage()
        .instance()
        .set(&DefaultRecoveryKey::RepaymentRecord(member.clone(), circle_id), &record);
    env.storage()
        .instance()
        .set(&DefaultRecoveryKey::RecoveryProgress(member, circle_id), &progress);

    record
}

/// Process a repayment from a defaulted member.
/// `payment_amount` is what the member is paying this round.
/// Marks the record as Repaid once total_owed is covered.
pub fn repay_default(
    env: &Env,
    member: Address,
    circle_id: u64,
    payment_amount: i128,
) -> RepaymentRecord {
    member.require_auth();

    assert!(payment_amount > 0, "Payment amount must be positive");

    let record_key = DefaultRecoveryKey::RepaymentRecord(member.clone(), circle_id);
    let mut record: RepaymentRecord = env
        .storage()
        .instance()
        .get(&record_key)
        .expect("No default record found for this member");

    assert!(
        record.status == RepaymentStatus::Pending,
        "Default is not in pending repayment state"
    );

    record.amount_paid += payment_amount;

    if record.amount_paid >= record.total_owed {
        record.status = RepaymentStatus::Repaid;
        record.repaid_at = Some(env.ledger().timestamp());
    }

    env.storage().instance().set(&record_key, &record);

    record
}

/// Called each round after the defaulted member makes an on-time contribution.
/// Incrementally restores their RI score. Full reinstatement requires
/// MIN_RECOVERY_ROUNDS of consecutive good standing post-repayment.
pub fn advance_ri_recovery(
    env: &Env,
    member: Address,
    circle_id: u64,
) -> RecoveryProgress {
    let progress_key = DefaultRecoveryKey::RecoveryProgress(member.clone(), circle_id);
    let mut progress: RecoveryProgress = env
        .storage()
        .instance()
        .get(&progress_key)
        .expect("No recovery progress record found");

    assert!(!progress.fully_reinstated, "Member is already fully reinstated");

    progress.recovery_rounds_completed += 1;

    let new_score = (progress.current_ri_score + RI_RECOVERY_PER_ROUND).min(MAX_RI_SCORE);
    progress.current_ri_score = new_score;

    if progress.recovery_rounds_completed >= MIN_RECOVERY_ROUNDS
        && progress.current_ri_score >= progress.ri_score_at_default
    {
        progress.fully_reinstated = true;
    }

    env.storage().instance().set(&progress_key, &progress);

    progress
}

/// Fetch the current repayment record for a member in a circle.
pub fn get_repayment_record(
    env: &Env,
    member: Address,
    circle_id: u64,
) -> Option<RepaymentRecord> {
    let key = DefaultRecoveryKey::RepaymentRecord(member, circle_id);
    env.storage().instance().get(&key)
}

/// Fetch the current RI recovery progress for a member in a circle.
pub fn get_recovery_progress(
    env: &Env,
    member: Address,
    circle_id: u64,
) -> Option<RecoveryProgress> {
    let key = DefaultRecoveryKey::RecoveryProgress(member, circle_id);
    env.storage().instance().get(&key)
}
