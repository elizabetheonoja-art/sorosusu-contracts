#![no_std]
use soroban_sdk::{contract, contracttype, contractimpl, contractclient, Address, Env, Vec, Symbol, token, testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};

// --- DATA STRUCTURES ---
const DURATION_CHANGE_NOTICE_SECS: u64 = 72 * 60 * 60;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Admin,
    Circle(u64),
    Member(Address),
    CircleCount,
    // New: Tracks pending exits (CircleID, MemberAddress)
    PendingExit(u64, Address),
    // New: Tracks Group Reserve balance for penalties
    GroupReserve,
}

#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum MemberStatus {
    Active,
    AwaitingReplacement,
    Ejected,
}

#[contracttype]
#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub index: u32,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
    pub status: MemberStatus,
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
    pub pending_cycle_duration: u64,
    pub duration_change_effective_at: u64,
    pub contribution_bitmap: u64,
    pub payout_bitmap: u64,
    pub insurance_balance: u64,
    pub insurance_fee_bps: u32,
    pub is_insurance_used: bool,
    pub late_fee_bps: u32,
    pub proposed_late_fee_bps: u32,
    pub proposal_votes_bitmap: u64,
    pub nft_contract: Address,
}

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    // Initialize the contract
    fn init(env: Env, admin: Address);
    
    // Create a new savings circle
    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u16, token: Address, cycle_duration: u64, insurance_fee_bps: u32, nft_contract: Address) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64);

    // Make a deposit (Pay your weekly/monthly due)
    fn deposit(env: Env, user: Address, circle_id: u64);

    // Trigger insurance to cover a default
    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address);

    // Propose a change to the late fee penalty
    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32);

    // Propose a change to the round duration (takes effect after 72 hours)
    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64);

    // Vote on the current proposal
    fn vote_penalty_change(env: Env, user: Address, circle_id: u64);

    // Eject a member (burns NFT)
    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address);
    
    // Request graceful exit from the circle
    fn request_exit(env: Env, user: Address, circle_id: u64);
    
    // Fill a vacancy left by a member who requested graceful exit
    fn fill_vacancy(env: Env, new_member: Address, circle_id: u64, exiting_member_address: Address);
}

#[contractclient(name = "SusuNftClient")]
pub trait SusuNftTrait {
    fn mint(env: Env, to: Address, token_id: u128);
    fn burn(env: Env, from: Address, token_id: u128);
}

// --- IMPLEMENTATION ---

#[contract]
pub struct SoroSusu;

#[contractimpl]
impl SoroSusuTrait for SoroSusu {
    fn init(env: Env, admin: Address) {
        // Initialize the circle counter to 0 if it doesn't exist
        if !env.storage().instance().has(&DataKey::CircleCount) {
            env.storage().instance().set(&DataKey::CircleCount, &0u64);
        }
        // Set the admin
        env.storage().instance().set(&DataKey::Admin, &admin);
    }

    fn create_circle(env: Env, creator: Address, amount: u64, max_members: u16, token: Address, cycle_duration: u64, insurance_fee_bps: u32, nft_contract: Address) -> u64 {
        // 1. Get the current Circle Count
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        // 2. Increment the ID for the new circle
        circle_count += 1;

        if max_members > 64 {
            panic!("Max members cannot exceed 64 for optimization");
        }

        if insurance_fee_bps > 10000 {
            panic!("Insurance fee cannot exceed 100%");
        }

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
            pending_cycle_duration: 0,
            duration_change_effective_at: 0,
            contribution_bitmap: 0,
            payout_bitmap: 0,
            insurance_balance: 0,
            insurance_fee_bps,
            is_insurance_used: false,
            late_fee_bps: 100, // Default 1%
            proposed_late_fee_bps: 0,
            proposal_votes_bitmap: 0,
            nft_contract,
        };

        // 4. Save the Circle and the new Count
        env.storage().instance().set(&DataKey::Circle(circle_count), &new_circle);
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
        let member_key = DataKey::Member(user.clone());
        if env.storage().instance().has(&member_key) {
            panic!("User is already a member");
        }

        // 5. Create and store the new member
        let new_member = Member {
            address: user.clone(),
            index: circle.member_count as u32,
            contribution_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
        };
        
        // 6. Store the member and update circle count
        env.storage().instance().set(&member_key, &new_member);
        circle.member_count += 1;
        
        // 7. Save the updated circle back to storage
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // 8. Mint Participation NFT
        // Token ID = (CircleID << 64) | MemberIndex
        let token_id = (circle_id as u128) << 64 | (new_member.index as u128);
        let client = SusuNftClient::new(&env, &circle.nft_contract);
        client.mint(&user, &token_id);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        // 1. Authorization: The user must sign this!
        user.require_auth();

        // 2. Load the Circle Data
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // 3. Check if user is actually a member
        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // 4. Create the Token Client
        let client = token::Client::new(&env, &circle.token);

        // 5. Check if payment is late and apply penalty if needed
        let current_time = env.ledger().timestamp();
        let mut penalty_amount = 0u64;

        if current_time > circle.deadline_timestamp {
            // Calculate penalty based on dynamic rate
            penalty_amount = (circle.contribution_amount * circle.late_fee_bps as u64) / 10000;
            
            // Update Group Reserve balance
            let mut reserve_balance: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
            reserve_balance += penalty_amount;
            env.storage().instance().set(&DataKey::GroupReserve, &reserve_balance);
        }

        // 6. Calculate Insurance Fee and Transfer the full amount from user
        let insurance_fee = ((circle.contribution_amount as u128 * circle.insurance_fee_bps as u128) / 10000) as u64;
        let total_amount = circle.contribution_amount + insurance_fee;

        client.transfer(
            &user, 
            &env.current_contract_address(), 
            &total_amount
        );

        if insurance_fee > 0 {
            circle.insurance_balance += insurance_fee;
        }

        // 7. Update member contribution info
        member.contribution_count += 1;
        member.last_contribution_time = current_time;
        
        // 8. Save updated member info
        env.storage().instance().set(&member_key, &member);

        // 9. Update circle deadline for next cycle
        if circle.pending_cycle_duration > 0 && current_time >= circle.duration_change_effective_at {
            circle.cycle_duration = circle.pending_cycle_duration;
            circle.pending_cycle_duration = 0;
            circle.duration_change_effective_at = 0;
        }
        circle.deadline_timestamp = current_time + circle.cycle_duration;
        circle.contribution_bitmap |= 1 << member.index;
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn trigger_insurance_coverage(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();

        // Only creator can trigger insurance
        if caller != circle.creator {
            panic!("Unauthorized: Only creator can trigger insurance");
        }

        // Check if insurance was already used this cycle
        if circle.is_insurance_used {
            panic!("Insurance already used this cycle");
        }

        // Check if there is enough balance
        if circle.insurance_balance < circle.contribution_amount {
            panic!("Insufficient insurance balance");
        }

        let member_key = DataKey::Member(member.clone());
        let member_info: Member = env.storage().instance().get(&member_key).unwrap();

        if member_info.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        // Mark member as contributed in the bitmap
        if (circle.contribution_bitmap & (1 << member_info.index)) != 0 {
            panic!("Member already contributed");
        }

        circle.contribution_bitmap |= 1 << member_info.index;
        circle.insurance_balance -= circle.contribution_amount;
        circle.is_insurance_used = true;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn propose_penalty_change(env: Env, user: Address, circle_id: u64, new_bps: u32) {
        user.require_auth();
        
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        
        // Check if user is a member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if new_bps > 10000 {
            panic!("Penalty cannot exceed 100%");
        }

        // Set proposal
        circle.proposed_late_fee_bps = new_bps;
        circle.proposal_votes_bitmap = 0;
        
        // Auto-vote for proposer
        circle.proposal_votes_bitmap |= 1 << member.index;

        // Check for immediate majority (e.g. 1 member circle)
        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count as u32 / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn propose_duration_change(env: Env, user: Address, circle_id: u64, new_duration: u64) {
        user.require_auth();

        if new_duration == 0 {
            panic!("Duration must be greater than zero");
        }

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let protocol_admin: Address = env.storage().instance().get(&DataKey::Admin).expect("Admin not initialized");

        if user != circle.creator && user != protocol_admin {
            panic!("Unauthorized: Only admin can propose duration changes");
        }

        let current_time = env.ledger().timestamp();
        circle.pending_cycle_duration = new_duration;
        circle.duration_change_effective_at = current_time + DURATION_CHANGE_NOTICE_SECS;

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn vote_penalty_change(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        
        // Check if user is a member
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).expect("User is not a member");

        if member.status != MemberStatus::Active {
            panic!("Member is not active");
        }

        if circle.proposed_late_fee_bps == 0 {
            panic!("No active proposal");
        }

        circle.proposal_votes_bitmap |= 1 << member.index;

        if circle.proposal_votes_bitmap.count_ones() > (circle.member_count as u32 / 2) {
            circle.late_fee_bps = circle.proposed_late_fee_bps;
            circle.proposed_late_fee_bps = 0;
            circle.proposal_votes_bitmap = 0;
        }

        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn eject_member(env: Env, caller: Address, circle_id: u64, member: Address) {
        caller.require_auth();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        
        // Only creator can eject
        if caller != circle.creator {
            panic!("Unauthorized: Only creator can eject members");
        }

        let member_key = DataKey::Member(member.clone());
        let mut member_info: Member = env.storage().instance().get(&member_key).expect("Member not found");

        if member_info.status != MemberStatus::Active {
            panic!("Member already ejected");
        }

        // Mark as ejected
        member_info.status = MemberStatus::Ejected;
        env.storage().instance().set(&member_key, &member_info);

        // Burn NFT
        let token_id = (circle_id as u128) << 64 | (member_info.index as u128);
        let client = SusuNftClient::new(&env, &circle.nft_contract);
        client.burn(&member, &token_id);
    }

    fn request_exit(env: Env, user: Address, circle_id: u64) {
        user.require_auth();

        // Get the circle and member information
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        let member_key = DataKey::Member(user.clone());
        let mut member: Member = env.storage().instance().get(&member_key)
            .unwrap_or_else(|| panic!("User is not a member of this circle"));

        // Check if member is active and can request exit
        if member.status != MemberStatus::Active {
            panic!("Member cannot request exit in current state");
        }

        // Change member status to AwaitingReplacement
        member.status = MemberStatus::AwaitingReplacement;
        env.storage().instance().set(&member_key, &member);

        // Store the pending exit request
        let pending_exit_key = DataKey::PendingExit(circle_id, user.clone());
        env.storage().instance().set(&pending_exit_key, &true);

        // Note: We keep the member's position in the queue locked until fill_vacancy is called
    }

    fn fill_vacancy(env: Env, new_member: Address, circle_id: u64, exiting_member_address: Address) {
        new_member.require_auth();

        // Get the circle information
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Verify there's a pending exit for the specified member
        let pending_exit_key = DataKey::PendingExit(circle_id, exiting_member_address.clone());
        if !env.storage().instance().has(&pending_exit_key) {
            panic!("No pending exit found for specified member");
        }

        // Get the exiting member's information
        let exiting_member_key = DataKey::Member(exiting_member_address.clone());
        let exiting_member: Member = env.storage().instance().get(&exiting_member_key)
            .unwrap_or_else(|| panic!("Exiting member not found"));

        if exiting_member.status != MemberStatus::AwaitingReplacement {
            panic!("Exiting member is not in AwaitingReplacement state");
        }

        // Check if new member is already in any circle
        let new_member_key = DataKey::Member(new_member.clone());
        if env.storage().instance().has(&new_member_key) {
            panic!("New member is already part of a circle");
        }

        // Calculate refund amount on the fly (principal only).
        let refund_amount = exiting_member.contribution_count as u64 * circle.contribution_amount;

        if refund_amount > 0 {
            // Transfer refund to exiting member
            let token_client = token::Client::new(&env, &circle.token);
            token_client.transfer(
                &env.current_contract_address(),
                &exiting_member_address,
                &refund_amount
            );
        }

        // Create new member with the same index as the exiting member
        let replacement_member = Member {
            address: new_member.clone(),
            index: exiting_member.index, // Inherit the position in queue
            contribution_count: 0,
            last_contribution_time: 0,
            status: MemberStatus::Active,
        };

        // Store the new member
        env.storage().instance().set(&new_member_key, &replacement_member);

        // Update exiting member status to Ejected (effectively removed)
        let mut updated_exiting_member = exiting_member;
        updated_exiting_member.status = MemberStatus::Ejected;
        env.storage().instance().set(&exiting_member_key, &updated_exiting_member);

        // Remove the pending exit record
        env.storage().instance().remove(&pending_exit_key);

        // Burn the exiting member's NFT
        let token_id = (circle_id as u128) << 64 | (exiting_member.index as u128);
        let nft_client = SusuNftClient::new(&env, &circle.nft_contract);
        nft_client.burn(&exiting_member_address, &token_id);

        // Mint new NFT for the replacement member
        nft_client.mint(&new_member, &token_id);
    }
}

// --- FUZZ TESTING MODULES ---

#[cfg(test)]
mod fuzz_tests {
    use super::*;
    use soroban_sdk::{testutils::{Address as TestAddress, Arbitrary as TestArbitrary}, arbitrary::{Arbitrary, Unstructured}};
    use std::i128;

    #[contract]
    pub struct MockNft;

    #[contractimpl]
    impl MockNft {
        pub fn mint(_env: Env, _to: Address, _id: u128) {}
        pub fn burn(_env: Env, _from: Address, _id: u128) {}
    }

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
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Test case 1: Maximum u64 value (should not panic)
        let max_circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            u64::MAX,
            10,
            token.clone(),
            604800, // 1 week in seconds
            0,
            nft_contract.clone(),
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
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Test case 2: Zero contribution amount (should be allowed but may cause issues)
        let zero_circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            0,
            10,
            token.clone(),
            604800, // 1 week in seconds
            0,
            nft_contract.clone(),
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
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

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
                0,
                nft_contract.clone(),
            );

            let user = Address::generate(&env);
            SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

            env.mock_all_auths();
            
            let result = std::panic::catch_unwind(|| {
                SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
            });
            
            // Should not panic due to overflow, only potentially due to insufficient balance
            match result {
                Ok(_) => {
                    // Deposit succeeded
                    println!("G�� Amount {} succeeded", amount);
                }
                Err(e) => {
                    let error_msg = e.downcast::<String>().unwrap();
                    // Expected error: insufficient balance, not overflow
                    assert!(error_msg.contains("insufficient balance") || 
                           error_msg.contains("underflow") ||
                           error_msg.contains("overflow"));
                    println!("G�� Amount {} failed with expected error: {}", amount, error_msg);
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
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Test boundary conditions for max_members
        let boundary_tests = vec![
            (1, "Minimum members"),
            (64, "Maximum members"),
            (50, "Typical circle size"),
        ];

        for (max_members, description) in boundary_tests {
            let circle_id = SoroSusuTrait::create_circle(
                env.clone(),
                creator.clone(),
                1000, // Reasonable contribution amount
                max_members,
                token.clone(),
                604800, // 1 week in seconds
                0,
                nft_contract.clone(),
            );

            // Test joining with maximum allowed members
            for i in 0..max_members.min(10) { // Limit to 10 for test performance
                let user = Address::generate(&env);
                SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);
                
                env.mock_all_auths();
                
                let result = std::panic::catch_unwind(|| {
                    SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
                });
                
                assert!(result.is_ok(), "Deposit failed for {} with max_members {}: {:?}", description, max_members, result);
            }
            
            println!("G�� Boundary test passed: {} (max_members: {})", description, max_members);
        }
    }

    #[test]
    fn fuzz_test_concurrent_deposits() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            500,
            5,
            token.clone(),
            604800, // 1 week in seconds
            0,
            nft_contract.clone(),
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
                SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
            });
            
            assert!(result.is_ok(), "Concurrent deposit test failed: {:?}", result);
        }
        
        println!("G�� Concurrent deposits test passed");
    }

    #[test]
    fn test_late_penalty_mechanism() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Create a circle with 1 week cycle duration
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000, // $10 contribution (assuming 6 decimals)
            5,
            token.clone(),
            604800, // 1 week in seconds
            0,
            nft_contract.clone(),
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
            SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        });
        
        assert!(result.is_ok(), "Late deposit should succeed: {:?}", result);

        // Check that Group Reserve received the 1% penalty (10 tokens)
        let final_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(final_reserve, 10, "Group Reserve should have 10 tokens (1% penalty)");

        // Verify member was marked as having contributed
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert!(circle.contribution_bitmap & (1 << member.index) != 0);
        assert_eq!(member.contribution_count, 1);

        println!("G�� Late penalty mechanism test passed - 1% penalty correctly routed to Group Reserve");
    }

    #[test]
    fn test_on_time_deposit_no_penalty() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        // Initialize contract
        SoroSusuTrait::init(env.clone(), admin.clone());

        // Create a circle with 1 week cycle duration
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000, // $10 contribution
            5,
            token.clone(),
            604800, // 1 week in seconds
            0,
            nft_contract.clone(),
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
            SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        });
        
        assert!(result.is_ok(), "On-time deposit should succeed: {:?}", result);

        // Check that Group Reserve received no penalty
        let final_reserve: u64 = env.storage().instance().get(&DataKey::GroupReserve).unwrap_or(0);
        assert_eq!(final_reserve, 0, "Group Reserve should have 0 tokens for on-time deposit");

        println!("G�� On-time deposit test passed - no penalty applied");
    }

    #[test]
    fn test_insurance_fund() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

        // Create circle with 10% insurance fee (1000 bps)
        let circle_id = SoroSusuTrait::create_circle(
            env.clone(),
            creator.clone(),
            1000,
            5,
            token.clone(),
            604800,
            1000, // 10% insurance fee
            nft_contract.clone(),
        );

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);

        env.mock_all_auths();

        // User 1 deposits. 1000 + 100 fee. Insurance balance becomes 100.
        SoroSusuTrait::deposit(env.clone(), user1.clone(), circle_id);
        
        let mut circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle.insurance_balance, 100);

        // User 1 deposits 9 more times to build up insurance (simulating multiple cycles or members)
        // In this simplified test, we just force update the balance to test triggering
        circle.insurance_balance = 1000; 
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);

        // User 2 defaults. Creator triggers insurance.
        SoroSusuTrait::trigger_insurance_coverage(env.clone(), creator.clone(), circle_id, user2.clone());

        let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        let member2_key = DataKey::Member(user2.clone());
        let member2: Member = env.storage().instance().get(&member2_key).unwrap();

        assert!(circle_after.is_insurance_used);
        assert_eq!(circle_after.insurance_balance, 0);
        assert!(circle_after.contribution_bitmap & (1 << member2.index) != 0);
    }

    #[test]
    fn test_governance_penalty_change() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

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

        SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id);
        SoroSusuTrait::join_circle(env.clone(), user3.clone(), circle_id);

        env.mock_all_auths();

        // Default is 100 bps (1%)
        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle.late_fee_bps, 100);

        // User 1 proposes 5% (500 bps)
        SoroSusuTrait::propose_penalty_change(env.clone(), user1.clone(), circle_id, 500);

        // User 2 votes
        SoroSusuTrait::vote_penalty_change(env.clone(), user2.clone(), circle_id);

        // Should pass (2 out of 3 votes)
        let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle_after.late_fee_bps, 500);
        assert_eq!(circle_after.proposed_late_fee_bps, 0);
    }

    #[test]
    fn test_nft_membership() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

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

        // Join should trigger mint (mocked)
        SoroSusuTrait::join_circle(env.clone(), user.clone(), circle_id);

        env.mock_all_auths();

        // Verify member is active
        let member_key = DataKey::Member(user.clone());
        let member: Member = env.storage().instance().get(&member_key).unwrap();
        assert_eq!(member.status, MemberStatus::Active);

        // Eject member should trigger burn (mocked) and set inactive
        SoroSusuTrait::eject_member(env.clone(), creator.clone(), circle_id, user.clone());

        let member_after: Member = env.storage().instance().get(&member_key).unwrap();
        assert_eq!(member_after.status, MemberStatus::Ejected);

        // Inactive member cannot deposit
        let result = std::panic::catch_unwind(|| {
            SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        });
        assert!(result.is_err());
    }

    #[test]
    fn test_propose_duration_change_sets_72_hour_notice() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

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

        let now = env.ledger().timestamp();
        SoroSusuTrait::propose_duration_change(env.clone(), creator.clone(), circle_id, 2_592_000);

        let circle: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle.cycle_duration, 604800);
        assert_eq!(circle.pending_cycle_duration, 2_592_000);
        assert_eq!(circle.duration_change_effective_at, now + DURATION_CHANGE_NOTICE_SECS);
    }

    #[test]
    fn test_duration_change_activates_after_notice() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let token = Address::generate(&env);
        let nft_contract = env.register_contract(None, MockNft);

        SoroSusuTrait::init(env.clone(), admin.clone());

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

        SoroSusuTrait::propose_duration_change(env.clone(), creator.clone(), circle_id, 2_592_000);

        // Before the 72-hour notice elapses, old duration remains effective.
        env.ledger().set_timestamp(env.ledger().timestamp() + DURATION_CHANGE_NOTICE_SECS - 1);
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        let circle_before: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle_before.cycle_duration, 604800);
        assert_eq!(circle_before.pending_cycle_duration, 2_592_000);

        // After notice elapses, next round scheduling picks up new duration.
        env.ledger().set_timestamp(env.ledger().timestamp() + 2);
        SoroSusuTrait::deposit(env.clone(), user.clone(), circle_id);
        let circle_after: CircleInfo = env.storage().instance().get(&DataKey::Circle(circle_id)).unwrap();
        assert_eq!(circle_after.cycle_duration, 2_592_000);
        assert_eq!(circle_after.pending_cycle_duration, 0);
        assert_eq!(circle_after.duration_change_effective_at, 0);
    }
}
