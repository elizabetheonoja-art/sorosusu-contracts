# Formal Verification Specification

This document defines the mathematical specifications and safety properties for formal verification of the SoroSusu ROSCA contract.

## Safety Invariants

### 1. Vault Balance Integrity

**Invariant**: At any point in time, the total deposits minus total payouts must equal the current vault balance.

```
∀ circle_id, token:
  vault_balance(circle_id, token) = total_deposits(circle_id, token) - total_payouts(circle_id, token)
```

**Variables**:
- `vault_balance(c, t)`: Current token balance held by contract for circle `c`
- `total_deposits(c, t)`: Cumulative sum of all deposits to circle `c` in token `t`
- `total_payouts(c, t)`: Cumulative sum of all payouts from circle `c` in token `t`

**Verification Points**:
- After `deposit()`: `vault_balance += amount`
- After `payout()`: `vault_balance -= (payout_amount + fee)`
- After `compute_and_transfer_payout()`: `vault_balance -= gross_payout`

### 2. Fee Consistency

**Invariant**: Protocol fees must never exceed the payout amount.

```
∀ payout_amount, fee_basis_points:
  fee = (payout_amount × fee_basis_points) / 10000
  fee ≤ payout_amount
  fee_basis_points ≤ 10000
```

**Constraints**:
- `0 ≤ fee_basis_points ≤ 10000` (0% to 100%)
- `net_payout = payout_amount - fee`
- `net_payout ≥ 0`

### 3. Contribution Completeness

**Invariant**: A payout can only occur when all members have contributed for the current cycle.

```
∀ circle_id, cycle:
  can_payout(circle_id, cycle) ⟺ contributions_count(circle_id, cycle) = member_count(circle_id)
```

### 4. Member Uniqueness

**Invariant**: Each member can join a circle at most once.

```
∀ circle_id, member:
  is_member(circle_id, member) ⟹ join_count(circle_id, member) = 1
```

### 5. Non-Negative Balances

**Invariant**: All balances must remain non-negative.

```
∀ circle_id, token:
  vault_balance(circle_id, token) ≥ 0
  total_deposits(circle_id, token) ≥ 0
  total_payouts(circle_id, token) ≥ 0
```

## State Transition Properties

### Deposit Operation

**Preconditions**:
- `is_member(circle_id, caller) = true`
- `token.allowance(caller, contract) ≥ amount`
- `amount > 0`

**Postconditions**:
- `vault_balance' = vault_balance + amount`
- `total_deposits' = total_deposits + amount`
- `contributions_count' = contributions_count + 1`

### Payout Operation

**Preconditions**:
- `contributions_count(circle_id, cycle) = member_count(circle_id)`
- `vault_balance ≥ gross_payout`
- `is_member(circle_id, recipient) = true`

**Postconditions**:
- `fee = (gross_payout × fee_basis_points) / 10000`
- `net_payout = gross_payout - fee`
- `vault_balance' = vault_balance - gross_payout`
- `total_payouts' = total_payouts + gross_payout`
- `recipient_balance' = recipient_balance + net_payout`
- `treasury_balance' = treasury_balance + fee` (if `fee > 0`)

## Authorization Properties

### Admin-Only Operations

```
∀ operation ∈ {initialize, set_protocol_fee}:
  can_execute(operation, caller) ⟺ caller = admin
```

### Member-Only Operations

```
∀ operation ∈ {deposit}:
  can_execute(operation, caller, circle_id) ⟺ is_member(circle_id, caller)
```

## Liveness Properties

### Eventual Payout

**Property**: If all members contribute, a payout must be possible.

```
contributions_count(circle_id, cycle) = member_count(circle_id) ⟹ ◇ can_payout(circle_id, cycle)
```

## Verification Tool Integration

### Halmos (Symbolic Testing)

Target functions for symbolic execution:
- `deposit()`
- `compute_and_transfer_payout()`
- `set_protocol_fee()`

### Certora (Formal Verification)

Recommended rules:
- `vaultBalanceIntegrity`: Verify invariant #1
- `feeConsistency`: Verify invariant #2
- `noNegativeBalances`: Verify invariant #5
- `authorizationCheck`: Verify admin/member permissions

## Test Scenarios

1. **Deposit-Payout Cycle**: Verify vault balance after complete cycle
2. **Fee Calculation**: Verify fee deduction accuracy across edge cases (0%, 100%, 0.5%)
3. **Concurrent Deposits**: Verify balance integrity with multiple simultaneous deposits
4. **Unauthorized Access**: Verify rejection of non-admin/non-member operations
5. **Overflow Protection**: Verify arithmetic operations don't overflow

## Notes

- All arithmetic operations must be checked for overflow/underflow
- Token transfers must be atomic (all-or-nothing)
- State changes must be consistent across all storage entries
