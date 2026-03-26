# SoroSusu Governance Token Mining Logic

## Overview

This implementation introduces a comprehensive "Save-to-Earn" governance token mining system that rewards active SoroSusu participants with governance tokens through a vesting mechanism. The system aligns long-term user interests with protocol success by turning every Susu member into a stakeholder.

## Key Features

### 🏦 **Vesting Vault System**
- Tokens are held in a smart contract vault until vesting conditions are met
- Prevents immediate token dumping and encourages long-term participation
- Configurable cliff period and vesting duration

### ⛏️ **Save-to-Earn Mining**
- Every successful contribution mints governance tokens
- Mining rate is configurable per contribution
- Maximum mining limits per circle to control token supply

### 📊 **Comprehensive Tracking**
- User mining statistics (contributions, tokens earned, tokens claimed)
- Individual vesting schedules per user
- Global mining metrics and limits

### 🔄 **Cycle-Based Vesting**
- Vesting progresses based on completed savings cycles
- Aligns token release with actual protocol participation
- Automatic cycle completion detection

## Architecture

### Data Structures

#### MiningConfig
```rust
pub struct MiningConfig {
    pub tokens_per_contribution: u64,    // Tokens earned per contribution
    pub vesting_duration_cycles: u32,     // Total vesting period in cycles
    pub cliff_cycles: u32,               // Cliff period before any vesting
    pub max_mining_per_circle: u64,       // Maximum tokens per circle
    pub is_mining_enabled: bool,         // Mining enabled flag
}
```

#### UserVestingInfo
```rust
pub struct UserVestingInfo {
    pub total_allocated: u64,      // Total tokens allocated to user
    pub vested_amount: u64,        // Currently vested amount
    pub claimed_amount: u64,       // Already claimed tokens
    pub start_cycle: u32,          // Cycle when vesting started
    pub contributions_made: u32,   // Number of contributions
    pub is_active: bool,           // Vesting active status
}
```

#### UserMiningStats
```rust
pub struct UserMiningStats {
    pub total_contributions: u32,        // Total contributions made
    pub total_tokens_earned: u64,        // Total tokens earned
    pub total_tokens_claimed: u64,       // Total tokens claimed
    pub join_timestamp: u64,            // When user joined
    pub last_mining_timestamp: u64,      // Last mining activity
}
```

### Core Functions

#### Mining Functions
- `set_governance_token()` - Set the governance token contract
- `configure_mining()` - Configure mining parameters
- `claim_vested_tokens()` - Claim available vested tokens
- `get_user_vesting_info()` - Get user's vesting information
- `get_mining_stats()` - Get user's mining statistics

#### Internal Logic
- `mine_governance_tokens()` - Core mining logic called on deposits
- `calculate_vested_amount()` - Calculate vested tokens based on cycles
- `check_and_complete_cycle()` - Detect and handle cycle completion

## Mining Mechanics

### 1. Token Allocation
When a user makes a successful contribution:
1. System checks if mining is enabled and governance token is set
2. Verifies user hasn't already mined for this contribution
3. Checks if circle mining limit hasn't been exceeded
4. Allocates tokens to user's vesting schedule
5. Mints tokens to contract vault
6. Emits mining event

### 2. Vesting Schedule
- **Cliff Period**: No tokens vest during initial cycles (default: 3 cycles)
- **Linear Vesting**: Tokens vest linearly after cliff (default: 12 cycles total)
- **Cycle-Based**: Vesting progresses with completed savings cycles

### 3. Token Claiming
Users can claim vested tokens:
1. Calculate currently vested amount
2. Subtract already claimed amount
3. Transfer claimable tokens to user
4. Update claimed amount and statistics

## Configuration

### Default Mining Configuration
```rust
MiningConfig {
    tokens_per_contribution: 100,  // 100 tokens per contribution
    vesting_duration_cycles: 12,   // 12 cycles (1 year if monthly)
    cliff_cycles: 3,               // 3 cycles cliff period
    max_mining_per_circle: 1000,   // Max 1000 tokens per circle
    is_mining_enabled: false,       // Disabled until token set
}
```

### Setup Process
1. Deploy contract and initialize
2. Set governance token address (enables mining)
3. Configure mining parameters if needed
4. Users join circles and start earning

## Events

### Mining Events
- `tokens_mined(user, circle_id, amount)` - Emitted when tokens are mined
- `tokens_claimed(user, amount)` - Emitted when tokens are claimed
- `cycle_completed(circle_id, cycle_count)` - Emitted when cycle completes

## Security Features

### Access Control
- Only admin can set governance token and configure mining
- User authorization required for token claims
- Mining limits prevent unlimited token generation

### Economic Safeguards
- Maximum mining per circle controls supply
- Vesting prevents immediate dumping
- Cliff period ensures commitment

### State Management
- Contribution tracking prevents double-mining
- Vesting state persistence across cycles
- Comprehensive statistics for transparency

## Integration Points

### Existing Functions Enhanced
- `deposit()` - Now includes mining logic
- `join_circle()` - Initializes mining stats and vesting
- `eject_member()` - Deactivates vesting for ejected members

### New Storage Keys
- `GovernanceToken` - Governance token contract address
- `MiningConfig` - Mining configuration
- `UserVesting(address)` - Per-user vesting information
- `UserMiningStats(address)` - Per-user mining statistics
- `TotalMinedTokens` - Global mining counter

## Testing

Comprehensive test suite covers:
- ✅ Mining setup and configuration
- ✅ Token allocation on contributions
- ✅ Vesting calculations and schedules
- ✅ Token claiming mechanics
- ✅ Mining limits and controls
- ✅ Cycle completion handling
- ✅ Member ejection effects
- ✅ Event emission verification

## Usage Example

```rust
// Admin setup
contract.set_governance_token(admin, governance_token_address);

// Configure mining (optional)
let config = MiningConfig {
    tokens_per_contribution: 100,
    vesting_duration_cycles: 12,
    cliff_cycles: 3,
    max_mining_per_circle: 1000,
    is_mining_enabled: true,
};
contract.configure_mining(admin, config);

// User participation
user.join_circle(circle_id);
user.deposit(circle_id); // Mines 100 tokens to vesting

// Later - claim vested tokens
user.claim_vested_tokens(); // Claims available tokens
```

## Benefits

### For Users
- **Earn Governance Power**: Active participation grants governance rights
- **Long-Term Alignment**: Vesting encourages continued participation
- **Transparent Rewards**: Clear mining rates and vesting schedules

### For Protocol
- **Decentralization**: Distributes governance to active users
- **User Retention**: Vesting creates stickiness
- **Controlled Supply**: Mining limits manage token economics

### For Ecosystem
- **Sustainable Growth**: Aligns user and protocol success
- **Fair Distribution**: Rewards actual participation
- **Governance Distribution**: Power to engaged community members

## Future Enhancements

- Multi-tier mining rates for different contribution sizes
- Bonus tokens for early adopters
- Governance power multipliers for long-term participants
- Integration with external governance protocols
- Dynamic mining rates based on protocol metrics

This implementation provides a robust foundation for decentralized governance token distribution while maintaining economic stability and encouraging long-term participation in the SoroSusu ecosystem.
