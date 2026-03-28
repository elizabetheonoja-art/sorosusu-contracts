# Vesting-Vault Lien Implementation

## Overview

The Vesting-Vault Lien feature enables users to use their future vesting tokens as collateral for high-value SoroSusu groups. This creates an "Inter-Contract Lien" system where SoroSusu can lock a portion of a user's future vesting, and if they default on the Susu, the funds are automatically pulled from the Vesting-Vault once they unlock.

This "Unified Wealth" model represents the ultimate goal of the JerryIdoko ecosystem—allowing a developer to use their future salary (Vesting) to help their family buy a house today (SoroSusu).

## Architecture

### Key Components

1. **VestingVaultLienInfo**: Complete lien information including vesting contract details
2. **LienInfo**: Summary information for circle-level lien tracking
3. **LienStatus**: Tracks the lifecycle of each lien (Active, Claimed, Released, Expired)
4. **Cross-Contract Communication**: Interface for interacting with Vesting-Vault contracts

### Data Flow

```
Vesting-Vault Contract ←→ SoroSusu Contract
        ↓                        ↓
   Future Tokens ←→ Lien Creation → Circle Participation
        ↓                        ↓
   Vesting Release ←→ Default Handling → Fund Claim
```

## Core Functions

### 1. create_vesting_lien

Creates a lien on a member's future vesting tokens.

```rust
fn create_vesting_lien(
    env: Env,
    user: Address,
    circle_id: u64,
    vesting_vault_contract: Address,
    lien_amount: i128,
) -> u64
```

**Requirements:**
- User must authorize the lien creation
- Circle must exist and require collateral
- Vesting vault contract must be verified
- Lien amount must be sufficient (≥ 20% of total contribution value)
- No existing lien for this member/circle combination

**Process:**
1. Verifies circle and user authorization
2. Validates vesting vault compatibility
3. Creates lien record with `Active` status
4. Adds lien to circle's lien registry
5. Logs audit entry

### 2. claim_vesting_lien

Claims lien funds when a member defaults.

```rust
fn claim_vesting_lien(
    env: Env,
    caller: Address,
    circle_id: u64,
    member: Address,
)
```

**Authorization:** Circle creator or admin only

**Process:**
1. Verifies caller authorization
2. Confirms member is marked as defaulted
3. Updates lien status to `Claimed`
4. Triggers cross-contract call to Vesting-Vault
5. Updates circle lien registry
6. Logs audit entry

### 3. release_vesting_lien

Releases lien after successful circle completion.

```rust
fn release_vesting_lien(
    env: Env,
    caller: Address,
    circle_id: u64,
    member: Address,
)
```

**Authorization:** Member, circle creator, or admin

**Process:**
1. Verifies caller authorization
2. Confirms member completed all obligations
3. Updates lien status to `Released`
4. Updates circle lien registry
5. Logs audit entry

### 4. mark_member_defaulted

Marks a member as defaulted and automatically claims their lien.

```rust
fn mark_member_defaulted(
    env: Env,
    caller: Address,
    circle_id: u64,
    member: Address,
)
```

**Authorization:** Circle creator or admin only

**Process:**
1. Updates member status to `Defaulted`
2. Adds member to defaulted members list
3. **Automatically claims any active Vesting-Vault lien**
4. Handles traditional collateral if present
5. Logs audit entry

## Integration Points

### Circle Join Process

The `join_circle` function now accepts both traditional collateral and Vesting-Vault liens:

```rust
// Check for traditional collateral first
match collateral_info {
    Some(collateral) => {
        // Verify traditional collateral
    }
    None => {
        // Check for Vesting-Vault lien
        let lien_info = get_vesting_lien(user, circle_id);
        match lien_info {
            Some(lien) => {
                // Verify lien is active and sufficient
                let required_lien_amount = calculate_required_lien(circle);
                if lien.lien_amount < required_lien_amount {
                    panic!("Vesting lien amount insufficient");
                }
            }
            None => panic!("Collateral or Vesting lien required"),
        }
    }
}
```

### Circle Completion Process

The `claim_pot` function automatically releases liens upon successful completion:

```rust
// Auto-release Vesting-Vault lien
if let Some(mut lien_info) = get_vesting_lien(user, circle_id) {
    if lien_info.status == LienStatus::Active {
        lien_info.status = LienStatus::Released;
        lien_info.release_timestamp = Some(current_time);
        // Update storage and registry
    }
}
```

## Usage Examples

### Basic Usage Flow

```rust
// 1. Create a high-value circle (requires collateral)
let circle_id = client.create_circle(
    &creator,
    &2_000_000_0, // 200 XLM per contribution
    &5u32,        // 5 members = 1000 XLM total
    &token,
    &86400u64,    // 1 day cycles
    &100u32,      // 1% insurance
    &nft_contract,
    &admin,
);

// 2. Create Vesting-Vault lien
let required_lien = 400_000_0; // 20% of contribution amount
let lien_id = client.create_vesting_lien(
    &user,
    &circle_id,
    &vesting_vault_contract,
    &required_lien,
);

// 3. Join circle using lien as collateral
client.join_circle(
    &user,
    &circle_id,
    &1u32,        // tier multiplier
    &None,        // no referrer
);

// 4. Participate normally...
client.deposit(&user, &circle_id);
// ... continue contributions

// 5. Complete successfully - lien auto-releases
client.claim_pot(&user, &circle_id);
// Lien automatically released back to user
```

### Default Handling Flow

```rust
// If member fails to contribute and defaults:
client.mark_member_defaulted(
    &creator,
    &circle_id,
    &defaulting_member,
);

// Lien automatically claimed:
// - Status changes to Claimed
// - Cross-contract call to Vesting-Vault
// - Funds transferred to group reserve
```

## Security Considerations

### 1. Access Control

- **Lien Creation**: Only user can create lien for themselves
- **Lien Claim**: Only circle creator or admin can claim liens
- **Lien Release**: Member, creator, or admin can release liens
- **Default Marking**: Only creator or admin can mark defaults

### 2. State Validation

- Lien status transitions are strictly enforced
- Double lien creation is prevented
- Claim only allowed for defaulted members
- Release only allowed for active members who completed obligations

### 3. Economic Security

- Minimum 20% lien requirement ensures skin in the game
- Automatic claim on default protects remaining members
- Vesting period verification prevents expired liens
- Cross-contract verification ensures vault compatibility

### 4. Attack Vectors Mitigated

- **Lien Double-Dipping**: One lien per member/circle combination
- **Insufficient Coverage**: Minimum lien amount enforced
- **Expired Vesting**: Vesting end time verification
- **Unauthorized Claims**: Strict authorization controls

## Constants and Limits

```rust
const MAX_LEEN_PERCENTAGE: u32 = 8000; // Maximum 80% of vesting can be liened
const LIEN_VERIFICATION_TIMEOUT: u64 = 300; // 5 minutes for vesting vault verification
const MIN_VESTING_REMAINING: u64 = 604800; // Minimum 7 days remaining vesting required
const DEFAULT_COLLATERAL_BPS: u32 = 2000; // 20% default collateral requirement
```

## Error Codes

| Code | Variant | Description |
|---|---|---|
| 15 | `LienNotFound` | No lien exists for this member/circle |
| 16 | `LienAlreadyExists` | Lien already exists for this member/circle |
| 17 | `InsufficientVestingBalance` | Vesting balance insufficient for lien |
| 18 | `VestingVaultNotFound` | Vesting vault contract not found |
| 19 | `LienNotActive` | Lien is not in active state |
| 20 | `VestingPeriodExpired` | Vesting period has expired |
| 21 | `InvalidVestingContract` | Vesting contract is not compatible |
| 22 | `LienAmountExceedsVesting` | Lien amount exceeds available vesting |

## Audit Trail

All lien operations are logged with comprehensive audit entries:

- `CreateLien`: When a new lien is created
- `ClaimLien`: When a lien is claimed due to default
- `ReleaseLien`: When a lien is released after completion

## Testing

The implementation includes comprehensive tests covering:

1. **Lien Creation**: Valid and invalid lien creation scenarios
2. **Circle Join**: Joining with sufficient/insufficient liens
3. **Default Handling**: Automatic lien claiming on default
4. **Completion Flow**: Automatic lien release on completion
5. **Query Functions**: Lien retrieval and circle-wide queries
6. **Security**: Authorization and edge case testing

Run tests with:
```bash
cargo test --test vesting_vault_lien_test
```

## Future Enhancements

### 1. Dynamic Lien Rates

```rust
fn calculate_lien_percentage(total_value: i128, vesting_remaining: u64) -> u32 {
    match (total_value, vesting_remaining) {
        (v, r) if v >= 10_000_000_0 && r >= 2592000 => 1500, // 15% for high value, long vesting
        (v, r) if v >= 5_000_000_0 && r >= 1728000 => 2000,  // 20% for medium value, medium vesting
        _ => 2500, // 25% default
    }
}
```

### 2. Lien Insurance

```rust
fn insure_lien(env: Env, user: Address, circle_id: u64, premium: i128) {
    // Pay premium to insure against lien loss
    // Reduces required lien amount
}
```

### 3. Partial Lien Release

```rust
fn release_lien_graduated(env: Env, circle_id: u64, member: Address, contribution_count: u32) {
    // Release portion of lien after each successful contribution
    let release_percentage = (contribution_count * 10000) / circle.max_members;
    // Release calculated percentage
}
```

## Migration Guide

### For Existing Circles

- Existing circles continue without lien requirements
- New circles automatically support lien-based collateral
- Manual upgrade path available for existing high-value circles

### For Users

- **Low-value circles**: No change in behavior
- **High-value circles**: Can use either traditional collateral OR Vesting-Vault liens
- **Existing members**: Grandfathered in (no collateral required)

## Conclusion

The Vesting-Vault Lien implementation represents a significant advancement in decentralized finance, enabling users to leverage their future earnings for present-day financial needs. This creates a powerful bridge between long-term vesting schedules and short-term liquidity requirements, embodying the "Unified Wealth" vision of the JerryIdoko ecosystem.

The implementation maintains the core principles of the SoroSusu protocol—security, accessibility, and community trust—while expanding the collateral options to include future vesting tokens. This enables larger, more ambitious capital rotation while maintaining robust protection against default risk.

The automatic nature of lien management (creation on default, release on completion) ensures a seamless user experience while providing strong economic guarantees for all participants in the savings circle.
