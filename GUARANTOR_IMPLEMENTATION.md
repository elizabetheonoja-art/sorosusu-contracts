# Guarantor System for Social Underwriting

## Overview

The Guarantor system implements a "Social Underwriting" model that allows high-reputation users to co-sign for new members, effectively putting their own collateral on the line if the new member defaults. This enables unbanked users to join high-trust savings circles by leveraging existing social hierarchies on-chain.

## Key Features

### 1. Guarantor Registration
- High-reputation users can register as guarantors by staking initial collateral
- Minimum reputation score required: 100 points
- Initial collateral requirement: Flexible (set by guarantor)
- Reputation system managed by protocol admin

### 2. Voucher System
- Guarantors can create vouchers for specific members in specific circles
- Each voucher represents a co-signing agreement
- Maximum concurrent vouchers per guarantor: 5
- Collateral requirement: 150% of vouched amount

### 3. Automatic Default Protection
- When a member defaults, the system automatically claims from their guarantor
- Funds are pulled from the guarantor's vault to cover the defaulted obligation
- Guarantor's reputation and statistics are updated accordingly

### 4. Collateral Management
- Guarantors can add or withdraw collateral from their vault
- Withdrawal limits ensure sufficient coverage for active vouchers
- Real-time balance tracking and validation

## Data Structures

### GuarantorInfo
```rust
pub struct GuarantorInfo {
    pub address: Address,
    pub reputation_score: u32,
    pub total_vouched_amount: i128,
    pub active_vouchers_count: u32,
    pub successful_vouchers: u32,
    pub claimed_vouchers: u32,
    pub status: GuarantorStatus,
    pub registered_timestamp: u64,
    pub vault_balance: i128,
}
```

### VoucherInfo
```rust
pub struct VoucherInfo {
    pub guarantor: Address,
    pub member: Address,
    pub circle_id: u64,
    pub vouched_amount: i128,
    pub status: VoucherStatus,
    pub created_timestamp: u64,
    pub claimed_timestamp: Option<u64>,
    pub collateral_required: i128,
}
```

### GuarantorStatus
- `Registered`: Initial state after registration
- `Active`: Can create new vouchers
- `Suspended`: Reputation below minimum threshold
- `Blacklisted`: Permanently disabled (admin action)

### VoucherStatus
- `Active`: Currently covering member
- `Claimed`: Used to cover member default
- `Expired`: Member completed all obligations
- `Cancelled`: Manually cancelled

## Core Functions

### Registration & Management

#### `register_guarantor(user, initial_collateral)`
Registers a new guarantor with initial collateral stake.
- **Requirements**: Minimum reputation (100), positive collateral
- **Effects**: Creates GuarantorInfo, stakes collateral in vault

#### `update_guarantor_reputation(admin, guarantor, new_score)`
Updates guarantor reputation score (admin only).
- **Requirements**: Admin authorization
- **Effects**: Updates reputation, adjusts status based on thresholds

#### `add_guarantor_collateral(guarantor, amount)`
Adds collateral to guarantor's vault.
- **Requirements**: Positive amount, guarantor authorization
- **Effects**: Increases vault balance, transfers tokens

#### `withdraw_guarantor_collateral(guarantor, amount)`
Withdraws excess collateral from vault.
- **Requirements**: Sufficient remaining collateral, guarantor authorization
- **Effects**: Decreases vault balance, transfers tokens back

### Voucher Operations

#### `create_voucher(guarantor, member, circle_id, vouched_amount)`
Creates a new voucher for a member.
- **Requirements**: Active guarantor, sufficient reputation, available voucher slots, sufficient collateral
- **Effects**: Locks collateral, creates voucher, links member to guarantor

#### `claim_voucher(caller, circle_id, member)`
Claims voucher to cover member default (admin or member).
- **Requirements**: Member defaulted, active voucher exists
- **Effects**: Transfers collateral, updates statistics, marks voucher as claimed

### Query Functions

#### `get_guarantor_info(guarantor) -> GuarantorInfo`
Returns complete guarantor information.

#### `get_voucher_info(guarantor, circle_id) -> VoucherInfo`
Returns voucher details for specific guarantor-circle pair.

#### `get_member_guarantor(member) -> Option<Address>`
Returns the guarantor for a specific member.

#### `get_guarantor_vault_balance(guarantor) -> i128`
Returns current vault balance.

## Constants & Limits

```rust
const MIN_GUARANTOR_REPUTATION: u32 = 100; // Minimum reputation to become guarantor
const MAX_VOUCHURES_PER_GUARANTOR: u32 = 5; // Maximum concurrent vouchers
const GUARANTOR_COLLATERAL_MULTIPLIER: u32 = 150; // 150% of vouched amount
```

## Error Codes

| Code | Error | Description |
|------|-------|-------------|
| 21 | `GuarantorNotFound` | Guarantor not registered |
| 22 | `InsufficientReputation` | Reputation below minimum |
| 23 | `GuarantorNotRegistered` | Not registered as guarantor |
| 24 | `VoucherAlreadyExists` | Voucher already exists for this circle |
| 25 | `VoucherNotFound` | Voucher not found |
| 26 | `GuarantorOverextended` | Maximum vouchers reached |
| 27 | `SelfGuaranteeNotAllowed` | Cannot guarantee self |
| 28 | `GuarantorInsufficientFunds` | Insufficient vault balance |

## Usage Flow

### 1. Becoming a Guarantor
```rust
// High reputation user registers as guarantor
register_guarantor(guarantor_address, 1000); // Stake 1000 tokens
```

### 2. Vouching for a New Member
```rust
// Guarantor creates voucher for unbanked user
create_voucher(
    guarantor_address,
    new_member_address,
    circle_id,
    500 // Vouch for 500 tokens worth of contributions
);
```

### 3. Member Joins Circle
```rust
// New member can now join high-value circle
join_circle(new_member_address, circle_id, 1, None);
```

### 4. Default Protection
```rust
// If member defaults, system automatically protects the circle
mark_member_defaulted(admin_address, circle_id, new_member_address);
// Voucher is automatically claimed, funds pulled from guarantor
```

## Security Considerations

### 1. Collateral Requirements
- 150% multiplier ensures over-collateralization
- Real-time balance validation prevents under-collateralization
- Withdrawal restrictions maintain coverage for active vouchers

### 2. Reputation System
- Admin-controlled reputation updates prevent manipulation
- Minimum thresholds ensure only trusted users can be guarantors
- Automatic suspension for low reputation

### 3. Voucher Limits
- Maximum 5 concurrent vouchers per guarantor
- Prevents concentration of risk
- Encourages distribution of social capital

### 4. Automatic Claims
- Immediate voucher claims on member default
- No manual intervention required
- Reduces administrative overhead

## Social Impact

### 1. Financial Inclusion
- Enables unbanked users to access formal savings mechanisms
- Leverages existing social trust networks
- Reduces barriers to entry for high-value circles

### 2. Risk Distribution
- Spreads risk across multiple guarantors
- Prevents single points of failure
- Creates ecosystem of mutual support

### 3. Reputation Economy
- Incentivizes good behavior through reputation scoring
- Creates market for social capital
- Enables community-based underwriting

## Integration with Existing Features

### 1. Collateral System
- Works alongside existing member collateral
- Provides alternative to direct collateral staking
- Maintains same security guarantees

### 2. Default Handling
- Integrates with existing default detection
- Automatic voucher claims supplement insurance
- Preserves circle integrity

### 3. Member Management
- Guarantor information stored in member profiles
- Seamless integration with join_circle flow
- Maintains compatibility with existing circles

## Future Enhancements

### 1. Dynamic Reputation
- Algorithmic reputation scoring based on history
- Automatic reputation adjustments
- Community-based reputation voting

### 2. Voucher Marketplace
- Secondary market for vouchers
- Risk-based pricing
- Liquidity options for guarantors

### 3. Insurance Integration
- Reinsurance for guarantors
- Risk pooling mechanisms
- Reduced capital requirements

### 4. Multi-circle Vouchers
- Single voucher covering multiple circles
- Portfolio-based risk assessment
- Enhanced capital efficiency

## Testing

Comprehensive test suite covers:
- Guarantor registration and management
- Voucher creation and constraints
- Default protection and claims
- Collateral management
- Query functions
- Error conditions and edge cases

Run tests with:
```bash
cargo test --package sorosusu-contracts --lib guarantor_tests
```

## Deployment Notes

1. **Migration**: Existing circles remain unaffected
2. **Backward Compatibility**: All existing functions continue to work
3. **Gas Optimization**: Storage layout optimized for minimal gas usage
4. **Security Audits**: All new functions audited for common vulnerabilities

## Conclusion

The Guarantor system successfully implements social underwriting for the SoroSusu protocol, enabling financial inclusion while maintaining security and trust. By leveraging existing social hierarchies and reputation systems, it creates a scalable solution for bringing unbanked users into formal savings circles.
