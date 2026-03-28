# Buddy System Feature

## Overview
The buddy system allows two members to pair their accounts for mutual payment protection. If one member (Buddy A) misses a payment, the system automatically checks for a "Safety Deposit" from their paired member (Buddy B) to cover the gap and prevent group penalties.

## Implementation

### New Data Keys
- `BuddyPair(Address)`: Maps member address to their buddy's address
- `SafetyDeposit(Address, u64)`: Tracks safety deposits by member and circle ID

### New Member Field
- `buddy: Option<Address>`: Optional buddy address for each member

### New Functions

#### `pair_with_member(user: Address, buddy_address: Address)`
- Allows a member to pair with another active member as their buddy
- Both members must be active in the system
- Updates the user's buddy field and creates a buddy pair mapping

#### `set_safety_deposit(user: Address, circle_id: u64, amount: u64)`
- Allows a member to deposit tokens as safety backup for their buddy
- Transfers tokens from user to contract
- Stores the deposit amount for the specific circle

### Modified Functions

#### `deposit(user: Address, circle_id: u64)`
- Enhanced with buddy system fallback logic
- If primary member's payment fails:
  1. Checks if member has a paired buddy
  2. Verifies buddy has sufficient safety deposit for the circle
  3. Uses buddy's safety deposit to cover the payment
  4. Updates or removes the safety deposit accordingly
- If no buddy or insufficient safety deposit, payment fails with appropriate error

## Usage Flow

1. **Pairing**: Member A calls `pair_with_member(buddy_address)` to pair with Member B
2. **Safety Deposit**: Member B calls `set_safety_deposit(circle_id, amount)` to provide backup funds
3. **Payment Protection**: When Member A's payment fails, the system automatically uses Member B's safety deposit
4. **Deposit Management**: Safety deposits are reduced by used amounts and can be topped up as needed

## Benefits

- **Risk Mitigation**: Reduces individual payment failure impact on group
- **Social Trust**: Encourages member accountability through buddy relationships  
- **Group Stability**: Prevents circle disruption from single member defaults
- **Flexible Protection**: Members can choose their level of safety deposit coverage