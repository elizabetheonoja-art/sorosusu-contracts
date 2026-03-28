// Tranche-Based Payout System - Anti "Payout-and-Run" Protection
use soroban_sdk::{Address, Env, Symbol, token, Vec};
use crate::{
    DataKey, CircleInfo, Member, TrancheSchedule, TrancheInfo, TrancheStatus, 
    MemberContributionRecord, Error,
};

// Constants imported from lib.rs
const TRANCHE_IMMEDIATE_PAYOUT_BPS: u32 = 7000; // 70% paid immediately
const TRANCHE_LOCKED_PERCENTAGE_BPS: u32 = 3000; // 30% locked in tranches
const TRANCHE_COUNT: u32 = 2; // 2 tranches over next 2 rounds
const TRANCHE_CLAIM_GRACE_PERIOD: u64 = 2592000; // 30 days grace period

/// Create a tranche schedule for a pot winner
pub fn create_tranche_schedule(
    env: &Env,
    circle: &CircleInfo,
    winner: &Address,
    total_pot: i128,
) -> TrancheSchedule {
    let current_time = env.ledger().timestamp();
    let current_round = circle.current_recipient_index;
    
    // Calculate immediate payout (70%)
    let immediate_payout = (total_pot * TRANCHE_IMMEDIATE_PAYOUT_BPS as i128) / 10000;
    
    // Calculate locked amount (30%)
    let locked_amount = (total_pot * TRANCHE_LOCKED_PERCENTAGE_BPS as i128) / 10000;
    
    // Split locked amount into tranches (equal parts over 2 rounds)
    let tranche_amount = locked_amount / TRANCHE_COUNT as i128;
    
    // Create tranche vector
    let mut tranches = Vec::new(env);
    
    for i in 0..TRANCHE_COUNT {
        let unlock_round = current_round + 1 + i;
        let unlock_timestamp = current_time + (circle.cycle_duration * (i + 1) as u64);
        
        let tranche = TrancheInfo {
            amount: tranche_amount,
            unlock_round,
            unlock_timestamp,
            status: TrancheStatus::Locked,
            created_at: current_time,
            claimed_at: None,
        };
        
        tranches.push_back(tranche);
    }
    
    let schedule = TrancheSchedule {
        winner: winner.clone(),
        circle_id: circle.id,
        total_pot,
        immediate_payout,
        tranches,
        created_at: current_time,
        is_complete: false,
    };
    
    // Store the schedule
    env.storage().instance().set(
        &DataKey::TrancheSchedule(circle.id, winner.clone()),
        &schedule,
    );
    
    schedule
}

/// Claim an unlocked tranche
pub fn claim_tranche(
    env: &Env,
    member: &Address,
    circle_id: u64,
    tranche_index: u32,
) -> Result<i128, Error> {
    // Get the tranche schedule
    let schedule_key = DataKey::TrancheSchedule(circle_id, member.clone());
    let mut schedule: TrancheSchedule = env
        .storage()
        .instance()
        .get(&schedule_key)
        .ok_or(Error::TrancheNotFound)?;
    
    // Check if tranche index is valid
    if tranche_index >= schedule.tranches.len() {
        return Err(Error::TrancheNotFound);
    }
    
    // Get the specific tranche
    let mut tranche = schedule.tranches.get(tranche_index).unwrap();
    
    // Check tranche status
    if tranche.status == TrancheStatus::Claimed {
        return Err(Error::TrancheAlreadyClaimed);
    }
    
    if tranche.status == TrancheStatus::ClawedBack {
        return Err(Error::MemberDefaulted);
    }
    
    // Check if tranche is unlocked (by round or timestamp)
    let circle: CircleInfo = env
        .storage()
        .instance()
        .get(&DataKey::Circle(circle_id))
        .ok_or(Error::CircleNotFound)?;
    
    let current_time = env.ledger().timestamp();
    let is_unlocked_by_round = circle.current_recipient_index >= tranche.unlock_round;
    let is_unlocked_by_time = current_time >= tranche.unlock_timestamp;
    
    if !is_unlocked_by_round && !is_unlocked_by_time {
        return Err(Error::TrancheNotUnlocked);
    }
    
    // Check if member has been defaulted
    let default_key = DataKey::DefaultedMember(circle_id, member.clone());
    if env.storage().instance().has(&default_key) {
        // Mark tranche as clawed back
        tranche.status = TrancheStatus::ClawedBack;
        schedule.tranches.set(tranche_index, tranche.clone());
        env.storage().instance().set(&schedule_key, &schedule);
        return Err(Error::MemberDefaulted);
    }
    
    // Check if member contributed in the previous round (eligibility check)
    let contribution_key = DataKey::MemberContributionRecord(
        circle_id,
        circle.current_recipient_index.saturating_sub(1),
        member.clone(),
    );
    
    if let Some(contribution_record) = env.storage().instance().get::<_, MemberContributionRecord>(&contribution_key) {
        if !contribution_record.contributed_on_time || contribution_record.is_defaulted {
            // Member didn't contribute - lock the tranche
            return Err(Error::MemberDefaulted);
        }
    }
    
    // Update tranche status
    tranche.status = TrancheStatus::Unlocked;
    
    // Execute the transfer
    let token_client = token::Client::new(env, &circle.token);
    token_client.transfer(
        &env.current_contract_address(),
        member,
        &tranche.amount,
    );
    
    // Mark as claimed
    tranche.status = TrancheStatus::Claimed;
    tranche.claimed_at = Some(current_time);
    schedule.tranches.set(tranche_index, tranche.clone());
    
    // Check if all tranches are complete
    let all_claimed_or_clawed = schedule.tranches.iter().all(|t: TrancheInfo| {
        t.status == TrancheStatus::Claimed || t.status == TrancheStatus::ClawedBack
    });
    
    if all_claimed_or_clawed {
        schedule.is_complete = true;
    }
    
    // Update storage
    env.storage().instance().set(&schedule_key, &schedule);
    
    // Emit event
    env.events().publish(
        (Symbol::new(env, "TRANCHE_CLAIMED"), circle_id, member.clone(), tranche_index),
        (tranche.amount, tranche.unlock_round, current_time),
    );
    
    Ok(tranche.amount)
}

/// Mark a member as defaulted
pub fn mark_member_defaulted(
    env: &Env,
    admin: &Address,
    circle_id: u64,
    member: &Address,
) -> Result<(), Error> {
    // Verify admin (in production, add proper authorization)
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Unauthorized)?;
    
    if *admin != stored_admin {
        return Err(Error::Unauthorized);
    }
    
    // Mark member as defaulted
    let default_key = DataKey::DefaultedMember(circle_id, member.clone());
    env.storage().instance().set(&default_key, &true);
    
    // Update member contribution record
    let circle: CircleInfo = env
        .storage()
        .instance()
        .get(&DataKey::Circle(circle_id))
        .ok_or(Error::CircleNotFound)?;
    
    let contribution_key = DataKey::MemberContributionRecord(
        circle_id,
        circle.current_recipient_index,
        member.clone(),
    );
    
    let contribution_record = MemberContributionRecord {
        member: member.clone(),
        circle_id,
        round: circle.current_recipient_index,
        contributed_on_time: false,
        contribution_timestamp: 0,
        is_defaulted: true,
    };
    
    env.storage().instance().set(&contribution_key, &contribution_record);
    
    // Emit event
    env.events().publish(
        (Symbol::new(env, "MEMBER_DEFAULTED"), circle_id, member.clone()),
        (circle.current_recipient_index, env.ledger().timestamp()),
    );
    
    Ok(())
}

/// Execute clawback of defaulted member's locked tranches
pub fn execute_tranche_clawback(
    env: &Env,
    admin: &Address,
    circle_id: u64,
    member: &Address,
) -> Result<i128, Error> {
    // Verify admin
    let stored_admin: Address = env
        .storage()
        .instance()
        .get(&DataKey::Admin)
        .ok_or(Error::Unauthorized)?;
    
    if *admin != stored_admin {
        return Err(Error::Unauthorized);
    }
    
    // Check if member is actually defaulted
    let default_key = DataKey::DefaultedMember(circle_id, member.clone());
    if !env.storage().instance().has(&default_key) {
        return Err(Error::MemberNotFound);
    }
    
    // Get the tranche schedule
    let schedule_key = DataKey::TrancheSchedule(circle_id, member.clone());
    let mut schedule: TrancheSchedule = env
        .storage()
        .instance()
        .get(&schedule_key)
        .ok_or(Error::TrancheNotFound)?;
    
    // Calculate total amount to claw back (all unclaimed tranches)
    let mut total_clawback = 0i128;
    let circle: CircleInfo = env
        .storage()
        .instance()
        .get(&DataKey::Circle(circle_id))
        .ok_or(Error::CircleNotFound)?;
    
    for i in 0..schedule.tranches.len() {
        let mut tranche = schedule.tranches.get(i).unwrap();
        
        if tranche.status == TrancheStatus::Locked || tranche.status == TrancheStatus::Unlocked {
            total_clawback += tranche.amount;
            tranche.status = TrancheStatus::ClawedBack;
            schedule.tranches.set(i, tranche.clone());
        }
    }
    
    if total_clawback == 0 {
        return Err(Error::TrancheClawbackFailed);
    }
    
    // Transfer clawed back amount to group treasury or insurance fund
    let token_client = token::Client::new(env, &circle.token);
    
    // Try to send to group insurance fund first, fallback to treasury
    let insurance_fund_key = DataKey::GroupInsuranceFund(circle_id);
    if let Some(_insurance_fund) = env.storage().instance().get::<_, crate::GroupInsuranceFund>(&insurance_fund_key) {
        // Send to insurance fund
        token_client.transfer(
            &env.current_contract_address(),
            &env.current_contract_address(), // In production, use actual insurance fund address
            &total_clawback,
        );
    } else {
        // Send to circle creator (organizer) as temporary holder
        token_client.transfer(
            &env.current_contract_address(),
            &circle.creator,
            &total_clawback,
        );
    }
    
    // Mark schedule as complete
    schedule.is_complete = true;
    env.storage().instance().set(&schedule_key, &schedule);
    
    // Emit event
    env.events().publish(
        (Symbol::new(env, "TRANCHE_CLAWBACK"), circle_id, member.clone()),
        (total_clawback, env.ledger().timestamp()),
    );
    
    Ok(total_clawback)
}

/// Record member contribution for tranche eligibility tracking
pub fn record_contribution(
    env: &Env,
    circle_id: u64,
    member: &Address,
    contributed_on_time: bool,
) {
    let circle: CircleInfo = env
        .storage()
        .instance()
        .get(&DataKey::Circle(circle_id))
        .expect("Circle not found");
    
    let current_time = env.ledger().timestamp();
    
    let contribution_record = MemberContributionRecord {
        member: member.clone(),
        circle_id,
        round: circle.current_recipient_index,
        contributed_on_time,
        contribution_timestamp: current_time,
        is_defaulted: !contributed_on_time,
    };
    
    let contribution_key = DataKey::MemberContributionRecord(
        circle_id,
        circle.current_recipient_index,
        member.clone(),
    );
    
    env.storage().instance().set(&contribution_key, &contribution_record);
}

/// Check if member is eligible to claim their next tranche
pub fn is_member_eligible_for_tranche(
    env: &Env,
    circle_id: u64,
    member: &Address,
    tranche_index: u32,
) -> bool {
    // Check if member is defaulted
    let default_key = DataKey::DefaultedMember(circle_id, member.clone());
    if env.storage().instance().has(&default_key) {
        return false;
    }
    
    // Get tranche schedule
    let schedule_key = DataKey::TrancheSchedule(circle_id, member.clone());
    let schedule: Option<TrancheSchedule> = env.storage().instance().get(&schedule_key);
    
    if let Some(schedule) = schedule {
        if tranche_index >= schedule.tranches.len() {
            return false;
        }
        
        let tranche = schedule.tranches.get(tranche_index).unwrap();
        
        // Check status
        if tranche.status != TrancheStatus::Locked && tranche.status != TrancheStatus::Unlocked {
            return false;
        }
        
        // Check unlock conditions
        let circle: Option<CircleInfo> = env.storage().instance().get(&DataKey::Circle(circle_id));
        if let Some(circle) = circle {
            let current_time = env.ledger().timestamp();
            return circle.current_recipient_index >= tranche.unlock_round || current_time >= tranche.unlock_timestamp;
        }
    }
    
    false
}
