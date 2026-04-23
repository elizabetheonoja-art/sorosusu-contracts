// --- RELIABILITY ORACLE INTERFACE MODULE ---
//
// Exposes a standardized Reputation Proof that third-party protocols can query.
// read_reputation() aggregates on-chain member behaviour (contribution history,
// default record, vouching activity) into a single portable proof struct.

#![no_std]

use soroban_sdk::{contracttype, Address, Env, Symbol, Vec};

// --- CONSTANTS ---

/// Version tag stamped on every ReputationProof for consumer compatibility checks
const PROOF_VERSION: u32 = 1;

/// RI score considered "excellent" — qualifies for tier-3 perks with 3rd parties
const RI_TIER_EXCELLENT: u32 = 850;

/// RI score considered "good"
const RI_TIER_GOOD: u32 = 650;

/// RI score considered "fair" — minimum for most integrations
const RI_TIER_FAIR: u32 = 400;

// --- DATA KEYS ---

#[contracttype]
#[derive(Clone)]
pub enum OracleDataKey {
    MemberReputation(Address),   // ReputationRecord per member
    ProofNonce(Address),         // Monotonically increasing nonce per address
}

// --- DATA STRUCTURES ---

/// Tier label derived from the member's raw RI score
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReputationTier {
    Excellent,
    Good,
    Fair,
    Poor,
}

/// Raw reputation data stored per member on-chain
#[contracttype]
#[derive(Clone)]
pub struct ReputationRecord {
    pub member: Address,
    pub ri_score: u32,
    pub total_contributions: u32,
    pub on_time_contributions: u32,
    pub defaults_count: u32,
    pub vouches_given: u32,
    pub vouches_received: u32,
    pub circles_participated: u32,
    pub last_updated: u64,
}

/// Standardised portable proof returned to third-party callers.
/// Consumers must verify `proof_version` before interpreting fields.
#[contracttype]
#[derive(Clone)]
pub struct ReputationProof {
    pub proof_version: u32,
    pub member: Address,
    pub ri_score: u32,
    pub tier: ReputationTier,
    pub on_time_rate_bps: u32,    // (on_time / total) * 10_000
    pub defaults_count: u32,
    pub circles_participated: u32,
    pub vouches_given: u32,
    pub vouches_received: u32,
    pub generated_at: u64,
    pub nonce: u32,
}

// --- HELPERS ---

fn score_to_tier(score: u32) -> ReputationTier {
    if score >= RI_TIER_EXCELLENT {
        ReputationTier::Excellent
    } else if score >= RI_TIER_GOOD {
        ReputationTier::Good
    } else if score >= RI_TIER_FAIR {
        ReputationTier::Fair
    } else {
        ReputationTier::Poor
    }
}

// --- FUNCTIONS ---

/// Store or update the on-chain reputation record for a member.
/// This is called by internal contract logic after each contribution cycle.
pub fn update_reputation(
    env: &Env,
    member: Address,
    ri_score: u32,
    total_contributions: u32,
    on_time_contributions: u32,
    defaults_count: u32,
    vouches_given: u32,
    vouches_received: u32,
    circles_participated: u32,
) -> ReputationRecord {
    let record = ReputationRecord {
        member: member.clone(),
        ri_score,
        total_contributions,
        on_time_contributions,
        defaults_count,
        vouches_given,
        vouches_received,
        circles_participated,
        last_updated: env.ledger().timestamp(),
    };

    env.storage()
        .instance()
        .set(&OracleDataKey::MemberReputation(member), &record);

    record
}

/// Public oracle entrypoint: returns a standardised ReputationProof for a given address.
/// Third-party lending or governance protocols call this to assess a user's track record.
///
/// Returns None if no reputation record exists for the address.
pub fn read_reputation(env: &Env, user: Address) -> Option<ReputationProof> {
    let record: ReputationRecord = env
        .storage()
        .instance()
        .get(&OracleDataKey::MemberReputation(user.clone()))?;

    let nonce_key = OracleDataKey::ProofNonce(user.clone());
    let nonce: u32 = env.storage().instance().get(&nonce_key).unwrap_or(0);
    env.storage().instance().set(&nonce_key, &(nonce + 1));

    let on_time_rate_bps = if record.total_contributions > 0 {
        (record.on_time_contributions as u64 * 10_000 / record.total_contributions as u64) as u32
    } else {
        0
    };

    let proof = ReputationProof {
        proof_version: PROOF_VERSION,
        member: user,
        ri_score: record.ri_score,
        tier: score_to_tier(record.ri_score),
        on_time_rate_bps,
        defaults_count: record.defaults_count,
        circles_participated: record.circles_participated,
        vouches_given: record.vouches_given,
        vouches_received: record.vouches_received,
        generated_at: env.ledger().timestamp(),
        nonce,
    };

    Some(proof)
}

/// Check whether a given address meets a minimum RI threshold.
/// Convenience wrapper for integrations that only need a boolean gate.
pub fn meets_reputation_threshold(env: &Env, user: Address, min_ri: u32) -> bool {
    match read_reputation(env, user) {
        Some(proof) => proof.ri_score >= min_ri,
        None => false,
    }
}
