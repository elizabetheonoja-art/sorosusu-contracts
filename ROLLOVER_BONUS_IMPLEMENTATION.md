# Rollover Bonus Incentive Logic Implementation

## Overview

The Rollover Bonus Incentive Logic is a loyalty multiplier system designed to reduce group churn and encourage long-term communal wealth building in the SoroSusu protocol. Groups that vote to "rollover" into a new cycle receive a bonus from a portion of the platform fee, making SoroSusu the "sticky" core of user retention on the Stellar network.

## Key Features

### 1. Loyalty Multiplier
- Groups that complete successful cycles and vote to continue together receive bonuses
- Bonus is calculated as a percentage of the platform fee that would normally be charged
- Default bonus rate: 50% of platform fee (configurable via `fee_percentage_bps`)

### 2. Democratic Voting System
- Rollover proposals require 60% quorum participation
- 66% supermajority approval required for bonus activation
- 48-hour voting period to ensure adequate consideration
- Only active members can vote on rollover proposals

### 3. Smart Integration
- Bonus automatically applied to the first pot of the next cycle
- Seamlessly integrated with existing payout logic
- Full audit trail and event logging

## Implementation Details

### Data Structures

#### RolloverBonus
```rust
pub struct RolloverBonus {
    pub circle_id: u64,
    pub bonus_amount: i128,
    pub fee_percentage: u32, // Percentage of platform fee to refund
    pub created_timestamp: u64,
    pub status: RolloverStatus,
    pub voting_deadline: u64,
    pub for_votes: u32,
    pub against_votes: u32,
    pub total_votes_cast: u32,
    pub applied_cycle: Option<u64>,
}
```

#### RolloverVote
```rust
pub struct RolloverVote {
    pub voter: Address,
    pub circle_id: u64,
    pub vote_choice: RolloverVoteChoice,
    pub timestamp: u64,
}
```

### Constants

```rust
const ROLLOVER_VOTING_PERIOD: u64 = 172800; // 48 hours
const ROLLOVER_QUORUM: u32 = 60; // 60% quorum
const ROLLOVER_MAJORITY: u32 = 66; // 66% supermajority
const DEFAULT_ROLLOVER_BONUS_BPS: u32 = 5000; // 50% of platform fee
```

### Core Functions

#### propose_rollover_bonus(env, user, circle_id, fee_percentage_bps)
- Initiates a rollover bonus proposal
- Only callable after first complete cycle
- Proposer automatically votes "For"
- Emits `ROLLOVER_PROPOSED` event

#### vote_rollover_bonus(env, user, circle_id, vote_choice)
- Allows active members to vote on rollover proposals
- Prevents double voting
- Auto-approves if quorum and majority thresholds met
- Emits `ROLLOVER_VOTE` event

#### apply_rollover_bonus(env, circle_id)
- Applies approved rollover bonus to group reserve
- Tracks which cycle the bonus applies to
- Emits `ROLLOVER_APPLIED` event

### Bonus Calculation

The rollover bonus is calculated as:

```
total_pot = contribution_amount × member_count
platform_fee = (total_pot × protocol_fee_bps) / 10000
rollover_bonus = (platform_fee × fee_percentage_bps) / 10000
```

### Integration with Payout Logic

The `claim_pot` function automatically checks for applied rollover bonuses:

```rust
if rollover_bonus.status == RolloverStatus::Applied {
    if applied_cycle == circle.current_recipient_index {
        total_payout += rollover_bonus.bonus_amount;
    }
}
```

## Usage Flow

1. **Complete First Cycle**: Group must successfully complete at least one full cycle
2. **Propose Rollover**: Any active member can propose a rollover bonus
3. **Vote**: All active members vote during the 48-hour window
4. **Approval**: If quorum (60%) and majority (66%) thresholds are met, proposal is approved
5. **Apply**: Approved bonus is applied to group reserve
6. **Payout**: Next cycle's first recipient receives the rollover bonus

## Economic Impact

### For Groups
- **Reduced Churn**: Financial incentive to stay together
- **Increased Savings**: Bonus effectively reduces net cost of participation
- **Community Building**: Rewards long-term collaboration

### For Protocol
- **Higher Retention**: Groups stay active longer
- **Network Effects**: Successful groups attract new members
- **Revenue Share**: Shares platform revenue with loyal users

### For Members
- **Lower Effective Fees**: Bonus offsets platform fees
- **Predictable Benefits**: Clear formula for bonus calculation
- **Democratic Control**: Members decide on rollover participation

## Security Considerations

1. **Access Control**: Only active members can propose and vote
2. **Timing Restrictions**: Rollover only after first complete cycle
3. **Vote Integrity**: One vote per member, immutable recording
4. **Audit Trail**: All actions logged with full transparency
5. **Rate Limiting**: Prevents spam proposals

## Events

### ROLLOVER_PROPOSED
```
(circle_id, proposer, bonus_amount, fee_percentage, voting_deadline)
```

### ROLLOVER_VOTE
```
(circle_id, voter, vote_choice, for_votes, against_votes)
```

### ROLLOVER_APPLIED
```
(circle_id, bonus_amount, applied_cycle)
```

### ROLLOVER_BONUS_APPLIED
```
(circle_id, recipient, bonus_amount, cycle)
```

## Testing

Comprehensive test suite includes:
- Successful rollover proposal and voting
- Bonus calculation verification
- Payout integration testing
- Rejection scenarios
- Edge cases and error conditions

## Future Enhancements

1. **Dynamic Bonus Rates**: Tiered bonuses based on group longevity
2. **Multi-Cycle Bonuses**: Cumulative bonuses for multiple consecutive rollovers
3. **Performance Metrics**: Bonus adjustments based on group performance
4. **Cross-Group Bonuses**: Special bonuses for groups that refer new successful groups

## Conclusion

The Rollover Bonus Incentive Logic provides a powerful mechanism for reducing group churn and encouraging long-term participation in the SoroSusu protocol. By sharing platform revenue with loyal groups and implementing democratic decision-making, the system creates strong incentives for groups to stay together and build lasting wealth on the Stellar network.
