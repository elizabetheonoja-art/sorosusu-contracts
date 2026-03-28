# Sub-Susu Credit Line Feature

## Overview
This feature introduces a "Community Bank" capability to SoroSusu, allowing high-reputation members to draw an early advance (loan) on their expected future payout. This provides instant liquidity for emergencies without requiring external collateral.

## Mechanism
1. **Credit Checking**: The oracle evaluates the user's `Reliability_Score` (must be >= 500) and ensures the advance doesn't exceed their `total_volume_saved`.
2. **Limit Logic**: The user can draw up to **50% of their expected future payout**.
3. **Approval**: The Group Lead (`creator` of the circle) must invoke `approve_credit_advance` to authorise the disbursement.
4. **Auto-Repayment**: A 5% interest fee is appended to the principal debt. When the user's turn arrives to `claim_pot()`, the contract automatically deducts the principal and interest from their net payout.
5. **Group Reserve Injection**: The 5% interest fee is transferred directly into the circle's `GroupReserve` to benefit the community.

## Security Considerations
- Debt is capped securely to prevent protocol insolvency.
- Unpaid debts are virtually impossible because the advance is deducted directly from the smart contract during the `claim_pot` disbursement phase.
- The Group Lead assumes implicit risk management by acting as the human-verifier to authorise loans manually before funds move.

## Testing 
Tested formally in `test_sub_susu_credit_line()` verifying point allocations, advance disbursal, and auto-deduction.