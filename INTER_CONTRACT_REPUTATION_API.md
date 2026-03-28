# SoroSusu Inter-Contract Reputation Query Interface

## Overview

The SoroSusu protocol now provides a standardized ABI for inter-contract reputation queries, allowing third-party developers to easily integrate SoroSusu reputation data into their Stellar-based applications. This makes SoroSusu the "Identity Layer" for Stellar SocialFi.

## Interface Specification

### Function Signature

```rust
fn get_reputation(env: Env, user: Address) -> ReputationData
```

### Return Type: ReputationData

```rust
pub struct ReputationData {
    pub user_address: Address,     // The user's Stellar address
    pub susu_score: u32,           // 0-10000 bps (0-100%) - Overall reputation score
    pub reliability_score: u32,    // 0-10000 bps (0-100%) - Payment reliability
    pub total_contributions: u32,   // Total number of contributions made
    pub on_time_rate: u32,         // 0-10000 bps (0-100%) - On-time payment rate
    pub volume_saved: i128,        // Total volume saved in stroops (1 XLM = 10^7 stroops)
    pub social_capital: u32,       // 0-10000 bps (0-100%) - Social trust score
    pub last_updated: u64,         // Unix timestamp of last update
    pub is_active: bool,           // Currently active in SoroSusu circles
}
```

## Usage Examples

### For Lending Applications

```rust
// Check if user qualifies for loan based on Susu Score
let reputation = susu_contract.get_reputation(&user_address);

// High Susu Score (70%+) indicates good creditworthiness
if reputation.susu_score >= 7000 {
    // User qualifies for premium lending rates
    approve_loan(&user, premium_rate);
} else if reputation.susu_score >= 5000 {
    // User qualifies for standard lending rates
    approve_loan(&user, standard_rate);
} else {
    // User has poor reputation - deny or require collateral
    deny_loan(&user);
}
```

### For Stellar Marketplaces

```rust
// Use reputation as trust signal for marketplace transactions
let reputation = susu_contract.get_reputation(&seller_address);

// Display trust badge based on Susu Score
if reputation.susu_score >= 8000 {
    display_trust_badge("Platinum Seller");
} else if reputation.susu_score >= 6000 {
    display_trust_badge("Gold Seller");
} else if reputation.susu_score >= 4000 {
    display_trust_badge("Verified Seller");
}

// Use reliability score for transaction limits
let max_transaction = calculate_transaction_limit(reputation.reliability_score);
```

### For Social Applications

```rust
// Verify user's social capital for community features
let reputation = susu_contract.get_reputation(&user_address);

// High social capital users get enhanced features
if reputation.social_capital >= 7000 {
    unlock_premium_features(&user);
    grant_moderator_privileges(&user);
}
```

## Score Interpretation

### Susu Score (Overall Reputation)
- **8000-10000 (80-100%)**: Excellent - Highly trusted user
- **6000-7999 (60-79%)**: Good - Reliable user with good track record
- **4000-5999 (40-59%)**: Fair - Moderate reputation, some risk
- **2000-3999 (20-39%)**: Poor - High risk, limited trust
- **0-1999 (0-19%)**: Very Poor - Untrusted or new user

### Reliability Score
- Based on on-time payment history and volume saved
- Higher scores indicate consistent payment behavior
- Automatically boosted by higher savings volumes

### Social Capital
- Derived from trust scores within SoroSusu circles
- Reflects community standing and peer trust
- Increased through positive social interactions

## Integration Guidelines

### 1. Contract Client Setup

```rust
use soroban_sdk::{Address, Env};
use sorosusu_contracts::{SoroSusuClient, ReputationData};

let susu_contract = Address::from_string(&env, "CDLZFC3SYJYDZT7K67VY751LEV6W7QD3VDCDYWCQXVLNS7R5R6C4C");
let client = SoroSusuClient::new(&env, &susu_contract);
```

### 2. Error Handling

```rust
let reputation = match client.try_get_reputation(&user_address) {
    Ok(reputation) => reputation,
    Err(_) => {
        // Handle contract call errors
        ReputationData::default() // or appropriate fallback
    }
};
```

### 3. Caching Strategy

- Reputation data is updated in real-time
- Consider caching for 5-15 minutes to reduce gas costs
- Use `last_updated` field for cache invalidation

### 4. Rate Limiting

- Be mindful of rate limits when making frequent queries
- Implement client-side caching to respect network resources

## Security Considerations

### 1. Contract Address Verification
Always verify you're calling the official SoroSusu contract:
```
Mainnet: CDLZFC3SYJYDZT7K67VY751LEV6W7QD3VDCDYWCQXVLNS7R5R6C4C
Testnet: [Testnet address to be announced]
```

### 2. Data Validation
- Validate that returned scores are within expected ranges (0-10000)
- Check `last_updated` to ensure data freshness
- Handle edge cases for new users (zero scores)

### 3. Integration Testing
- Test with various user reputation levels
- Verify error handling for edge cases
- Ensure gas costs are acceptable for your use case

## Benefits of Integration

1. **Enhanced Trust**: Leverage proven on-chain reputation data
2. **Risk Reduction**: Make informed decisions based on user history
3. **User Acquisition**: Tap into the existing SoroSusu user base
4. **Ecosystem Growth**: Contribute to the Stellar SocialFi ecosystem
5. **Standardized Interface**: Consistent API across all integrations

## Support

For integration support:
- GitHub Issues: https://github.com/SoroSusu-Protocol/sorosusu-contracts
- Documentation: https://docs.sorosusu.com
- Community: https://discord.gg/sorosusu

## License

This interface is provided under the MIT License. See LICENSE for details.
