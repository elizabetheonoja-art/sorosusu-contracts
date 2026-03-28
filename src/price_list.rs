#![no_std]
use soroban_sdk::{
    contract, contractclient, contracterror, contractimpl, contracttype, symbol_short, token,
    Address, Env, String, Symbol, Vec, Map, i128, u64, u32,
};

use crate::{
    SoroSusuTrait, Error, DataKey, CircleInfo, Member, UserStats, 
    SusuNftClient, SusuNftTrait, AuditEntry, AuditAction
};

// --- POT LIQUIDITY BUFFER FOR BANK HOLIDAYS ---

#[contract]
pub struct PotLiquidityBuffer;

#[contractimpl]
impl PotLiquidityBuffer {
    // Initialize liquidity buffer
    pub fn init_liquidity_buffer(env: Env, admin: Address) {
        admin.require_auth();
        
        // Set admin (reuse existing admin storage)
        env.storage().instance().set(&DataKey::Admin, &admin);
        
        // Initialize liquidity buffer configuration
        let config = LiquidityBufferConfig {
            is_enabled: true,
            advance_period: LIQUIDITY_BUFFER_ADVANCE_PERIOD,
            min_reputation: LIQUIDITY_BUFFER_MIN_REPUTATION,
            max_advance_bps: LIQUIDITY_BUFFER_MAX_ADVANCE_BPS,
            platform_fee_allocation: LIQUIDITY_BUFFER_PLATFORM_FEE_ALLOCATION,
            min_reserve: LIQUIDITY_BUFFER_MIN_RESERVE,
            max_reserve: LIQUIDITY_BUFFER_MAX_RESERVE,
            advance_fee_bps: LIQUIDITY_BUFFER_ADVANCE_FEE_BPS,
            grace_period: LIQUIDITY_BUFFER_GRACE_PERIOD,
            max_advances_per_round: LIQUIDITY_BUFFER_MAX_ADVANCES_PER_ROUND,
        };
        
        env.storage().instance().set(&DataKey::LiquidityBufferConfig, &config);
        
        // Initialize advance counter
        env.storage().instance().set(&DataKey::LiquidityAdvanceCounter, &0u64);
        
        // Initialize platform fee allocation tracking
        let allocation = PlatformFeeAllocation {
            total_fees_collected: 0,
            buffer_allocation_amount: 0,
            treasury_allocation_amount: 0,
            last_allocation_timestamp: env.ledger().timestamp(),
            allocation_frequency: 86400, // Daily allocation
        };
        
        env.storage().instance().set(&DataKey::PlatformFeeAllocation, &allocation);
        
        // Initialize statistics
        let stats = LiquidityBufferStats {
            total_reserve_balance: 0,
            total_platform_fees_allocated: 0,
            total_advances_provided: 0,
            total_advances_completed: 0,
            total_advances_defaulted: 0,
            total_advance_amount: 0,
            total_fees_collected: 0,
            active_advances_count: 0,
            average_advance_size: 0,
            buffer_utilization_rate: 0,
            last_updated: env.ledger().timestamp(),
        };
        
        env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: admin,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: 0,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Signal advance request for reputation-based early payment
    pub fn signal_advance_request(
        env: Env,
        member: Address,
        circle_id: u64,
        contribution_amount: i128,
        reason: String,
    ) -> u64 {
        member.require_auth();
        
        // Get liquidity buffer config
        let config: LiquidityBufferConfig = env.storage().instance()
            .get(&DataKey::LiquidityBufferConfig)
            .unwrap_or_else(|| panic!("Liquidity buffer not initialized"));
        
        if !config.is_enabled {
            panic!("Liquidity buffer is disabled");
        }
        
        // Check member eligibility
        if !Self::check_advance_eligibility(&env, member.clone(), circle_id) {
            panic!("Member not eligible for advance");
        }
        
        // Get circle info
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));
        
        // Validate contribution amount
        if contribution_amount <= 0 {
            panic!("Invalid contribution amount");
        }
        
        // Check if within advance limits
        let max_advance = (contribution_amount * config.max_advance_bps) / 10000;
        if max_advance > contribution_amount {
            panic!("Advance amount exceeds contribution");
        }
        
        // Check member's advance history for current round
        let member_history: MemberAdvanceHistory = env.storage().instance()
            .get(&DataKey::MemberAdvanceHistory(member.clone()))
            .unwrap_or_else(|| MemberAdvanceHistory {
                member: member.clone(),
                total_advances_taken: 0,
                total_advance_amount: 0,
                total_fees_paid: 0,
                current_round_advances: 0,
                last_advance_timestamp: None,
                repayment_history: Vec::new(&env),
                default_count: 0,
                reputation_score: 10000, // Default to perfect reputation
            });
        
        if member_history.current_round_advances >= config.max_advances_per_round {
            panic!("Maximum advances per round exceeded");
        }
        
        // Get current reserve balance
        let reserve_balance: i128 = env.storage().instance()
            .get(&DataKey::LiquidityBufferReserve)
            .unwrap_or(0);
        
        if reserve_balance < max_advance {
            panic!("Insufficient reserve balance");
        }
        
        // Create advance request
        let advance_id: u64 = env.storage().instance()
            .get(&DataKey::LiquidityAdvanceCounter)
            .unwrap_or(0) + 1;
        
        let advance_fee = (max_advance * config.advance_fee_bps) / 10000;
        let repayment_amount = max_advance + advance_fee;
        
        let advance = LiquidityAdvance {
            advance_id,
            member: member.clone(),
            circle_id,
            round_number: circle.current_round,
            contribution_amount,
            advance_amount: max_advance,
            advance_fee,
            repayment_amount,
            status: LiquidityAdvanceStatus::Pending,
            requested_timestamp: env.ledger().timestamp(),
            provided_timestamp: None,
            repayment_deadline: env.ledger().timestamp() + config.advance_period + config.grace_period,
            repaid_timestamp: None,
            reason: reason.clone(),
        };
        
        // Store advance
        env.storage().instance().set(&DataKey::LiquidityAdvance(advance_id), &advance);
        env.storage().instance().set(&DataKey::LiquidityAdvanceCounter, &advance_id);
        
        // Update member history
        let mut updated_history = member_history;
        updated_history.total_advances_taken += 1;
        updated_history.total_advance_amount += max_advance;
        updated_history.current_round_advances += 1;
        updated_history.last_advance_timestamp = Some(env.ledger().timestamp());
        updated_history.repayment_history.push_back(advance_id);
        
        env.storage().instance().set(&DataKey::MemberAdvanceHistory(member.clone()), &updated_history);
        
        // Update statistics
        let mut stats: LiquidityBufferStats = env.storage().instance()
            .get(&DataKey::LiquidityBufferStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.total_advances_provided += 1;
        stats.total_advance_amount += max_advance;
        stats.last_updated = env.ledger().timestamp();
        
        // Update average advance size
        if stats.total_advances_provided > 0 {
            stats.average_advance_size = stats.total_advance_amount / stats.total_advances_provided as i128;
        }
        
        env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: member,
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: advance_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
        
        advance_id
    }

    // Provide advance to member
    pub fn provide_advance(env: Env, advance_id: u64) {
        // Get advance and validate
        let mut advance: LiquidityAdvance = env.storage().instance()
            .get(&DataKey::LiquidityAdvance(advance_id))
            .unwrap_or_else(|| panic!("Advance not found"));
        
        if advance.status != LiquidityAdvanceStatus::Pending {
            panic!("Advance is not in pending status");
        }
        
        // Get reserve balance
        let mut reserve_balance: i128 = env.storage().instance()
            .get(&DataKey::LiquidityBufferReserve)
            .unwrap_or(0);
        
        if reserve_balance < advance.advance_amount {
            panic!("Insufficient reserve balance");
        }
        
        // Update advance status
        advance.status = LiquidityAdvanceStatus::Active;
        advance.provided_timestamp = Some(env.ledger().timestamp());
        
        // Deduct from reserve
        reserve_balance -= advance.advance_amount;
        env.storage().instance().set(&DataKey::LiquidityBufferReserve, &reserve_balance);
        
        // Store updated advance
        env.storage().instance().set(&DataKey::LiquidityAdvance(advance_id), &advance);
        
        // Update statistics
        let mut stats: LiquidityBufferStats = env.storage().instance()
            .get(&DataKey::LiquidityBufferStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.active_advances_count += 1;
        stats.last_updated = env.ledger().timestamp();
        
        // Update utilization rate
        let config: LiquidityBufferConfig = env.storage().instance()
            .get(&DataKey::LiquidityBufferConfig)
            .unwrap_or_else(|| panic!("Config not found"));
        
        if config.max_reserve > 0 {
            stats.buffer_utilization_rate = ((config.max_reserve - reserve_balance) * 10000) / config.max_reserve;
        }
        
        env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
        
        // In a real implementation, this would transfer tokens to the member
        // For now, we just update the state
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: advance.member.clone(),
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: advance_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Cancel advance request
    pub fn cancel_advance_request(env: Env, advance_id: u64) {
        // Get advance and validate
        let mut advance: LiquidityAdvance = env.storage().instance()
            .get(&DataKey::LiquidityAdvance(advance_id))
            .unwrap_or_else(|| panic!("Advance not found"));
        
        if advance.status != LiquidityAdvanceStatus::Pending {
            panic!("Cannot cancel advance in current status");
        }
        
        // Check authorization
        advance.member.require_auth();
        
        // Update advance status
        advance.status = LiquidityAdvanceStatus::Cancelled;
        
        // Store updated advance
        env.storage().instance().set(&DataKey::LiquidityAdvance(advance_id), &advance);
        
        // Update member history
        let mut member_history: MemberAdvanceHistory = env.storage().instance()
            .get(&DataKey::MemberAdvanceHistory(advance.member.clone()))
            .unwrap_or_else(|| panic!("Member history not found"));
        
        member_history.current_round_advances -= 1;
        member_history.total_advances_taken -= 1;
        member_history.total_advance_amount -= advance.advance_amount;
        
        env.storage().instance().set(&DataKey::MemberAdvanceHistory(advance.member.clone()), &member_history);
        
        // Update statistics
        let mut stats: LiquidityBufferStats = env.storage().instance()
            .get(&DataKey::LiquidityBufferStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.total_advances_provided -= 1;
        stats.total_advance_amount -= advance.advance_amount;
        stats.last_updated = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
        
        // Log audit entry
        let audit_count: u64 = env.storage().instance()
            .get(&DataKey::AuditCount)
            .unwrap_or(0);
        
        let audit_entry = AuditEntry {
            id: audit_count,
            actor: advance.member.clone(),
            action: AuditAction::AdminAction,
            timestamp: env.ledger().timestamp(),
            resource_id: advance_id,
        };
        
        env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
        env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
    }

    // Process advance refill from member deposit
    pub fn process_advance_refill(env: Env, member: Address, circle_id: u64, deposit_amount: i128) {
        // Get member's advance history
        let member_history: MemberAdvanceHistory = env.storage().instance()
            .get(&DataKey::MemberAdvanceHistory(member.clone()))
            .unwrap_or_else(|| return); // No advances to refill
        
        // Find active advances for this member and circle
        let mut refilled_amount = 0;
        
        for advance_id in member_history.repayment_history.iter() {
            let mut advance: LiquidityAdvance = env.storage().instance()
                .get(&DataKey::LiquidityAdvance(*advance_id))
                .unwrap_or_else(|| continue);
            
            // Only process advances for the same circle
            if advance.circle_id != circle_id {
                continue;
            }
            
            // Only process active advances
            if advance.status != LiquidityAdvanceStatus::Active {
                continue;
            }
            
            // Calculate how much to apply to this advance
            let remaining_amount = advance.repayment_amount - advance.advance_amount;
            let apply_amount = if deposit_amount - refilled_amount >= remaining_amount {
                remaining_amount
            } else {
                deposit_amount - refilled_amount
            };
            
            if apply_amount > 0 {
                // Update advance
                advance.advance_amount += apply_amount;
                
                // Check if fully repaid
                if advance.advance_amount >= advance.repayment_amount {
                    advance.status = LiquidityAdvanceStatus::Completed;
                    advance.repaid_timestamp = Some(env.ledger().timestamp());
                    
                    // Update member history
                    let mut updated_history = member_history.clone();
                    updated_history.total_fees_paid += advance.advance_fee;
                    updated_history.current_round_advances = 
                        updated_history.current_round_advances.saturating_sub(1);
                    
                    env.storage().instance().set(&DataKey::MemberAdvanceHistory(member.clone()), &updated_history);
                    
                    // Update statistics
                    let mut stats: LiquidityBufferStats = env.storage().instance()
                        .get(&DataKey::LiquidityBufferStats)
                        .unwrap_or_else(|| panic!("Stats not found"));
                    
                    stats.total_advances_completed += 1;
                    stats.total_fees_collected += advance.advance_fee;
                    stats.active_advances_count = stats.active_advances_count.saturating_sub(1);
                    stats.last_updated = env.ledger().timestamp();
                    
                    env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
                }
                
                // Refill reserve
                let mut reserve_balance: i128 = env.storage().instance()
                    .get(&DataKey::LiquidityBufferReserve)
                    .unwrap_or(0);
                
                reserve_balance += apply_amount;
                env.storage().instance().set(&DataKey::LiquidityBufferReserve, &reserve_balance);
                
                // Store updated advance
                env.storage().instance().set(&DataKey::LiquidityAdvance(*advance_id), &advance);
                
                refilled_amount += apply_amount;
                
                // Log audit entry
                let audit_count: u64 = env.storage().instance()
                    .get(&DataKey::AuditCount)
                    .unwrap_or(0);
                
                let audit_entry = AuditEntry {
                    id: audit_count,
                    actor: member.clone(),
                    action: AuditAction::AdminAction,
                    timestamp: env.ledger().timestamp(),
                    resource_id: *advance_id,
                };
                
                env.storage().instance().set(&DataKey::AuditEntry(audit_count), &audit_entry);
                env.storage().instance().set(&DataKey::AuditCount, &(audit_count + 1));
            }
            
            if refilled_amount >= deposit_amount {
                break;
            }
        }
    }

    // Check advance eligibility
    pub fn check_advance_eligibility(env: Env, member: Address, circle_id: u64) -> bool {
        // Get liquidity buffer config
        let config: LiquidityBufferConfig = env.storage().instance()
            .get(&DataKey::LiquidityBufferConfig)
            .unwrap_or_else(|| return false);
        
        // Check if buffer is enabled
        if !config.is_enabled {
            return false;
        }
        
        // Get member reputation
        let user_stats: UserStats = env.storage().instance()
            .get(&DataKey::UserStats(member.clone()))
            .unwrap_or_else(|| UserStats {
                total_volume_saved: 0,
                on_time_contributions: 0,
                late_contributions: 0,
            });
        
        // Calculate reputation score
        let total_contributions = user_stats.on_time_contributions + user_stats.late_contributions;
        let reputation_score = if total_contributions > 0 {
            (user_stats.on_time_contributions * 10000) / total_contributions
        } else {
            0
        };
        
        // Check minimum reputation requirement
        if reputation_score < config.min_reputation {
            return false;
        }
        
        // Check if member is part of the circle
        let circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| return false);
        
        if !circle.members.contains(&member) {
            return false;
        }
        
        // Check if member has any defaulted advances
        let member_history: MemberAdvanceHistory = env.storage().instance()
            .get(&DataKey::MemberAdvanceHistory(member.clone()))
            .unwrap_or_else(|| MemberAdvanceHistory {
                member: member.clone(),
                total_advances_taken: 0,
                total_advance_amount: 0,
                total_fees_paid: 0,
                current_round_advances: 0,
                last_advance_timestamp: None,
                repayment_history: Vec::new(&env),
                default_count: 0,
                reputation_score,
            });
        
        if member_history.default_count > 0 {
            return false;
        }
        
        true
    }

    // Get liquidity advance
    pub fn get_liquidity_advance(env: Env, advance_id: u64) -> LiquidityAdvance {
        env.storage().instance()
            .get(&DataKey::LiquidityAdvance(advance_id))
            .unwrap_or_else(|| panic!("Advance not found"))
    }

    // Get member advance history
    pub fn get_member_advance_history(env: Env, member: Address) -> MemberAdvanceHistory {
        env.storage().instance()
            .get(&DataKey::MemberAdvanceHistory(member))
            .unwrap_or_else(|| MemberAdvanceHistory {
                member,
                total_advances_taken: 0,
                total_advance_amount: 0,
                total_fees_paid: 0,
                current_round_advances: 0,
                last_advance_timestamp: None,
                repayment_history: Vec::new(&env),
                default_count: 0,
                reputation_score: 10000,
            })
    }

    // Get liquidity buffer statistics
    pub fn get_liquidity_buffer_stats(env: Env) -> LiquidityBufferStats {
        env.storage().instance()
            .get(&DataKey::LiquidityBufferStats)
            .unwrap_or_else(|| panic!("Stats not found"))
    }

    // Allocate platform fees to buffer
    pub fn allocate_platform_fees_to_buffer(env: Env, fee_amount: i128) {
        // Get allocation tracking
        let mut allocation: PlatformFeeAllocation = env.storage().instance()
            .get(&DataKey::PlatformFeeAllocation)
            .unwrap_or_else(|| PlatformFeeAllocation {
                total_fees_collected: 0,
                buffer_allocation_amount: 0,
                treasury_allocation_amount: 0,
                last_allocation_timestamp: env.ledger().timestamp(),
                allocation_frequency: 86400,
            });
        
        // Get config
        let config: LiquidityBufferConfig = env.storage().instance()
            .get(&DataKey::LiquidityBufferConfig)
            .unwrap_or_else(|| panic!("Config not found"));
        
        // Calculate allocation amounts
        let buffer_amount = (fee_amount * config.platform_fee_allocation) / 10000;
        let treasury_amount = fee_amount - buffer_amount;
        
        // Update allocation tracking
        allocation.total_fees_collected += fee_amount;
        allocation.buffer_allocation_amount += buffer_amount;
        allocation.treasury_allocation_amount += treasury_amount;
        allocation.last_allocation_timestamp = env.ledger().timestamp();
        
        // Update reserve balance
        let mut reserve_balance: i128 = env.storage().instance()
            .get(&DataKey::LiquidityBufferReserve)
            .unwrap_or(0);
        
        reserve_balance += buffer_amount;
        
        // Enforce maximum reserve limit
        if reserve_balance > config.max_reserve {
            reserve_balance = config.max_reserve;
        }
        
        // Store updated values
        env.storage().instance().set(&DataKey::PlatformFeeAllocation, &allocation);
        env.storage().instance().set(&DataKey::LiquidityBufferReserve, &reserve_balance);
        
        // Update statistics
        let mut stats: LiquidityBufferStats = env.storage().instance()
            .get(&DataKey::LiquidityBufferStats)
            .unwrap_or_else(|| panic!("Stats not found"));
        
        stats.total_reserve_balance = reserve_balance;
        stats.total_platform_fees_allocated = allocation.buffer_allocation_amount;
        stats.last_updated = env.ledger().timestamp();
        
        env.storage().instance().set(&DataKey::LiquidityBufferStats, &stats);
    }


    #[cfg(test)]
mod governance_token_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};

    #[contract]
    pub struct MockGovernanceToken;

    #[contractimpl]
    impl MockGovernanceToken {
        pub fn mint(env: Env, to: Address, amount: u64) {
            // Mock implementation - in real implementation this would mint tokens
        }
        
        pub fn transfer(env: Env, from: Address, to: Address, amount: u64) {
            // Mock implementation
        }
    }

    #[contract]
    pub struct MockNft;

    #[contractimpl]
    impl MockNft {
        pub fn mint(_env: Env, _to: Address, _id: u128) {}
        pub fn burn(_env: Env, _from: Address, _id: u128) {}
    }

    #[test]
    fn test_governance_token_mining_setup() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Set governance token
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Verify mining is enabled
        let config: MiningConfig = env.storage().instance().get(&DataKey::MiningConfig).unwrap();
        assert!(config.is_mining_enabled);

        // Verify governance token is set
        let stored_token: Address = env.storage().instance().get(&DataKey::GovernanceToken).unwrap();
        assert_eq!(stored_token, governance_token);
    }

    #[test]
    fn test_mining_on_successful_contribution() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        // Mock token balance and authorization
        env.mock_all_auths();

        // Make deposit (should trigger mining)
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // Check user mining stats
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_contributions, 1);
        assert_eq!(stats.total_tokens_earned, 100);
        assert_eq!(stats.total_tokens_claimed, 0);

        // Check user vesting info
        let vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(vesting.total_allocated, 100);
        assert_eq!(vesting.contributions_made, 1);
        assert!(vesting.is_active);
    }

    #[test]
    fn test_mining_disabled_when_token_not_set() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract but don't set governance token
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // Make deposit (should not trigger mining since token not set)
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // Check no mining occurred
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_tokens_earned, 0);

        let vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(vesting.total_allocated, 0);
        assert!(!vesting.is_active);
    }

    #[test]
    fn test_vesting_calculation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining with 12 cycle vesting, 3 cycle cliff
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Simulate user vesting info
        let vesting_key = DataKey::UserVesting(user.clone());
        let vesting_info = UserVestingInfo {
            total_allocated: 1200, // 12 contributions * 100 tokens
            vested_amount: 0,
            claimed_amount: 0,
            start_cycle: 0,
            contributions_made: 12,
            is_active: true,
        };
        env.storage().instance().set(&vesting_key, &vesting_info);

        // Test vesting calculation
        let vested_0 = SoroSusu::calculate_vested_amount(1200, 0, 0, 12);
        assert_eq!(vested_0, 0); // Before cliff

        let vested_3 = SoroSusu::calculate_vested_amount(1200, 0, 3, 12);
        assert_eq!(vested_3, 0); // At cliff

        let vested_6 = SoroSusu::calculate_vested_amount(1200, 0, 6, 12);
        assert_eq!(vested_6, 300); // 3 cycles after cliff = 25% vested

        let vested_12 = SoroSusu::calculate_vested_amount(1200, 0, 12, 12);
        assert_eq!(vested_12, 1200); // Fully vested

        let vested_15 = SoroSusu::calculate_vested_amount(1200, 0, 15, 12);
        assert_eq!(vested_15, 1200); // Fully vested (past end)
    }

    #[test]
    fn test_token_claiming() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        // Configure mining
        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Set up user vesting with vested tokens
        let vesting_key = DataKey::UserVesting(user.clone());
        let mut vesting_info = UserVestingInfo {
            total_allocated: 1200,
            vested_amount: 300,
            claimed_amount: 0,
            start_cycle: 0,
            contributions_made: 12,
            is_active: true,
        };
        env.storage().instance().set(&vesting_key, &vesting_info);

        // Mock user authorization
        user.require_auth();

        // Claim tokens
        SoroSusuTrait::claim_vested_tokens(env.clone(), user.clone());

        // Verify claim updated stats
        let updated_stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(updated_stats.total_tokens_claimed, 300);

        let updated_vesting = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert_eq!(updated_vesting.claimed_amount, 300);
    }

    #[test]
    fn test_max_mining_limit_per_circle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining with low limit
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 150, // Low limit
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join users
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            2,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);

        env.mock_all_auths();

        // First user deposits (should mine 100 tokens)
        SoroSusuTrait::deposit(env.clone(), user1.clone(), circle_id);
        let stats1 = SoroSusuTrait::get_mining_stats(env.clone(), user1.clone());
        assert_eq!(stats1.total_tokens_earned, 100);

        // Second user deposits (should mine 50 tokens, hitting the limit)
        SoroSusuTrait::deposit(env.clone(), user2.clone(), circle_id);
        let stats2 = SoroSusuTrait::get_mining_stats(env.clone(), user2.clone());
        assert_eq!(stats2.total_tokens_earned, 50); // Only 50 remaining from limit

        // Check total mined
        let total_mined: u64 = env.storage().instance().get(&DataKey::TotalMinedTokens).unwrap();
        assert_eq!(total_mined, 150); // At the limit
    }

    #[test]
    fn test_mining_config_validation() {
        let env = Env::default();
        let admin = Address::generate(&env);

        SoroSusuTrait::init(env.clone(), admin.clone());

        // Test invalid config - zero tokens per contribution
        let invalid_config1 = MiningConfig {
            tokens_per_contribution: 0,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };

        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::configure_mining(env.clone(), admin.clone(), invalid_config1);
        });
        assert!(result.is_err());

        // Test invalid config - cliff longer than vesting
        let invalid_config2 = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 6,
            cliff_cycles: 8,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };

        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::configure_mining(env.clone(), admin.clone(), invalid_config2);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_cycle_completion_and_vesting_progress() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 4, // Short vesting for testing
            cliff_cycles: 1,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create 2-member circle
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            2,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);

        env.mock_all_auths();

        // Both users deposit - should complete cycle
        SoroSusuTrait::deposit(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::deposit(env.clone(), user2.clone(), circle_id);

        // Check cycle was completed
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle.cycle_count, 1);
        assert_eq!(circle.contribution_bitmap, 0); // Reset for next cycle

        // Check users started vesting from cycle 0
        let vesting1 = SoroSusuTrait::get_user_vesting_info(env.clone(), user1.clone());
        assert_eq!(vesting1.start_cycle, 0);
        assert!(vesting1.is_active);
    }

    #[test]
    fn test_ejected_member_mining_deactivation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // User deposits and earns tokens
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        
        let vesting_before = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert!(vesting_before.is_active);

        // Eject member
        SoroSusuTrait::eject_member(env.clone(), creator.clone(), circle_id, user.clone());

        // Check vesting was deactivated
        let vesting_after = SoroSusuTrait::get_user_vesting_info(env.clone(), user.clone());
        assert!(!vesting_after.is_active);
    }

    #[test]
    fn test_save_to_earn_events() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let governance_token = env.register_contract(None, MockGovernanceToken);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize and set up mining
        SoroSusuTrait::init(env.clone(), admin.clone());
        SoroSusuTrait::set_governance_token(env.clone(), admin.clone(), governance_token.clone());

        let config = MiningConfig {
            tokens_per_contribution: 100,
            vesting_duration_cycles: 12,
            cliff_cycles: 3,
            max_mining_per_circle: 1000,
            is_mining_enabled: true,
        };
        SoroSusuTrait::configure_mining(env.clone(), admin.clone(), config);

        // Create circle and join
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            0,
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // Make deposit and check for events
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);

        // In a real test environment, you would verify the events were emitted
        // For this mock test, we just ensure the function completes without error
        let stats = SoroSusuTrait::get_mining_stats(env.clone(), user.clone());
        assert_eq!(stats.total_tokens_earned, 100);
    }
}

}
