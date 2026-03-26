# Risk Multiplier (Aggressive Recovery) Implementation

## Overview

The Risk Multiplier feature implements an "Aggressive Recovery" mechanism that addresses the security vulnerability where members have the highest incentive to default after receiving their pot payout. This feature applies a 3x multiplier to late fees for members who have already won the pot, providing both financial compensation for increased risk and a strong psychological deterrent against "Payout-and-Ghost" behavior.

## Problem Statement

In traditional ROSCA (Rotating Savings and Credit Association) systems, members face the highest temptation to default after receiving their payout. Once a member has received the lump sum pot, their remaining obligations represent pure cost without immediate benefit, creating a moral hazard that can jeopardize the entire group's financial stability.

## Solution: Risk Multiplier Logic

### Core Mechanism

- **Normal Late Fee**: 1% of contribution amount (configurable via `late_fee_bps`)
- **Risk Multiplier**: 3x the normal late fee after pot win
- **Trigger Condition**: Member has `PotWinner(address, circle_id)` record in storage
- **Application**: Applied to all late contributions after winning the pot

### Implementation Details

#### 1. Constants Added
```rust
const RISK_MULTIPLIER: u32 = 3; // 3x late fees after pot win (Aggressive Recovery)
```

#### 2. Storage Key Added
```rust
PotWinner(Address, u64), // Track members who have won the pot in each circle
```

#### 3. Modified Functions

**deposit() function enhancement:**
```rust
// Apply Risk Multiplier for Aggressive Recovery if member has already won the pot
let pot_winner_key = DataKey::PotWinner(user.clone(), circle_id);
if env.storage().instance().has(&pot_winner_key) {
    // Member has won the pot before - apply 3x risk multiplier
    base_penalty = base_penalty * RISK_MULTIPLIER as i128;
    
    // Emit event for aggressive recovery
    env.events().publish(
        (Symbol::new(&env, "AGGRESSIVE_RECOVERY"), circle_id, user.clone()),
        (base_penalty, RISK_MULTIPLIER),
    );
}
```

**claim_pot() function enhancement:**
```rust
// Track that this member has won the pot (for Risk Multiplier application)
let pot_winner_key = DataKey::PotWinner(user.clone(), circle_id);
env.storage().instance().set(&pot_winner_key, &true);

// Emit event for pot win tracking
env.events().publish(
    (Symbol::new(&env, "POT_WINNER_TRACKED"), circle_id, user.clone()),
    (env.ledger().timestamp(), total_payout),
);
```

## Behavioral Impact

### Financial Incentives

1. **Before Pot Win**: Standard late fees apply (e.g., 1% = 10 tokens on 1000 token contribution)
2. **After Pot Win**: 3x late fees apply (e.g., 3% = 30 tokens on 1000 token contribution)
3. **Referral Discount**: Still applies to the multiplied amount (5% discount on 30 tokens = 1.5 tokens)

### Psychological Deterrent

- **Increased Cost**: 3x penalty makes defaulting significantly more expensive
- **Group Protection**: Higher penalties compensate the group for increased default risk
- **Fairness**: Members who haven't yet benefited from the system pay lower penalties

### Risk Compensation

The 3x multiplier serves two purposes:
1. **Deterrence**: Makes post-payout default economically irrational
2. **Compensation**: Provides additional funds to the group reserve to offset potential losses

## Event Emissions

### New Events

1. **POT_WINNER_TRACKED**: Fired when a member claims the pot
   - Topics: `(circle_id, member_address)`
   - Data: `(timestamp, payout_amount)`

2. **AGGRESSIVE_RECOVERY**: Fired when risk multiplier is applied
   - Topics: `(circle_id, member_address)`
   - Data: `(penalty_amount, multiplier_applied)`

## Testing Coverage

### Test Cases Implemented

1. **Normal Late Fee Before Pot Win**
   - Verifies standard 1% late fee applies before winning pot
   - Expected: 10 tokens fee on 1000 token contribution

2. **Risk Multiplier After Pot Win**
   - Verifies 3x late fee applies after winning pot
   - Expected: 30 tokens fee on 1000 token contribution

3. **Risk Multiplier with Referral Discount**
   - Verifies referral discount applies to multiplied amount
   - Expected: 28.5 tokens fee (30 - 5% discount)

4. **Pot Winner Tracking Persistence**
   - Verifies tracking persists across multiple cycles
   - Ensures consistent application of risk multiplier

5. **Event Emission**
   - Verifies AGGRESSIVE_RECOVERY events are emitted
   - Ensures proper event data structure

### Test Results

All tests validate:
- ✅ Normal late fees before pot win
- ✅ 3x risk multiplier after pot win
- ✅ Referral discounts work with multiplier
- ✅ Pot winner tracking persists
- ✅ Event emissions work correctly

## Security Considerations

### Attack Mitigation

1. **Payout-and-Ghost Attack**: 3x penalty makes this attack economically unattractive
2. **Strategic Default**: Increased cost outweighs benefits of late payment
3. **Group Stability**: Higher penalties fund group reserve for risk mitigation

### Economic Rationality

For a 1000 token contribution:
- **Normal late fee**: 10 tokens (1%)
- **Risk multiplier fee**: 30 tokens (3%)
- **Net loss from default**: 30 tokens vs continuing participation

The 3x multiplier ensures that continuing participation is economically preferable to defaulting.

## Configuration

### Current Settings

- **Risk Multiplier**: 3x (hardcoded constant)
- **Late Fee Basis Points**: 100 (1%) - configurable per circle
- **Referral Discount**: 500 (5%) - global constant

### Future Enhancements

Potential configurable parameters:
- Risk multiplier value (currently fixed at 3x)
- Different multipliers based on member history
- Time-based decay of multiplier effect

## Integration Points

### Existing Features

1. **Referral System**: Discounts apply to multiplied penalties
2. **Group Reserve**: Increased penalties fund group protection
3. **Event System**: New events for monitoring and analytics
4. **Member Statistics**: Late contribution tracking includes multiplier penalties

### Compatible Features

- Collateral systems (additional security layer)
- Insurance mechanisms (complementary protection)
- Reputation systems (long-term behavior tracking)

## Monitoring and Analytics

### Key Metrics

1. **Aggressive Recovery Events**: Track frequency of 3x penalties
2. **Post-Payout Default Rate**: Measure effectiveness of deterrent
3. **Group Reserve Growth**: Monitor additional protection funding
4. **Member Retention**: Assess impact on member behavior

### Event Analysis

The emitted events enable:
- Real-time monitoring of risk multiplier application
- Historical analysis of default patterns
- Economic impact assessment of the deterrent

## Conclusion

The Risk Multiplier feature provides a robust solution to the "Payout-and-Ghost" vulnerability by:

1. **Financial Deterrence**: 3x penalties make post-payout default economically irrational
2. **Risk Compensation**: Additional fees fund group protection mechanisms
3. **Behavioral Incentive**: Aligns member incentives with group stability
4. **Security Enhancement**: Addresses a critical vulnerability in ROSCA systems

This implementation maintains compatibility with existing features while significantly improving the security and stability of the SoroSusu protocol.
