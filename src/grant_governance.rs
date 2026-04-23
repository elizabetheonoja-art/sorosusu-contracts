// Grant Settlement, Voting Snapshot, and Dynamic Reputation System
use soroban_sdk::{Address, Env, Symbol, token, panic, Vec, i128, u64, u32, Map, BytesN};
use sha2::{Sha256, Digest};
use crate::{DataKey, CircleInfo, Member};

// ============================================
// FEATURE 1: GRANT SETTLEMENT SYSTEM
// ============================================

/// Grant settlement data structure
#[derive(Clone)]
pub struct GrantSettlement {
    pub grant_id: u64,
    pub grantee: Address,
    pub total_grant_amount: i128,
    pub amount_dripped: i128,      // Amount already paid out via drip
    pub work_in_progress_pay: i128, // Calculated WIP payment
    pub treasury_return: i128,      // Amount to return to treasury
    pub settlement_timestamp: u64,
    pub cancellation_reason: String,
    pub is_malicious: bool,
}

/// Calculate exact drip amount owed up to current timestamp
pub fn calculate_drip_settlement(
    env: &Env,
    grant_id: u64,
    grantee: &Address,
    total_grant: i128,
    grant_duration_seconds: u64,
    start_timestamp: u64,
    current_timestamp: u64,
) -> Result<GrantSettlement, ()> {
    // Calculate elapsed time
    let elapsed_time = if current_timestamp > start_timestamp {
        current_timestamp - start_timestamp
    } else {
        0
    };
    
    // Calculate proportional amount earned
    // drip_amount = (elapsed_time / grant_duration) * total_grant
    let drip_amount = if grant_duration_seconds > 0 {
        (total_grant * elapsed_time as i128) / grant_duration_seconds as i128
    } else {
        total_grant
    };
    
    // Cap at total grant amount
    let capped_drip = drip_amount.min(total_grant);
    
    // Work-in-progress pay is the dripped amount
    let wip_pay = capped_drip;
    
    // Treasury return is remaining amount
    let treasury_return = total_grant - capped_drip;
    
    let settlement = GrantSettlement {
        grant_id,
        grantee: grantee.clone(),
        total_grant_amount: total_grant,
        amount_dripped: capped_drip,
        work_in_progress_pay: wip_pay,
        treasury_return,
        settlement_timestamp: current_timestamp,
        cancellation_reason: String::from_str(env, "Project pivoted"),
        is_malicious: false,
    };
    
    Ok(settlement)
}

/// Terminate grant amicably with settlement
pub fn terminate_grant_amicably(
    env: &Env,
    admin: &Address,
    grant_id: u64,
    grantee: &Address,
    total_grant: i128,
    grant_duration: u64,
    start_timestamp: u64,
    treasury_address: &Address,
    token: &Address,
) -> Result<GrantSettlement, ()> {
    // Verify admin authorization
    let stored_admin: Address = env.storage().instance()
        .get(&DataKey::Admin)
        .ok_or(())?;
    
    if *admin != stored_admin {
        panic!("Unauthorized: Only admin can terminate grants");
    }
    
    let current_timestamp = env.ledger().timestamp();
    
    // Calculate settlement
    let settlement = calculate_drip_settlement(
        env,
        grant_id,
        grantee,
        total_grant,
        grant_duration,
        start_timestamp,
        current_timestamp,
    )?;
    
    // Execute payments
    let token_client = token::Client::new(env, token);
    
    // Pay WIP amount to grantee
    if settlement.work_in_progress_pay > 0 {
        token_client.transfer(
            &env.current_contract_address(),
            grantee,
            &settlement.work_in_progress_pay,
        );
    }
    
    // Return remainder to treasury
    if settlement.treasury_return > 0 {
        token_client.transfer(
            &env.current_contract_address(),
            treasury_address,
            &settlement.treasury_return,
        );
    }
    
    // Store settlement record
    env.storage().instance().set(
        &DataKey::GrantSettlement(grant_id),
        &settlement,
    );
    
    // Emit event
    env.events().publish(
        (Symbol::new(env, "GRANT_TERMINATED_AMICABLY"), grant_id),
        (
            grantee,
            settlement.work_in_progress_pay,
            settlement.treasury_return,
            current_timestamp,
        ),
    );
    
    Ok(settlement)
}

// ============================================
// FEATURE 2: VOTING SNAPSHOT FOR AUDITS
// ============================================

/// Voting snapshot for audit purposes
#[derive(Clone)]
pub struct VotingSnapshot {
    pub proposal_id: u64,
    pub total_votes: u32,
    pub for_votes: u32,
    pub against_votes: u32,
    pub abstain_votes: u32,
    pub quorum_required: u32,
    pub quorum_met: bool,
    pub result: Symbol,
    pub vote_hash: BytesN<32>, // SHA-256 hash of voting ledger
    pub snapshot_timestamp: u64,
}

/// Create compressed hash of voting ledger
pub fn create_voting_snapshot(
    env: &Env,
    proposal_id: u64,
    votes: &Vec<(Address, u32, Symbol)>, // (voter, weight, choice)
    quorum_required: u32,
) -> VotingSnapshot {
    let current_timestamp = env.ledger().timestamp();
    
    // Tally votes
    let mut for_votes: u32 = 0;
    let mut against_votes: u32 = 0;
    let mut abstain_votes: u32 = 0;
    
    for vote in votes.iter() {
        match vote.2 {
            s if s == Symbol::new(env, "For") => {
                for_votes += vote.1;
            }
            s if s == Symbol::new(env, "Against") => {
                against_votes += vote.1;
            }
            _ => {
                abstain_votes += vote.1;
            }
        }
    }
    
    let total_votes = for_votes + against_votes + abstain_votes;
    let quorum_met = total_votes >= quorum_required;
    
    // Determine result
    let result = if !quorum_met {
        Symbol::new(env, "QUORUM_NOT_MET")
    } else if for_votes > against_votes {
        Symbol::new(env, "APPROVED")
    } else if against_votes > for_votes {
        Symbol::new(env, "REJECTED")
    } else {
        Symbol::new(env, "TIED")
    };
    
    // Create hash of voting ledger for verification
    let mut hasher = Sha256::new();
    hasher.update(proposal_id.to_be_bytes());
    hasher.update(total_votes.to_be_bytes());
    hasher.update(for_votes.to_be_bytes());
    hasher.update(against_votes.to_be_bytes());
    hasher.update(abstain_votes.to_be_bytes());
    hasher.update(quorum_required.to_be_bytes());
    hasher.update(current_timestamp.to_be_bytes());
    
    // Add individual vote hashes
    for vote in votes.iter() {
        hasher.update(vote.0.to_string().as_bytes());
        hasher.update(vote.1.to_be_bytes());
        hasher.update(vote.2.to_string().as_bytes());
    }
    
    let hash_result = hasher.finalize();
    let vote_hash = BytesN::<32>::from_slice(env, &hash_result);
    
    VotingSnapshot {
        proposal_id,
        total_votes,
        for_votes,
        against_votes,
        abstain_votes,
        quorum_required,
        quorum_met,
        result,
        vote_hash,
        snapshot_timestamp: current_timestamp,
    }
}

/// Get voting snapshot for audit verification
pub fn get_voting_snapshot_for_audit(
    env: &Env,
    proposal_id: u64,
) -> Option<VotingSnapshot> {
    env.storage().instance().get(&DataKey::VotingSnapshot(proposal_id))
}

// ============================================
// FEATURE 3: DYNAMIC REPUTATION VIA NFT METADATA
// ============================================

/// Milestone progress status
#[derive(Clone, Debug, PartialEq)]
pub enum MilestoneProgress {
    NotStarted,
    InProgress,
    Phase1Complete,
    Phase2Complete,
    Phase3Complete,
    Completed,
    Cancelled,
}

/// Impact certificate metadata
#[derive(Clone)]
pub struct ImpactCertificateMetadata {
    pub certificate_id: u128,
    pub holder: Address,
    pub milestone_status: MilestoneProgress,
    pub phases_completed: u32,
    pub total_phases: u32,
    pub impact_score: u32, // 0-10000 bps
    pub last_updated: u64,
    pub metadata_uri: String,
    pub on_chain_badge: Symbol,
}

/// Update NFT metadata when milestone is verified
pub fn update_impact_certificate_metadata(
    env: &Env,
    certificate_id: u128,
    new_phase: u32,
    total_phases: u32,
    impact_score_delta: i32,
) -> Result<ImpactCertificateMetadata, ()> {
    // Get existing certificate
    let mut cert: ImpactCertificateMetadata = env.storage().instance()
        .get(&DataKey::ImpactCertificate(certificate_id))
        .ok_or(())?;
    
    let current_timestamp = env.ledger().timestamp();
    
    // Update phase progress
    cert.phases_completed = new_phase;
    
    // Determine milestone status based on phases completed
    cert.milestone_status = if new_phase == 0 {
        MilestoneProgress::NotStarted
    } else if new_phase >= total_phases {
        MilestoneProgress::Completed
    } else {
        MilestoneProgress::InProgress
    };
    
    // Update impact score (ensure it stays in 0-10000 range)
    let new_score = if impact_score_delta > 0 {
        (cert.impact_score as i32 + impact_score_delta).min(10000) as u32
    } else {
        (cert.impact_score as i32 + impact_score_delta).max(0) as u32
    };
    cert.impact_score = new_score;
    
    // Update on-chain badge symbol
    cert.on_chain_badge = match cert.milestone_status {
        MilestoneProgress::Phase1Complete => Symbol::new(env, "PHASE1_HERO"),
        MilestoneProgress::Phase2Complete => Symbol::new(env, "PHASE2_CHAMPION"),
        MilestoneProgress::Phase3Complete => Symbol::new(env, "PHASE3_LEGEND"),
        MilestoneProgress::Completed => Symbol::new(env, "IMPACT_MASTER"),
        _ => Symbol::new(env, "IN_PROGRESS"),
    };
    
    // Update metadata URI to reflect new status
    cert.metadata_uri = format!(
        "https://metadata.sorosusu.com/impact/{}/phase{}",
        certificate_id,
        new_phase
    );
    
    cert.last_updated = current_timestamp;
    
    // Store updated certificate
    env.storage().instance().set(
        &DataKey::ImpactCertificate(certificate_id),
        &cert,
    );
    
    // Emit event for explorers and indexers
    env.events().publish(
        (Symbol::new(env, "IMPACT_CERTIFICATE_UPDATED"), certificate_id),
        (
            cert.holder.clone(),
            cert.phases_completed,
            cert.total_phases,
            cert.impact_score,
            cert.on_chain_badge.clone(),
            cert.metadata_uri.clone(),
        ),
    );
    
    Ok(cert)
}

/// Initialize impact certificate for new grant recipient
pub fn initialize_impact_certificate(
    env: &Env,
    holder: &Address,
    certificate_id: u128,
    total_phases: u32,
    initial_metadata_uri: &str,
) -> ImpactCertificateMetadata {
    let current_timestamp = env.ledger().timestamp();
    
    let cert = ImpactCertificateMetadata {
        certificate_id,
        holder: holder.clone(),
        milestone_status: MilestoneProgress::NotStarted,
        phases_completed: 0,
        total_phases,
        impact_score: 5000, // Start at 50% (neutral)
        last_updated: current_timestamp,
        metadata_uri: String::from_str(env, initial_metadata_uri),
        on_chain_badge: Symbol::new(env, "NEWCOMER"),
    };
    
    env.storage().instance().set(
        &DataKey::ImpactCertificate(certificate_id),
        &cert,
    );
    
    env.events().publish(
        (Symbol::new(env, "IMPACT_CERTIFICATE_CREATED"), certificate_id),
        (holder.clone(), total_phases, cert.metadata_uri.clone()),
    );
    
    cert
}

/// Get visual progress bar data for ecosystem dashboard
pub fn get_progress_bar_data(
    env: &Env,
    certificate_id: u128,
) -> Option<Map<Symbol, Symbol>> {
    let cert: ImpactCertificateMetadata = env.storage().instance()
        .get(&DataKey::ImpactCertificate(certificate_id))?;
    
    let mut progress_map = Map::new(env);
    
    // Progress percentage
    let progress_percent = if cert.total_phases > 0 {
        (cert.phases_completed * 100) / cert.total_phases
    } else {
        0
    };
    
    progress_map.set(
        Symbol::new(env, "progress"),
        Symbol::new(env, &format!("{}", progress_percent)),
    );
    
    // Status badge
    progress_map.set(
        Symbol::new(env, "badge"),
        cert.on_chain_badge,
    );
    
    // Impact tier
    let tier = if cert.impact_score >= 9000 {
        Symbol::new(env, "PLATINUM")
    } else if cert.impact_score >= 7000 {
        Symbol::new(env, "GOLD")
    } else if cert.impact_score >= 5000 {
        Symbol::new(env, "SILVER")
    } else {
        Symbol::new(env, "BRONZE")
    };
    progress_map.set(Symbol::new(env, "tier"), tier);
    
    Some(progress_map)
}
