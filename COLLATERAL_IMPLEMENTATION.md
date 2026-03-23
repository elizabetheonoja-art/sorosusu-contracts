# Collateralized Entry Barrier for High-Value Cycles

## Overview

This feature implements a collateral vault system that requires members to lock a percentage of the total cycle value before participating in high-value Susu groups. This creates a "Financial Commitment" layer that protects the integrity of the group and mitigates the risk of "payout and run" scenarios.

## Architecture

### Key Components

1. **Automatic High-Value Detection**: Circles with total cycle value ≥ 1000 XLM automatically require collateral
2. **Collateral Vault**: Secure storage for member collateral funds
3. **Slashing Mechanism**: Automatic redistribution of defaulted member collateral
4. **Auto-Release System**: Collateral released upon successful completion

### Data Structures

#### CollateralInfo
```rust
pub struct CollateralInfo {
    pub member: Address,
    pub circle_id: u64,
    pub amount: i128,
    pub status: CollateralStatus,
    pub staked_timestamp: u64,
    pub release_timestamp: Option<u64>,
}
```

#### CollateralStatus
- `NotStaked`: Initial state
- `Staked`: Collateral locked and active
- `Slashed`: Collateral confiscated due to default
- `Released`: Collateral returned to member

#### MemberStatus (Extended)
- `Active`: Member in good standing
- `AwaitingReplacement`: Member being replaced
- `Ejected`: Member removed from circle
- `Defaulted`: Member failed to meet obligations (NEW)

## Constants

```rust
const DEFAULT_COLLATERAL_BPS: u32 = 2000; // 20%
const HIGH_VALUE_THRESHOLD: i128 = 1_000_000_0; // 1000 XLM
```

## Core Functions

### 1. stake_collateral(env, user, circle_id, amount)
Locks collateral funds before joining a high-value circle.

**Requirements:**
- Circle must require collateral (total value ≥ 1000 XLM)
- Amount must be ≥ required collateral (20% of total cycle value)
- User must not have already staked collateral

**Process:**
1. Validates circle and amount requirements
2. Transfers collateral tokens to contract
3. Creates CollateralInfo record with `Staked` status

### 2. slash_collateral(env, caller, circle_id, member)
Confiscates collateral from defaulted members and redistributes to group.

**Authorization:** Circle creator or admin only

**Process:**
1. Verifies member is marked as defaulted
2. Transfers collateral to group reserve
3. Updates collateral status to `Slashed`

### 3. release_collateral(env, caller, circle_id, member)
Returns collateral to members who complete all contributions.

**Authorization:** Member, circle creator, or admin

**Process:**
1. Verifies member completed all contributions
2. Transfers collateral back to member
3. Updates collateral status to `Released`

### 4. mark_member_defaulted(env, caller, circle_id, member)
Marks a member as defaulted and triggers automatic collateral slashing.

**Authorization:** Circle creator or admin only

**Process:**
1. Updates member status to `Defaulted`
2. Adds member to defaulted list
3. Automatically triggers collateral slashing

## Integration Points

### Circle Creation
```rust
// Automatic collateral requirement calculation
let total_cycle_value = amount * (max_members as i128);
let requires_collateral = total_cycle_value >= HIGH_VALUE_THRESHOLD;
let collateral_bps = if requires_collateral { DEFAULT_COLLATERAL_BPS } else { 0 };
```

### Join Circle
```rust
// Collateral verification before joining
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
```

### Claim Pot
```rust
// Auto-release collateral upon completion
if circle.requires_collateral {
    if member_info.contribution_count >= circle.max_members {
        // Auto-release collateral
        token_client.transfer(&env.current_contract_address(), &user, &collateral_info.amount);
        collateral_info.status = CollateralStatus::Released;
    }
}
```

## Usage Examples

### Creating a High-Value Circle
```rust
// This circle will automatically require collateral
let circle_id = client.create_circle(
    &creator,
    &2_000_000_0, // 2000 XLM per contribution
    &5u32,        // 5 members = 10,000 XLM total value
    &token,
    &86400u64,    // 1 day cycles
    &100u32,      // 1% insurance
    &nft_contract,
);
// requires_collateral = true, collateral_bps = 2000 (20%)
```

### Staking Collateral
```rust
// Calculate required collateral: 20% of 10,000 XLM = 2,000 XLM
let required_collateral = 2_000_000_0; // 2000 XLM

client.stake_collateral(
    &user,
    &circle_id,
    &required_collateral,
);
```

### Joining the Circle
```rust
// Now user can join (collateral already staked)
client.join_circle(
    &user,
    &circle_id,
    &1u32,        // tier multiplier
    &None,        // no referrer
);
```

### Handling Default
```rust
// Member fails to contribute, mark as defaulted
client.mark_member_defaulted(
    &creator,
    &circle_id,
    &defaulting_member,
);
// Collateral automatically slashed to group reserve
```

### Completing Successfully
```rust
// Member completes all contributions and claims pot
client.claim_pot(&user, &circle_id);
// Collateral automatically released back to member
```

## Security Considerations

### 1. Access Control
- Collateral operations require proper authorization
- Only circle creators or admins can slash collateral
- Members can only release their own collateral

### 2. State Validation
- Collateral status transitions are strictly enforced
- Double staking is prevented
- Release only allowed after completion

### 3. Economic Security
- 20% collateral provides significant skin in the game
- Automatic slashing protects remaining members
- High-value threshold (1000 XLM) focuses on serious groups

### 4. Attack Vectors Mitigated
- **Payout and Run**: Collateral locked until completion
- **Sybil Attacks**: High entry cost discourages spam
- **Default Risk**: Collateral compensates remaining members

## Gas Optimization

### Storage Efficiency
- Collateral records use compact data structures
- Status enums minimize storage overhead
- Timestamps only stored when necessary

### Batch Operations
- Multiple collateral operations can be batched
- Group reserve accumulation is efficient
- Auto-release reduces manual transactions

## Future Enhancements

### 1. Dynamic Collateral Rates
```rust
// Potential enhancement: tiered collateral rates
fn calculate_collateral_bps(total_value: i128) -> u32 {
    match total_value {
        v if v >= 10_000_000_0 => 3000, // 30% for very high value
        v if v >= 5_000_000_0  => 2500, // 25% for high value
        v if v >= 1_000_000_0  => 2000, // 20% for medium value
        _ => 0,                           // No collateral for low value
    }
}
```

### 2. Collateral Insurance
```rust
// Potential enhancement: collateral insurance pool
fn insure_collateral(env: Env, user: Address, circle_id: u64, premium: i128) {
    // Pay premium to insure against collateral loss
    // Reduces net collateral requirement
}
```

### 3. Graduated Release
```rust
// Potential enhancement: gradual collateral release
fn release_collateral_graduated(env: Env, circle_id: u64, member: Address) {
    // Release 25% after each successful contribution
    // Provides liquidity while maintaining commitment
}
```

## Testing

The collateral system includes comprehensive tests covering:

1. **High-value detection** - Automatic collateral requirement
2. **Staking logic** - Proper collateral locking
3. **Join restrictions** - Collateral verification before joining
4. **Default handling** - Slashing and redistribution
5. **Completion flow** - Auto-release after success
6. **Edge cases** - Insufficient amounts, double staking, etc.

Run tests with:
```bash
cargo test --test collateral_test
```

## Migration Guide

### For Existing Circles
- Existing circles continue without collateral requirements
- New circles automatically evaluated based on total value
- Manual upgrade path available for existing high-value circles

### For Users
- Low-value circles: No change in behavior
- High-value circles: Must stake collateral before joining
- Existing members: Grandfathered in (no collateral required)

## Conclusion

The collateralized entry barrier significantly enhances the security and reliability of high-value Susu groups on the Stellar network. By requiring members to lock substantial collateral, the system creates strong financial incentives for participation and completion, while protecting remaining members from default losses.

This implementation enables larger, more serious capital rotation while maintaining the core principles of the SoroSusu protocol: accessibility, security, and community trust.
