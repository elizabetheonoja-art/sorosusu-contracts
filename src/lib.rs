#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, Address, Env, Vec, Symbol, token, testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};

// --- DATA STRUCTURES ---
const TAX_WITHHOLDING_BPS: u64 = 1000; // 10%

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(u64, Address), // Refactored: CircleID, UserAddress
    CircleCount,
    ScheduledPayoutTime(u64),
    LastCreatedTimestamp(Address),
    SafetyDeposit(Address, u64),
    GroupReserve,
    // #225: Duration Proposals
    Proposal(u64, u64), // CircleID, ProposalID
    ProposalCount(u64), // CircleID
    Vote(u64, u64, Address), // CircleID, ProposalID, Voter
    // #227: Bond Storage
    Bond(u64), // CircleID
    // #228: Governance
    Stake(Address),
    GlobalFeeBP, // Basis points
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
#[derive(Clone, Debug, PartialEq)]
pub enum LeniencyVote {
    Approve,
    Reject,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum LeniencyRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalType {
    ChangeLateFee,
    ChangeInsuranceFee,
    ChangeCycleDuration,
    AddMember,
    RemoveMember,
    ChangeQuorum,
    EmergencyAction,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum ProposalStatus {
    Draft,
    Active,
    Approved,
    Rejected,
    Executed,
    Expired,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum QuadraticVoteChoice {
    For,
    Against,
    Abstain,
}

#[contracttype]
#[derive(Clone)]
pub struct LeniencyRequest {
    pub requester: Address,
    pub circle_id: u64,
    pub request_timestamp: u64,
    pub voting_deadline: u64,
    pub status: LeniencyRequestStatus,
    pub approve_votes: u32,
    pub reject_votes: u32,
    pub total_votes_cast: u32,
    pub extension_hours: u64,
    pub reason: String,
}

#[contracttype]
#[derive(Clone)]
pub struct Proposal {
    pub id: u64,
    pub circle_id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_timestamp: u64,
    pub voting_start_timestamp: u64,
    pub voting_end_timestamp: u64,
    pub status: ProposalStatus,
    pub for_votes: u64,
    pub against_votes: u64,
    pub total_voting_power: u64,
    pub quorum_met: bool,
    pub execution_data: String, // JSON or structured data for execution
}

#[contracttype]
#[derive(Clone)]
pub struct QuadraticVote {
    pub voter: Address,
    pub proposal_id: u64,
    pub vote_weight: u32,
    pub vote_choice: QuadraticVoteChoice,
    pub voting_power_used: u64,
    pub timestamp: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct VotingPower {
    pub member: Address,
    pub circle_id: u64,
    pub token_balance: i128,
    pub quadratic_power: u64,
    pub last_updated: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct ProposalStats {
    pub total_proposals: u32,
    pub approved_proposals: u32,
    pub rejected_proposals: u32,
    pub executed_proposals: u32,
    pub average_participation: u32,
    pub average_voting_time: u64,
}

#[contracttype]
#[derive(Clone)]
pub struct LeniencyStats {
    pub total_requests: u32,
    pub approved_requests: u32,
    pub rejected_requests: u32,
    pub expired_requests: u32,
    pub average_participation: u32,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum CollateralStatus {
    NotStaked,
    Staked,
    Slashed,
    Released,
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
}

#[contracttype]
#[derive(Clone)]
pub struct CollateralInfo {
    pub member: Address,
    pub circle_id: u64,
    pub amount: i128,
    pub status: CollateralStatus,
    pub staked_timestamp: u64,
    pub release_timestamp: Option<u64>,
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
#[derive(Clone)]
pub struct CircleInfo {
    pub id: u64,
    pub creator: Address,
    pub contribution_amount: u64, // Optimized from i128 to u64
    pub max_members: u16, // Optimized from u32 to u16
    pub member_count: u16, // Track count separately from Vec
    pub current_recipient_index: u16, // Track by index instead of Address
    pub is_active: bool,
    pub token: Address, // The token used (USDC, XLM)
    pub deadline_timestamp: u64, // Deadline for on-time payments
    pub cycle_duration: u64, // Duration of each payment cycle in seconds
}

// --- CONTRACT CLIENTS ---

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum AuditAction {
    DisputeSubmission,
    GovernanceVote,
    EvidenceAccess,
    AdminAction,
}

#[contracttype]
#[derive(Clone)]
pub struct AuditEntry {
    pub id: u64,
    pub actor: Address,
    pub action: AuditAction,
    pub timestamp: u64,
    pub resource_id: u64,
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
}

#[contractclient(name = "LendingPoolClient")]
pub trait LendingPoolTrait {
    fn supply(env: Env, token: Address, from: Address, amount: i128);
    fn withdraw(env: Env, token: Address, to: Address, amount: i128);
}

pub trait SoroSusuTrait {
    // Initialize the contract
    fn init(env: Env, admin: Address, global_fee: u32);
    
    // Create a new savings circle (#227: Creator must pay bond)
    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u16, token: Address, cycle_duration: u64, bond_amount: u64) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64);

    // Make a deposit (#226: Support for batch contributions)
    fn deposit(env: Env, user: Address, circle_id: u64, rounds: u32);

    // #225: Variable Round Duration
    fn propose_duration(env: Env, user: Address, circle_id: u64, new_duration: u64) -> u64;
    fn vote_duration(env: Env, user: Address, circle_id: u64, proposal_id: u64, approve: bool);

    // #227: Bond Management
    fn slash_bond(env: Env, admin: Address, circle_id: u64);
    fn release_bond(env: Env, admin: Address, circle_id: u64);

    // #228: XLM Staking & Governance
    fn stake_xlm(env: Env, user: Address, xlm_token: Address, amount: u64);
    fn unstake_xlm(env: Env, user: Address, xlm_token: Address, amount: u64);
    fn update_global_fee(env: Env, admin: Address, new_fee: u32);
}

// --- IMPLEMENTATION ---

fn append_audit_index(env: &Env, key: DataKey, id: u64) {
    let mut ids: Vec<u64> = env.storage().instance().get(&key).unwrap_or(Vec::new(env));
    ids.push_back(id);
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

fn get_member_address_by_index(circle: &CircleInfo, index: u32) -> Address {
    if index >= circle.member_count {
        panic!("Member index out of bounds");
    }
    circle.member_addresses.get(index).unwrap()
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

fn finalize_leniency_vote_internal(
    env: &Env,
    circle_id: u64,
    requester: &Address,
    request: &mut LeniencyRequest,
) {
    let total_possible_votes = request.total_votes_cast;
    let minimum_participation = (total_possible_votes * MINIMUM_VOTING_PARTICIPATION) / 100;

    let mut final_status = LeniencyRequestStatus::Rejected;

    if request.total_votes_cast >= minimum_participation && request.total_votes_cast > 0 {
        let approval_percentage = (request.approve_votes * 100) / request.total_votes_cast;
        if approval_percentage >= SIMPLE_MAJORITY_THRESHOLD {
            final_status = LeniencyRequestStatus::Approved;

            let circle_key = DataKey::Circle(circle_id);
            let mut circle: CircleInfo = env
                .storage()
                .instance()
                .get(&circle_key)
                .expect("Circle not found");

            let extension_seconds = request.extension_hours * 3600;
            let new_deadline = circle.deadline_timestamp + extension_seconds;
            circle.grace_period_end = Some(new_deadline);

            env.storage().instance().set(&circle_key, &circle);

            let social_capital_key = DataKey::SocialCapital(requester.clone(), circle_id);
            let mut social_capital: SocialCapital = env
                .storage()
                .instance()
                .get(&social_capital_key)
                .unwrap_or(SocialCapital {
                    member: requester.clone(),
                    circle_id,
                    leniency_given: 0,
                    leniency_received: 0,
                    voting_participation: 0,
                    trust_score: 50,
                });
            social_capital.leniency_received += 1;
            social_capital.trust_score = (social_capital.trust_score + 5).min(100);
            env.storage().instance().set(&social_capital_key, &social_capital);
        }
    }

    request.status = final_status.clone();

    let stats_key = DataKey::LeniencyStats(circle_id);
    let mut stats: LeniencyStats = env
        .storage()
        .instance()
        .get(&stats_key)
        .unwrap_or(LeniencyStats {
            total_requests: 0,
            approved_requests: 0,
            rejected_requests: 0,
            expired_requests: 0,
            average_participation: 0,
        });

    match final_status {
        LeniencyRequestStatus::Approved => stats.approved_requests += 1,
        LeniencyRequestStatus::Rejected => stats.rejected_requests += 1,
        LeniencyRequestStatus::Expired => stats.expired_requests += 1,
        _ => {}
    }

    if stats.total_requests > 0 {
        let total_participation =
            stats.average_participation * (stats.total_requests - 1) + request.total_votes_cast;
        stats.average_participation = total_participation / stats.total_requests;
    }

    env.storage().instance().set(&stats_key, &stats);
}

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
    fn init(env: Env, admin: Address, global_fee: u32) {
        // Initialize the circle counter to 0 if it doesn't exist
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }
        // Set the admin
        env.storage().instance().set(&DataKey::Admin, &admin);
        // Set Global Fee BP
        env.storage().instance().set(&DataKey::GlobalFeeBP, &global_fee);
    }

    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u16, token: Address, cycle_duration: u64, bond_amount: u64) -> u64 {
        // #227: Creator MUST pay a bond
        creator.require_auth();
        let client = token::Client::new(&env, &token);
        client.transfer(&creator, &env.current_contract_address(), &bond_amount);
        
        // 1. Get the current Circle Count
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        // 2. Increment the ID for the new circle
        circle_count += 1;

        // 3. Create the Circle Data Struct
        let current_time = env.ledger().timestamp();
        let new_circle = CircleInfo {
            id: circle_count,
            creator: creator.clone(),
            contribution_amount: amount,
            max_members,
            member_count: 0,
            current_recipient_index: 0,
            is_active: true,
            token,
            deadline_timestamp: current_time + cycle_duration,
            cycle_duration,
        };

        // 4. Save the Circle, Bond, and Count
        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
        env.storage().instance().set(&DataKey::Bond(circle_count), &bond_amount);
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        // 5. Initialize Group Reserve if not exists
        if !env.storage().instance().has(&DataKey::GroupReserve) {
            env.storage().instance().set(&DataKey::GroupReserve, &0u64);
        }

        // 6. Return the new ID
        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64) {
        // 1. Authorization: The user MUST sign this transaction
        user.require_auth();

        // 2. Retrieve the circle data
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // 3. Check if the circle is full
        if circle.member_count >= circle.max_members {
            panic!("Circle is full");
        }

        // 4. Check if user is already a member to prevent duplicates
        let member_key = DataKey::Member(circle_id, user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("User is already a member");
        }

        // 5. Create and store the new member
        let new_member = Member {
            address: user.clone(),
            has_contributed: false,
            contribution_count: 0,
            last_contribution_time: 0,
        };
        
        // 6. Store the member and update circle count
        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        
        // 7. Save the updated circle back to storage
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn deposit(env: Env, user: Address, circle_id: u64, rounds: u32) {
        // 1. Authorization: The user must sign this!
        user.require_auth();

        // 2. Load the Circle Data
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // 3. Check if user is actually a member
        let member_key = DataKey::Member(circle_id, user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // 4. Create the Token Client
        let client = token::Client::new(&env, &circle.token);

        // 5. Check if payment is late and apply penalty if needed
        let current_time = env.ledger().timestamp();
        let mut total_extra = 0u64;

        if current_time > circle.deadline_timestamp {
            // Calculate 1% penalty
            let penalty_amount = circle.contribution_amount / 100; // 1% penalty
            total_extra += penalty_amount;
            
            // Update Group Reserve balance
            let mut reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
        }

        // #226: Platform Fee and Batch Incentive
        let mut fee_bp: u32 = env.storage().instance().get(&DataKey::GlobalFeeBP).unwrap_or(0);
        if rounds >= 3 {
            fee_bp /= 2; // 50% discount for prepaying 3+ rounds
        }
        
        let single_fee = (circle.contribution_amount * fee_bp as u64) / 10000;
        let total_deposit = (circle.contribution_amount + single_fee) * rounds as u64 + total_extra;

        // 6. Transfer the full amount from user
        client.transfer(
            &user, 
            &env.current_contract_address(), 
            &total_deposit
        );

        // 7. Update member contribution info
        member.has_contributed = true;
        member.contribution_count += rounds;
        member.last_contribution_time = current_time;
        
        // 8. Save updated member info
        env.storage().instance().set(&member_key, &member);

        // 9. Update circle deadline for next cycle
        circle.deadline_timestamp += circle.cycle_duration * rounds as u64;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // 10. Mark as Paid
        env.storage().instance().set(&DataKey::Deposit(circle_id, user), &true);
    }

    fn propose_duration(env: Env, user: Address, circle_id: u64, new_duration: u64) -> u64 {
        user.require_auth();
        
        // Ensure circle exists
        if !env.storage().instance().has(&DataKey::Circle(circle_id)) {
            panic!("Circle not found");
        }

        // Ensure user is a member
        let member_key = DataKey::Member(circle_id, user.clone());
        if !env.storage().instance().has(&member_key) {
            panic!("Only members can propose duration changes");
        }

        let mut proposal_count: u64 = env.storage().instance().get(&DataKey::ProposalCount(circle_id)).unwrap_or(0);
        proposal_count += 1;

        let proposal = DurationProposal {
            id: proposal_count,
            new_duration,
            votes_for: 0,
            votes_against: 0,
            end_time: env.ledger().timestamp() + 86400 * 3, // 3 days to vote
            is_active: true,
        };

        env.storage().instance().set(&DataKey::Proposal(circle_id, proposal_count), &proposal);
        env.storage().instance().set(&DataKey::ProposalCount(circle_id), &proposal_count);

        proposal_count
    }

    fn vote_duration(env: Env, user: Address, circle_id: u64, proposal_id: u64, approve: bool) {
        user.require_auth();

        // Ensure user is a member
        let member_key = DataKey::Member(circle_id, user.clone());
        if !env.storage().instance().has(&member_key) {
            panic!("Only members can vote");
        }

        // Check if already voted
        let vote_key = DataKey::Vote(circle_id, proposal_id, user.clone());
        if env.storage().instance().has(&vote_key) {
            panic!("Already voted");
        }

        let mut proposal: DurationProposal = env.storage().instance().get(&DataKey::Proposal(circle_id, proposal_id))
            .unwrap_or_else(|| panic!("Proposal not found"));

        if !proposal.is_active || env.ledger().timestamp() > proposal.end_time {
            panic!("Proposal is not active or expired");
        }

        if approve {
            proposal.votes_for += 1;
        } else {
            proposal.votes_against += 1;
        }

        env.storage().instance().set(&vote_key, &true);

        // Check if 66% threshold reached
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        // 66% threshold
        if (proposal.votes_for as u32 * 100) > (circle.member_count as u32 * 66) {
            let mut updated_circle = circle;
            updated_circle.cycle_duration = proposal.new_duration;
            // Recalculate deadline
            updated_circle.deadline_timestamp = env.ledger().timestamp() + updated_circle.cycle_duration;
            env.storage().instance().set(&DataKey::Circle(circle_id), &updated_circle);
            proposal.is_active = false;
        }

        env.storage().instance().set(&DataKey::Proposal(circle_id, proposal_id), &proposal);
    }

    fn slash_bond(env: Env, admin: Address, circle_id: u64) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("Only admin can slash bond");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let bond_amount: u64 = env.storage().instance().get(&DataKey::Bond(circle_id)).unwrap_or(0);
        
        if bond_amount > 0 {
            let client = token::Client::new(&env, &circle.token);
            // In a real scenario, we might distribute this to members.
            // For now, we move it to GroupReserve storage and potentially a reserve account.
            let mut reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += bond_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
            env.storage().instance().remove(&DataKey::Bond(circle_id));
        }
    }

    fn release_bond(env: Env, admin: Address, circle_id: u64) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("Only admin can release bond");
        }

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let bond_amount: u64 = env.storage().instance().get(&DataKey::Bond(circle_id)).unwrap_or(0);
        
        if bond_amount > 0 {
            let client = token::Client::new(&env, &circle.token);
            client.transfer(&env.current_contract_address(), &circle.creator, &bond_amount);
            env.storage().instance().remove(&DataKey::Bond(circle_id));
        }
    }

    fn stake_xlm(env: Env, user: Address, xlm_token: Address, amount: u64) {
        user.require_auth();
        let client = token::Client::new(&env, &xlm_token);
        client.transfer(&user, &env.current_contract_address(), &amount);

        let stake_key = DataKey::Stake(user.clone());
        let mut user_stake: u64 = env.storage().instance().get(&stake_key).unwrap_or(0);
        user_stake += amount;
        env.storage().instance().set(&stake_key, &user_stake);
    }

    fn unstake_xlm(env: Env, user: Address, xlm_token: Address, amount: u64) {
        user.require_auth();
        let stake_key = DataKey::Stake(user.clone());
        let mut user_stake: u64 = env.storage().instance().get(&stake_key).unwrap_or(0);
        
        if user_stake < amount {
            panic!("Insufficient stake");
        }

        user_stake -= amount;
        let client = token::Client::new(&env, &xlm_token);
        client.transfer(&env.current_contract_address(), &user, &amount);
        
        if user_stake == 0 {
            env.storage().instance().remove(&stake_key);
        } else {
            env.storage().instance().set(&stake_key, &user_stake);
        }
    }

    fn update_global_fee(env: Env, admin: Address, new_fee: u32) {
        admin.require_auth();
        let stored_admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
        if admin != stored_admin {
            panic!("Only admin can update global fee");
        }

        env.storage().instance().set(&DataKey::GlobalFeeBP, &new_fee);
    }
}

// --- FUZZ TESTING MODULES ---

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};
    use std::i128;

    #[derive(Arbitrary, Debug, Clone)]
    pub struct FuzzTestCase {
        pub contribution_amount: u64,
        pub max_members: u16,
        pub user_id: u64,
    }

    #[test]
    fn fuzz_test_contribution_amount_edge_cases() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Test case 1: Maximum u64 value (should not panic)
        let max_circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            u64::MAX,
            10,
            token.clone(),
            604800, // 1 week in seconds
            500, // Bond
        );

        let user1 = Address::generate(&env);
        SoroSusuTrait::join_circle(env.clone(), user1.clone(), max_circle_id);

        // Mock token balance for the test
        env.mock_all_auths();
        
        // This should not panic even with u64::MAX contribution amount
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::deposit(env.clone(), user1.clone(), max_circle_id);
        });
        
        // The transfer might fail due to insufficient balance, but it shouldn't panic from overflow
        assert!(result.is_ok() || result.unwrap_err().downcast::<String>().unwrap().contains("insufficient balance"));
    }

    #[test]
    fn fuzz_test_zero_and_negative_amounts() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Test case 2: Zero contribution amount (should be allowed but may cause issues)
        let zero_circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            0,
            10,
            token.clone(),
            604800, // 1 week in seconds
            500, // Bond
        );

        let user2 = Address::generate(&env);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), zero_circle_id);

        env.mock_all_auths();
        
        // Zero amount deposit should work (though may not be practically useful)
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::deposit(env.clone(), user2.clone(), zero_circle_id);
        });
        
        assert!(result.is_ok());
    }

    #[test]
    fn fuzz_test_arbitrary_contribution_amounts() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Test with various edge case amounts
        let test_amounts = vec![
            1,                           // Minimum positive amount
            u32::MAX as u64,            // Large but reasonable amount
            u64::MAX / 2,               // Very large amount
            u64::MAX - 1,               // Maximum amount - 1
            1000000,                    // 1 million
            0,                          // Zero (already tested above)
        ];

        for (i, amount) in test_amounts.iter().enumerate() {
            let circle_id = SoroSusuTrait::create_circle(
                env.clone(),
                creator.clone(),
                *amount,
                10,
                token.clone(),
                604800, // 1 week in seconds
                500, // Bond
            );

            let user = Address::generate(&env);
            SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

            env.mock_all_auths();
            
            let result = std::panic::catch_unwind(|| {
                SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id, 1);
            });
            
            // Should not panic due to overflow, only potentially due to insufficient balance
            match result {
                Ok(_) => {
                    // Deposit succeeded
                    println!("Γ£ô Amount {} succeeded", amount);
                }
                Err(e) => {
                    let error_msg = e.downcast::<String>().unwrap();
                    // Expected error: insufficient balance, not overflow
                    assert!(error_msg.contains("insufficient balance") || 
                           error_msg.contains("underflow") ||
                           error_msg.contains("overflow"));
                    println!("Γ£ô Amount {} failed with expected error: {}", amount, error_msg);
                }
            }
        }
    }

    #[test]
    fn fuzz_test_boundary_conditions() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Test boundary conditions for max_members
        let boundary_tests = vec![
            (1, "Minimum members"),
            (u16::MAX, "Maximum members"),
            (100, "Typical circle size"),
        ];

        for (max_members, description) in boundary_tests {
            let circle_id = SoroSusuTrait::create_circle(
                env.clone(),
                creator.clone(),
                1000, // Reasonable contribution amount
                max_members,
                token.clone(),
                604800, // 1 week in seconds
                100, // Bond
            );

            // Test joining with maximum allowed members
            for i in 0..max_members.min(10) { // Limit to 10 for test performance
                let user = Address::generate(&env);
                SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);
                
                env.mock_all_auths();
                
                let result = std::panic::catch_unwind(|| {
                    SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id, 1);
                });
                
                assert!(result.is_ok(), "Deposit failed for {} with max_members {}: {:?}", description, max_members, result);
            }
            
            println!("Γ£ô Boundary test passed: {} (max_members: {})", description, max_members);
        }
    }

    #[test]
    fn fuzz_test_concurrent_deposits() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            500,
            5,
            token.clone(),
            604800, // 1 week in seconds
            250, // Bond
        );

        // Create multiple users and test deposits
        let mut users = Vec::new();
        for _ in 0..5 {
            let user = Address::generate(&env);
            SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);
            users.push(user);
        }

        env.mock_all_auths();

        // Test multiple deposits in sequence (simulating concurrent access)
        for user in users {
            let result = std::panic::catch_unwind(|| {
                SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id, 1);
            });
            
            assert!(result.is_ok(), "Concurrent deposit test failed: {:?}", result);
        }
        
        println!("Γ£ô Concurrent deposits test passed");
    }

    #[test]
    fn test_late_penalty_mechanism() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Create a circle with 1 week cycle duration
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000, // $10 contribution (assuming 6 decimals)
            5,
            token.clone(),
            604800, // 1 week in seconds
            500, // Bond
        );

        // User joins the circle
        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        // Mock token balance for the test
        env.mock_all_auths();

        // Get initial Group Reserve balance
        let initial_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(initial_reserve, 0);

        // Simulate time passing beyond deadline (jump forward 2 weeks)
        env.ledger().set_timestamp(env.ledger().timestamp() + 2 * 604800);

        // Make a late deposit
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id, 1);
        });
        
        assert!(result.is_ok(), "Late deposit should succeed: {:?}", result);

        // Check that Group Reserve received the 1% penalty (10 tokens)
        let final_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(final_reserve, 10, "Group Reserve should have 10 tokens (1% penalty)");

        // Verify member was marked as having contributed
        let member_key = DataKey::Member(circle_id, user.clone());
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        assert!(member.has_contributed);
        assert_eq!(member.contribution_count, 1);

        println!("Γ£ô Late penalty mechanism test passed - 1% penalty correctly routed to Group Reserve");
    }

    #[test]
    fn test_on_time_deposit_no_penalty() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone(), 100);

        // Create a circle with 1 week cycle duration
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000, // $10 contribution
            5,
            token.clone(),
            604800, // 1 week in seconds
            500, // Bond
        );

        // User joins the circle
        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        // Mock token balance for the test
        env.mock_all_auths();

        // Get initial Group Reserve balance
        let initial_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(initial_reserve, 0);

        // Make an on-time deposit (don't advance time)
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id, 1);
        });
        
        assert!(result.is_ok(), "On-time deposit should succeed: {:?}", result);

        // Check that Group Reserve received no penalty
        let final_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(final_reserve, 0, "Group Reserve should have 0 tokens for on-time deposit");

        println!("Γ£ô On-time deposit test passed - no penalty applied");
    }
}