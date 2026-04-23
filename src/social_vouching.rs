// --- SOCIAL VOUCHING MODULE ---
//
// Implements the social-vouching mechanism where a high-reputation member can
// lock capital to guarantee a new (unproven) member's participation. If the
// new member defaults, the voucher's locked capital is slashed proportionally.

#![no_std]

use soroban_sdk::{contracttype, Address, Env, Vec};

// --- CONSTANTS ---

/// Minimum Reliability Index score required to vouch for another member
const MIN_VOUCHER_RI_SCORE: u32 = 700;

/// Maximum number of active vouches a single member can hold at once
const MAX_ACTIVE_VOUCHES: u32 = 3;

/// Penalty applied to voucher's RI when the vouched member defaults (basis points)
const VOUCHER_PENALTY_BPS: u32 = 2000; // 20%

/// Duration (in ledger seconds) a vouch stays active before expiring
const VOUCH_EXPIRY_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days

// --- DATA KEYS ---

#[contracttype]
#[derive(Clone)]
pub enum VouchDataKey {
    VouchRecord(Address, Address), // (voucher, vouched) -> VouchRecord
    VoucherActiveCount(Address),   // active vouch count per voucher
    VouchedMemberVoucher(Address), // vouched member -> who vouched them
}

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum VouchStatus {
    Active,
    Redeemed,
    Slashed,
    Expired,
}

/// A record of one member vouching for another within a circle
#[contracttype]
#[derive(Clone)]
pub struct VouchRecord {
    pub voucher: Address,
    pub vouched: Address,
    pub circle_id: u64,
    pub locked_amount: i128,
    pub created_at: u64,
    pub expires_at: u64,
    pub status: VouchStatus,
    pub slash_amount: i128,
}

/// Summary of a member's vouching activity
#[contracttype]
#[derive(Clone)]
pub struct VoucherProfile {
    pub address: Address,
    pub active_vouches: u32,
    pub total_vouches_given: u32,
    pub total_slashes_received: u32,
    pub total_slashed_amount: i128,
}

// --- FUNCTIONS ---

/// Allow a high-RI member to vouch for a new member by locking capital.
/// Returns an updated VouchRecord on success.
///
/// The voucher must have an RI score >= MIN_VOUCHER_RI_SCORE and must not
/// exceed MAX_ACTIVE_VOUCHES. If the vouched member later defaults, the
/// voucher's locked_amount is slashed via `slash_voucher`.
pub fn vouch_for_user(
    env: &Env,
    voucher: Address,
    vouched: Address,
    circle_id: u64,
    locked_amount: i128,
    voucher_ri_score: u32,
) -> VouchRecord {
    voucher.require_auth();

    assert!(voucher_ri_score >= MIN_VOUCHER_RI_SCORE, "RI score too low to vouch");
    assert!(locked_amount > 0, "Locked amount must be positive");
    assert!(voucher != vouched, "Cannot vouch for yourself");

    let active_key = VouchDataKey::VoucherActiveCount(voucher.clone());
    let active_count: u32 = env.storage().instance().get(&active_key).unwrap_or(0);
    assert!(active_count < MAX_ACTIVE_VOUCHES, "Max active vouches reached");

    let now = env.ledger().timestamp();
    let record = VouchRecord {
        voucher: voucher.clone(),
        vouched: vouched.clone(),
        circle_id,
        locked_amount,
        created_at: now,
        expires_at: now + VOUCH_EXPIRY_SECONDS,
        status: VouchStatus::Active,
        slash_amount: 0,
    };

    let record_key = VouchDataKey::VouchRecord(voucher.clone(), vouched.clone());
    env.storage().instance().set(&record_key, &record);
    env.storage().instance().set(&active_key, &(active_count + 1));
    env.storage()
        .instance()
        .set(&VouchDataKey::VouchedMemberVoucher(vouched), &voucher);

    record
}

/// Called when a vouched member defaults. Slashes a portion of the voucher's
/// locked capital and marks the vouch as slashed.
pub fn slash_voucher(
    env: &Env,
    voucher: Address,
    vouched: Address,
) -> VouchRecord {
    let record_key = VouchDataKey::VouchRecord(voucher.clone(), vouched.clone());
    let mut record: VouchRecord = env
        .storage()
        .instance()
        .get(&record_key)
        .expect("Vouch record not found");

    assert!(record.status == VouchStatus::Active, "Vouch is not active");

    let slash = (record.locked_amount * VOUCHER_PENALTY_BPS as i128) / 10_000;
    record.slash_amount = slash;
    record.status = VouchStatus::Slashed;

    env.storage().instance().set(&record_key, &record);

    let active_key = VouchDataKey::VoucherActiveCount(voucher);
    let count: u32 = env.storage().instance().get(&active_key).unwrap_or(1);
    if count > 0 {
        env.storage().instance().set(&active_key, &(count - 1));
    }

    record
}

/// Retrieve the current vouch record between a voucher and a vouched member.
pub fn get_vouch_record(
    env: &Env,
    voucher: Address,
    vouched: Address,
) -> Option<VouchRecord> {
    let key = VouchDataKey::VouchRecord(voucher, vouched);
    env.storage().instance().get(&key)
}

/// Returns how many vouches are currently active for a given voucher.
pub fn get_active_vouch_count(env: &Env, voucher: Address) -> u32 {
    let key = VouchDataKey::VoucherActiveCount(voucher);
    env.storage().instance().get(&key).unwrap_or(0)
}
