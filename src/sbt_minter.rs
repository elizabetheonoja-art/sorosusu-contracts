use soroban_sdk::{
    contract, contractimpl, contracttype, symbol_short,
    Address, Env, String,
};

use crate::DataKey;

// --- SOROSUSU SOULBOUND TOKEN (SBT) SYSTEM ---

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum SbtStatus {
    Discovery,
    Pathfinder,
    Guardian,
    Luminary,
    SusuLegend,
}

#[derive(Clone, Debug, PartialEq)]
#[contracttype]
pub enum ReputationTier {
    Bronze,
    Silver,
    Gold,
    Platinum,
    Diamond,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct SoroSusuCredential {
    pub token_id: u128,
    pub user: Address,
    pub status: SbtStatus,
    pub reputation_score: u32,
    pub metadata_uri: String,
    pub issue_date: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct UserReputationMetrics {
    pub reliability_score: u32, // Based on contribution timeliness (0-10000)
    pub social_capital_score: u32, // Based on leniency/participation
    pub total_cycles: u32,
    pub perfect_cycles: u32,
    pub total_volume_saved: i128,
    pub last_updated: u64,
}

#[derive(Clone, Debug)]
#[contracttype]
pub struct ReputationMilestone {
    pub user: Address,
    pub required_cycles: u32,
    pub description: String,
    pub tier: ReputationTier,
    pub is_completed: bool,
}

#[contract]
pub struct SoroSusuSbtMinter;

#[contractimpl]
impl SoroSusuSbtMinter {
    // Initialize SBT Minter with admin
    pub fn init_sbt_minter(env: Env, admin: Address) {
        admin.require_auth();
        env.storage().instance().set(&DataKey::K(symbol_short!("SbtAdm")), &admin);
        env.storage().instance().set(&DataKey::K(symbol_short!("MileCnt")), &0u64);
    }

    // Set new admin
    pub fn set_sbt_minter_admin(env: Env, admin: Address, new_admin: Address) {
        let current_admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        if admin != current_admin { panic!(); }
        env.storage().instance().set(&DataKey::K(symbol_short!("SbtAdm")), &new_admin);
    }

    // Issue SBT credential
    pub fn issue_credential(env: Env, user: Address, milestone_id: u64, metadata_uri: String) -> u128 {
        let milestone: ReputationMilestone = env.storage().instance().get(&DataKey::K1(symbol_short!("Mile"), milestone_id)).unwrap();
        if milestone.user != user { panic!(); }
        if milestone.is_completed { panic!(); }
        
        let metrics: UserReputationMetrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user.clone())).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, last_updated: 0, total_volume_saved: 0,
        });

        let reputation_score = (metrics.reliability_score + metrics.social_capital_score) / 2;
        let token_id = env.ledger().timestamp() as u128 + metrics.total_cycles as u128; // Simple unique ID for mock

        let credential = SoroSusuCredential {
            token_id,
            user: user.clone(),
            status: match milestone.tier {
                ReputationTier::Bronze => SbtStatus::Discovery,
                ReputationTier::Silver => SbtStatus::Pathfinder,
                ReputationTier::Gold => SbtStatus::Guardian,
                ReputationTier::Platinum => SbtStatus::Luminary,
                ReputationTier::Diamond => SbtStatus::SusuLegend,
            },
            reputation_score,
            metadata_uri,
            issue_date: env.ledger().timestamp(),
        };

        env.storage().instance().set(&DataKey::K1U(symbol_short!("Cred"), token_id), &credential);
        env.storage().instance().set(&DataKey::K1A(symbol_short!("UCred"), user), &token_id);
        
        token_id
    }

    pub fn update_credential_status(env: Env, token_id: u128, new_status: SbtStatus) {
        let admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        let mut credential: SoroSusuCredential = env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), token_id)).unwrap();
        credential.status = new_status;
        env.storage().instance().set(&DataKey::K1U(symbol_short!("Cred"), token_id), &credential);
    }

    pub fn get_credential(env: Env, token_id: u128) -> SoroSusuCredential {
        env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), token_id)).unwrap()
    }

    pub fn get_user_credential(env: Env, user: Address) -> Option<SoroSusuCredential> {
        let token_id: Option<u128> = env.storage().instance().get(&DataKey::K1A(symbol_short!("UCred"), user));
        token_id.map(|id| env.storage().instance().get(&DataKey::K1U(symbol_short!("Cred"), id)).unwrap())
    }

    pub fn create_reputation_milestone(env: Env, user: Address, cycles: u32, desc: String, tier: ReputationTier) -> u64 {
        let admin: Address = env.storage().instance().get(&DataKey::K(symbol_short!("SbtAdm"))).unwrap();
        admin.require_auth();
        let mut count: u64 = env.storage().instance().get(&DataKey::K(symbol_short!("MileCnt"))).unwrap_or(0);
        count += 1;
        let milestone = ReputationMilestone { user, required_cycles: cycles, description: desc, tier, is_completed: false };
        env.storage().instance().set(&DataKey::K1(symbol_short!("Mile"), count), &milestone);
        env.storage().instance().set(&DataKey::K(symbol_short!("MileCnt")), &count);
        count
    }

    pub fn get_reputation_milestone(env: Env, milestone_id: u64) -> ReputationMilestone {
        env.storage().instance().get(&DataKey::K1(symbol_short!("Mile"), milestone_id)).unwrap()
    }

    pub fn get_user_reputation_score(env: Env, user: Address) -> (u32, u32, u32) {
        let metrics: UserReputationMetrics = env.storage().instance().get(&DataKey::K1A(symbol_short!("URep"), user)).unwrap_or(UserReputationMetrics {
            reliability_score: 5000, social_capital_score: 5000, total_cycles: 0, perfect_cycles: 0, last_updated: 0, total_volume_saved: 0,
        });
        (metrics.reliability_score, metrics.social_capital_score, (metrics.reliability_score + metrics.social_capital_score) / 2)
    }
}
