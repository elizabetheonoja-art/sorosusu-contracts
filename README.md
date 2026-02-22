# SoroSusu: Decentralized Savings Circle

A trustless Rotating Savings and Credit Association (ROSCA) built on Stellar Soroban.

## Deployed Contract
- **Network:** Stellar Mainnet
- **Contract ID:** CAH65U2KXQ34G7AT7QMWP6WUFYWAV6RPJRSDOB4KID6TP3OORS3BQHCX

## Features
- Create savings circles with fixed contribution amounts
- Join existing circles
- Deposit USDC/XLM securely
- Automated payouts (Coming Soon)

## Protocol fee (monetization)

The protocol takes a configurable fee from every payout (e.g. 0.5%).

- **fee_basis_points**: Fee rate in basis points (e.g. `50` = 0.5%). Set via `set_protocol_fee` (admin only). Capped at 10,000 (100%).
- **treasury_address**: Recipient of the fee. Set together with the fee; required when fee &gt; 0.
- Payouts deduct the fee from the payout amount: the recipient receives `payout_amount - fee`, and the fee is transferred to `treasury_address`.

After deploy, call `initialize(admin)` once, then `set_protocol_fee(fee_basis_points, treasury)` to enable fees. When implementing the payout flow, use `compute_and_transfer_payout(env, token, from, recipient, gross_payout)` so every payout is fee-deducted and the fee is sent to the treasury.

## How to Build
```bash
cargo build --target wasm32-unknown-unknown --release
```

## Troubleshooting

This section documents common contract errors and how to resolve them.

Error Code Reference

If your contract uses an error enum, consider mapping them like this:

Code	Error	Description
1001	CycleNotComplete	Contributions for the current round are incomplete
1002	InsufficientAllowance	Token allowance is lower than required contribution
1003	AlreadyJoined	Member already part of circle
1004	CircleNotFound	Invalid circle ID
1005	Unauthorized	Caller not permitted to perform action
1006	InvalidFeeConfig	Fee basis points &gt; 10,000 or treasury not set when fee &gt; 0
1️⃣ Cycle Not Complete

Error: CycleNotComplete

Cause:
Payout attempted before all members completed their contributions.

Resolution:

Ensure all members have deposited

Verify contribution count in storage

Retry payout after completion

2️⃣ Insufficient Allowance

Error: InsufficientAllowance

Cause:
The contract was not approved to transfer sufficient tokens.

Resolution:

Call approve() on the token contract

Approve at least the contribution amount

Retry deposit()

3️⃣ Already Joined

Error: AlreadyJoined

User attempted to join the same circle twice.

Resolution:

Check membership before calling join_circle

4️⃣ Circle Not Found

Error: CircleNotFound

Invalid circle ID supplied.

Resolution:

Query contract storage first

Validate ID on frontend

5️⃣ Unauthorized

Error: Unauthorized

Caller is not permitted to execute the requested function.

Resolution:

Verify admin or member role

Ensure correct signing address