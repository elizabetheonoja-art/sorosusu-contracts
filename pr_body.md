## Summary

This PR implements two major features for SoroSusu Protocol:

### Issue #268: Staking-Gated Entry for High-Value Rounds
- **Mandatory collateral requirement** for pools defined as "High-Value" (>5,000 XLM)
- **stake_collateral()** function to lock assets in a Vault contract
- **slash_stake()** function to compensate victims when users default
- **release_collateral()** function to return stake after successful cycle completion
- **20% minimum collateral requirement** based on contribution amount
- **Collateral vault tracking** with proper balance management

### Issue #266: Reliability-Index (RI) Calculation Engine
- **Weighted scoring system** (0-1000) stored in persistent storage
- **Timeliness factor** (40% weight): Tracks on-time vs late contributions
- **Volume factor** (30% weight): Based on total contribution volume
- **Frequency factor** (20% weight): Cycles completed and participation streaks
- **Consistency factor** (10% weight): Consecutive participation tracking
- **Decay function**: 5 points per day after 30 days of inactivity
- **Automatic RI calculation** integrated into deposit function

## Key Features

### Security & Risk Management
- Collateral requirements protect against defaults in high-value rounds
- Slashing mechanism compensates victims when users default
- Admin-controlled collateral release after successful cycles

### Reputation System
- Comprehensive reliability scoring based on multiple factors
- Activity tracking for timeliness, volume, frequency, and consistency
- Automatic decay ensures scores reflect current behavior
- Persistent storage maintains user reputation over time

## Implementation Details

### New Data Structures
- `StakedCollateral`: Tracks user collateral stakes
- `UserActivity`: Records contribution patterns for RI calculation
- `ReliabilityIndex`: Stores user reliability scores with decay tracking

### New Functions
- `stake_collateral()`: Lock collateral for high-value rounds
- `slash_stake()`: Admin function to slash defaulting users
- `release_collateral()`: Admin function to release successful stakes
- `calculate_reliability_index()`: Core RI calculation engine
- `update_user_activity()`: Track user contributions for RI
- `apply_ri_decay()`: Apply decay for inactive users
- `get_reliability_index()`: Retrieve user RI scores

### Integration Points
- Modified `join_circle()` to check collateral requirements
- Enhanced `deposit()` to update user activity and calculate RI
- All functions include proper authorization and validation

## Testing
- Contract compiles successfully with Soroban SDK
- All type safety checks pass
- Proper error handling and validation implemented

This implementation significantly enhances the protocol's security and reputation systems while maintaining backward compatibility with existing functionality.
