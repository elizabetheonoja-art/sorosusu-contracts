#![no_std]


// --- DATA STRUCTURES ---

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),

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
    pub shares: u32, // New field for shares (1 = standard, 2 = double contribution/payout)
    pub referrer: Option<Address>,
    pub buddy: Option<Address>,

}

#[contracttype]
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: i128,
    pub max_members: u32,
    pub member_count: u32,
    pub total_shares: u32, // Total shares in the circle (sum of all member shares)
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
    pub requires_collateral: bool,
    pub collateral_bps: u32,
    pub member_addresses: Vec<Address>,
    pub leniency_enabled: bool,
    pub grace_period_end: Option<u64>,
    pub quadratic_voting_enabled: bool,
    pub proposal_count: u32,
    pub dissolution_status: u32,
    pub dissolution_deadline: u64,
    pub proposed_late_fee_bps: u32,
    pub proposal_votes_bitmap: u64,
    pub recovery_old_address: Option<Address>,
    pub recovery_new_address: Option<Address>,
    pub recovery_votes_bitmap: u64,
    pub arbitrator: Address,
}

// --- CONTRACT CLIENTS ---

}

#[contracttype]
#[derive(Clone)]
pub struct NftBadgeMetadata {
    pub volume_tier: u32,        // 1=Bronze, 2=Silver, 3=Gold based on total_volume_saved
    pub perfect_attendance: bool, // true if zero late contributions
    pub group_lead_status: bool,  // true if member is the circle creator
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
    fn mint_badge(env: Env, to: Address, token_id: u128, metadata: NftBadgeMetadata);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: i128);
    fn withdraw(env: Env, token: Address, to: Address, amount: i128);
}

pub trait SoroSusuTrait {
    fn init(env: Env, admin: Address);
    fn set_lending_pool(env: Env, admin: Address, pool: Address);
    fn set_protocol_fee(env: Env, admin: Address, fee_basis_points: u32, treasury: Address);

    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
    ) -> u64;

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        shares: u32, // New shares parameter (1 = standard, 2 = double)
        referrer: Option<Address>,
    );
    fn deposit(env: Env, user: Address, circle_id: u64);

    fn finalize_round(env: Env, caller: Address, circle_id: u64);
    fn claim_pot(env: Env, user: Address, circle_id: u64);

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32);
    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64);
    fn vote_penalty_change(env: Env, user: Address, circle_id: u64);

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    );
    fn vote_for_recovery(env: Env, user: Address, circle_id: u64);

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);

    fn pair_with_member(env: Env, user: Address, buddy_address: Address);
    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128);

    // Rollover Bonus Incentive Logic
    fn propose_rollover_bonus(env: Env, user: Address, circle_id: u64, fee_percentage_bps: u32);
    fn vote_rollover_bonus(env: Env, user: Address, circle_id: u64, vote_choice: RolloverVoteChoice);
    fn apply_rollover_bonus(env: Env, circle_id: u64);

    // Idle Pot Yield Delegation to Stellar Pools
    fn propose_yield_delegation(env: Env, user: Address, circle_id: u64, delegation_percentage: u32, pool_address: Address, pool_type: YieldPoolType);
    fn vote_yield_delegation(env: Env, user: Address, circle_id: u64, vote_choice: YieldVoteChoice);
    fn approve_yield_delegation(env: Env, circle_id: u64);
    fn execute_yield_delegation(env: Env, circle_id: u64);
    fn compound_yield(env: Env, circle_id: u64);
    fn withdraw_yield_delegation(env: Env, circle_id: u64);
    fn distribute_yield_earnings(env: Env, circle_id: u64);

    // Path Payment Contribution Support
    fn propose_path_payment_support(env: Env, user: Address, circle_id: u64);
    fn vote_path_payment_support(env: Env, user: Address, circle_id: u64, vote_choice: PathPaymentVoteChoice);
    fn approve_path_payment_support(env: Env, circle_id: u64);
    fn execute_path_payment(env: Env, user: Address, circle_id: u64, source_token: Address, source_amount: i128);
    fn register_supported_token(env: Env, user: Address, token_address: Address, token_symbol: String, decimals: u32, is_stable: bool);
    fn register_dex(env: Env, user: Address, dex_address: Address, dex_name: String, is_trusted: bool);

    // Inter-contract reputation query interface
    fn get_reputation(env: Env, user: Address) -> ReputationData;
}

// --- IMPLEMENTATION ---

fn append_audit_index(env: &Env, key: DataKey, audit_id: u64) {
    let mut ids: Vec<u64> = env.storage().instance().get(&key).unwrap_or(Vec::new(env));
    ids.push_back(audit_id);
    env.storage().instance().set(&key, &ids);
}

fn write_audit(env: &Env, actor: &Address, action: AuditAction, resource_id: u64) {
    let mut audit_count: u64 = env.storage().instance().get(&DataKey::AuditCount).unwrap_or(0);
    audit_count += 1;

    let entry = AuditEntry {
        id: audit_count,
        actor: actor.clone(),
        action,
        timestamp: env.ledger().timestamp(),
        resource_id,
    };

    env.storage()
        .instance()
        .set(&DataKey::AuditEntry(audit_count), &entry);
    env.storage().instance().set(&DataKey::AuditCount, &audit_count);

    append_audit_index(env, DataKey::AuditAll, audit_count);
    append_audit_index(env, DataKey::AuditByActor(actor.clone()), audit_count);
    append_audit_index(env, DataKey::AuditByResource(resource_id), audit_count);

    env.events().publish(
        (symbol_short!("AUDIT"), actor.clone(), resource_id),
        (audit_count, entry.timestamp),
    );
}

fn append_audit_index(env: &Env, index_key: DataKey, audit_id: u64) {
    let mut index: Vec<u64> = env.storage().instance().get(&index_key).unwrap_or(Vec::new(env));
    index.push_back(audit_id);
    env.storage().instance().set(&index_key, &index);
}

fn calculate_rollover_bonus(env: &Env, circle_id: u64, fee_percentage_bps: u32) -> i128 {
    // Get the protocol fee settings
    let fee_bps: u32 = env.storage().instance().get(&DataKey::ProtocolFeeBps).unwrap_or(0);
    if fee_bps == 0 {
        return 0; // No protocol fee, no bonus
    }

    // Calculate the total pot amount for this circle
    let circle_key = DataKey::Circle(circle_id);
    let circle: CircleInfo = env.storage().instance().get(&circle_key)
        .expect("Circle not found");
    
    let total_pot = circle.contribution_amount * (circle.total_shares as i128);
    
    // Calculate the platform fee that would be charged
    let platform_fee = (total_pot * fee_bps as i128) / 10000;
    
    // Calculate the rollover bonus (percentage of platform fee to refund)
    let bonus_amount = (platform_fee * fee_percentage_bps as i128) / 10000;
    
    bonus_amount
}

fn get_member_address_by_index(circle: &CircleInfo, index: u32) -> Address {
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    circle.member_addresses.get(index).unwrap()
}

fn execute_stellar_path_payment(env: &Env, source_token: &Address, target_token: &Address, source_amount: i128, max_slippage_bps: u32) -> (i128, i128, u32) {
    // This is a simplified implementation - in production would call actual Stellar Path Payment
    // For now, we'll simulate the swap with a basic exchange rate
    
    // Get token info for decimals
    let source_token_key = DataKey::SupportedTokens(source_token.clone());
    let source_token_info: SupportedToken = env.storage().instance().get(&source_token_key)
        .expect("Source token not supported");
    
    let target_token_key = DataKey::SupportedTokens(target_token.clone());
    let target_token_info: SupportedToken = env.storage().instance().get(&target_token_key)
        .expect("Target token not supported");

    // Calculate exchange rate (simplified - would use actual DEX rates)
    // Assume 1:1 rate for demonstration, adjust based on token types
    let rate_adjustment = if source_token_info.is_stable && !target_token_info.is_stable {
        10000 // Stable to volatile might need premium
    } else if !source_token_info.is_stable && target_token_info.is_stable {
        9800 // Stable to stable might have small discount
    } else {
        10000 // Default 1:1 rate
    };

    let exchange_rate = rate_adjustment;
    let target_amount = (source_amount * exchange_rate) / 10000;
    
    // Calculate slippage (0 for this simplified implementation)
    let slippage_bps = 0;
    
    // In real implementation, this would:
    // 1. Call Stellar Path Payment contract
    // 2. Handle slippage protection
    // 3. Handle partial fills
    // 4. Handle failed transactions
    
    (target_amount, exchange_rate, slippage_bps)
}

fn count_active_members(env: &Env, circle: &CircleInfo) -> u32 {
    let mut active_count = 0u32;
    for i in 0..circle.member_count {
        let member_address = circle.member_addresses.get(i).unwrap();
        let key = DataKey::Member(member_address);
        if let Some(member) = env.storage().instance().get::<DataKey, Member>(&key) {
            if member.status == MemberStatus::Active {
                active_count += 1;
            }
        }
    }
    active_count
}

fn apply_recovery_if_consensus(env: &Env, actor: &Address, circle_id: u64, circle: &mut CircleInfo) {
    let active_members = count_active_members(env, circle);
    if active_members == 0 {
        panic!("No active members");
    }

    let votes = circle.recovery_votes_bitmap.count_ones();
    if votes * 100 <= active_members * 70 {
        return;
    }

    let old_address = circle
        .recovery_old_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));
    let new_address = circle
        .recovery_new_address
        .clone()
        .unwrap_or_else(|| panic!("No recovery proposal"));

    let old_member_key = DataKey::Member(old_address);
    let mut old_member: Member = env
        .storage()
        .instance()
        .get(&old_member_key)
        .unwrap_or_else(|| panic!("Old member not found"));

    if old_member.status != MemberStatus::Active {
        panic!("Only active members can be recovered");
    }

    let new_member_key = DataKey::Member(new_address.clone());
    if env.storage().instance().has(&new_member_key) {
        panic!("New address is already a member");
    }

    old_member.address = new_address.clone();
    env.storage().instance().set(&new_member_key, &old_member);
    env.storage().instance().remove(&old_member_key);

    circle
        .member_addresses
        .set(old_member.index, new_address);
    circle.recovery_old_address = None;
    circle.recovery_new_address = None;
    circle.recovery_votes_bitmap = 0;

    write_audit(env, actor, AuditAction::AdminAction, circle_id);
}

fn query_from_indexed_ids(
    env: &Env,
    ids: Vec<u64>,
    start_time: u64,
    end_time: u64,
    offset: u32,
    limit: u32,
) -> Vec<AuditEntry> {
    let mut output = Vec::new(env);
    if limit == 0 || start_time > end_time {
        return output;
    }

    let bounded_limit = if limit > MAX_QUERY_LIMIT {
        MAX_QUERY_LIMIT
    } else {
        limit
    };

    let mut skipped = 0u32;
    for i in 0..ids.len() {
        let id = ids.get(i).unwrap();
        let entry: AuditEntry = env
            .storage()
            .instance()
            .get(&DataKey::AuditEntry(id))
            .unwrap_or_else(|| panic!("Audit entry missing"));

        if entry.timestamp < start_time || entry.timestamp > end_time {
            continue;
        }

        if skipped < offset {
            skipped += 1;
            continue;
        }

        if output.len() >= bounded_limit {
            break;
        }

        output.push_back(entry);
    }

    output
}

    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

fn execute_proposal_logic(env: &Env, proposal: &Proposal) {
    let proposal_key = DataKey::Proposal(proposal.id);
    let mut updated_proposal = proposal.clone();
    updated_proposal.status = ProposalStatus::Executed;
    env.storage().instance().set(&proposal_key, &updated_proposal);
}

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        env.storage().instance().set(&DataKey::Admin, &admin);
        env.storage().instance().set(&DataKey::CircleCount, &0u64);
        env.storage().instance().set(&DataKey::AuditCount, &0u64);
    }

    fn set_lending_pool(env: Env, admin: Address, pool: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        env.storage().instance().set(&DataKey::LendingPool, &pool);
        write_audit(&env, &admin, AuditAction::AdminAction, 0);
    }

    fn set_protocol_fee(env: Env, admin: Address, fee_basis_points: u32, treasury: Address) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Not initialized");
        if admin != stored_admin {
            panic!("Unauthorized");
        }
        if fee_basis_points > 10000 {
            panic!("InvalidFeeConfig");
        }
        env.storage().instance().set(&DataKey::ProtocolFeeBps, &fee_basis_points);
        env.storage().instance().set(&DataKey::ProtocolTreasury, &treasury);
    }

    fn create_circle(
        env: Env,
        creator: Address,
        amount: i128,
        max_members: u32,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
    ) -> u64 {
        creator.require_auth();
        if max_members == 0 {
            panic!("Max members must be greater than zero");
        }

        let current_time = env.ledger().timestamp();
        let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());
        if let Some(last_created) = env.storage().instance().get::<DataKey, u64>(&rate_limit_key) {
            if current_time < last_created + RATE_LIMIT_SECONDS {
                panic!("Rate limit exceeded");
            }
        }
        env.storage().instance().set(&rate_limit_key, &current_time);

        let mut circle_count: u64 = env
            .storage()
            .instance()
            .get(&DataKey::CircleCount)
            .unwrap_or(0);
        circle_count += 1;

        // Calculate total cycle value and determine collateral requirements
        let total_cycle_value = amount * (max_members as i128);
        let requires_collateral = total_cycle_value >= HIGH_VALUE_THRESHOLD;
        let collateral_bps = if requires_collateral { DEFAULT_COLLATERAL_BPS } else { 0 };

        let new_circle = CircleInfo {
            id: circle_count,
            creator,
            contribution_amount: amount,
            max_members,
            member_count: 0,
            total_shares: 0, // Initialize total shares
            current_recipient_index: 0,
            is_active: true,
            token,
            deadline_timestamp: current_time + cycle_duration,
            cycle_duration,
            contribution_bitmap: 0,
            insurance_balance: 0,
            insurance_fee_bps,
            is_insurance_used: false,
            late_fee_bps: 100,
            nft_contract,
            is_round_finalized: false,
            current_pot_recipient: None,
            requires_collateral,
            collateral_bps,
            member_addresses: Vec::new(&env),
            leniency_enabled: true,
            grace_period_end: None,
            quadratic_voting_enabled: max_members >= MIN_GROUP_SIZE_FOR_QUADRATIC,
            proposal_count: 0,
            dissolution_status: 0,
            dissolution_deadline: 0,
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            recovery_old_address: None,
            recovery_new_address: None,
            recovery_votes_bitmap: 0,
            arbitrator,
        };

        env.storage()
            .instance()
            .set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);
        circle_count
    }

    fn join_circle(
        env: Env,
        user: Address,
        circle_id: u64,
        tier_multiplier: u32,
        shares: u32, // New shares parameter (1 = standard, 2 = double)
        referrer: Option<Address>,
    ) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        // Validate shares parameter (must be 1 or 2)
        if shares != 1 && shares != 2 {
            panic!("Shares must be either 1 (standard) or 2 (double)");
        }

        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("Already member");
        }

        // Check collateral requirement for high-value circles
        if circle.requires_collateral {
            let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
            let collateral_info: Option<CollateralInfo> = env.storage().instance().get(&collateral_key);
            
            match collateral_info {
                Some(collateral) => {
                    if collateral.status != CollateralStatus::Staked {
                        panic!("Collateral not properly staked");
                    }
                }
                None => panic!("Collateral required for this circle"),
            }
        }

        let new_member = Member {
            address: user.clone(),
            index: circle.member_count,
            contribution_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
            tier_multiplier: shares, // Set tier_multiplier equal to shares for backward compatibility
            shares,
            referrer,
            buddy: None,
        };

        env.storage().instance().set(&member_key, &new_member);
        env.storage().instance().set(&DataKey::CircleMember(circle_id, circle.member_count), &user);
        circle.member_count += 1;
        circle.total_shares += shares; // Update total shares
        circle.member_addresses.push_back(user.clone());
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        if member.status != MemberStatus::Active {
            panic!("Member not active");
        }

        let current_time = env.ledger().timestamp();
        let base_amount = circle.contribution_amount * member.tier_multiplier as i128;
        let mut penalty_amount = 0i128;
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        // Check if contribution is late
        if current_time > circle.deadline_timestamp {
            let base_penalty = (base_amount * circle.late_fee_bps as i128) / 10000;
            // Apply referral discount
            let mut discount = 0i128;
            if let Some(ref_addr) = &member.referrer {
                let ref_key = DataKey::Member(ref_addr.clone());
                if env.storage().instance().has(&ref_key) {
                    discount = (base_penalty * REFERRAL_DISCOUNT_BPS as i128) / 10000;
                }
            }
            penalty_amount = base_penalty - discount;

            let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve);
        }

        // Update user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        if penalty_amount > 0 {
            user_stats.late_contributions += 1;
        } else {
            user_stats.on_time_contributions += 1;
        }

        user_stats.total_volume_saved += base_amount;
        env.storage().instance().set(&user_stats_key, &user_stats);

        env.events().publish(
            (Symbol::new(&env, "USER_STATS"), user.clone()),
            (user_stats.on_time_contributions, user_stats.late_contributions, user_stats.total_volume_saved)
        );

        let insurance_fee = (base_amount * circle.insurance_fee_bps as i128) / 10000;
        let total_amount = base_amount + insurance_fee + penalty_amount;

        let token_client = token::Client::new(&env, &circle.token);
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &total_amount);
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            if let Some(buddy_addr) = member.buddy.clone() {
                let safety_key = DataKey::SafetyDeposit(buddy_addr, circle_id);
                let safety_balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
                if safety_balance < total_amount {
                    panic!("Insufficient funds and buddy deposit");
                }
                env.storage()
                    .instance()
                    .set(&safety_key, &(safety_balance - total_amount));
            } else {
                panic!("Insufficient funds");
            }
        }

        if insurance_fee > 0 {
            circle.insurance_balance += insurance_fee;
        }

        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn finalize_round(env: Env, caller: Address, circle_id: u64) {
        caller.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");
        if caller != circle.creator && caller != stored_admin {
            panic!("Unauthorized");
        }

        if circle.is_round_finalized {
            panic!("Round already finalized");
        }

        let expected_bitmap = (1u64 << circle.member_count) - 1;
        if circle.contribution_bitmap != expected_bitmap {
            panic!("Not all contributed");
        }

        // Set round as finalized and determine next recipient
        circle.is_round_finalized = true;
        
        // Set next recipient (round-robin)
        let next_recipient_index = (circle.current_recipient_index + 1) % circle.member_count;
        let next_recipient = get_member_address_by_index(&circle, next_recipient_index);
        
        circle.current_recipient_index = next_recipient_index;
        circle.current_pot_recipient = Some(next_recipient);
        
        // Schedule payout time (end of month from now)
        let current_time = env.ledger().timestamp();
        let payout_time = current_time + (30 * 24 * 60 * 60); // 30 days from now
        env.storage().instance().set(&DataKey::ScheduledPayoutTime(circle_id), &payout_time);
        
        // Reset for next round
        circle.contribution_bitmap = 0;
        circle.deadline_timestamp = current_time + circle.cycle_duration;
        
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        
        // Publish round finalization event
        env.events().publish(
            (Symbol::new(&env, "ROUND_FINALIZED"), circle_id),
            (next_recipient, payout_time, next_recipient_index),
        );


    }

    fn claim_pot(env: Env, user: Address, circle_id: u64) {
        user.require_auth();
        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        if !circle.is_round_finalized {
            panic!("Round not finalized");
        }

        let recipient = circle
            .current_pot_recipient
            .clone()
            .unwrap_or_else(|| panic!("No recipient set"));
        if user != recipient {
            panic!("Unauthorized recipient");
        }

        let scheduled_time: u64 = env
            .storage()
            .instance()
            .get(&DataKey::ScheduledPayoutTime(circle_id))
            .expect("Payout not scheduled");
        if env.ledger().timestamp() < scheduled_time {
            panic!("Payout too early");
        }

        // Get member info to check shares
        let member_key = DataKey::Member(user.clone());
        let member_info: Member = env.storage().instance().get(&member_key)
            .expect("Member not found");

        // Calculate pot amount based on total shares, not member count
        let pot_amount = circle.contribution_amount * (circle.total_shares as i128);
        
        // Apply shares multiplier to payout (double payout for 2 shares)
        let mut total_payout = pot_amount;
        if member_info.shares == 2 {
            total_payout = pot_amount * 2; // Double payout for 2 shares
        }
        
        // Check for rollover bonus and add to first pot of new cycles
        let rollover_key = DataKey::RolloverBonus(circle_id);
        if let Some(rollover_bonus) = env.storage().instance().get::<DataKey, RolloverBonus>(&rollover_key) {
            if rollover_bonus.status == RolloverStatus::Applied {
                if let Some(applied_cycle) = rollover_bonus.applied_cycle {
                    if applied_cycle == circle.current_recipient_index {
                        total_payout += rollover_bonus.bonus_amount;
                        
                        env.events().publish(
                            (Symbol::new(&env, "ROLLOVER_BONUS_APPLIED"), circle_id, user.clone()),
                            (rollover_bonus.bonus_amount, applied_cycle),
                        );
                    }
                }
            }
        }
        
        let token_client = token::Client::new(&env, &circle.token);
        
        let fee_bps: u32 = env.storage().instance().get(&DataKey::ProtocolFeeBps).unwrap_or(0);
        if fee_bps > 0 {
            let treasury: Address = env.storage().instance().get(&DataKey::ProtocolTreasury).expect("Treasury not set");
            let fee = (total_payout * fee_bps as i128) / 10000;
            let net_payout = total_payout - fee;
            token_client.transfer(&env.current_contract_address(), &treasury, &fee);
            token_client.transfer(&env.current_contract_address(), &user, &net_payout);
        } else {
            token_client.transfer(&env.current_contract_address(), &user, &total_payout);
        }

        // Auto-release collateral if member has completed all contributions
        if circle.requires_collateral {
            let member_key = DataKey::Member(user.clone());
            if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
                if member_info.contribution_count >= circle.max_members {
                    let collateral_key = DataKey::CollateralVault(user.clone(), circle_id);
                    if let Some(mut collateral_info) = env.storage().instance().get::<DataKey, CollateralInfo>(&collateral_key) {
                        if collateral_info.status == CollateralStatus::Staked {
                            // Release collateral back to member
                            token_client.transfer(&env.current_contract_address(), &user, &collateral_info.amount);
                            
                            // Update collateral status
                            collateral_info.status = CollateralStatus::Released;
                            collateral_info.release_timestamp = Some(env.ledger().timestamp());
                            env.storage().instance().set(&collateral_key, &collateral_info);
                        }
                    }
                }
            }
        }

        circle.is_round_finalized = false;
        circle.contribution_bitmap = 0;
        circle.is_insurance_used = false;

        // Mint soulbound "Susu Master" badge when the full cycle completes
        let next_index = circle.current_recipient_index + 1;
        if next_index >= circle.max_members {
            let member_key = DataKey::Member(user.clone());
            if let Some(member_info) = env.storage().instance().get::<DataKey, Member>(&member_key) {
                let stats_key = DataKey::UserStats(user.clone());
                let stats: UserStats = env.storage().instance().get(&stats_key).unwrap_or(UserStats {
                    total_volume_saved: 0,
                    on_time_contributions: 0,
                    late_contributions: 0,
                });
                let volume_tier: u32 = if stats.total_volume_saved >= 10_000_000_000 { 3 }
                    else if stats.total_volume_saved >= 1_000_000_000 { 2 }
                    else { 1 };
                let metadata = NftBadgeMetadata {
                    volume_tier,
                    perfect_attendance: stats.late_contributions == 0,
                    group_lead_status: member_info.address == circle.creator,
                };
                // token_id: circle_id in upper 64 bits, member index in lower 64 bits
                let token_id: u128 = ((circle_id as u128) << 64) | (member_info.index as u128);
                let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
                nft_client.mint_badge(&user, &token_id, &metadata);
                env.storage().instance().set(&DataKey::CycleBadge(user.clone(), circle_id), &token_id);
                env.events().publish(
                    (symbol_short!("BADGE"), symbol_short!("MINT")),
                    (user.clone(), circle_id, token_id, metadata),
                );
            }
        }

        circle.current_recipient_index = next_index;
        env.storage().instance().remove(&DataKey::ScheduledPayoutTime(circle_id));
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        if circle.is_insurance_used {
            panic!("Insurance already used");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        let amount_needed = circle.contribution_amount * member_info.tier_multiplier as i128;
        if circle.insurance_balance < amount_needed {
            panic!("Insufficient insurance");
        }

        circle.contribution_bitmap |= 1u64 << member_info.index;
        circle.insurance_balance -= amount_needed;
        circle.is_insurance_used = true;

        // The member defaulted and needed an insurance bailout, increment late count
        let user_stats_key = DataKey::UserStats(member.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });
        user_stats.late_contributions += 1;
        env.storage().instance().set(&user_stats_key, &user_stats);

        env.events().publish(
            (Symbol::new(&env, "USER_STATS"), member.clone()),
            (user_stats.on_time_contributions, user_stats.late_contributions, user_stats.total_volume_saved)
        );

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }
        if new_bps > 10000 {
            panic!("Penalty cannot exceed 100%");
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64) {
        user.require_auth();
        if new_duration == 0 {
            panic!("Duration must be greater than zero");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let protocol_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("Not initialized");

        if user != circle.creator && user != protocol_admin {
            panic!("Unauthorized");
        }

        circle.cycle_duration = new_duration;
        circle.deadline_timestamp = env.ledger().timestamp() + new_duration;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::AdminAction, circle_id);
    }

    fn vote_penalty_change(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let member_key = DataKey::Member(user.clone());
        let member: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if circle.proposed_late_fee_bps == 0 {
            panic!("No active proposal");
        }

        circle.proposal_votes_bitmap |= 1u64 << member.index;

        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn propose_address_change(
        env: Env,
        user: Address,
        circle_id: u64,
        old_address: Address,
        new_address: Address,
    ) {
        user.require_auth();

        if old_address == new_address {
            panic!("Old and new addresses must differ");
        }

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let proposer_key = DataKey::Member(user.clone());
        let proposer: Member = env
            .storage()
            .instance()
            .get(&proposer_key)
            .expect("User is not a member");
        if proposer.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        let old_member_key = DataKey::Member(old_address.clone());
        let old_member: Member = env
            .storage()
            .instance()
            .get(&old_member_key)
            .expect("Old address is not a member");
        if old_member.status != MemberStatus::Active {
            panic!("Old address member is not active");
        }

        let new_member_key = DataKey::Member(new_address.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("New address is already a member");
        }

        circle.recovery_old_address = Some(old_address);
        circle.recovery_new_address = Some(new_address);
        circle.recovery_votes_bitmap = 1u64 << proposer.index;

        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);
    }

    fn vote_for_recovery(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if circle.recovery_old_address.is_none() || circle.recovery_new_address.is_none() {
            panic!("No active recovery proposal");
        }

        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("Not a member");

        circle.recovery_votes_bitmap |= 1u64 << member.index;
        apply_recovery_if_consensus(&env, &user, circle_id, &mut circle);

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        if caller != circle.creator {
            panic!("Unauthorized");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env
            .storage()
            .instance()
            .get(&member_key)
            .expect("Member not found");

        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        nft_client.burn(&member, &token_id);
        write_audit(&env, &caller, AuditAction::AdminAction, circle_id);
    }

    fn pair_with_member(env: Env, user: Address, buddy_address: Address) {
        user.require_auth();
        let user_key = DataKey::Member(user.clone());
        let mut user_info: Member = env
            .storage()
            .instance()
            .get(&user_key)
            .expect("Member not found");

        user_info.buddy = Some(buddy_address);
        env.storage().instance().set(&user_key, &user_info);
        write_audit(&env, &user, AuditAction::AdminAction, 0);
    }

    fn set_safety_deposit(env: Env, user: Address, circle_id: u64, amount: i128) {
        user.require_auth();
        let circle: CircleInfo = env
            .storage()
            .instance()
            .get(&DataKey::Circle(circle_id))
            .expect("Circle not found");

        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &amount);

        let safety_key = DataKey::SafetyDeposit(user.clone(), circle_id);
        let mut balance: i128 = env.storage().instance().get(&safety_key).unwrap_or(0);
        balance += amount;
        env.storage().instance().set(&safety_key, &balance);
    }

    #[test]
    fn test_credit_score_oracle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
    fn get_reputation(env: Env, user: Address) -> ReputationData {
        let current_time = env.ledger().timestamp();
        
        // Get user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        // Get member information to check if user is active
        let member_key = DataKey::Member(user.clone());
        let is_active = if let Some(member) = env.storage().instance().get::<DataKey, Member>(&member_key) {
            member.status == MemberStatus::Active
        } else {
            false
        };

        // Calculate total contributions
        let total_contributions = user_stats.on_time_contributions + user_stats.late_contributions;
        
        // Calculate on-time rate (in basis points)
        let on_time_rate = if total_contributions > 0 {
            (user_stats.on_time_contributions * 10000) / total_contributions
        } else {
            0
        };

        // Calculate reliability score based on on-time rate and volume
        let mut reliability_score = on_time_rate;
        
        // Boost reliability based on volume saved (higher volume = higher reliability)
        if user_stats.total_volume_saved > 0 {
            let volume_bonus = ((user_stats.total_volume_saved / 1_000_000_0) * 100).min(2000); // Max 20% bonus
            reliability_score = (reliability_score + volume_bonus).min(10000);
        }

        // Calculate social capital (sum of trust scores across all circles)
        let mut social_capital = 0u32;
        let mut circle_count = 0u32;
        
        // Get all circles the user is part of by checking member data
        // For now, we'll use a simplified approach - in a full implementation,
        // you might want to maintain an index of user's circles
        for circle_id in 1..=1000 { // Reasonable limit for iteration
            let circle_key = DataKey::Circle(circle_id);
            if let Some(_circle) = env.storage().instance().get::<DataKey, CircleInfo>(&circle_key) {
                let social_capital_key = DataKey::SocialCapital(user.clone(), circle_id);
                if let Some(soc_cap) = env.storage().instance().get::<DataKey, SocialCapital>(&social_capital_key) {
                    social_capital += soc_cap.trust_score;
                    circle_count += 1;
                }
            }
        }

        // Average social capital across circles
        let avg_social_capital = if circle_count > 0 {
            (social_capital / circle_count) * 100 // Convert to basis points
        } else {
            0
        };

        // Calculate final Susu Score (weighted combination)
        // Weight: 50% reliability, 30% social capital, 20% activity
        let activity_score = if total_contributions > 0 {
            ((total_contributions as u32).min(50) * 200) // Max 10% from activity
        } else {
            0
        };

        let susu_score = (
            (reliability_score * 50) / 100 +  // 50% weight
            (avg_social_capital * 30) / 100 +  // 30% weight  
            (activity_score * 20) / 100         // 20% weight
        ).min(10000);

        ReputationData {
            user_address: user.clone(),
            susu_score,
            reliability_score,
            total_contributions,
            on_time_rate,
            volume_saved: user_stats.total_volume_saved,
            social_capital: avg_social_capital,
            last_updated: current_time,
            is_active,
        }
    }

    fn propose_rollover_bonus(env: Env, user: Address, circle_id: u64, fee_percentage_bps: u32) {
        user.require_auth();

        if fee_percentage_bps > 10000 {
            panic!("Fee percentage cannot exceed 100%");
        }

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active rollover proposal
        let rollover_key = DataKey::RolloverBonus(circle_id);
        if let Some(existing_rollover) = env.storage().instance().get::<DataKey, RolloverBonus>(&rollover_key) {
            if existing_rollover.status == RolloverStatus::Voting {
                panic!("Rollover bonus proposal already active");
            }
        }

        // Only allow rollover proposals after the first round is complete
        if !circle.is_round_finalized || circle.current_recipient_index == 0 {
            panic!("Rollover can only be proposed after first complete cycle");
        }

        let current_time = env.ledger().timestamp();
        let bonus_amount = calculate_rollover_bonus(&env, circle_id, fee_percentage_bps);

        let rollover_bonus = RolloverBonus {
            circle_id,
            bonus_amount,
            fee_percentage: fee_percentage_bps,
            created_timestamp: current_time,
            status: RolloverStatus::Voting,
            voting_deadline: current_time + ROLLOVER_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            applied_cycle: None,
        };

        env.storage().instance().set(&rollover_key, &rollover_bonus);
        
        // The proposer automatically votes for
        let vote_key = DataKey::RolloverVote(circle_id, user.clone());
        let vote = RolloverVote {
            voter: user.clone(),
            circle_id,
            vote_choice: RolloverVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_rollover = rollover_bonus;
        updated_rollover.for_votes = 1;
        updated_rollover.total_votes_cast = 1;
        env.storage().instance().set(&rollover_key, &updated_rollover);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_PROPOSED"), circle_id, user.clone()),
            (bonus_amount, fee_percentage_bps, updated_rollover.voting_deadline),
        );
    }

    fn vote_rollover_bonus(env: Env, user: Address, circle_id: u64, vote_choice: RolloverVoteChoice) {
        user.require_auth();

        let rollover_key = DataKey::RolloverBonus(circle_id);
        let mut rollover_bonus: RolloverBonus = env.storage().instance().get(&rollover_key)
            .expect("No active rollover proposal");

        if rollover_bonus.status != RolloverStatus::Voting {
            panic!("Rollover proposal is not in voting period");
        }

        if env.ledger().timestamp() > rollover_bonus.voting_deadline {
            rollover_bonus.status = RolloverStatus::Rejected;
            env.storage().instance().set(&rollover_key, &rollover_bonus);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::RolloverVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = RolloverVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            RolloverVoteChoice::For => rollover_bonus.for_votes += 1,
            RolloverVoteChoice::Against => rollover_bonus.against_votes += 1,
        }
        rollover_bonus.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (rollover_bonus.total_votes_cast * 100) >= (active_members * ROLLOVER_QUORUM);
        
        if quorum_met && rollover_bonus.total_votes_cast > 0 {
            let approval_percentage = (rollover_bonus.for_votes * 100) / rollover_bonus.total_votes_cast;
            if approval_percentage >= ROLLOVER_MAJORITY {
                rollover_bonus.status = RolloverStatus::Approved;
            }
        }

        env.storage().instance().set(&rollover_key, &rollover_bonus);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_VOTE"), circle_id, user.clone()),
            (vote_choice, rollover_bonus.for_votes, rollover_bonus.against_votes),
        );
    }

    fn apply_rollover_bonus(env: Env, circle_id: u64) {
        let rollover_key = DataKey::RolloverBonus(circle_id);
        let mut rollover_bonus: RolloverBonus = env.storage().instance().get(&rollover_key)
            .expect("No rollover bonus proposal found");

        if rollover_bonus.status != RolloverStatus::Approved {
            panic!("Rollover bonus is not approved");
        }

        let circle_key = DataKey::Circle(circle_id);
        let mut circle: CircleInfo = env.storage().instance().get(&circle_key)
            .expect("Circle not found");

        // Apply the bonus to the group reserve (will be used in next cycle's first pot)
        let mut reserve: i128 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        reserve += rollover_bonus.bonus_amount;
        env.storage().instance().set(&DataKey::GroupReserve, &reserve);

        // Mark as applied and track the cycle
        rollover_bonus.status = RolloverStatus::Applied;
        rollover_bonus.applied_cycle = Some(circle.current_recipient_index + 1);
        env.storage().instance().set(&rollover_key, &rollover_bonus);

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "ROLLOVER_APPLIED"), circle_id),
            (rollover_bonus.bonus_amount, rollover_bonus.applied_cycle.unwrap()),
        );
    }

    fn propose_yield_delegation(env: Env, user: Address, circle_id: u64, delegation_percentage: u32, pool_address: Address, pool_type: YieldPoolType) {
        user.require_auth();

        if delegation_percentage > MAX_DELEGATION_PERCENTAGE {
            panic!("Delegation percentage exceeds maximum");
        }

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active yield delegation proposal
        let delegation_key = DataKey::YieldDelegation(circle_id);
        if let Some(existing_delegation) = env.storage().instance().get::<DataKey, YieldDelegation>(&delegation_key) {
            if existing_delegation.status == YieldDelegationStatus::Voting || 
               existing_delegation.status == YieldDelegationStatus::Active {
                panic!("Yield delegation already active");
            }
        }

        // Only allow yield delegation after round is finalized but before payout
        if !circle.is_round_finalized {
            panic!("Round must be finalized before yield delegation");
        }

        let current_time = env.ledger().timestamp();
        let pot_amount = circle.contribution_amount * (circle.total_shares as i128);
        let delegation_amount = (pot_amount * delegation_percentage as i128) / 10000;

        if delegation_amount < MIN_DELEGATION_AMOUNT {
            panic!("Delegation amount below minimum");
        }

        let yield_delegation = YieldDelegation {
            circle_id,
            delegation_amount,
            pool_address: pool_address.clone(),
            pool_type: pool_type.clone(),
            delegation_percentage,
            created_timestamp: current_time,
            status: YieldDelegationStatus::Voting,
            voting_deadline: current_time + YIELD_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            start_time: None,
            end_time: None,
            total_yield_earned: 0,
            yield_distributed: 0,
            last_compound_time: current_time,
        };

        env.storage().instance().set(&delegation_key, &yield_delegation);
        
        // The proposer automatically votes for
        let vote_key = DataKey::YieldVote(circle_id, user.clone());
        let vote = YieldVote {
            voter: user.clone(),
            circle_id,
            vote_choice: YieldVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_delegation = yield_delegation;
        updated_delegation.for_votes = 1;
        updated_delegation.total_votes_cast = 1;
        env.storage().instance().set(&delegation_key, &updated_delegation);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_PROPOSED"), circle_id, user.clone()),
            (delegation_amount, delegation_percentage, pool_address, updated_delegation.voting_deadline),
        );
    }

    fn vote_yield_delegation(env: Env, user: Address, circle_id: u64, vote_choice: YieldVoteChoice) {
        user.require_auth();

        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No active yield delegation proposal");

        if delegation.status != YieldDelegationStatus::Voting {
            panic!("Yield delegation is not in voting period");
        }

        if env.ledger().timestamp() > delegation.voting_deadline {
            delegation.status = YieldDelegationStatus::Rejected;
            env.storage().instance().set(&delegation_key, &delegation);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::YieldVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = YieldVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            YieldVoteChoice::For => delegation.for_votes += 1,
            YieldVoteChoice::Against => delegation.against_votes += 1,
        }
        delegation.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (delegation.total_votes_cast * 100) >= (active_members * YIELD_QUORUM);
        
        if quorum_met && delegation.total_votes_cast > 0 {
            let approval_percentage = (delegation.for_votes * 100) / delegation.total_votes_cast;
            if approval_percentage >= YIELD_MAJORITY {
                delegation.status = YieldDelegationStatus::Approved;
            }
        }

        env.storage().instance().set(&delegation_key, &delegation);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_VOTE"), circle_id, user.clone()),
            (vote_choice, delegation.for_votes, delegation.against_votes),
        );
    }

    fn approve_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation proposal found");

        if delegation.status != YieldDelegationStatus::Approved {
            panic!("Yield delegation is not approved");
        }

        // Register the yield pool if not already registered
        let pool_registry_key = DataKey::YieldPoolRegistry;
        let mut pool_registry: Vec<Address> = env.storage().instance().get(&pool_registry_key).unwrap_or(Vec::new(&env));
        
        if !pool_registry.contains(&delegation.pool_address) {
            pool_registry.push_back(delegation.pool_address.clone());
            env.storage().instance().set(&pool_registry_key, &pool_registry);
        }

        // Update pool info
        let pool_info = YieldPoolInfo {
            pool_address: delegation.pool_address.clone(),
            pool_type: delegation.pool_type.clone(),
            is_active: true,
            total_delegated: delegation.delegation_amount,
            apy_bps: 500, // Default 5% APY (would be fetched from pool)
            last_updated: env.ledger().timestamp(),
        };
        env.storage().instance().set(&DataKey::YieldDelegation(circle_id), &pool_info);

        // Execute the delegation
        execute_yield_delegation_internal(&env, circle_id, &mut delegation);

        env.storage().instance().set(&delegation_key, &delegation);
        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_APPROVED"), circle_id),
            (delegation.delegation_amount, delegation.pool_address),
        );
    }

    fn execute_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Approved && delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not approved");
        }

        execute_yield_delegation_internal(&env, circle_id, &mut delegation);
        env.storage().instance().set(&delegation_key, &delegation);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_EXECUTED"), circle_id),
            (delegation.delegation_amount, delegation.pool_address),
        );
    }

    fn compound_yield(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not active");
        }

        let current_time = env.ledger().timestamp();
        if current_time < delegation.last_compound_time + YIELD_COMPOUNDING_FREQUENCY {
            panic!("Too early to compound");
        }

        // Calculate yield (simplified - would query actual yield from pool)
        let time_elapsed = current_time - delegation.last_compound_time;
        let yield_earned = calculate_yield_from_pool(&env, &delegation, time_elapsed);

        delegation.total_yield_earned += yield_earned;
        delegation.last_compound_time = current_time;

        env.storage().instance().set(&delegation_key, &delegation);

        env.events().publish(
            (Symbol::new(&env, "YIELD_COMPOUNDED"), circle_id),
            (yield_earned, delegation.total_yield_earned),
        );
    }

    fn withdraw_yield_delegation(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let mut delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.status != YieldDelegationStatus::Active {
            panic!("Yield delegation is not active");
        }

        // Final compound before withdrawal
        let current_time = env.ledger().timestamp();
        let time_elapsed = current_time - delegation.last_compound_time;
        let final_yield = calculate_yield_from_pool(&env, &delegation, time_elapsed);
        delegation.total_yield_earned += final_yield;

        // Withdraw from pool (simplified - would call actual pool contract)
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let token_client = token::Client::new(&env, &circle.token);
        
        // In real implementation, this would withdraw from the actual yield pool
        let total_withdrawn = delegation.delegation_amount + delegation.total_yield_earned;
        
        // Return funds to contract
        // token_client.transfer(&delegation.pool_address, &env.current_contract_address(), &total_withdrawn);

        delegation.status = YieldDelegationStatus::Completed;
        delegation.end_time = Some(current_time);

        env.storage().instance().set(&delegation_key, &delegation);

        // Distribute earnings
        distribute_yield_earnings(env, circle_id);

        env.events().publish(
            (Symbol::new(&env, "YIELD_DELEGATION_WITHDRAWN"), circle_id),
            (total_withdrawn, delegation.total_yield_earned),
        );
    }

    fn distribute_yield_earnings(env: Env, circle_id: u64) {
        let delegation_key = DataKey::YieldDelegation(circle_id);
        let delegation: YieldDelegation = env.storage().instance().get(&delegation_key)
            .expect("No yield delegation found");

        if delegation.total_yield_earned <= delegation.yield_distributed {
            panic!("No new yield to distribute");
        }

        let new_yield = delegation.total_yield_earned - delegation.yield_distributed;
        
        // Calculate 50/50 split
        let recipient_share = (new_yield * YIELD_DISTRIBUTION_RECIPIENT_BPS as i128) / 10000;
        let treasury_share = (new_yield * YIELD_DISTRIBUTION_TREASURY_BPS as i128) / 10000;

        // Get current round recipient
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        if let Some(recipient) = &circle.current_pot_recipient {
            // Transfer to current recipient
            let token_client = token::Client::new(&env, &circle.token);
            // token_client.transfer(&env.current_contract_address(), recipient, &recipient_share);
        }

        // Add to group treasury
        let treasury_key = DataKey::GroupTreasury(circle_id);
        let mut treasury: i128 = env.storage().instance().get(&treasury_key).unwrap_or(0);
        treasury += treasury_share;
        env.storage().instance().set(&treasury_key, &treasury);

        // Update delegation record
        let mut updated_delegation = delegation;
        updated_delegation.yield_distributed += new_yield;
        env.storage().instance().set(&delegation_key, &updated_delegation);

        // Create distribution record
        let distribution = YieldDistribution {
            circle_id,
            recipient_share,
            treasury_share,
            total_yield: new_yield,
            distribution_time: env.ledger().timestamp(),
            round_number: circle.current_recipient_index,
        };

        env.events().publish(
            (Symbol::new(&env, "YIELD_DISTRIBUTED"), circle_id),
            (recipient_share, treasury_share, new_yield),
        );

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);
    }

    fn propose_path_payment_support(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if there's already an active path payment proposal
        let path_payment_key = DataKey::PathPayment(circle_id);
        if let Some(existing_payment) = env.storage().instance().get::<DataKey, PathPayment>(&path_payment_key) {
            if existing_payment.status == PathPaymentStatus::Proposed || 
               existing_payment.status == PathPaymentStatus::Executing ||
               existing_payment.status == PathPaymentStatus::Completed {
                panic!("Path payment already active");
            }
        }

        let current_time = env.ledger().timestamp();
        
        let path_payment = PathPayment {
            circle_id,
            source_token: Address::generate(&env), // Will be set during execution
            target_token: circle.token.clone(),
            source_amount: 0, // Will be set during execution
            target_amount: 0, // Will be calculated during execution
            exchange_rate: 0,
            slippage_bps: 0,
            dex_address: Address::generate(&env), // Will be set during execution
            path_payment: Address::generate(&env), // Will be set during execution
            created_timestamp: current_time,
            status: PathPaymentStatus::Proposed,
            voting_deadline: current_time + PATH_PAYMENT_VOTING_PERIOD,
            for_votes: 0,
            against_votes: 0,
            total_votes_cast: 0,
            execution_timestamp: None,
            completion_timestamp: None,
            refund_amount: None,
        };

        env.storage().instance().set(&path_payment_key, &path_payment);
        
        // The proposer automatically votes for
        let vote_key = DataKey::PathPaymentVote(circle_id, user.clone());
        let vote = PathPaymentVote {
            voter: user.clone(),
            circle_id,
            vote_choice: PathPaymentVoteChoice::For,
            timestamp: current_time,
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        let mut updated_payment = path_payment;
        updated_payment.for_votes = 1;
        updated_payment.total_votes_cast = 1;
        env.storage().instance().set(&path_payment_key, &updated_payment);

        write_audit(&env, &user, AuditAction::DisputeSubmission, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_PROPOSED"), circle_id, user.clone()),
            (circle.token.clone(), updated_payment.voting_deadline),
        );
    }

    fn vote_path_payment_support(env: Env, user: Address, circle_id: u64, vote_choice: PathPaymentVoteChoice) {
        user.require_auth();

        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No active path payment proposal");

        if payment.status != PathPaymentStatus::Proposed {
            panic!("Path payment is not in voting period");
        }

        if env.ledger().timestamp() > payment.voting_deadline {
            payment.status = PathPaymentStatus::Failed;
            env.storage().instance().set(&payment_key, &payment);
            panic!("Voting period has expired");
        }

        // Check if user is an active member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key)
            .expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Check if already voted
        let vote_key = DataKey::PathPaymentVote(circle_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        // Record the vote
        let vote = PathPaymentVote {
            voter: user.clone(),
            circle_id,
            vote_choice: vote_choice.clone(),
            timestamp: env.ledger().timestamp(),
        };
        env.storage().instance().set(&vote_key, &vote);

        // Update vote counts
        match vote_choice {
            PathPaymentVoteChoice::For => payment.for_votes += 1,
            PathPaymentVoteChoice::Against => payment.against_votes += 1,
        }
        payment.total_votes_cast += 1;

        // Check if voting criteria are met
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        let active_members = count_active_members(&env, &circle);
        
        let quorum_met = (payment.total_votes_cast * 100) >= (active_members * PATH_PAYMENT_QUORUM);
        
        if quorum_met && payment.total_votes_cast > 0 {
            let approval_percentage = (payment.for_votes * 100) / payment.total_votes_cast;
            if approval_percentage >= PATH_PAYMENT_MAJORITY {
                payment.status = PathPaymentStatus::Approved;
            }
        }

        env.storage().instance().set(&payment_key, &payment);
        write_audit(&env, &user, AuditAction::GovernanceVote, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_VOTE"), circle_id, user.clone()),
            (vote_choice, payment.for_votes, payment.against_votes),
        );
    }

    fn approve_path_payment_support(env: Env, circle_id: u64) {
        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No path payment proposal found");

        if payment.status != PathPaymentStatus::Approved {
            panic!("Path payment is not approved");
        }

        payment.status = PathPaymentStatus::Executing;
        env.storage().instance().set(&payment_key, &payment);

        write_audit(&env, &env.current_contract_address(), AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_APPROVED"), circle_id),
            (payment.source_token, payment.target_token),
        );
    }

    fn execute_path_payment(env: Env, user: Address, circle_id: u64, source_token: Address, source_amount: i128) {
        user.require_auth();

        let payment_key = DataKey::PathPayment(circle_id);
        let mut payment: PathPayment = env.storage().instance().get(&payment_key)
            .expect("No path payment proposal found");

        if payment.status != PathPaymentStatus::Approved && payment.status != PathPaymentStatus::Executing {
            panic!("Path payment is not approved for execution");
        }

        // Validate source token is supported
        let source_token_key = DataKey::SupportedTokens(source_token.clone());
        let source_token_info: SupportedToken = env.storage().instance().get(&source_token_key)
            .expect("Source token not supported");

        if !source_token_info.is_active {
            panic!("Source token is not active");
        }

        // Validate minimum amount
        if source_amount < MIN_PATH_PAYMENT_AMOUNT {
            panic!("Amount below minimum path payment");
        }

        // Get target token info (circle's token)
        let target_token_key = DataKey::SupportedTokens(payment.target_token.clone());
        let target_token_info: SupportedToken = env.storage().instance().get(&target_token_key)
            .expect("Target token not supported");

        if !target_token_info.is_active {
            panic!("Target token is not supported");
        }

        let current_time = env.ledger().timestamp();
        
        // Update payment details
        payment.source_token = source_token.clone();
        payment.source_amount = source_amount;
        payment.execution_timestamp = Some(current_time);
        payment.status = PathPaymentStatus::Executing;

        // Execute the swap via Stellar Path Payments
        let (target_amount, exchange_rate, slippage_bps) = execute_stellar_path_payment(
            &env, 
            &source_token, 
            &payment.target_token, 
            source_amount,
            MAX_SLIPPAGE_TOLERANCE_BPS
        );

        // Update payment with execution results
        payment.target_amount = target_amount;
        payment.exchange_rate = exchange_rate;
        payment.slippage_bps = slippage_bps;

        // Deposit target tokens to circle
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .expect("Circle not found");
        
        let token_client = token::Client::new(&env, &payment.target_token);
        let transfer_result = token_client.try_transfer(&user, &env.current_contract_address(), &target_amount);
        
        let transfer_success = match transfer_result {
            Ok(inner) => inner.is_ok(),
            Err(_) => false,
        };

        if !transfer_success {
            payment.status = PathPaymentStatus::Failed;
            payment.refund_amount = Some(source_amount);
            env.storage().instance().set(&payment_key, &payment);
            panic!("Token transfer failed");
        }

        // Update member contribution
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .expect("Member not found");

        let contribution_amount = circle.contribution_amount * member.tier_multiplier as i128;
        
        // Update user statistics
        let user_stats_key = DataKey::UserStats(user.clone());
        let mut user_stats: UserStats = env.storage().instance().get(&user_stats_key).unwrap_or(UserStats {
            total_volume_saved: 0,
            on_time_contributions: 0,
            late_contributions: 0,
        });

        user_stats.total_volume_saved += contribution_amount;
        user_stats.on_time_contributions += 1;
        env.storage().instance().set(&user_stats_key, &user_stats);

        // Update member and circle
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        circle.contribution_bitmap |= 1u64 << member.index;

        env.storage().instance().set(&member_key, &member);
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // Mark as completed
        payment.status = PathPaymentStatus::Completed;
        payment.completion_timestamp = Some(current_time);
        env.storage().instance().set(&payment_key, &payment);

        write_audit(&env, &user, AuditAction::AdminAction, circle_id);

        env.events().publish(
            (Symbol::new(&env, "PATH_PAYMENT_EXECUTED"), circle_id, user.clone()),
            (source_amount, target_amount, exchange_rate, slippage_bps),
        );
    }

    fn register_supported_token(env: Env, user: Address, token_address: Address, token_symbol: String, decimals: u32, is_stable: bool) {
        user.require_auth();

        let token_key = DataKey::SupportedTokens(token_address.clone());
        if env.storage().instance().has(&token_key) {
            panic!("Token already registered");
        }

        let current_time = env.ledger().timestamp();
        let supported_token = SupportedToken {
            token_address: token_address.clone(),
            token_symbol,
            decimals,
            is_stable,
            is_active: true,
            last_updated: current_time,
        };

        env.storage().instance().set(&token_key, &supported_token);

        write_audit(&env, &user, AuditAction::AdminAction, 0);

        env.events().publish(
            (Symbol::new(&env, "TOKEN_REGISTERED"), token_address),
            (token_symbol, decimals, is_stable),
        );
    }

    fn register_dex(env: Env, user: Address, dex_address: Address, dex_name: String, is_trusted: bool) {
        user.require_auth();

        let dex_key = DataKey::DexRegistry(dex_address.clone());
        if env.storage().instance().has(&dex_key) {
            panic!("DEX already registered");
        }

        let current_time = env.ledger().timestamp();
        let dex_info = DexInfo {
            dex_address: dex_address.clone(),
            dex_name,
            supported_pairs: Vec::new(&env),
            is_trusted,
            is_active: true,
            last_updated: current_time,
        };

        env.storage().instance().set(&dex_key, &dex_info);

        write_audit(&env, &user, AuditAction::AdminAction, 0);

        env.events().publish(
            (Symbol::new(&env, "DEX_REGISTERED"), dex_address),
            (dex_name, is_trusted),
        );
    }
}

fn execute_yield_delegation_internal(env: &Env, circle_id: u64, delegation: &mut YieldDelegation) {
    let current_time = env.ledger().timestamp();
    
    // Transfer funds to yield pool
    let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
        .expect("Circle not found");
    let token_client = token::Client::new(env, &circle.token);
    
    // In real implementation, this would call the actual yield pool contract
    // token_client.transfer(&env.current_contract_address(), &delegation.pool_address, &delegation.delegation_amount);
    
    delegation.status = YieldDelegationStatus::Active;
    delegation.start_time = Some(current_time);
    delegation.last_compound_time = current_time;
}

fn calculate_yield_from_pool(env: &Env, delegation: &YieldDelegation, time_elapsed: u64) -> i128 {
    // Simplified yield calculation - in real implementation would query actual pool
    let apy_bps = 500; // 5% APY
    let seconds_in_year = 365 * 24 * 60 * 60;
    let time_fraction = time_elapsed as i128 * 10000 / seconds_in_year as i128;
    (delegation.delegation_amount * apy_bps as i128 * time_fraction) / (10000 * 10000)
}

    #[test]
    fn test_get_reputation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Test reputation for new user (should be zero/low)
        let reputation = client.get_reputation(&user);
        assert_eq!(reputation.susu_score, 0);
        assert_eq!(reputation.reliability_score, 0);
        assert_eq!(reputation.total_contributions, 0);
        assert_eq!(reputation.on_time_rate, 0);
        assert_eq!(reputation.volume_saved, 0);
        assert_eq!(reputation.is_active, false);
        
        // Create circle and add user
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&user, &circle_id, &1, &None);
        client.deposit(&user, &circle_id);
        
        // Test reputation after contribution
        let reputation = client.get_reputation(&user);
        assert!(reputation.susu_score > 0);
        assert!(reputation.reliability_score > 0);
        assert_eq!(reputation.total_contributions, 1);
        assert_eq!(reputation.on_time_rate, 10000); // 100% on-time rate
        assert_eq!(reputation.volume_saved, 1_000_000_000_000);
        assert_eq!(reputation.is_active, true);
    }

    #[test]
    fn test_credit_score_oracle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Start out unscored
        assert_eq!(client.get_user_reliability_score(&user), 0);

        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&user, &circle_id, &1, &None);
        client.deposit(&user, &circle_id);

        // Should earn positive reliability
        let score = client.get_user_reliability_score(&user);
        assert!(score > 0);
        
        let stats = client.get_user_stats(&user);
        assert_eq!(stats.on_time_contributions, 1);
        assert_eq!(stats.late_contributions, 0);
        assert_eq!(stats.total_volume_saved, 1_000_000_000_000);
    }

    #[test]
    fn test_slash_user_credit() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let user = Address::generate(&env);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        client.slash_user_credit(&admin, &user, &5);
        let stats = client.get_user_stats(&user);
        assert_eq!(stats.late_contributions, 5);
        assert_eq!(client.get_user_reliability_score(&user), 0);
    }

    #[test]
    fn test_cross_contract_oracle() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let oracle_id = env.register_contract(None, SoroSusu);
        let oracle_client = SoroSusuClient::new(&env, &oracle_id);
        
        let lending_id = env.register_contract(None, MockLending);
        let lending_client = MockLendingClient::new(&env, &lending_id);
        
        env.mock_all_auths();
        oracle_client.init(&admin);
        
        // Start out unscored, cannot borrow
        assert_eq!(lending_client.can_borrow(&oracle_id, &user), false);

        let circle_id = oracle_client.create_circle(
            &creator,
            &1_000_000_000_000,
            &10,
            &token_contract,
            &86400,
            &100, // 1%
            &nft_contract,
            &arbitrator,
        );
        
        oracle_client.join_circle(&user, &circle_id, &1, &None);
        oracle_client.deposit(&user, &circle_id);

        // After a successful on-time deposit, score surges past the 500 threshold
        assert_eq!(lending_client.can_borrow(&oracle_id, &user), true);
    }

    #[test]
    fn test_sub_susu_credit_line() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(&creator, &1000, &2, &token_contract, &86400, &100, &nft_contract, &arbitrator);
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Payout to creator first to establish history and boost user score
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Now user asks for credit advance. Expected payout = 2000. Limit is 1000.
        client.approve_credit_advance(&creator, &circle_id, &user, &1000);
        
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&user, &circle_id); // debt is deducted seamlessly!
    }

    #[test]
    fn test_rollover_bonus_proposal_and_voting() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Set up protocol fee for rollover bonus calculation
        client.set_protocol_fee(&admin, &100, &admin); // 1% fee
        
        // Create circle with 2 members
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000, // 1000 tokens
            &2,
            &token_contract,
            &86400,
            &100, // 1% insurance
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&user1, &circle_id);
        
        // Now propose rollover bonus (50% of platform fee)
        client.propose_rollover_bonus(&creator, &circle_id, &5000);
        
        // Second member votes for the rollover
        client.vote_rollover_bonus(&user1, &circle_id, &RolloverVoteChoice::For);
        
        // Apply the rollover bonus
        client.apply_rollover_bonus(&circle_id);
        
        // Start third cycle - first recipient should get rollover bonus
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Check that rollover bonus is applied to payout
        let initial_balance = token_contract.mock_balance(&creator);
        client.claim_pot(&creator, &circle_id);
        let final_balance = token_contract.mock_balance(&creator);
        
        // Should receive regular pot (2000) minus fee (1% = 20) plus rollover bonus (50% of fee = 10)
        let expected_payout = 2000 - 20 + 10; // 1990
        assert_eq!(final_balance - initial_balance, expected_payout);
    }

    #[test]
    fn test_rollover_bonus_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        client.set_protocol_fee(&admin, &100, &admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Propose rollover bonus
        client.propose_rollover_bonus(&creator, &circle_id, &5000);
        
        // Second member votes against - should not meet majority threshold
        client.vote_rollover_bonus(&user1, &circle_id, &RolloverVoteChoice::Against);
        
        // Try to apply should fail since not approved
        std::panic::catch_unwind(|| {
            client.apply_rollover_bonus(&circle_id);
        }).expect_err("Should panic when trying to apply unapproved rollover");
    }

    #[test]
    fn test_yield_delegation_proposal_and_voting() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with 3 members for higher quorum requirements
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000, // 1000 tokens
            &3,
            &token_contract,
            &86400,
            &100, // 1% insurance
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        client.join_circle(&user2, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and finalize again
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose yield delegation (50% of pot)
        let pool_address = Address::generate(&env);
        client.propose_yield_delegation(
            &creator, 
            &circle_id, 
            &5000, // 50%
            &pool_address,
            &YieldPoolType::StellarLiquidityPool
        );
        
        // Other members vote for the delegation
        client.vote_yield_delegation(&user1, &circle_id, &YieldVoteChoice::For);
        client.vote_yield_delegation(&user2, &circle_id, &YieldVoteChoice::For);
        
        // Approve and execute delegation
        client.approve_yield_delegation(&circle_id);
        client.execute_yield_delegation(&circle_id);
        
        // Test compounding
        env.ledger().set_timestamp(env.ledger().timestamp() + YIELD_COMPOUNDING_FREQUENCY + 1);
        client.compound_yield(&circle_id);
        
        // Test withdrawal and distribution
        client.withdraw_yield_delegation(&circle_id);
    }

    #[test]
    fn test_yield_delegation_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose yield delegation
        let pool_address = Address::generate(&env);
        client.propose_yield_delegation(
            &creator, 
            &circle_id, 
            &5000,
            &pool_address,
            &YieldPoolType::StellarLiquidityPool
        );
        
        // Second member votes against - should not meet 80% majority
        client.vote_yield_delegation(&user1, &circle_id, &YieldVoteChoice::Against);
        
        // Try to approve should fail since not approved
        std::panic::catch_unwind(|| {
            client.approve_yield_delegation(&circle_id);
        }).expect_err("Should panic when trying to approve rejected delegation");
    }

    #[test]
    fn test_path_payment_support_proposal_and_execution() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with USDC as target token
        let usdc_address = Address::generate(&env);
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000, // 1000 tokens
            &3,
            &usdc_address, // USDC as target token
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Register XLM as supported token
        client.register_supported_token(
            &creator,
            &token_contract, // XLM token address
            &String::from_str(&env, "XLM"),
            &7,
            &true
        );
        
        // Register USDC as supported token
        client.register_supported_token(
            &creator,
            &usdc_address, // USDC token address
            &String::from_str(&env, "USDC"),
            &6,
            &true
        );
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and propose path payment support
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose path payment support (XLM to USDC)
        client.propose_path_payment_support(&creator, &circle_id);
        
        // Vote for path payment support
        client.vote_path_payment_support(&user1, &circle_id, &PathPaymentVoteChoice::For);
        
        // Approve and execute path payment
        client.approve_path_payment_support(&circle_id);
        
        // Execute path payment (user sends XLM, gets USDC in circle)
        let xlm_address = token_contract;
        client.execute_path_payment(
            &user1,
            &circle_id,
            &xlm_address,
            &500_000_000 // 500 XLM
        );
    }

    #[test]
    fn test_path_payment_support_rejection() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let usdc_address = Address::generate(&env);
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000,
            &2,
            &usdc_address, // USDC as target token
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        
        // Complete first cycle
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Start second cycle and propose path payment support
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        
        // Propose path payment support
        client.propose_path_payment_support(&creator, &circle_id);
        
        // Second member votes against - should not meet 66% majority
        client.vote_path_payment_support(&user1, &circle_id, &PathPaymentVoteChoice::Against);
        
        // Try to approve should fail since not approved
        std::panic::catch_unwind(|| {
            client.approve_path_payment_support(&circle_id);
        }).expect_err("Should panic when trying to approve rejected path payment");
    }
}
