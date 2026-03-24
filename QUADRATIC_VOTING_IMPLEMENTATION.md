# Quadratic Voting for Large Groups - Democratic Governance

## Overview

Quadratic Voting is a sophisticated governance mechanism that prevents wealthy members from dominating group decisions by implementing a quadratic cost function for voting power. This ensures that influence grows proportionally to the square root of token holdings, creating a more democratic decision-making process for large Susu groups.

## Architecture

### Core Components

1. **Proposal System**: Structured proposals for rule changes and governance decisions
2. **Quadratic Voting Power**: Voting power calculated as sqrt(token_balance)
3. **Cost Function**: Vote cost = weight² (quadratic relationship)
4. **Supermajority Requirements**: 60% approval threshold for rule changes
5. **Quorum Enforcement**: 40% minimum participation for validity

### Key Constants

```rust
const QUADRATIC_VOTING_PERIOD: u64 = 604800; // 7 days for rule changes
const QUADRATIC_QUORUM: u32 = 40; // 40% quorum for quadratic voting
const QUADRATIC_MAJORITY: u32 = 60; // 60% supermajority for rule changes
const MAX_VOTE_WEIGHT: u32 = 100; // Maximum quadratic vote weight
const MIN_GROUP_SIZE_FOR_QUADRATIC: u32 = 10; // Enable for groups >= 10 members
```

## Data Structures

### Proposal
```rust
pub struct Proposal {
    pub id: u64,
    pub circle_id: u64,
    pub proposer: Address,
    pub proposal_type: ProposalType,
    pub title: String,
    pub description: String,
    pub created_timestamp: u64,
    pub voting_start_timestamp: u64,
    pub voting_end_timestamp: u64,
    pub status: ProposalStatus,
    pub for_votes: u64,
    pub against_votes: u64,
    pub total_voting_power: u64,
    pub quorum_met: bool,
    pub execution_data: String, // JSON or structured data for execution
}
```

### ProposalType
- `ChangeLateFee`: Modify late fee percentage
- `ChangeInsuranceFee`: Adjust insurance fee rates
- `ChangeCycleDuration`: Alter contribution cycle timing
- `AddMember`: Add new members to existing circle
- `RemoveMember`: Remove members from circle
- `ChangeQuorum`: Modify voting requirements
- `EmergencyAction`: Emergency measures

### QuadraticVote
```rust
pub struct QuadraticVote {
    pub voter: Address,
    pub proposal_id: u64,
    pub vote_weight: u32,
    pub vote_choice: QuadraticVoteChoice,
    pub voting_power_used: u64,
    pub timestamp: u64,
}
```

### VotingPower
```rust
pub struct VotingPower {
    pub member: Address,
    pub circle_id: u64,
    pub token_balance: i128,
    pub quadratic_power: u64,    // sqrt(token_balance)
    pub last_updated: u64,
}
```

## Quadratic Voting Mathematics

### Voting Power Calculation
```rust
// Quadratic voting power = sqrt(token_balance)
let quadratic_power = sqrt(token_balance);

// Example calculations:
// 100 XLM tokens = sqrt(100) = 10 voting power
// 1,000 XLM tokens = sqrt(1000) ≈ 31.6 voting power
// 10,000 XLM tokens = sqrt(10000) = 100 voting power
```

### Vote Cost Function
```rust
// Cost to vote with weight w = w²
let vote_cost = vote_weight * vote_weight;

// Examples:
// Weight 1 vote = 1² = 1 voting power
// Weight 5 vote = 5² = 25 voting power
// Weight 10 vote = 10² = 100 voting power
// Weight 20 vote = 20² = 400 voting power
```

### Wealth Influence Mitigation
```rust
// Traditional voting: 10x wealth = 10x influence
// Quadratic voting: 10x wealth = sqrt(10x) ≈ 3.16x influence

// This dramatically reduces the advantage of wealthy members
// while still allowing meaningful participation
```

## Core Functions

### 1. create_proposal(env, proposer, circle_id, type, title, description, execution_data)
Creates a governance proposal for group voting.

**Requirements:**
- Proposer must be active member
- Circle must have ≥ 10 members (quadratic voting enabled)
- Valid proposal type and description

**Process:**
1. Validates proposer and circle requirements
2. Creates proposal with 7-day voting period
3. Sets initial status to `Active`
4. Updates circle statistics

### 2. quadratic_vote(env, voter, proposal_id, vote_weight, vote_choice)
Casts a quadratic vote on a proposal.

**Requirements:**
- Voter must be active member
- Proposal must be in `Active` status
- Voting period not expired
- Sufficient voting power available
- Vote weight ≤ MAX_VOTE_WEIGHT

**Process:**
1. Validates voting eligibility and timing
2. Calculates quadratic cost: weight²
3. Verifies sufficient voting power
4. Records vote and updates proposal tallies
5. Checks quorum requirements

### 3. execute_proposal(env, caller, proposal_id)
Finalizes and executes proposal results.

**Requirements:**
- Voting period expired
- Quorum met (40% participation)
- Proposal still in `Active` status

**Process:**
1. Validates voting completion
2. Calculates approval percentage
3. Applies supermajority threshold (60%)
4. Executes proposal if approved
5. Updates statistics

### 4. update_voting_power(env, member, circle_id, token_balance)
Updates member's voting power based on token holdings.

**Process:**
1. Calculates quadratic power: sqrt(token_balance)
2. Updates VotingPower record
3. Timestamps for freshness

## Governance Rules

### Eligibility Requirements
- **Group Size**: Quadratic voting enabled for circles ≥ 10 members
- **Member Status**: Only active members can propose and vote
- **Token Holdings**: Voting power derived from token balance
- **One Vote**: Each member can vote once per proposal

### Decision Rules
- **Quorum**: 40% of members must participate
- **Supermajority**: 60% approval required for passage
- **Voting Period**: 7 days for all proposals
- **Early Execution**: Possible after voting deadline

### Vote Weight Limits
- **Maximum Weight**: 100 units per vote
- **Cost Function**: weight² voting power consumed
- **Power Allocation**: Based on sqrt(token_balance)

## Usage Examples

### Basic Proposal Flow
```rust
// 1. Create proposal to reduce late fees
let title = String::from_str(&env, "Reduce Late Fee to 0.5%");
let description = String::from_str(&env, "Lower late fee from 1% to 0.5% to help members");
let execution_data = String::from_str(&env, "{\"late_fee_bps\": 50}");

let proposal_id = client.create_proposal(
    &proposer,
    &circle_id,
    &ProposalType::ChangeLateFee,
    &title,
    &description,
    &execution_data,
);

// 2. Members vote with different weights
// Member A: 100 XLM = 10 voting power
client.update_voting_power(&member_a, &circle_id, &100_000_0);
client.quadratic_vote(&member_a, &proposal_id, &8u32, &QuadraticVoteChoice::For); // Cost: 64

// Member B: 1,000 XLM = ~31.6 voting power  
client.update_voting_power(&member_b, &circle_id, &1_000_000_0);
client.quadratic_vote(&member_b, &proposal_id, &20u32, &QuadraticVoteChoice::For); // Cost: 400

// Member C: 10,000 XLM = 100 voting power
client.update_voting_power(&member_c, &circle_id, &10_000_000_0);
client.quadratic_vote(&member_c, &proposal_id, &10u32, &QuadraticVoteChoice::Against); // Cost: 100

// 3. Execute after voting period
client.execute_proposal(&admin, &proposal_id);
```

### Wealth Influence Comparison
```rust
// Traditional vs Quadratic Voting influence:

// Wealthy member: 10,000 XLM
// Traditional: 10,000x influence (if 1 XLM = 1 vote)
// Quadratic: sqrt(10000) = 100 voting power

// Small member: 100 XLM  
// Traditional: 100x influence
// Quadratic: sqrt(100) = 10 voting power

// Influence ratio reduced from 100:1 to 10:1
```

### Checking Voting Power
```rust
// Get member's current voting power
let voting_power = client.get_voting_power(&member, &circle_id);
println!("Token balance: {}", voting_power.token_balance);
println!("Quadratic power: {}", voting_power.quadratic_power);

// Calculate maximum vote weight possible
let max_weight = (voting_power.quadratic_power as f64).sqrt() as u32;
println!("Maximum vote weight: {}", max_weight.min(100)); // Capped at 100
```

### Proposal Statistics
```rust
// Get circle governance statistics
let stats = client.get_proposal_stats(&circle_id);
println!("Total proposals: {}", stats.total_proposals);
println!("Approval rate: {}%", 
    (stats.approved_proposals * 100) / stats.total_proposals);
println!("Average participation: {}", stats.average_participation);
```

## Security Considerations

### Attack Prevention
1. **Wealth Dominance**: Quadratic function limits wealthy influence
2. **Sybil Attacks**: Only verified circle members can vote
3. **Double Voting**: One vote per member per proposal
4. **Vote Manipulation**: Immutable vote records

### Economic Security
1. **Cost Scaling**: Increasing costs prevent vote spam
2. **Power Limits**: Maximum vote weight prevents extreme influence
3. **Quorum Requirements**: Prevents small group decisions
4. **Supermajority**: Ensures broad consensus for changes

### Governance Security
1. **Proposal Validation**: Only valid proposal types allowed
2. **Execution Safety**: Controlled execution of approved changes
3. **Time Locks**: 7-day voting period for considered decisions
4. **Transparency**: All votes and proposals publicly recorded

## Mathematical Analysis

### Influence Distribution
```rust
// Traditional voting linear relationship:
// influence = k * wealth

// Quadratic voting sublinear relationship:
// influence = k * sqrt(wealth)

// Derivative (marginal influence):
// d(influence)/d(wealth) = k / (2 * sqrt(wealth))
// Marginal influence decreases with wealth
```

### Cost-Benefit Analysis
```rust
// Cost to double voting power:
// From weight w to 2w requires (2w)² - w² = 3w² additional power
// Diminishing returns encourage distributed voting

// Example: Going from weight 5 to 10
// Cost: 10² - 5² = 100 - 25 = 75 additional voting power
// Same as going from weight 10 to 15: 15² - 10² = 225 - 100 = 125
```

### Equilibrium Analysis
```rust
// In equilibrium, marginal cost = marginal benefit
// For wealthy members, high cost reduces excessive voting
// For smaller members, lower costs enable meaningful participation
```

## Integration with Existing Systems

### Leniency Voting Integration
```rust
// Small groups (< 10): Use simple leniency voting
// Large groups (≥ 10): Use quadratic voting for rule changes
// Both systems can coexist in the same protocol

let circle = client.get_circle_info(circle_id);
if circle.quadratic_voting_enabled {
    // Use quadratic voting for major changes
} else {
    // Use simple voting for leniency requests
}
```

### Proposal Execution
```rust
// Different execution paths for different proposal types
match proposal.proposal_type {
    ProposalType::ChangeLateFee => {
        // Update circle.late_fee_bps
        // Requires circle-wide consensus
    }
    ProposalType::EmergencyAction => {
        // Immediate execution for emergencies
        // Higher threshold required
    }
    // ... other types
}
```

## Analytics and Insights

### Governance Metrics
The system provides valuable insights into group dynamics:

- **Participation Rates**: Member engagement in governance
- **Vote Distribution**: Understanding of member preferences
- **Wealth Influence**: Effectiveness of quadratic mitigation
- **Proposal Success**: Types of changes that gain support

### Trust and Cooperation
```rust
// High participation + diverse voting = healthy governance
// Low participation = potential disengagement issues
// Consistent voting patterns = stable group dynamics
// Sudden changes = potential conflicts or external factors
```

## Future Enhancements

### 1. Dynamic Quorum
```rust
// Adjust quorum based on proposal importance
fn calculate_quorum(proposal_type: ProposalType) -> u32 {
    match proposal_type {
        ProposalType::EmergencyAction => 25, // Lower quorum for emergencies
        ProposalType::ChangeQuorum => 50,    // Higher quorum for rule changes
        _ => 40,                             // Standard quorum
    }
}
```

### 2. Delegated Voting
```rust
// Allow members to delegate voting power
pub struct VotingDelegation {
    pub delegator: Address,
    pub delegate: Address,
    pub delegation_ratio: u32, // Percentage of power delegated
    pub expires_at: u64,
}
```

### 3. Reputation Weighting
```rust
// Combine quadratic power with reputation scores
fn calculate_enhanced_power(
    quadratic_power: u64,
    reputation_score: u32,
    participation_history: f64,
) -> u64 {
    let reputation_multiplier = 1.0 + (reputation_score as f64 / 100.0);
    let participation_bonus = participation_history.min(1.5);
    (quadratic_power as f64 * reputation_multiplier * participation_bonus) as u64
}
```

### 4. Proposal Categorization
```rust
// Different thresholds for different proposal categories
pub enum ProposalCategory {
    Operational,    // Day-to-day operations (50% threshold)
    Financial,      // Financial changes (60% threshold)  
    Governance,     // Rule changes (70% threshold)
    Emergency,      // Emergency actions (40% threshold)
}
```

## Testing

The quadratic voting system includes comprehensive tests covering:

1. **Group Size Detection**: Automatic enabling for large groups
2. **Proposal Creation**: Valid and invalid proposal scenarios
3. **Voting Power**: Correct sqrt calculations and limits
4. **Quadratic Costs**: Proper weight² cost enforcement
5. **Quorum Requirements**: 40% participation validation
6. **Supermajority**: 60% approval threshold enforcement
7. **Security**: Double voting, insufficient power prevention
8. **Execution**: Proper proposal execution logic

Run tests with:
```bash
cargo test --test quadratic_voting_test
```

## Migration Guide

### For Existing Circles
- Circles < 10 members: Continue using simple governance
- Circles ≥ 10 members: Quadratic voting automatically enabled
- No breaking changes to existing functionality
- Gradual adoption encouraged

### For Members
- Voting power automatically calculated from token holdings
- No action required to participate
- Optional: Review voting power before voting
- Education materials recommended

## Conclusion

Quadratic Voting represents a significant advancement in DAO governance, addressing the critical issue of wealth concentration in decision-making. By implementing a quadratic cost function, the system ensures that:

1. **Wealth Influence is Mitigated**: Rich members cannot dominate decisions
2. **Small Voices Are Heard**: Limited resources still enable meaningful participation  
3. **Decisions Are Democratic**: Broad consensus required for changes
4. **Security Is Maintained**: Robust protections against manipulation

This implementation provides a practical, mathematically sound approach to governance that scales effectively while preserving democratic principles. It demonstrates how sophisticated economic mechanisms can be applied to create more equitable and resilient decentralized organizations.

The system balances the need for efficient decision-making with the imperative of inclusive governance, making it ideal for large Susu groups where traditional voting would be vulnerable to wealth-based capture.
