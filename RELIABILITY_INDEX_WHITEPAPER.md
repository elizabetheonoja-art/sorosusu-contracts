# SoroSusu Reliability Index (RI) Technical Whitepaper

## Overview

The Reliability Index (RI) is SoroSusu's proprietary "Social Credit" system that quantifies user trustworthiness and reliability within the decentralized savings circle ecosystem. The RI serves as a decentralized reputation score that influences protocol fees, access to advanced features, and integration with third-party DeFi services.

## Core Components

The RI is calculated as the arithmetic mean of two primary sub-scores:

**RI = (Reliability Score + Social Capital Score) / 2**

Both sub-scores are expressed in basis points (0-10000, representing 0-100%).

### 1. Reliability Score

The Reliability Score measures a user's historical performance in meeting their financial obligations within savings circles.

#### Calculation Formula:
```
Reliability Score = min(10000, On-Time Rate + Volume Bonus)
```

Where:
- **On-Time Rate** = (Total On-Time Contributions × 10000) / Total Expected Contributions
- **Volume Bonus** = min(100, Total Volume Saved / 1000000) × 50

#### Parameters:
- **On-Time Contributions**: Contributions made before or on the circle's deadline
- **Total Expected Contributions**: All contributions the user was scheduled to make
- **Total Volume Saved**: Cumulative value of payouts received (in stroops)

#### Scoring Examples:
- Perfect record (100% on-time, high volume): 10000 bps
- 95% on-time rate, moderate volume: ~9750 bps
- 85% on-time rate, low volume: ~8500 bps
- New user (no history): 5000 bps (baseline)

### 2. Social Capital Score

The Social Capital Score measures a user's participation in community governance and social support mechanisms.

#### Calculation Formula:
```
Social Capital Score = min(10000, Baseline + Leniency Bonus + Voting Bonus + Decay Penalty)
```

Where:
- **Baseline**: 5000 bps (neutral starting point)
- **Leniency Bonus**: +50 bps per leniency vote given, +25 bps per leniency received
- **Voting Bonus**: +10 bps per governance vote cast
- **Decay Penalty**: -5% monthly if inactive for >6 months

#### Parameters:
- **Leniency Given**: Number of times user voted to grant extensions to struggling members
- **Leniency Received**: Number of times user received leniency from the community
- **Governance Votes**: Total quadratic votes cast in proposals
- **Activity Timestamp**: Last recorded participation in any circle activity

## RI Update Triggers

The RI is recalculated and stored whenever a user performs any of the following actions:

1. **Deposit Contribution**: Updates reliability score based on timeliness
2. **Cast Governance Vote**: Increases social capital score
3. **Request/Grant Leniency**: Updates social capital for both parties
4. **Complete Circle Cycle**: Updates volume metrics
5. **Monthly Heartbeat Check**: Applies inactivity decay

## RI Applications

### 1. Fee Discount Logic
Protocol fees are discounted based on RI tiers:

- **RI ≥ 9000 (Diamond)**: 75% fee discount
- **RI ≥ 8000 (Platinum)**: 50% fee discount  
- **RI ≥ 7000 (Gold)**: 25% fee discount
- **RI ≥ 6000 (Silver)**: 10% fee discount
- **RI < 6000 (Bronze)**: Standard fees

### 2. Access Control
Higher RI unlocks advanced features:

- **RI ≥ 8000**: Early access to liquidity advances
- **RI ≥ 7000**: Priority in randomized payout queues
- **RI ≥ 6000**: Eligibility for governance voting
- **RI ≥ 5000**: Basic circle participation

### 3. Third-Party Integrations
RI scores are exposed via the `get_reputation()` function for:

- DeFi lending protocols (credit scoring)
- Insurance providers (risk assessment)
- NFT marketplaces (soulbound token minting)
- Cross-chain bridges (identity verification)

## Sybil Resistance Mechanisms

To prevent reputation manipulation through multiple accounts:

### 1. Proof of Personhood Integration
- `increase_reputation()` functions require verified unique identity
- Integration with decentralized identity providers
- On-chain verification of personhood proofs

### 2. Activity-Based Validation
- RI increases only through provable on-chain actions
- No direct reputation transfers between accounts
- Decay mechanisms prevent reputation hoarding

### 3. Economic Incentives
- High RI provides tangible benefits (fee discounts)
- Sybil attacks become economically unviable
- Community reporting mechanisms for suspicious activity

## Inactivity Decay

To ensure RI reflects current reliability:

### Decay Formula:
```
New RI = Current RI × (1 - 0.05)  // 5% monthly decay
```

### Trigger Conditions:
- No circle participation for >6 months
- Automatic monthly check via "heartbeat" mechanism
- Decay applies to both sub-scores equally

### Recovery:
- Decay can be reversed through renewed participation
- No minimum RI floor (can decay to 0)
- Historical performance provides foundation for recovery

## Technical Implementation

### Data Structures

```rust
#[contracttype]
pub struct UserReputationMetrics {
    pub reliability_score: u32,     // 0-10000 bps
    pub social_capital_score: u32, // 0-10000 bps
    pub total_cycles: u32,
    pub perfect_cycles: u32,
    pub total_volume_saved: i128,
    pub last_activity: u64,
    pub last_decay: u64,
}

#[contracttype]
pub struct ReputationData {
    pub user_address: Address,
    pub susu_score: u32,        // RI (0-10000 bps)
    pub reliability_score: u32, // 0-10000 bps
    pub total_contributions: u32,
    pub on_time_rate: u32,      // 0-10000 bps
    pub volume_saved: i128,
    pub social_capital: u32,    // 0-10000 bps
    pub last_updated: u64,
    pub is_active: bool,
}
```

### Storage Keys
- `URep:{user_address}`: UserReputationMetrics
- `LastDecay:{user_address}`: Last decay timestamp

### Public Functions
- `get_reputation(user: Address) -> ReputationData`
- `calculate_reliability_score(user: Address) -> u32`
- `apply_inactivity_decay(user: Address)`
- `update_reputation_on_deposit(user: Address, was_on_time: bool)`

## Security Considerations

### Manipulation Prevention
- All reputation updates require on-chain transaction proof
- No admin override capabilities for reputation scores
- Cryptographic verification of contribution timestamps

### Privacy Protection
- Reputation data is publicly readable but not linkable without user consent
- No personal identifying information stored
- Soulbound token mechanism for optional public verification

### Economic Security
- Fee discounts create natural demand for high RI
- Decay prevents long-term reputation squatting
- Community governance over reputation parameters

## Future Enhancements

### Advanced Metrics
- Cross-circle reputation aggregation
- Temporal weighting (recent activity > old activity)
- Predictive scoring using machine learning

### Integration APIs
- Standardized reputation oracle interface
- Cross-protocol reputation portability
- Decentralized identity verification bridges

### Governance
- Community voting on reputation parameters
- Adjustable decay rates and thresholds
- Emergency reputation reset mechanisms

---

*This document provides the technical foundation for SoroSusu's Reliability Index system. Implementation details may evolve through community governance and security audits.*