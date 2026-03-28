// --- AUTO-ROLLOVER GOVERNOR VOTE THRESHOLD MODULE ---
//
// This module implements the "Continuity Vote" feature for ROSCA circles.
// A vote is triggered 2 rounds before the circle ends. If 51% agree, the 
// contract prepares a new instance with the same members.
//
// Key Features:
// - Automatic trigger 2 rounds before end
// - 51% approval threshold (simple majority)
// - Low-friction transition for keeping TVL stable
// - Maintains same members and circle parameters

#![no_std]

use soroban_sdk::{contracttype, Address, Env, Vec};

// --- CONSTANTS ---

/// Minimum approval threshold for rollover (51% = 5100 basis points)
const ROLLOVER_APPROVAL_THRESHOLD_BPS: u32 = 5100;

/// Number of rounds before end to trigger continuity vote
const ROLLOVER_TRIGGER_ROUNDS_BEFORE_END: u32 = 2;

/// Data key for rollover state per circle
#[contracttype]
#[derive(Clone)]
pub enum RolloverDataKey {
    RolloverVote(u64),           // RolloverVote for a circle
    RolloverVotes(u64),         // Vec of individual vote records
    RolloverPrepared(u64),      // Whether new instance has been prepared
    RolloverVotesCount(u64),    // Count of yes/no votes
}

// --- DATA STRUCTURES ---

/// Represents a continuity vote for circle rollover
#[contracttype]
#[derive(Clone)]
pub struct RolloverVote {
    pub circle_id: u64,
    pub triggered_at_round: u32,        // Round when vote was triggered
    pub end_round: u32,                // Original circle end round
    pub vote_start_time: u64,          // Ledger timestamp when voting started
    pub vote_deadline: u64,            // Ledger timestamp when voting ends
    pub yes_votes: u32,                // Number of yes votes
    pub no_votes: u32,                 // Number of no votes
    pub has_voted: Vec<Address>,       // Members who have voted
    pub is_active: bool,               // Is vote still open
    pub is_passed: Option<bool>,       // None = pending, Some(true) = approved, Some(false) = rejected
    pub total_eligible_voters: u32,    // Total members who can vote
}

/// Individual vote record
#[contracttype]
#[derive(Clone)]
pub struct VoteRecord {
    pub voter: Address,
    pub vote: RolloverVoteChoice,
    pub timestamp: u64,
}

/// Vote choice enumeration
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum RolloverVoteChoice {
    Yes,
    No,
    Abstain,
}

/// Rollover preparation state
#[contracttype]
#[derive(Clone)]
pub struct RolloverPreparation {
    pub new_circle_id: Option<u64>,     // ID of newly prepared circle
    pub original_circle_id: u64,        // ID of circle being rolled over
    pub new_contribution_amount: u64,   // Same as original
    pub new_max_members: u32,            // Same as original
    pub new_token: Address,             // Same as original
    pub new_cycle_duration: u64,        // Same as original
    pub prepared_at: u64,               // Timestamp of preparation
    pub is_complete: bool,              // Whether rollover is complete
    pub members_joined: Vec<Address>,  // Members who confirmed rollover participation
}

/// Default implementation for RolloverVote
impl Default for RolloverVote {
    fn default() -> Self {
        Self {
            circle_id: 0,
            triggered_at_round: 0,
            end_round: 0,
            vote_start_time: 0,
            vote_deadline: 0,
            yes_votes: 0,
            no_votes: 0,
            has_voted: Vec::new(&Env::default()),
            is_active: false,
            is_passed: None,
            total_eligible_voters: 0,
        }
    }
}

/// Default implementation for RolloverPreparation
impl Default for RolloverPreparation {
    fn default() -> Self {
        let env = Env::default();
        Self {
            new_circle_id: None,
            original_circle_id: 0,
            new_contribution_amount: 0,
            new_max_members: 0,
            new_token: Address::from_account_id(&Env::default().hash([0u8; 32])),
            new_cycle_duration: 0,
            prepared_at: 0,
            is_complete: false,
            members_joined: Vec::new(&env),
        }
    }
}

// --- CORE FUNCTIONS ---

/// Check if rollover vote should be triggered
/// Returns true if we are 2 rounds before the end
pub fn should_trigger_rollover_vote(
    current_round: u32,
    total_rounds: u32,
) -> bool {
    // Trigger if we are ROLLOVER_TRIGGER_ROUNDS_BEFORE_END rounds from the end
    let rounds_remaining = total_rounds.saturating_sub(current_round);
    rounds_remaining == ROLLOVER_TRIGGER_ROUNDS_BEFORE_END
}

/// Trigger a continuity vote for the circle
pub fn trigger_continuity_vote(
    env: &Env,
    circle_id: u64,
    current_round: u32,
    total_rounds: u32,
    member_count: u32,
) -> RolloverVote {
    let current_time = env.ledger().timestamp();
    
    // Calculate vote deadline (e.g., 1 day to vote)
    let vote_duration_seconds: u64 = 86400; // 24 hours
    let vote_deadline = current_time + vote_duration_seconds;
    
    let vote = RolloverVote {
        circle_id,
        triggered_at_round: current_round,
        end_round: total_rounds,
        vote_start_time: current_time,
        vote_deadline,
        yes_votes: 0,
        no_votes: 0,
        has_voted: Vec::new(env),
        is_active: true,
        is_passed: None,
        total_eligible_voters: member_count,
    };
    
    // Store the vote
    let key = RolloverDataKey::RolloverVote(circle_id);
    env.storage().instance().set(&key, &vote);
    
    vote
}

/// Cast a vote for continuity
pub fn cast_rollover_vote(
    env: &Env,
    circle_id: u64,
    voter: Address,
    choice: RolloverVoteChoice,
) -> Result<(), RolloverError> {
    // Require voter authorization
    voter.require_auth();
    
    // Get the vote
    let key = RolloverDataKey::RolloverVote(circle_id);
    let mut vote: RolloverVote = env.storage().instance()
        .get(&key)
        .ok_or(RolloverError::NoActiveVote)?;
    
    // Check if vote is still active
    if !vote.is_active {
        return Err(RolloverError::VoteClosed);
    }
    
    // Check if vote has ended
    let current_time = env.ledger().timestamp();
    if current_time > vote.vote_deadline {
        vote.is_active = false;
        env.storage().instance().set(&key, &vote);
        return Err(RolloverError::VoteExpired);
    }
    
    // Check if voter has already voted
    for v in vote.has_voted.iter() {
        if v == voter {
            return Err(RolloverError::AlreadyVoted);
        }
    }
    
    // Record the vote
    let current_time = env.ledger().timestamp();
    match choice {
        RolloverVoteChoice::Yes => vote.yes_votes += 1,
        RolloverVoteChoice::No => vote.no_votes += 1,
        RolloverVoteChoice::Abstain => {} // Abstain doesn't count
    }
    
    vote.has_voted.push_back(voter.clone());
    
    // Save updated vote
    env.storage().instance().set(&key, &vote);
    
    // Store individual vote record
    let vote_record = VoteRecord {
        voter,
        vote: choice,
        timestamp: current_time,
    };
    
    // Check if vote should be tallied (all eligible voters have voted or time expired)
    let total_votes = vote.yes_votes + vote.no_votes;
    if total_votes >= vote.total_eligible_voters || current_time > vote.vote_deadline {
        finalize_vote(env, circle_id)?;
    }
    
    Ok(())
}

/// Finalize the vote and determine outcome
pub fn finalize_vote(env: &Env, circle_id: u64) -> Result<bool, RolloverError> {
    let key = RolloverDataKey::RolloverVote(circle_id);
    let mut vote: RolloverVote = env.storage().instance()
        .get(&key)
        .ok_or(RolloverError::NoActiveVote)?;
    
    if !vote.is_active {
        return Err(RolloverError::VoteClosed);
    }
    
    // Close the vote
    vote.is_active = false;
    
    // Calculate approval percentage
    let total_votes = vote.yes_votes + vote.no_votes;
    if total_votes == 0 {
        vote.is_passed = Some(false);
        env.storage().instance().set(&key, &vote);
        return Ok(false);
    }
    
    // Calculate yes percentage in basis points
    let approval_percentage = (vote.yes_votes as u64 * 10000 / total_votes as u64) as u32;
    
    // Check against threshold
    let passed = approval_percentage >= ROLLOVER_APPROVAL_THRESHOLD_BPS;
    vote.is_passed = Some(passed);
    
    env.storage().instance().set(&key, &vote);
    
    // If passed, prepare new instance
    if passed {
        prepare_rollover_instance(env, circle_id)?;
    }
    
    Ok(passed)
}

/// Get the current rollover vote status
pub fn get_rollover_vote(env: &Env, circle_id: u64) -> RolloverVote {
    let key = RolloverDataKey::RolloverVote(circle_id);
    env.storage().instance()
        .get(&key)
        .unwrap_or_default()
}

/// Check if rollover is passed and instance is prepared
pub fn is_rollover_prepared(env: &Env, circle_id: u64) -> bool {
    let key = RolloverDataKey::RolloverPrepared(circle_id);
    env.storage().instance()
        .get(&key)
        .unwrap_or(false)
}

/// Prepare the new circle instance for rollover
fn prepare_rollover_instance(env: &Env, circle_id: u64) -> Result<(), RolloverError> {
    // This would integrate with the main contract's circle creation
    // For now, mark as prepared
    let key = RolloverDataKey::RolloverPrepared(circle_id);
    env.storage().instance().set(&key, &true);
    
    Ok(())
}

/// Get rollover preparation status
pub fn get_rollover_preparation(env: &Env, circle_id: u64) -> RolloverPreparation {
    // This would fetch the actual preparation state
    RolloverPreparation::default()
}

// --- ERROR TYPES ---

#[contracttype]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum RolloverError {
    NoActiveVote = 1,
    VoteClosed = 2,
    VoteExpired = 3,
    AlreadyVoted = 4,
    PreparationFailed = 5,
    NotAuthorized = 6,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_should_trigger_rollover_vote() {
        // Should trigger when 2 rounds before end
        assert!(should_trigger_rollover_vote(8, 10));
        
        // Should NOT trigger when more than 2 rounds before end
        assert!(!should_trigger_rollover_vote(7, 10));
        assert!(!should_trigger_rollover_vote(5, 10));
        
        // Should NOT trigger when at or after end
        assert!(!should_trigger_rollover_vote(10, 10));
        assert!(!should_trigger_rollover_vote(11, 10));
    }
    
    #[test]
    fn test_rollover_vote_approval_calculation() {
        // 51% approval should pass
        let yes_votes = 51u32;
        let no_votes = 49u32;
        let total_votes = yes_votes + no_votes;
        let approval_percentage = (yes_votes as u64 * 10000 / total_votes as u64) as u32;
        
        assert!(approval_percentage >= ROLLOVER_APPROVAL_THRESHOLD_BPS);
        
        // 50% approval should fail
        let yes_votes = 50u32;
        let no_votes = 50u32;
        let total_votes = yes_votes + no_votes;
        let approval_percentage = (yes_votes as u64 * 10000 / total_votes as u64) as u32;
        
        assert!(approval_percentage < ROLLOVER_APPROVAL_THRESHOLD_BPS);
    }
    
    #[test]
    fn test_rollover_vote_default() {
        let vote = RolloverVote::default();
        assert!(!vote.is_active);
        assert!(vote.is_passed.is_none());
        assert_eq!(vote.yes_votes, 0);
        assert_eq!(vote.no_votes, 0);
    }
}