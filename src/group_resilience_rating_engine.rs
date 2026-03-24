#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec,
};

// --- ERROR CODES ---

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    GroupNotFound = 1,
    InvalidParameters = 2,
    CalculationError = 3,
    Unauthorized = 4,
}

// --- CONSTANTS ---

const REPUTATION_WEIGHT: u32 = 4000; // 40% weight for aggregate reputation
const PAYMENT_CONSISTENCY_WEIGHT: u32 = 3500; // 35% weight for payment consistency
const MEMBER_STABILITY_WEIGHT: u32 = 1500; // 15% weight for member stability
const HISTORICAL_PERFORMANCE_WEIGHT: u32 = 1000; // 10% weight for historical performance
const TOTAL_WEIGHT_BPS: u32 = 10000; // 100% in basis points

const HIGH_HEALTH_THRESHOLD: u32 = 8000; // 80%+ = High Health
const MEDIUM_HEALTH_THRESHOLD: u32 = 6000; // 60-79% = Medium Health
const LOW_HEALTH_THRESHOLD: u32 = 4000; // 40-59% = Low Health
const CRITICAL_HEALTH_THRESHOLD: u32 = 2000; // <40% = Critical Health

// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum GroupHealthRating {
    Excellent,
    Good,
    Fair,
    Poor,
    Critical,
}

#[contracttype]
#[derive(Clone)]
pub struct GroupHealthMetrics {
    pub circle_id: u64,
    pub aggregate_reputation: u32, // 0-10000 bps
    pub payment_consistency: u32, // 0-10000 bps
    pub member_stability: u32, // 0-10000 bps
    pub historical_performance: u32, // 0-10000 bps
    pub health_score: u32, // 0-10000 bps
    pub rating: GroupHealthRating,
    pub last_updated: u64,
    pub total_members: u32,
    pub active_members: u32,
    pub defaulted_members: u32,
    pub avg_trust_score: u32,
    pub on_time_payment_rate: u32,
    pub completed_cycles: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct PaymentHistory {
    pub circle_id: u64,
    pub total_payments: u32,
    pub on_time_payments: u32,
    pub late_payments: u32,
    pub very_late_payments: u32,
    pub avg_lateness_hours: u64,
    pub payment_consistency_trend: Vec<u32>, // Last 12 cycles consistency
}

#[contracttype]
#[derive(Clone)]
pub struct MemberStabilityMetrics {
    pub circle_id: u64,
    pub member_turnover_rate: u32, // bps
    pub default_rate: u32, // bps
    pub avg_member_tenure: u64, // in seconds
    pub new_member_rate: u32, // bps
    pub retention_rate: u32, // bps
}

#[contracttype]
#[derive(Clone)]
pub struct HistoricalPerformance {
    pub circle_id: u64,
    pub completed_cycles: u32,
    pub successful_cycles: u32,
    pub avg_cycle_completion_time: u64,
    pub total_milestones_achieved: u32,
    pub crisis_recovery_count: u32,
    pub longest_healthy_streak: u32,
}

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    GroupHealthMetrics(u64),
    PaymentHistory(u64),
    MemberStabilityMetrics(u64),
    HistoricalPerformance(u64),
    HealthScoreHistory(u64, u64), // circle_id, timestamp
    Admin,
}

// --- CONTRACT CLIENTS ---

#[contractclient(name = "SoroSusuClient")]
pub trait SoroSusuTrait {
    fn get_circle(env: Env, circle_id: u64) -> CircleInfo;
    fn get_member(env: Env, member: Address) -> Member;
    fn get_social_capital(env: Env, member: Address, circle_id: u64) -> SocialCapital;
    fn get_milestone_stats(env: Env, circle_id: u64) -> MilestoneStats;
}

// Mock data structures (these would be imported from the main contract)
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
    pub leniency_enabled: bool,
    pub grace_period_end: Option<u64>,
    pub quadratic_voting_enabled: bool,
    pub proposal_count: u64,
    pub requires_collateral: bool,
    pub collateral_bps: u32,
    pub total_cycle_value: i128,
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
#[derive(Clone, Debug, PartialEq)]
pub enum MemberStatus {
    Active,
    AwaitingReplacement,
    Ejected,
    Defaulted,
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
    pub last_activity_timestamp: u64,
    pub decay_count: u32,
}

#[contracttype]
#[derive(Clone)]
pub struct MilestoneStats {
    pub circle_id: u64,
    pub total_milestones_completed: u32,
    pub total_bonus_points_distributed: u32,
    pub members_with_milestones: u32,
    pub most_common_milestone: MilestoneType,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MilestoneType {
    ConsecutiveOnTimePayments,
    FirstGroupOrganized,
    PerfectAttendance,
    EarlyBirdStreak,
    ReferralMaster,
    VouchingChampion,
    CommunityLeader,
    ReliabilityStar,
}

// --- CONTRACT TRAIT ---

pub trait GroupResilienceRatingEngineTrait {
    fn init(env: Env, admin: Address);
    fn calculate_group_health_score(env: Env, circle_id: u64) -> GroupHealthMetrics;
    fn get_group_health_metrics(env: Env, circle_id: u64) -> GroupHealthMetrics;
    fn get_payment_history(env: Env, circle_id: u64) -> PaymentHistory;
    fn get_member_stability_metrics(env: Env, circle_id: u64) -> MemberStabilityMetrics;
    fn get_historical_performance(env: Env, circle_id: u64) -> HistoricalPerformance;
    fn search_healthy_groups(env: Env, min_rating: GroupHealthRating, limit: u32) -> Vec<u64>;
    fn get_health_score_history(env: Env, circle_id: u64, limit: u32) -> Vec<GroupHealthMetrics>;
    fn batch_update_health_scores(env: Env, circle_ids: Vec<u64>) -> Vec<GroupHealthMetrics>;
    fn get_top_performing_groups(env: Env, limit: u32) -> Vec<u64>;
    fn get_groups_at_risk(env: Env, max_rating: GroupHealthRating, limit: u32) -> Vec<u64>;
}

// --- IMPLEMENTATION ---

#[contract]
pub struct GroupResilienceRatingEngine;

#[contractimpl]
impl GroupResilienceRatingEngineTrait for GroupResilienceRatingEngine {
    fn init(env: Env, admin: Address) {
        if !env.storage().instance().has(&DataKey::Admin) {
            env.storage().instance().set(&DataKey::Admin, &admin);
        }
    }

    fn calculate_group_health_score(env: Env, circle_id: u64) -> GroupHealthMetrics {
        let current_time = env.ledger().timestamp();
        
        // Calculate individual components
        let aggregate_reputation = Self::calculate_aggregate_reputation(&env, circle_id);
        let payment_consistency = Self::calculate_payment_consistency(&env, circle_id);
        let member_stability = Self::calculate_member_stability(&env, circle_id);
        let historical_performance = Self::calculate_historical_performance(&env, circle_id);
        
        // Calculate weighted health score
        let health_score = (
            (aggregate_reputation * REPUTATION_WEIGHT) / TOTAL_WEIGHT_BPS +
            (payment_consistency * PAYMENT_CONSISTENCY_WEIGHT) / TOTAL_WEIGHT_BPS +
            (member_stability * MEMBER_STABILITY_WEIGHT) / TOTAL_WEIGHT_BPS +
            (historical_performance * HISTORICAL_PERFORMANCE_WEIGHT) / TOTAL_WEIGHT_BPS
        ).min(10000); // Cap at 10000 bps (100%)
        
        // Determine rating
        let rating = Self::determine_health_rating(health_score);
        
        // Get additional metrics
        let (total_members, active_members, defaulted_members, avg_trust_score) = 
            Self::get_member_metrics(&env, circle_id);
        let on_time_payment_rate = Self::get_on_time_payment_rate(&env, circle_id);
        let completed_cycles = Self::get_completed_cycles(&env, circle_id);
        
        let metrics = GroupHealthMetrics {
            circle_id,
            aggregate_reputation,
            payment_consistency,
            member_stability,
            historical_performance,
            health_score,
            rating: rating.clone(),
            last_updated: current_time,
            total_members,
            active_members,
            defaulted_members,
            avg_trust_score,
            on_time_payment_rate,
            completed_cycles,
        };
        
        // Store the metrics
        env.storage().instance().set(&DataKey::GroupHealthMetrics(circle_id), &metrics);
        
        // Store in history for tracking trends
        let history_key = DataKey::HealthScoreHistory(circle_id, current_time);
        env.storage().instance().set(&history_key, &metrics);
        
        metrics
    }

    fn get_group_health_metrics(env: Env, circle_id: u64) -> GroupHealthMetrics {
        env.storage().instance()
            .get(&DataKey::GroupHealthMetrics(circle_id))
            .unwrap_or_else(|| Self::calculate_group_health_score(env, circle_id))
    }

    fn get_payment_history(env: Env, circle_id: u64) -> PaymentHistory {
        env.storage().instance()
            .get(&DataKey::PaymentHistory(circle_id))
            .unwrap_or_else(|| PaymentHistory {
                circle_id,
                total_payments: 0,
                on_time_payments: 0,
                late_payments: 0,
                very_late_payments: 0,
                avg_lateness_hours: 0,
                payment_consistency_trend: Vec::new(&env),
            })
    }

    fn get_member_stability_metrics(env: Env, circle_id: u64) -> MemberStabilityMetrics {
        env.storage().instance()
            .get(&DataKey::MemberStabilityMetrics(circle_id))
            .unwrap_or_else(|| MemberStabilityMetrics {
                circle_id,
                member_turnover_rate: 0,
                default_rate: 0,
                avg_member_tenure: 0,
                new_member_rate: 0,
                retention_rate: 10000, // Start with 100% retention
            })
    }

    fn get_historical_performance(env: Env, circle_id: u64) -> HistoricalPerformance {
        env.storage().instance()
            .get(&DataKey::HistoricalPerformance(circle_id))
            .unwrap_or_else(|| HistoricalPerformance {
                circle_id,
                completed_cycles: 0,
                successful_cycles: 0,
                avg_cycle_completion_time: 0,
                total_milestones_achieved: 0,
                crisis_recovery_count: 0,
                longest_healthy_streak: 0,
            })
    }

    fn search_healthy_groups(env: Env, min_rating: GroupHealthRating, limit: u32) -> Vec<u64> {
        let mut healthy_groups = Vec::new(&env);
        let min_score = Self::rating_to_threshold(min_rating);
        
        // This is a simplified implementation
        // In practice, you'd want to maintain an index of groups by health score
        // For now, we'll simulate by checking a range of circle IDs
        for circle_id in 1..=100 { // Check first 100 potential circles
            if let Ok(metrics) = env.storage().instance().get::<DataKey, GroupHealthMetrics>(
                &DataKey::GroupHealthMetrics(circle_id)
            ) {
                if metrics.health_score >= min_score {
                    healthy_groups.push_back(circle_id);
                    if healthy_groups.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        healthy_groups
    }

    fn get_health_score_history(env: Env, circle_id: u64, limit: u32) -> Vec<GroupHealthMetrics> {
        let mut history = Vec::new(&env);
        let mut count = 0;
        
        // This is a simplified implementation
        // In practice, you'd want to store timestamps in a more efficient way
        for timestamp in 0..=env.ledger().timestamp() {
            let history_key = DataKey::HealthScoreHistory(circle_id, timestamp);
            if let Some(metrics) = env.storage().instance().get::<DataKey, GroupHealthMetrics>(&history_key) {
                history.push_back(metrics);
                count += 1;
                if count >= limit {
                    break;
                }
            }
        }
        
        history
    }

    fn batch_update_health_scores(env: Env, circle_ids: Vec<u64>) -> Vec<GroupHealthMetrics> {
        let mut results = Vec::new(&env);
        
        for circle_id in circle_ids {
            let metrics = Self::calculate_group_health_score(env.clone(), circle_id);
            results.push_back(metrics);
        }
        
        results
    }

    fn get_top_performing_groups(env: Env, limit: u32) -> Vec<GroupHealthMetrics> {
        let mut top_groups = Vec::new(&env);
        
        // This is a simplified implementation
        // In practice, you'd want to maintain a sorted index
        for circle_id in 1..=100 {
            if let Ok(metrics) = env.storage().instance().get::<DataKey, GroupHealthMetrics>(
                &DataKey::GroupHealthMetrics(circle_id)
            ) {
                top_groups.push_back(metrics);
            }
        }
        
        // Sort by health score (simplified - would need proper sorting implementation)
        top_groups
    }

    fn get_groups_at_risk(env: Env, max_rating: GroupHealthRating, limit: u32) -> Vec<u64> {
        let mut at_risk_groups = Vec::new(&env);
        let max_score = Self::rating_to_threshold(max_rating);
        
        for circle_id in 1..=100 {
            if let Ok(metrics) = env.storage().instance().get::<DataKey, GroupHealthMetrics>(
                &DataKey::GroupHealthMetrics(circle_id)
            ) {
                if metrics.health_score <= max_score {
                    at_risk_groups.push_back(circle_id);
                    if at_risk_groups.len() >= limit {
                        break;
                    }
                }
            }
        }
        
        at_risk_groups
    }
}

// --- HELPER FUNCTIONS ---

impl GroupResilienceRatingEngine {
    fn calculate_aggregate_reputation(env: &Env, circle_id: u64) -> u32 {
        // This would integrate with the main SoroSusu contract
        // For now, return a mock calculation
        // In practice: average trust score of all active members
        
        // Mock implementation - would fetch actual member data
        let base_reputation = 7000; // 70% base reputation
        let variance = (circle_id % 3000) as u32; // Some variance based on circle ID
        (base_reputation + variance).min(10000)
    }

    fn calculate_payment_consistency(env: &Env, circle_id: u64) -> u32 {
        // Calculate based on on-time payment rate, late payment frequency
        // and payment consistency trends
        
        // Mock implementation - would fetch actual payment data
        let base_consistency = 8000; // 80% base consistency
        let penalty = (circle_id % 2000) as u32; // Some penalty based on circle ID
        base_consistency.saturating_sub(penalty)
    }

    fn calculate_member_stability(env: &Env, circle_id: u64) -> u32 {
        // Calculate based on member turnover rate, default rate, and retention
        
        // Mock implementation - would fetch actual member stability data
        let base_stability = 7500; // 75% base stability
        let adjustment = ((circle_id % 1000) - 500) as i32; // +/- adjustment
        (base_stability as i32 + adjustment).max(0).min(10000) as u32
    }

    fn calculate_historical_performance(env: &Env, circle_id: u64) -> u32 {
        // Calculate based on completed cycles, milestones, and crisis recovery
        
        // Mock implementation - would fetch actual historical data
        let base_performance = 6000; // 60% base performance
        let bonus = (circle_id % 4000) as u32; // Bonus based on circle ID
        (base_performance + bonus).min(10000)
    }

    fn determine_health_rating(health_score: u32) -> GroupHealthRating {
        if health_score >= HIGH_HEALTH_THRESHOLD {
            GroupHealthRating::Excellent
        } else if health_score >= MEDIUM_HEALTH_THRESHOLD {
            GroupHealthRating::Good
        } else if health_score >= LOW_HEALTH_THRESHOLD {
            GroupHealthRating::Fair
        } else if health_score >= CRITICAL_HEALTH_THRESHOLD {
            GroupHealthRating::Poor
        } else {
            GroupHealthRating::Critical
        }
    }

    fn rating_to_threshold(rating: GroupHealthRating) -> u32 {
        match rating {
            GroupHealthRating::Excellent => HIGH_HEALTH_THRESHOLD,
            GroupHealthRating::Good => MEDIUM_HEALTH_THRESHOLD,
            GroupHealthRating::Fair => LOW_HEALTH_THRESHOLD,
            GroupHealthRating::Poor => CRITICAL_HEALTH_THRESHOLD,
            GroupHealthRating::Critical => 0,
        }
    }

    fn get_member_metrics(env: &Env, circle_id: u64) -> (u32, u32, u32, u32) {
        // Mock implementation - would fetch actual member data
        let total_members = ((circle_id % 10) + 5) as u32; // 5-14 members
        let active_members = (total_members * 85) / 100; // 85% active rate
        let defaulted_members = (total_members * 5) / 100; // 5% default rate
        let avg_trust_score = 75 + (circle_id % 20) as u32; // 75-95 average trust score
        
        (total_members, active_members, defaulted_members, avg_trust_score)
    }

    fn get_on_time_payment_rate(env: &Env, circle_id: u64) -> u32 {
        // Mock implementation - would fetch actual payment data
        8000 + ((circle_id % 1500) as u32) // 80-95% on-time rate
    }

    fn get_completed_cycles(env: &Env, circle_id: u64) -> u32 {
        // Mock implementation - would fetch actual cycle data
        (circle_id % 12) as u32 // 0-11 completed cycles
    }
}
