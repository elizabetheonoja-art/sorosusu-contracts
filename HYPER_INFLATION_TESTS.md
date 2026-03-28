# Hyper-Inflationary Scenario Testing

This document describes the test suite for handling high-decimal tokens and extreme transaction volumes.

## Test Coverage

### 1. 18-Decimal Token Support (`test_18_decimal_token_extreme_volumes`)

Tests tokens with 18 decimals (like ETH, DAI, USDC on Ethereum):
- Amount: 1 billion tokens = 1e27 stroops
- Verifies minting and transfers don't overflow
- Ensures balance tracking remains accurate

### 2. Total Group Volume Overflow Protection (`test_total_group_volume_no_overflow`)

Validates that total volume calculations across all members don't overflow:
- Max contribution: `u64::MAX / 100` (with safety margin)
- Members: 64 (maximum per circle)
- Cycles: 10 rounds
- Uses `checked_mul()` to detect overflow
- Verifies result fits in `i128` (Soroban token standard)

### 3. Contribution Amount Validation (`test_contribution_amount_with_18_decimals`)

Tests various 18-decimal contribution amounts:
- 1 token (1e18)
- 1,000 tokens (1e21)
- 1,000,000 tokens (1e24)

Validates:
- Amounts fit in `i128`
- Fee calculations (0.5%) don't overflow
- Results remain positive

### 4. Insurance Balance Accumulation (`test_insurance_balance_accumulation`)

Simulates long-term insurance fund growth:
- Contribution: 1 token (18 decimals)
- Insurance fee: 2% (200 bps)
- Members: 64
- Cycles: 100

Ensures accumulated insurance balance doesn't overflow `i128`.

### 5. Payout Calculation (`test_payout_calculation_extreme_values`)

Tests maximum safe payout scenarios:
- Contribution: `u64::MAX / 100`
- Members: 64
- Protocol fee: 0.5%

Validates:
- Gross payout calculation
- Fee deduction
- Net payout remains positive

### 6. Bitmap Operations (`test_bitmap_operations_no_overflow`)

Verifies contribution tracking for 64 members:
- Sets all 64 bits in bitmap
- Counts contributions using `count_ones()`
- Ensures bitmap operations are safe

### 7. Late Fee Calculation (`test_late_fee_calculation_extreme_amounts`)

Tests penalty calculations on large amounts:
- Contribution: 1,000 tokens (18 decimals)
- Late fee: 5% (500 bps)
- Validates total due doesn't overflow

### 8. Multi-Circle Volume Tracking (`test_multi_circle_volume_tracking`)

Simulates protocol-wide volume:
- Circles: 1,000
- Contribution per circle: 1 token (18 decimals)
- Members per circle: 50

Ensures total protocol volume fits in `i128`.

### 9. Reserve Balance Accumulation (`test_reserve_balance_accumulation`)

Tests group reserve growth from penalties:
- Penalty per default: 0.1 token
- Defaults: 10,000 over time
- Validates reserve balance doesn't overflow

### 10. Timestamp Overflow Protection (`test_cycle_duration_timestamp_overflow`)

Tests deadline calculations over many cycles:
- Current time: ~2023 timestamp
- Cycle duration: 1 year
- Cycles: 100 years

Ensures timestamps remain within `u64` bounds.

### 11. Overflow Detection (`test_detect_overflow_in_unsafe_multiplication`)

Negative test that intentionally triggers overflow:
- Multiplies `u64::MAX` by 2
- Verifies panic occurs
- Confirms overflow detection works

## Overflow Prevention Strategy

All tests use **checked arithmetic**:

```rust
// Safe multiplication
let result = value
    .checked_mul(multiplier)
    .expect("Overflow detected");

// Safe addition
let total = amount
    .checked_add(fee)
    .expect("Addition overflow");

// Safe division
let fee = amount
    .checked_mul(fee_bps)
    .and_then(|v| v.checked_div(10000))
    .expect("Fee calculation overflow");
```

## Data Type Limits

| Type | Max Value | Use Case |
|------|-----------|----------|
| `u64` | 18,446,744,073,709,551,615 | Contribution amounts, timestamps |
| `u128` | 340,282,366,920,938,463,463,374,607,431,768,211,455 | Intermediate calculations |
| `i128` | ±170,141,183,460,469,231,731,687,303,715,884,105,727 | Soroban token amounts |

## Running Tests

```bash
cargo test --test hyper_inflation_test
```

## Expected Results

All tests should pass, confirming:
- ✅ No overflow in volume calculations
- ✅ 18-decimal tokens handled correctly
- ✅ Fee calculations remain accurate
- ✅ Bitmap operations are safe
- ✅ Timestamp arithmetic doesn't overflow
- ✅ Overflow detection works as expected
