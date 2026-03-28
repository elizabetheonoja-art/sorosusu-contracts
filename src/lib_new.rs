use soroban_sdk::{contract, contractimpl, Address, Env, Symbol, token, panic, Map, Vec, i128, u64, u32};

// --- DATA STRUCTURES ---

#[derive(Clone)]
pub struct CircleInfo {
    pub creator: Address,
    pub contribution_amount: u64,
    pub max_members: u16,
    pub current_members: u16,
    pub token: Address,
    pub cycle_duration: u64,
    pub insurance_fee_bps: u32, // basis points (100 = 1%)
    pub organizer_fee_bps: u32,  // basis points (100 = 1%)
    pub nft_contract: Address,
    pub arbitrator: Address,
    pub members: Vec<Address>,
    pub contributions: Map<Address, bool>,
    pub current_round: u32,
    pub round_start_time: u64,
    pub is_round_finalized: bool,
    pub current_pot_recipient: Option<Address>,
    pub gas_buffer_balance: i128, // XLM buffer for gas fees
    pub gas_buffer_enabled: bool,
}

#[derive(Clone)]
pub struct Member {
    pub address: Address,
    pub join_time: u64,
    pub total_contributions: i128,
    pub total_received: i128,
    pub has_contributed_current_round: bool,
    pub consecutive_missed_rounds: u32,
}

#[derive(Clone)]
pub struct GasBufferConfig {
    pub min_buffer_amount: i128,     // Minimum XLM to maintain as buffer
    pub max_buffer_amount: i128,     // Maximum XLM that can be buffered
    pub auto_refill_threshold: i128, // When to auto-refill the buffer
    pub emergency_buffer: i128,      // Emergency buffer for extreme network conditions
}

// --- STORAGE KEYS ---

#[derive(Clone)]
pub enum DataKey {
    Admin,
    CircleCount,
    Circle(u64),
    Member(Address),
    MemberByIndex(u64, u32), // For efficient recipient lookup
    GasBufferConfig(u64),  // Per-circle gas buffer config
    ProtocolConfig,
    ScheduledPayoutTime(u64),
}

// --- CONTRACT TRAIT ---

pub trait SoroSusuTrait {
    // Initialize the contract
    fn init(env: Env, admin: Address);
    
    // Create a new savings circle
    fn create_circle(
        env: Env,
        creator: Address,
        contribution_amount: u64,
        max_members: u16,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
        organizer_fee_bps: u32, // New parameter for commission
    ) -> u64;

    // Join an existing circle
    fn join_circle(env: Env, user: Address, circle_id: u64, guarantor: Option<Address>);

    // Make a deposit (Pay your weekly/monthly due)
    fn deposit(env: Env, user: Address, circle_id: u64);

    // NEW: Gas buffer management functions
    fn fund_gas_buffer(env: Env, circle_id: u64, amount: i128);
    fn set_gas_buffer_config(env: Env, circle_id: u64, config: GasBufferConfig);
    fn get_gas_buffer_balance(env: Env, circle_id: u64) -> i128;

    // NEW: Payout functions with gas buffer support
    fn distribute_payout(env: Env, caller: Address, circle_id: u64);
    fn trigger_payout(env: Env, admin: Address, circle_id: u64);
    fn finalize_round(env: Env, creator: Address, circle_id: u64);

    // Helper functions
    fn get_circle(env: Env, circle_id: u64) -> CircleInfo;
    fn get_member(env: Env, member: Address) -> Member;
    fn get_current_recipient(env: Env, circle_id: u64) -> Option<Address>;
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

    fn create_circle(
        env: Env,
        creator: Address,
        contribution_amount: u64,
        max_members: u16,
        token: Address,
        cycle_duration: u64,
        insurance_fee_bps: u32,
        nft_contract: Address,
        arbitrator: Address,
        organizer_fee_bps: u32,
    ) -> u64 {
        // Validate organizer fee (cannot exceed 100%)
        if organizer_fee_bps > 10_000 {
            panic!("Organizer fee cannot exceed 100%");
        }

        // Validate insurance fee (cannot exceed 100%)
        if insurance_fee_bps > 10_000 {
            panic!("Insurance fee cannot exceed 100%");
        }

        // Get the current Circle Count
        let mut circle_count: u64 = env.storage().instance().get(&DataKey::CircleCount).unwrap_or(0);
        
        // Increment for the new circle
        circle_count += 1;
        
        // Create the new circle
        let circle = CircleInfo {
            creator: creator.clone(),
            contribution_amount,
            max_members,
            current_members: 0,
            token: token.clone(),
            cycle_duration,
            insurance_fee_bps,
            organizer_fee_bps,
            nft_contract,
            arbitrator,
            members: Vec::new(&env),
            contributions: Map::new(&env),
            current_round: 0,
            round_start_time: env.ledger().timestamp(),
            is_round_finalized: false,
            current_pot_recipient: None,
            gas_buffer_balance: 0i128,
            gas_buffer_enabled: true, // Enable by default for reliability
        };

        // Store the circle
        env.storage().instance().set(&DataKey::Circle(circle_count), &circle);
        
        // Update the circle count
        env.storage().instance().set(&DataKey::CircleCount, &circle_count);

        // Set default gas buffer configuration for this circle
        let default_config = GasBufferConfig {
            min_buffer_amount: 10000000, // 0.01 XLM minimum
            max_buffer_amount: 1000000000, // 10 XLM maximum
            auto_refill_threshold: 5000000, // 0.005 XLM threshold
            emergency_buffer: 50000000, // 0.5 XLM emergency buffer
        };
        env.storage().instance().set(&DataKey::GasBufferConfig(circle_count), &default_config);

        circle_count
    }

    fn join_circle(env: Env, user: Address, circle_id: u64, guarantor: Option<Address>) {
        // Authorization: The user MUST sign this transaction
        user.require_auth();

        // Check if the circle exists
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check if the circle is full
        if circle.current_members >= circle.max_members {
            panic!("Circle is full");
        }

        // Check if the user is already a member
        if circle.members.contains(&user) {
            panic!("Already a member");
        }

        // Add the user to the members list
        circle.members.push_back(user.clone());
        circle.current_members += 1;

        // Store member by index for efficient lookup during payouts
        let member_index = circle.current_members - 1;
        env.storage().instance().set(&DataKey::MemberByIndex(circle_id, member_index as u32), &user);

        // Create member record
        let member = Member {
            address: user.clone(),
            join_time: env.ledger().timestamp(),
            total_contributions: 0i128,
            total_received: 0i128,
            has_contributed_current_round: false,
            consecutive_missed_rounds: 0,
        };

        // Store the member
        env.storage().instance().set(&DataKey::Member(user.clone()), &member);

        // Update the circle
        env.storage().instance().set(&DataKey::Circle(circle_id), &circle);
    }

    fn deposit(env: Env, user: Address, circle_id: u64) {
        // Authorization: The user must sign this!
        user.require_auth();

        // Get the circle
        let mut circle: CircleInfo = env.storage().instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Get the member
        let mut member: Member = env.storage::instance()
            .get(&DataKey::Member(user.clone()))
            .unwrap_or_else(|| panic!("Member not found"));

        // Check if already contributed this round
        if member.has_contributed_current_round {
            panic!("Already contributed this round");
        }

        // Calculate the total amount needed (contribution + insurance fee)
        let insurance_fee = (circle.contribution_amount as i128 * circle.insurance_fee_bps as i128) / 10_000;
        let total_amount = circle.contribution_amount as i128 + insurance_fee;

        // Transfer the tokens from user to contract
        let token_client = token::Client::new(&env, &circle.token);
        token_client.transfer(&user, &env.current_contract_address(), &total_amount);

        // Update member record
        member.has_contributed_current_round = true;
        member.total_contributions += total_amount;
        member.consecutive_missed_rounds = 0; // Reset missed rounds counter

        // Update circle contributions
        circle.contributions.set(user.clone(), true);

        // Store updated records
        env.storage::instance().set(&DataKey::Member(user), &member);
        env.storage::instance().set(&DataKey::Circle(circle_id), &circle);

        // Check if all members have contributed and auto-finalize if so
        Self::check_and_finalize_round(&env, circle_id);
    }

    // --- GAS BUFFER MANAGEMENT ---

    fn fund_gas_buffer(env: Env, circle_id: u64, amount: i128) {
        // Get the circle
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Get gas buffer config
        let config: GasBufferConfig = env.storage::instance()
            .get(&DataKey::GasBufferConfig(circle_id))
            .unwrap_or_else(|| panic!("Gas buffer config not found"));

        // Validate amount doesn't exceed maximum
        if circle.gas_buffer_balance + amount > config.max_buffer_amount {
            panic!("Amount exceeds maximum gas buffer limit");
        }

        // Transfer XLM from caller to contract
        let xlm_token = env.native_token();
        let token_client = token::Client::new(&env, &xlm_token);
        
        // Get caller address - in a real implementation, this would be extracted from auth
        let caller = env.current_contract_address(); 
        
        token_client.transfer(&caller, &env.current_contract_address(), &amount);

        // Update gas buffer balance
        circle.gas_buffer_balance += amount;

        // Store updated circle
        env.storage::instance().set(&DataKey::Circle(circle_id), &circle);

        // Emit event for gas buffer funding
        env.events().publish(
            (Symbol::new(&env, "gas_buffer_funded"), circle_id),
            (amount, circle.gas_buffer_balance),
        );
    }

    fn set_gas_buffer_config(env: Env, circle_id: u64, config: GasBufferConfig) {
        // Only circle creator can set config
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check authorization
        circle.creator.require_auth();

        // Validate config parameters
        if config.min_buffer_amount < 0 || config.max_buffer_amount <= config.min_buffer_amount {
            panic!("Invalid buffer configuration");
        }

        // Store the configuration
        env.storage::instance().set(&DataKey::GasBufferConfig(circle_id), &config);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "gas_buffer_config_updated"), circle_id),
            (config.min_buffer_amount, config.max_buffer_amount),
        );
    }

    fn get_gas_buffer_balance(env: Env, circle_id: u64) -> i128 {
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));
        
        circle.gas_buffer_balance
    }

    // --- PAYOUT FUNCTIONS WITH GAS BUFFER ---

    fn distribute_payout(env: Env, caller: Address, circle_id: u64) {
        // Authorization check
        caller.require_auth();

        // Get the circle
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Check if all members have contributed
        if !Self::all_members_contributed(&env, circle_id) {
            panic!("Not all members have contributed this cycle");
        }

        // Get the current recipient
        let recipient = Self::get_current_recipient(&env, circle_id)
            .unwrap_or_else(|| panic!("No recipient found"));

        // Calculate payout amounts
        let gross_payout = (circle.contribution_amount as i128) * (circle.current_members as i128);
        let organizer_fee = (gross_payout * circle.organizer_fee_bps as i128) / 10_000;
        let net_payout = gross_payout - organizer_fee;

        // Check gas buffer and ensure sufficient funds for transaction
        Self::ensure_gas_buffer(&env, circle_id);

        // Execute the payout with gas buffer protection
        Self::execute_payout_with_gas_protection(
            &env,
            &circle,
            &recipient,
            &circle.creator,
            net_payout,
            organizer_fee,
        ).expect("Payout execution failed");

        // Update circle state
        circle.current_round += 1;
        circle.round_start_time = env.ledger().timestamp();
        circle.is_round_finalized = false;
        circle.current_pot_recipient = None;

        // Reset contribution status for all members
        Self::reset_contributions(&env, circle_id);

        // Store updated circle
        env.storage::instance().set(&DataKey::Circle(circle_id), &circle);

        // Emit events
        env.events().publish(
            (Symbol::new(&env, "payout_distributed"), circle_id),
            (recipient, net_payout),
        );

        if organizer_fee > 0 {
            env.events().publish(
                (Symbol::new(&env, "commission_paid"), circle_id),
                (circle.creator, organizer_fee),
            );
        }
    }

    fn trigger_payout(env: Env, admin: Address, circle_id: u64) {
        // Admin-only function
        let stored_admin: Address = env.storage::instance()
            .get(&DataKey::Admin)
            .unwrap_or_else(|| panic!("Admin not set"));
        
        if admin != stored_admin {
            panic!("Unauthorized: Only admin can trigger payout");
        }

        // Call distribute_payout with admin as caller
        Self::distribute_payout(env, admin, circle_id);
    }

    fn finalize_round(env: Env, creator: Address, circle_id: u64) {
        // Check authorization (only creator can finalize)
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        if creator != circle.creator {
            panic!("Only creator can finalize round");
        }

        // Check if all members have contributed
        if !Self::all_members_contributed(&env, circle_id) {
            panic!("Not all members have contributed this cycle");
        }

        // Determine next recipient (simple round-robin for now)
        let next_recipient_index = circle.current_round % (circle.current_members as u32);
        let next_recipient = env.storage::instance()
            .get(&DataKey::MemberByIndex(circle_id, next_recipient_index))
            .unwrap_or_else(|| panic!("Member not found for next round"));

        // Update circle state
        let mut updated_circle = circle;
        updated_circle.is_round_finalized = true;
        updated_circle.current_pot_recipient = Some(next_recipient);
        updated_circle.round_start_time = env.ledger().timestamp();

        // Store updated circle
        env.storage::instance().set(&DataKey::Circle(circle_id), &updated_circle);

        // Schedule payout time
        let scheduled_time = env.ledger().timestamp() + updated_circle.cycle_duration;
        env.storage::instance().set(&DataKey::ScheduledPayoutTime(circle_id), &scheduled_time);

        // Emit event
        env.events().publish(
            (Symbol::new(&env, "round_finalized"), circle_id),
            (next_recipient, scheduled_time),
        );
    }

    // --- HELPER FUNCTIONS ---

    fn get_circle(env: Env, circle_id: u64) -> CircleInfo {
        env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"))
    }

    fn get_member(env: Env, member: Address) -> Member {
        env.storage::instance()
            .get(&DataKey::Member(member))
            .unwrap_or_else(|| panic!("Member not found"))
    }

    fn get_current_recipient(env: Env, circle_id: u64) -> Option<Address> {
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // If circle is finalized and has a designated recipient, use that
        if circle.is_round_finalized {
            return circle.current_pot_recipient;
        }

        // Otherwise, determine based on round number (round-robin)
        if circle.current_members == 0 {
            return None;
        }

        let recipient_index = circle.current_round % (circle.current_members as u32);
        env.storage::instance()
            .get(&DataKey::MemberByIndex(circle_id, recipient_index))
    }

    // --- INTERNAL HELPER FUNCTIONS ---

    fn all_members_contributed(env: &Env, circle_id: u64) -> bool {
        let circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        if circle.current_members == 0 {
            return false;
        }

        // Check if every member has contributed
        for member in circle.members.iter() {
            if !circle.contributions.get(member).unwrap_or(false) {
                return false;
            }
        }

        true
    }

    fn ensure_gas_buffer(env: &Env, circle_id: u64) {
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        let config: GasBufferConfig = env.storage::instance()
            .get(&DataKey::GasBufferConfig(circle_id))
            .unwrap_or_else(|| panic!("Gas buffer config not found"));

        // Check if gas buffer is enabled
        if !circle.gas_buffer_enabled {
            return;
        }

        // Check if buffer needs refilling
        if circle.gas_buffer_balance < config.auto_refill_threshold {
            // Use emergency buffer if available
            if circle.gas_buffer_balance >= config.emergency_buffer {
                // Allow payout but emit warning
                env.events().publish(
                    (Symbol::new(&env, "gas_buffer_warning"), circle_id),
                    ("Low gas buffer", circle.gas_buffer_balance),
                );
            } else {
                // Critical: buffer too low, attempt auto-refill from emergency funds
                if config.emergency_buffer > 0 {
                    env.events().publish(
                        (Symbol::new(&env, "emergency_gas_usage"), circle_id),
                        ("Using emergency buffer", config.emergency_buffer),
                    );
                    circle.gas_buffer_balance += config.emergency_buffer;
                    env.storage::instance().set(&DataKey::Circle(circle_id), &circle);
                } else {
                    panic!("Insufficient gas buffer for payout. Please fund the gas buffer.");
                }
            }
        }
    }

    fn execute_payout_with_gas_protection(
        env: &Env,
        circle: &CircleInfo,
        recipient: &Address,
        organizer: &Address,
        net_payout: i128,
        organizer_fee: i128,
    ) -> Result<(), ()> {
        let token_client = token::Client::new(env, &circle.token);

        // Calculate estimated gas cost (conservative estimate)
        let estimated_gas_cost = 2000000i128; // 2 XLM conservative estimate
        
        // Check if we have enough gas buffer
        if circle.gas_buffer_balance < estimated_gas_cost {
            return Err(());
        }

        // Execute transfers
        token_client.transfer(
            &env.current_contract_address(),
            recipient,
            &net_payout,
        );

        if organizer_fee > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                organizer,
                &organizer_fee,
            );
        }

        // Deduct gas cost from buffer (in a real implementation, this would be the actual gas used)
        let mut updated_circle = circle.clone();
        updated_circle.gas_buffer_balance -= estimated_gas_cost;
        env.storage::instance().set(&DataKey::Circle(circle_id), &updated_circle);

        Ok(())
    }

    fn check_and_finalize_round(env: &Env, circle_id: u64) {
        if Self::all_members_contributed(env, circle_id) {
            let circle: CircleInfo = env.storage::instance()
                .get(&DataKey::Circle(circle_id))
                .unwrap_or_else(|| panic!("Circle not found"));

            if !circle.is_round_finalized {
                // Auto-finalize the round
                Self::finalize_round(env.clone(), circle.creator.clone(), circle_id);
            }
        }
    }

    fn reset_contributions(env: &Env, circle_id: u64) {
        let mut circle: CircleInfo = env.storage::instance()
            .get(&DataKey::Circle(circle_id))
            .unwrap_or_else(|| panic!("Circle not found"));

        // Clear all contribution statuses
        circle.contributions = Map::new(env);

        // Reset member contribution flags
        for member in circle.members.iter() {
            let mut member_info: Member = env.storage::instance()
                .get(&DataKey::Member(member))
                .unwrap_or_else(|| panic!("Member not found"));
            
            member_info.has_contributed_current_round = false;
            env.storage::instance().set(&DataKey::Member(member), &member_info);
        }

        env.storage::instance().set(&DataKey::Circle(circle_id), &circle);
    }
}
