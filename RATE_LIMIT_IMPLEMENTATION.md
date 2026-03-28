# Rate-Limited Group Creation

## Overview

To prevent spam attacks where malicious actors create thousands of empty "zombie groups" to bloat the network, we've implemented a rate limiting mechanism on circle creation.

## Implementation

### Rate Limit Parameters

- **Cooldown Period**: 5 minutes (300 seconds)
- **Scope**: Per creator address
- **Enforcement**: Transaction reverts if rate limit is violated

### Storage

Added new storage key to track creation timestamps:

```rust
DataKey::LastCreatedTimestamp(Address)
```

This stores the timestamp of the last circle creation for each user address.

### Logic Flow

1. **Check Last Creation**: When `create_circle()` is called, retrieve the creator's last creation timestamp
2. **Calculate Time Elapsed**: Compute time difference between current timestamp and last creation
3. **Enforce Rate Limit**: If elapsed time < 300 seconds, revert with error message
4. **Update Timestamp**: On successful creation, update the creator's last creation timestamp
5. **Proceed with Creation**: Continue with normal circle creation logic

### Code Changes

```rust
fn create_circle(env: Env, creator: Address, ...) -> u64 {
    // Rate limiting check
    let current_time = env.ledger().timestamp();
    let rate_limit_key = DataKey::LastCreatedTimestamp(creator.clone());
    
    if let Some(last_created) = env.storage().instance().get::<DataKey, u64>(&rate_limit_key) {
        let time_elapsed = current_time.saturating_sub(last_created);
        const RATE_LIMIT_SECONDS: u64 = 300; // 5 minutes
        
        if time_elapsed < RATE_LIMIT_SECONDS {
            panic!("Rate limit: Must wait 5 minutes between circle creations");
        }
    }
    
    // Update timestamp
    env.storage().instance().set(&rate_limit_key, &current_time);
    
    // ... rest of creation logic
}
```

## Security Benefits

### Spam Prevention

- **Limits Attack Surface**: Attackers can only create 1 circle per 5 minutes per address
- **Resource Protection**: Prevents storage bloat from thousands of empty circles
- **Network Health**: Reduces unnecessary transactions and ledger entries

### Attack Cost Analysis

Without rate limiting:
- Attacker could create unlimited circles instantly
- Cost: Only transaction fees per circle

With rate limiting:
- Attacker limited to 12 circles per hour per address
- To create 1000 circles: Requires 84 addresses or 7 hours
- Significantly increases attack cost and complexity

## User Experience

### Normal Users

- **First Circle**: No delay, creates immediately
- **Subsequent Circles**: Must wait 5 minutes between creations
- **Multiple Addresses**: Each address has independent rate limit

### Error Handling

If rate limit is violated, transaction reverts with clear message:
```
"Rate limit: Must wait 5 minutes between circle creations"
```

Frontend should:
1. Display remaining cooldown time
2. Disable "Create Circle" button during cooldown
3. Show countdown timer

## Testing

Test suite covers:
- ✅ Rate limit enforcement (< 5 minutes blocked)
- ✅ Exact boundary condition (= 5 minutes allowed)
- ✅ Multiple users with independent limits
- ✅ Underflow protection with `saturating_sub()`
- ✅ Long period validation (> 5 minutes allowed)
- ✅ Constant validation (300 seconds = 5 minutes)

Run tests:
```bash
cargo test --test rate_limit_test
```

## Alternative Approaches Considered

### 1. Creation Fee (Not Implemented)

**Pros**: Economic deterrent, generates protocol revenue
**Cons**: Barrier to entry for legitimate users, requires XLM handling

### 2. Reputation System (Not Implemented)

**Pros**: Rewards good actors, flexible limits
**Cons**: Complex implementation, requires historical tracking

### 3. Admin Whitelist (Not Implemented)

**Pros**: Complete control over creators
**Cons**: Centralized, poor UX, doesn't scale

## Future Enhancements

1. **Dynamic Rate Limits**: Adjust cooldown based on user reputation
2. **Graduated Limits**: First circle instant, subsequent circles rate-limited
3. **Admin Override**: Allow admin to bypass rate limits for trusted users
4. **Metrics Dashboard**: Track creation patterns and spam attempts

## Configuration

To modify the rate limit duration, update the constant:

```rust
const RATE_LIMIT_SECONDS: u64 = 300; // Change to desired seconds
```

Recommended values:
- **Strict**: 600 seconds (10 minutes)
- **Moderate**: 300 seconds (5 minutes) ← Current
- **Lenient**: 60 seconds (1 minute)
