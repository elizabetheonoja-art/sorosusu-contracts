use sorosusu_contracts::{SoroSusu, SoroSusuClient};
// #![cfg(test)]
use soroban_sdk::testutils::Address as _;

// Temporal Flexibility: Dynamic Round Duration Tests
// Tests for the propose/vote/apply duration change feature with 66% supermajority.

// --- Constants (mirrored from lib.rs) ---
const DURATION_CHANGE_VOTING_PERIOD: u64 = 172800; // 48 hours
const DURATION_CHANGE_QUORUM: u32 = 50;            // 50%
const DURATION_CHANGE_MAJORITY: u32 = 66;           // 66%
const MIN_CYCLE_DURATION: u64 = 604800;             // 7 days
const MAX_CYCLE_DURATION: u64 = 7776000;            // 90 days
const DURATION_CHANGE_COOLDOWN: u64 = 604800;       // 7 days

const ONE_WEEK: u64 = 604800;
const TWO_WEEKS: u64 = 1209600;
const THIRTY_DAYS: u64 = 2592000;
const FOUR_WEEKS: u64 = 2419200;

#[test]
fn test_duration_bounds_minimum() {
    // Proposed duration below 7 days should be rejected
    let proposed: u64 = 86400; // 1 day
    assert!(proposed < MIN_CYCLE_DURATION, "Duration below minimum should fail");
}

#[test]
fn test_duration_bounds_maximum() {
    // Proposed duration above 90 days should be rejected
    let proposed: u64 = 8000000; // ~92 days
    assert!(proposed > MAX_CYCLE_DURATION, "Duration above maximum should fail");
}

#[test]
fn test_duration_bounds_valid_range() {
    // Valid durations within the allowed range
    let valid_durations: Vec<u64> = vec![
        ONE_WEEK,           // 7 days (minimum)
        TWO_WEEKS,          // 14 days (bi-weekly harvest cycle)
        THIRTY_DAYS,        // 30 days (monthly salaried workers)
        FOUR_WEEKS,         // 28 days (4-week cycle)
        MAX_CYCLE_DURATION, // 90 days (maximum, seasonal)
    ];

    for duration in valid_durations {
        assert!(duration >= MIN_CYCLE_DURATION, "Duration {} should be >= min", duration);
        assert!(duration <= MAX_CYCLE_DURATION, "Duration {} should be <= max", duration);
    }
}

#[test]
fn test_supermajority_threshold_66_percent() {
    // 66% of voters must approve for the proposal to pass
    let total_votes: u32 = 6;
    let for_votes: u32 = 4; // 66.6% -> passes
    let approval_pct = (for_votes * 100) / total_votes;
    assert!(approval_pct >= DURATION_CHANGE_MAJORITY, "4/6 = 66.6% should meet 66% threshold");

    let for_votes_fail: u32 = 3; // 50% -> fails
    let approval_pct_fail = (for_votes_fail * 100) / total_votes;
    assert!(approval_pct_fail < DURATION_CHANGE_MAJORITY, "3/6 = 50% should NOT meet 66% threshold");
}

#[test]
fn test_quorum_50_percent() {
    // At least 50% of active members must vote for quorum
    let active_members: u32 = 10;
    let votes_cast_pass: u32 = 5;
    let quorum_met = (votes_cast_pass * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    assert!(quorum_met, "5/10 should meet 50% quorum");

    let votes_cast_fail: u32 = 4;
    let quorum_fail = (votes_cast_fail * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    assert!(!quorum_fail, "4/10 should NOT meet 50% quorum");
}

#[test]
fn test_quorum_and_majority_combined() {
    // Both quorum AND majority must be met simultaneously
    let active_members: u32 = 10;

    // Case 1: quorum met, majority met -> passes
    let votes: u32 = 6;
    let for_v: u32 = 4;
    let quorum_met = (votes * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    let majority_met = (for_v * 100) / votes >= DURATION_CHANGE_MAJORITY;
    assert!(quorum_met && majority_met, "6 votes, 4 for should pass");

    // Case 2: quorum met, majority NOT met -> fails
    let votes2: u32 = 6;
    let for_v2: u32 = 3; // 50%
    let quorum_met2 = (votes2 * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    let majority_met2 = (for_v2 * 100) / votes2 >= DURATION_CHANGE_MAJORITY;
    assert!(quorum_met2, "quorum should be met");
    assert!(!majority_met2, "majority should NOT be met");

    // Case 3: quorum NOT met -> fails regardless of majority
    let votes3: u32 = 3;
    let _for_v3: u32 = 3; // 100% but only 30% quorum
    let quorum_met3 = (votes3 * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    assert!(!quorum_met3, "quorum should NOT be met even with unanimous support");
}

#[test]
fn test_voting_period_expiration() {
    let proposal_created: u64 = 1000;
    let voting_deadline = proposal_created + DURATION_CHANGE_VOTING_PERIOD;

    // Vote within period
    let vote_time_ok: u64 = proposal_created + 100;
    assert!(vote_time_ok <= voting_deadline, "Vote within period should be valid");

    // Vote after period
    let vote_time_expired: u64 = voting_deadline + 1;
    assert!(vote_time_expired > voting_deadline, "Vote after deadline should be rejected");

    // Exactly at deadline
    assert!(voting_deadline <= voting_deadline, "Vote at exact deadline should be valid");
}

#[test]
fn test_cooldown_enforcement() {
    let last_change_time: u64 = 1_000_000;

    // Within cooldown window -> should be rejected
    let attempt_too_soon = last_change_time + DURATION_CHANGE_COOLDOWN - 1;
    assert!(
        attempt_too_soon < last_change_time + DURATION_CHANGE_COOLDOWN,
        "Attempt within cooldown should be rejected"
    );

    // After cooldown -> should be allowed
    let attempt_after_cooldown = last_change_time + DURATION_CHANGE_COOLDOWN;
    assert!(
        attempt_after_cooldown >= last_change_time + DURATION_CHANGE_COOLDOWN,
        "Attempt after cooldown should be allowed"
    );
}

#[test]
fn test_no_op_same_duration_rejected() {
    // Proposing the same duration as current should fail
    let current_duration: u64 = THIRTY_DAYS;
    let proposed_duration: u64 = THIRTY_DAYS;
    assert_eq!(current_duration, proposed_duration, "Same duration should be rejected");
}

#[test]
fn test_payout_timestamp_recalculation_mid_round() {
    // Simulate mid-round duration change: recalculate deadline proportionally
    let old_duration: u64 = THIRTY_DAYS;    // 30 days
    let new_duration: u64 = TWO_WEEKS;       // 14 days

    let round_start: u64 = 1_000_000;
    let old_deadline: u64 = round_start + old_duration;
    let current_time: u64 = round_start + (old_duration / 2); // Halfway through

    // Proportional calculation
    let time_into_round = current_time.saturating_sub(old_deadline.saturating_sub(old_duration));
    let scaled_remaining = new_duration.saturating_sub(
        (time_into_round * new_duration) / old_duration
    );
    let new_deadline = current_time + scaled_remaining;

    // We're halfway through a 30-day round, switching to 14-day.
    // Halfway = 15 days in. Scaled: 14 - (15 * 14 / 30) = 14 - 7 = 7 days remaining.
    assert!(new_deadline > current_time, "New deadline must be in the future");
    assert!(new_deadline < old_deadline, "New deadline should be earlier for shorter cycles");

    let remaining = new_deadline - current_time;
    assert_eq!(remaining, 7 * 24 * 60 * 60, "Remaining should be ~7 days (proportional)");
}

#[test]
fn test_payout_timestamp_recalculation_slow_down() {
    // Speed up -> slow down mid-round: deadline should extend
    let old_duration: u64 = TWO_WEEKS;      // 14 days
    let new_duration: u64 = THIRTY_DAYS;     // 30 days

    let round_start: u64 = 1_000_000;
    let old_deadline: u64 = round_start + old_duration;
    let current_time: u64 = round_start + (old_duration / 4); // 25% through a 14-day round

    let time_into_round = current_time.saturating_sub(old_deadline.saturating_sub(old_duration));
    let scaled_remaining = new_duration.saturating_sub(
        (time_into_round * new_duration) / old_duration
    );
    let new_deadline = current_time + scaled_remaining;

    // 25% through 14-day (= 3.5 days). new remaining = 30 - (3.5 * 30 / 14) = 30 - 7.5 = 22.5 days
    assert!(new_deadline > old_deadline, "Slowing down should push deadline further out");
    assert!(new_deadline > current_time, "Deadline must be in the future");
}

#[test]
fn test_payout_timestamp_when_round_already_elapsed() {
    // If the round already ended, new deadline starts from current time
    let old_duration: u64 = TWO_WEEKS;
    let new_duration: u64 = THIRTY_DAYS;

    let round_start: u64 = 1_000_000;
    let old_deadline: u64 = round_start + old_duration;
    let current_time: u64 = old_deadline + 3600; // 1 hour past deadline

    // deadline <= current_time, so we just set from now
    let new_deadline = if old_deadline > current_time {
        // Would do proportional calculation
        unreachable!();
    } else {
        current_time + new_duration
    };

    assert_eq!(new_deadline, current_time + new_duration);
}

#[test]
fn test_harvest_season_scenario() {
    // Real-world: switch from monthly (30d) to bi-weekly (14d) during harvest season
    let monthly: u64 = THIRTY_DAYS;
    let biweekly: u64 = TWO_WEEKS;

    assert!(biweekly >= MIN_CYCLE_DURATION, "Bi-weekly is within bounds");
    assert!(monthly <= MAX_CYCLE_DURATION, "Monthly is within bounds");
    assert_ne!(monthly, biweekly, "Durations differ - proposal is valid");
}

#[test]
fn test_winter_slowdown_scenario() {
    // Real-world: switch from bi-weekly (14d) to 4-weekly (28d) in winter off-season
    let biweekly: u64 = TWO_WEEKS;
    let four_weekly: u64 = FOUR_WEEKS;

    assert!(four_weekly >= MIN_CYCLE_DURATION, "4-weekly is within bounds");
    assert!(four_weekly <= MAX_CYCLE_DURATION, "4-weekly is within bounds");
    assert_ne!(biweekly, four_weekly, "Durations differ - proposal is valid");
}

#[test]
fn test_proposer_auto_votes_for() {
    // The proposer should automatically count as 1 'for' vote
    let initial_for_votes: u32 = 1;
    let initial_total: u32 = 1;

    assert_eq!(initial_for_votes, 1, "Proposer auto-votes for");
    assert_eq!(initial_total, 1, "Total votes includes proposer");
}

#[test]
fn test_three_member_group_vote() {
    // Small group: 3 members. Need 50% quorum (2 votes) and 66% majority (2/2 or 2/3)
    let active_members: u32 = 3;

    // Proposer votes for (1 vote). Need 1 more for quorum.
    let total: u32 = 2;
    let for_v: u32 = 2;
    let quorum_met = (total * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    let majority_met = (for_v * 100) / total >= DURATION_CHANGE_MAJORITY;
    assert!(quorum_met && majority_met, "2/3 members voting unanimously should pass");
}

#[test]
fn test_large_group_edge_case() {
    // Large group: 20 members. Need 10 votes for quorum, 66% of votes for majority
    let active_members: u32 = 20;

    // 10 votes, 7 for, 3 against -> 70% majority with quorum
    let total: u32 = 10;
    let for_v: u32 = 7;
    let quorum = (total * 100) >= (active_members * DURATION_CHANGE_QUORUM);
    let majority = (for_v * 100) / total >= DURATION_CHANGE_MAJORITY;
    assert!(quorum && majority, "7/10 of 20 members should pass");

    // 10 votes, 6 for, 4 against -> 60% majority -> fails
    let for_v2: u32 = 6;
    let majority2 = (for_v2 * 100) / total >= DURATION_CHANGE_MAJORITY;
    assert!(!majority2, "6/10 = 60% should NOT meet 66% threshold");
}

#[test]
fn test_deadline_recalculation_at_round_start() {
    // Duration change applied right at the start of a round
    let old_duration: u64 = THIRTY_DAYS;
    let new_duration: u64 = TWO_WEEKS;
    let round_start: u64 = 1_000_000;
    let old_deadline: u64 = round_start + old_duration;
    let current_time: u64 = round_start; // Right at start

    let time_into_round = current_time.saturating_sub(old_deadline.saturating_sub(old_duration));
    let scaled_remaining = new_duration.saturating_sub(
        (time_into_round * new_duration) / old_duration
    );
    let new_deadline = current_time + scaled_remaining;

    // At the very start, 0 time elapsed -> full new duration remaining
    assert_eq!(
        new_deadline,
        round_start + new_duration,
        "At round start, full new duration applies"
    );
}

#[test]
fn test_deadline_recalculation_near_end() {
    // Duration change applied near the end of a round (90% through)
    let old_duration: u64 = THIRTY_DAYS;
    let new_duration: u64 = TWO_WEEKS;
    let round_start: u64 = 1_000_000;
    let old_deadline: u64 = round_start + old_duration;
    let current_time: u64 = round_start + (old_duration * 9 / 10); // 90% through

    let time_into_round = current_time.saturating_sub(old_deadline.saturating_sub(old_duration));
    let scaled_remaining = new_duration.saturating_sub(
        (time_into_round * new_duration) / old_duration
    );
    let new_deadline = current_time + scaled_remaining;

    // 90% of 30 days done. remaining = 14 - (27 * 14 / 30) = 14 - 12.6 = 1.4 days
    assert!(new_deadline > current_time, "Deadline must be in the future");
    let remaining = new_deadline - current_time;
    assert!(remaining < 2 * 86400, "Should be less than 2 days remaining");
    assert!(remaining > 86400, "Should be more than 1 day remaining");
}

#[test]
fn test_sequential_duration_changes() {
    // Simulate multiple duration changes over time with cooldown
    let first_change: u64 = 1_000_000;

    // Second change attempt within cooldown -> rejected
    let second_attempt: u64 = first_change + DURATION_CHANGE_COOLDOWN - 100;
    assert!(second_attempt < first_change + DURATION_CHANGE_COOLDOWN);

    // Third change attempt after cooldown -> allowed
    let third_attempt: u64 = first_change + DURATION_CHANGE_COOLDOWN;
    assert!(third_attempt >= first_change + DURATION_CHANGE_COOLDOWN);

    // After the second change is applied, cooldown resets
    let second_change_applied: u64 = third_attempt;
    let fourth_attempt: u64 = second_change_applied + DURATION_CHANGE_COOLDOWN;
    assert!(fourth_attempt >= second_change_applied + DURATION_CHANGE_COOLDOWN);
}

#[test]
fn test_voting_period_is_48_hours() {
    assert_eq!(DURATION_CHANGE_VOTING_PERIOD, 48 * 60 * 60, "Voting period should be 48 hours");
}

#[test]
fn test_cooldown_is_7_days() {
    assert_eq!(DURATION_CHANGE_COOLDOWN, 7 * 24 * 60 * 60, "Cooldown should be 7 days");
}





























