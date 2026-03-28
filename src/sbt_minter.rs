#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map,
};

use crate::{
    SoroSusuTrait, Error, DataKey, CircleInfo, Member, UserStats, NftBadgeMetadata, 
    SusuNftClient, SusuNftTrait, AuditEntry, AuditAction
};

// --- SOROSUSU SOULBOUND TOKEN (SBT) SYSTEM ---

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SbtStatus {
    Active,
    Dishonored,
    Revoked,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ReputationTier {
    Bronze,     // 0-2 cycles completed
    Silver,     // 3-5 cycles completed  
    Gold,       // 6-9 cycles completed
    Platinum,   // 10+ cycles completed
    Diamond,    // Legendary: 12+ cycles with perfect record
}

#[contracttype]
#[derive(Clone)]
pub struct SoroSusuCredential {
    pub token_id: u128,
    pub holder: Address,
    pub reputation_tier: ReputationTier,
    pub total_cycles_completed: u32,
    pub perfect_cycles: u32,
    pub on_time_rate: u32,        // Basis points (10000 = 100%)
    pub reliability_score: u32,     // 0-10000 bps
    pub social_capital_score: u32,  // 0-10000 bps
    pub total_volume_saved: i128,
    pub last_activity: u64,
    pub status: SbtStatus,
    pub minted_timestamp: u64,
    pub metadata_uri: String,
}

#[contracttype]
#[derive(Clone)]
pub struct ReputationMilestone {
    pub milestone_id: u64,
    pub user: Address,
    pub cycles_required: u32,
    pub description: String,
    pub is_completed: bool,
    pub completion_timestamp: Option<u64>,
    pub reward_tier: ReputationTier,
}

#[contracttype]
#[derive(Clone)]
pub struct UserReputationMetrics {
    pub reliability_score: u32,     // 0-10000 bps
    pub social_capital_score: u32,  // 0-10000 bps
    pub total_cycles: u32,
    pub perfect_cycles: u32,
    pub last_updated: u64,
    pub total_volume_saved: i128,
}

// --- SBT CREDENTIAL MINTER CONTRACT ---

#[contract]
pub struct SoroSusuSbtMinter;

#[contractimpl]
impl SoroSusuSbtMinter {
    // Initialize SBT Minter with admin
    pub fn init_sbt_minter(env: Env, admin: Address) {
        // Only admin can initialize
        admin.require_auth();
        
        // Set admin
        env.storage().instance().set(&DataKey::SbtMinterAdmin, &admin);
        
        // Initialize milestone counter
        env.storage().instance().set(&DataKey::MilestoneCounter, &0u64);
    }

    // Set new admin (for admin transfer)
    pub fn set_sbt_minter_admin(env: Env, admin: Address, new_admin: Address) {
        // Only current admin can set new admin
        let current_admin: Address = env.storage().instance()
            .get(&DataKey::SbtMinterAdmin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        admin.require_auth();
        if admin != current_admin {
            panic!("Unauthorized: Only current admin can set new admin");
        }
        
        env.storage().instance().set(&DataKey::SbtMinterAdmin, &new_admin);
    }

    // Issue SBT credential when user hits reputation milestone
    pub fn issue_credential(
        env: Env,
        user: Address,
        milestone_id: u64,
        metadata_uri: String,
    ) -> u128 {
        // Get milestone to validate
        let milestone: ReputationMilestone = env.storage().instance()
            .get(&DataKey::ReputationMilestone(milestone_id))
            .unwrap_or_else(|| panic!("Milestone not found"));
        
        // Verify user matches milestone
        if milestone.user != user {
            panic!("Unauthorized: Milestone belongs to different user");
        }
        
        // Check if milestone already completed
        if milestone.is_completed {
            panic!("Milestone already completed");
        }
        
        // Get current user reputation metrics
        let mut user_metrics: UserReputationMetrics = env.storage().instance()
            .get(&DataKey::UserReputationScore(user.clone()))
            .unwrap_or_else(|| UserReputationMetrics {
                reliability_score: 5000, // Start at 50%
                social_capital_score: 5000, // Start at 50%
                total_cycles: 0,
                perfect_cycles: 0,
                last_updated: env.ledger().timestamp(),
                total_volume_saved: 0,
            });
        
        // Update user metrics based on milestone completion
        user_metrics.total_cycles += milestone.cycles_required;
        user_metrics.last_updated = env.ledger().timestamp();
        
        // Calculate new reputation tier based on total cycles
        let new_tier = match user_metrics.total_cycles {
            0..=2 => ReputationTier::Bronze,
            3..=5 => ReputationTier::Silver,
            6..=9 => ReputationTier::Gold,
            10..=11 => ReputationTier::Platinum,
            _ => ReputationTier::Diamond,
        };
        
        // Create SBT credential
        let token_id = env.ledger().sequence() as u128; // Unique token ID
        let credential = SoroSusuCredential {
            token_id,
            holder: user.clone(),
            reputation_tier: new_tier.clone(),
            total_cycles_completed: user_metrics.total_cycles,
            perfect_cycles: user_metrics.perfect_cycles,
            on_time_rate: user_metrics.reliability_score,
            reliability_score: user_metrics.reliability_score,
            social_capital_score: user_metrics.social_capital_score,
            total_volume_saved: user_metrics.total_volume_saved,
            last_activity: env.ledger().timestamp(),
            status: SbtStatus::Active,
            minted_timestamp: env.ledger().timestamp(),
            metadata_uri: metadata_uri.clone(),
        };
        
        // Store the credential
        env.storage().instance().set(&DataKey::SoroSusuCredential(token_id), &credential);
        env.storage().instance().set(&DataKey::UserCredential(user.clone()), &token_id);
        
        // Update user reputation metrics
        env.storage().instance().set(&DataKey::UserReputationScore(user.clone()), &user_metrics);
        
        // Mark milestone as completed
        let mut updated_milestone = milestone;
        updated_milestone.is_completed = true;
        updated_milestone.completion_timestamp = Some(env.ledger().timestamp());
        env.storage().instance().set(&DataKey::ReputationMilestone(milestone_id), &updated_milestone);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: user.clone(), // User who achieved milestone
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: milestone_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
        
        token_id
    }

    // Update credential status (for revocation/dishonor)
    pub fn update_credential_status(
        env: Env,
        token_id: u128,
        new_status: SbtStatus,
    ) {
        // Only admin can update status
        let admin: Address = env.storage().instance()
            .get(&DataKey::SbtMinterAdmin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        admin.require_auth();
        
        let mut credential: SoroSusuCredential = env.storage().instance()
            .get(&DataKey::SoroSusuCredential(token_id))
            .unwrap_or_else(|| panic!("Credential not found"));
        
        let is_revoking = matches!(new_status, SbtStatus::Dishonored | SbtStatus::Revoked);
        credential.status = new_status;
        credential.last_activity = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::SoroSusuCredential(token_id), &credential);
        
        // If revoking, update user reputation
        if is_revoking {
            let mut user_metrics: UserReputationMetrics = env.storage().instance()
                .get(&DataKey::UserReputationScore(credential.holder.clone()))
                .unwrap_or_else(|| UserReputationMetrics {
                    reliability_score: 5000,
                    social_capital_score: 5000,
                    total_cycles: 0,
                    perfect_cycles: 0,
                    last_updated: env.ledger().timestamp(),
                    total_volume_saved: 0,
                });
            
            // Significantly reduce reputation scores
            user_metrics.reliability_score = user_metrics.reliability_score / 2;
            user_metrics.social_capital_score = user_metrics.social_capital_score / 2;
            user_metrics.last_updated = env.ledger().timestamp();
            
            env.storage().instance().set(&DataKey::UserReputationScore(credential.holder.clone()), &user_metrics);
        }
    }

    // Revoke credential with reason
    pub fn revoke_credential(env: Env, token_id: u128, reason: String) {
        let admin: Address = env.storage().instance()
            .get(&DataKey::SbtMinterAdmin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        admin.require_auth();
        
        let mut credential: SoroSusuCredential = env.storage().instance()
            .get(&DataKey::SoroSusuCredential(token_id))
            .unwrap_or_else(|| panic!("Credential not found"));
        
        credential.status = SbtStatus::Revoked;
        credential.last_activity = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::SoroSusuCredential(token_id), &credential);
        
        // Update user reputation (severe penalty)
        let mut user_metrics: UserReputationMetrics = env.storage().instance()
            .get(&DataKey::UserReputationScore(credential.holder.clone()))
            .unwrap_or_else(|| UserReputationMetrics {
                reliability_score: 5000,
                social_capital_score: 5000,
                total_cycles: 0,
                perfect_cycles: 0,
                last_updated: env.ledger().timestamp(),
                total_volume_saved: 0,
            });
        
        // Severe reputation penalty for revocation
        user_metrics.reliability_score = user_metrics.reliability_score / 4;
        user_metrics.social_capital_score = user_metrics.social_capital_score / 4;
        user_metrics.last_updated = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::UserReputationScore(credential.holder.clone()), &user_metrics);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: admin,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: token_id as u64,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Get credential by token ID
    pub fn get_credential(env: Env, token_id: u128) -> SoroSusuCredential {
        env.storage().instance()
            .get(&DataKey::SoroSusuCredential(token_id))
            .unwrap_or_else(|| panic!("Credential not found"))
    }

    // Get user's current credential
    pub fn get_user_credential(env: Env, user: Address) -> Option<SoroSusuCredential> {
        let token_id: Option<u128> = env.storage().instance()
            .get(&DataKey::UserCredential(user.clone()));
        
        token_id.map(|id| {
            env.storage().instance()
                .get(&DataKey::SoroSusuCredential(id))
                .unwrap_or_else(|| panic!("Credential not found"))
        })
    }

    // Create reputation milestone
    pub fn create_reputation_milestone(
        env: Env,
        user: Address,
        cycles_required: u32,
        description: String,
        reward_tier: ReputationTier,
    ) -> u64 {
        // Only admin can create milestones
        let admin: Address = env.storage().instance()
            .get(&DataKey::SbtMinterAdmin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        admin.require_auth();
        
        let milestone_id: u64 = env.storage().instance()
            .get(&DataKey::MilestoneCounter)
            .unwrap_or(0) + 1;
        
        let milestone = ReputationMilestone {
            milestone_id,
            user: user.clone(),
            cycles_required,
            description: description.clone(),
            is_completed: false,
            completion_timestamp: None,
            reward_tier,
        };
        
        env.storage().instance().set(&DataKey::MilestoneCounter, &milestone_id);
        env.storage().instance().set(&DataKey::ReputationMilestone(milestone_id), &milestone);
        
        milestone_id
    }

    // Get milestone by ID
    pub fn get_reputation_milestone(env: Env, milestone_id: u64) -> ReputationMilestone {
        env.storage().instance()
            .get(&DataKey::ReputationMilestone(milestone_id))
            .unwrap_or_else(|| panic!("Milestone not found"))
    }

    // Update user reputation based on current activity
    pub fn update_user_reputation(env: Env, user: Address) {
        // This is called automatically when users complete cycles, make deposits, etc.
        let mut user_metrics: UserReputationMetrics = env.storage().instance()
            .get(&DataKey::UserReputationScore(user.clone()))
            .unwrap_or_else(|| UserReputationMetrics {
                reliability_score: 5000, // Start at 50%
                social_capital_score: 5000, // Start at 50%
                total_cycles: 0,
                perfect_cycles: 0,
                last_updated: env.ledger().timestamp(),
                total_volume_saved: 0,
            });
        
        // Get user's current credential to check tier
        if let Some(token_id) = env.storage().instance().get(&DataKey::UserCredential(user.clone())) {
            if let Some(credential) = env.storage().instance().get::<DataKey, SoroSusuCredential>(&DataKey::SoroSusuCredential(token_id)) {
                // Update based on credential tier
                user_metrics.reliability_score = (credential.reliability_score + 5000).min(10000);
                user_metrics.social_capital_score = (credential.social_capital_score + 5000).min(10000);
                user_metrics.total_cycles = credential.total_cycles_completed;
                user_metrics.perfect_cycles = credential.perfect_cycles;
            }
        }
        
        user_metrics.last_updated = env.ledger().timestamp();
        env.storage().instance().set(&DataKey::UserReputationScore(user.clone()), &user_metrics);
    }

    // Get user reputation metrics
    pub fn get_user_reputation_score(env: Env, user: Address) -> (u32, u32, u32) {
        let metrics: UserReputationMetrics = env.storage().instance()
            .get(&DataKey::UserReputationScore(user.clone()))
            .unwrap_or_else(|| UserReputationMetrics {
                reliability_score: 5000,
                social_capital_score: 5000,
                total_cycles: 0,
                perfect_cycles: 0,
                last_updated: env.ledger().timestamp(),
                total_volume_saved: 0,
            });
        
        (
            metrics.reliability_score,
            metrics.social_capital_score,
            metrics.total_cycles,
        )
    }
}
