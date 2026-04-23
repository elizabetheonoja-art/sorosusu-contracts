// --- EARLY-BIRD RI BONUS MODULE ---
//
// Awards a Reliability Index (RI) bonus multiplier to members who contribute
// at least 24 hours before the circle's round deadline. The comparison is made
// between the ledger timestamp at the time of contribution and the circle's
// deadline_timestamp stored in CircleInfo.

#![no_std]

use soroban_sdk::{contracttype, Address, Env};

// --- CONSTANTS ---

/// Seconds before deadline that qualifies as an "early" contribution (24 h)
const EARLY_BIRD_THRESHOLD_SECS: u64 = 24 * 60 * 60;

/// Base RI points awarded for any on-time contribution
const BASE_RI_AWARD: u32 = 10;

/// Additional RI points awarded on top of BASE for early contributions
const EARLY_BIRD_BONUS_RI: u32 = 15;

/// Bonus multiplier applied to the member's current round weight (basis points).
/// 500 bps = 5% bonus on the contribution's weight in the pot ordering.
const EARLY_BIRD_WEIGHT_BONUS_BPS: u32 = 500;

// --- DATA KEYS ---

#[contracttype]
#[derive(Clone)]
pub enum EarlyBirdDataKey {
    ContributionRecord(Address, u64, u32), // (member, circle_id, round) -> EarlyBirdRecord
    CircleBonusStats(u64, u32),            // (circle_id, round) -> EarlyBirdRoundStats
}

// --- DATA STRUCTURES ---

/// Record of a single member's contribution timing within a round
#[contracttype]
#[derive(Clone)]
pub struct EarlyBirdRecord {
    pub member: Address,
    pub circle_id: u64,
    pub round: u32,
    pub contributed_at: u64,
    pub deadline: u64,
    pub seconds_early: u64,   // 0 if not early
    pub is_early_bird: bool,
    pub ri_awarded: u32,
    pub weight_bonus_bps: u32,
}

/// Aggregate stats for all early contributions in a round
#[contracttype]
#[derive(Clone)]
pub struct EarlyBirdRoundStats {
    pub circle_id: u64,
    pub round: u32,
    pub total_contributors: u32,
    pub early_bird_count: u32,
    pub total_bonus_ri_distributed: u32,
}

// --- FUNCTIONS ---

/// Record a member's contribution and apply the early-bird bonus if eligible.
///
/// `deadline_timestamp` comes from `CircleInfo.deadline_timestamp`.
/// Returns an EarlyBirdRecord with the RI awarded and weight bonus.
pub fn record_contribution_with_bonus(
    env: &Env,
    member: Address,
    circle_id: u64,
    round: u32,
    deadline_timestamp: u64,
) -> EarlyBirdRecord {
    let now = env.ledger().timestamp();

    assert!(now <= deadline_timestamp, "Contribution is past the deadline");

    let time_remaining = deadline_timestamp - now;
    let is_early_bird = time_remaining >= EARLY_BIRD_THRESHOLD_SECS;
    let seconds_early = if is_early_bird { time_remaining } else { 0 };

    let ri_awarded = BASE_RI_AWARD + if is_early_bird { EARLY_BIRD_BONUS_RI } else { 0 };
    let weight_bonus_bps = if is_early_bird { EARLY_BIRD_WEIGHT_BONUS_BPS } else { 0 };

    let record = EarlyBirdRecord {
        member: member.clone(),
        circle_id,
        round,
        contributed_at: now,
        deadline: deadline_timestamp,
        seconds_early,
        is_early_bird,
        ri_awarded,
        weight_bonus_bps,
    };

    env.storage().instance().set(
        &EarlyBirdDataKey::ContributionRecord(member, circle_id, round),
        &record,
    );

    // Update round aggregate stats
    let stats_key = EarlyBirdDataKey::CircleBonusStats(circle_id, round);
    let mut stats: EarlyBirdRoundStats = env
        .storage()
        .instance()
        .get(&stats_key)
        .unwrap_or(EarlyBirdRoundStats {
            circle_id,
            round,
            total_contributors: 0,
            early_bird_count: 0,
            total_bonus_ri_distributed: 0,
        });

    stats.total_contributors += 1;
    if is_early_bird {
        stats.early_bird_count += 1;
        stats.total_bonus_ri_distributed += EARLY_BIRD_BONUS_RI;
    }
    env.storage().instance().set(&stats_key, &stats);

    record
}

/// Calculate what RI bonus a member would receive if they contributed right now,
/// without persisting anything. Useful for front-end estimation.
pub fn estimate_bonus(
    env: &Env,
    deadline_timestamp: u64,
) -> (u32, u32) {
    let now = env.ledger().timestamp();
    if now >= deadline_timestamp {
        return (BASE_RI_AWARD, 0);
    }
    let time_remaining = deadline_timestamp - now;
    if time_remaining >= EARLY_BIRD_THRESHOLD_SECS {
        (BASE_RI_AWARD + EARLY_BIRD_BONUS_RI, EARLY_BIRD_WEIGHT_BONUS_BPS)
    } else {
        (BASE_RI_AWARD, 0)
    }
}

/// Retrieve the contribution record for a member in a specific round.
pub fn get_contribution_record(
    env: &Env,
    member: Address,
    circle_id: u64,
    round: u32,
) -> Option<EarlyBirdRecord> {
    let key = EarlyBirdDataKey::ContributionRecord(member, circle_id, round);
    env.storage().instance().get(&key)
}

/// Retrieve aggregate early-bird stats for a round.
pub fn get_round_stats(
    env: &Env,
    circle_id: u64,
    round: u32,
) -> Option<EarlyBirdRoundStats> {
    let key = EarlyBirdDataKey::CircleBonusStats(circle_id, round);
    env.storage().instance().get(&key)
}
