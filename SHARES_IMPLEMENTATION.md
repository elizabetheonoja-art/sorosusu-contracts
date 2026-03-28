# Shares Implementation for SoroSusu

## Overview

The shares feature allows members to contribute 2x the standard amount to receive 2x the pot, making the protocol a one-size-fits-all tool for communal finance. This flexibility enables families or small businesses to participate in the same Susu cycle while contributing at their individual comfort levels.

## Implementation Details

### Member Structure Changes

Added a new `shares` field to the `Member` struct:

```rust
pub struct Member {
    pub address: Address,
    pub index: u32,
    pub contribution_count: u32,
    pub last_contribution_time: u64,
    pub status: MemberStatus,
    pub tier_multiplier: u32,        // Kept for backward compatibility
    pub shares: u32,                  // New field: 1 = standard, 2 = double
    pub referrer: Option<Address>,
    pub buddy: Option<Address>,
}
```

### Circle Structure Changes

Added `total_shares` field to track the sum of all member shares:

```rust
pub struct CircleInfo {
    // ... existing fields ...
    pub member_count: u32,
    pub total_shares: u32,  // Total shares in the circle (sum of all member shares)
    // ... rest of fields ...
}
```

### Function Updates

#### `join_circle`
- Added `shares` parameter (must be 1 or 2)
- Validates shares input
- Sets `tier_multiplier = shares` for backward compatibility
- Updates circle's `total_shares` count

#### `deposit`
- Already uses `tier_multiplier` for contribution calculations
- Now automatically handles double contributions for 2-share members

#### `claim_pot`
- Calculates pot based on `total_shares` instead of `member_count`
- Applies 2x multiplier for members with 2 shares
- Double payout: `pot_amount * member_shares`

#### Other Functions Updated
- `calculate_rollover_bonus`: Uses `total_shares` for pot calculations
- `delegate_yield`: Uses `total_shares` for pot calculations

## Usage Examples

### Standard Member (1 Share)
```rust
susu_client.join_circle(
    &member,
    &circle_id,
    &1u32,  // tier_multiplier (automatically set to shares)
    &1u32,  // shares = 1 (standard contribution)
    &None::<Address>,
);
```

### Double Share Member (2 Shares)
```rust
susu_client.join_circle(
    &member,
    &circle_id,
    &2u32,  // tier_multiplier (automatically set to shares)
    &2u32,  // shares = 2 (double contribution and payout)
    &None::<Address>,
);
```

## Financial Impact

### Example Scenario
- Circle contribution amount: 100 USDC
- 3 members: Member A (1 share), Member B (2 shares), Member C (1 share)
- Total shares: 4

#### Contributions per Round:
- Member A: 100 USDC (1 share × 100)
- Member B: 200 USDC (2 shares × 100)
- Member C: 100 USDC (1 share × 100)
- Total Pot: 400 USDC

#### Payouts:
- Member A: 400 USDC (standard pot)
- Member B: 800 USDC (double pot for 2 shares)
- Member C: 400 USDC (standard pot)

## Benefits

1. **Flexibility**: Members can choose their contribution level
2. **Inclusivity**: Families and businesses can participate together
3. **Proportional Rewards**: Contribution level matches payout level
4. **Backward Compatibility**: Existing `tier_multiplier` logic preserved

## Validation Rules

- Shares must be either 1 or 2
- Invalid share values are rejected with an error
- Circle capacity is still based on member count, not share count

## Testing

Comprehensive tests are included in `tests/shares_test.rs`:
- Shares validation
- Pot calculations
- Double payout verification
- Integration with existing features

## Migration Notes

- Existing members will have `shares = 1` by default
- All existing functionality continues to work unchanged
- New shares functionality is opt-in via the `join_circle` parameter
