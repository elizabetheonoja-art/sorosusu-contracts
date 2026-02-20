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

## How to Build
```bash
cargo build --target wasm32-unknown-unknown --release

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