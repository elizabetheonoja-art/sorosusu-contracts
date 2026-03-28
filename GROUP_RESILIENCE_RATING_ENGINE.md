# Group Resilience Rating Engine

## Overview

The Group Resilience Rating Engine is a smart contract component that calculates and maintains health scores for SoroSusu groups (circles). This system provides prospective members with transparent due diligence information about group health before joining, encouraging self-moderation and healthy group dynamics.

## Features

### Health Score Calculation

The GroupHealthScore is calculated using a weighted algorithm:

- **Aggregate Reputation (40%)**: Average trust score of all active members
- **Payment Consistency (35%)**: On-time payment rate and late payment frequency
- **Member Stability (15%)**: Member turnover and default rates
- **Historical Performance (10%)**: Completed cycles and milestone achievements

### Health Ratings

Groups are categorized into five health ratings:

- **Excellent** (80-100%): Very healthy groups with strong performance
- **Good** (60-79%): Healthy groups with minor areas for improvement
- **Fair** (40-59%): Moderately healthy groups with some concerns
- **Poor** (20-39%): Unhealthy groups with significant issues
- **Critical** (0-19%): Very unhealthy groups at high risk

### Public Search Interface

The rating engine provides several public search functions:

- `search_healthy_groups()`: Find groups above a minimum health rating
- `get_groups_at_risk()`: Identify groups that need intervention
- `get_top_performing_groups()`: Get the highest-rated groups
- `get_health_score_history()`: Track health trends over time

## Data Structures

### GroupHealthMetrics
Comprehensive health information for a group including:
- Individual component scores
- Overall health score and rating
- Member statistics
- Payment performance metrics
- Historical performance data

### PaymentHistory
Detailed payment consistency tracking:
- Total, on-time, and late payment counts
- Average lateness metrics
- Payment consistency trends over time

### MemberStabilityMetrics
Member turnover and stability analysis:
- Member turnover rate
- Default rate
- Average member tenure
- Retention metrics

### HistoricalPerformance
Long-term performance tracking:
- Completed and successful cycles
- Milestone achievements
- Crisis recovery capabilities
- Health streak tracking

## Integration Points

The rating engine integrates with the main SoroSusu contract through:

1. **Member Data**: Access to member information and status
2. **Social Capital**: Trust scores and reputation data
3. **Payment History**: Contribution and payment records
4. **Milestone Data**: Achievement and performance metrics

## Usage Examples

### Calculating Group Health
```rust
let metrics = rating_engine.calculate_group_health_score(env, circle_id);
println!("Group {} Health Score: {} ({})", 
    circle_id, 
    metrics.health_score, 
    metrics.rating);
```

### Finding Healthy Groups
```rust
let healthy_groups = rating_engine.search_healthy_groups(
    env, 
    GroupHealthRating::Good, 
    10
);
```

### Monitoring At-Risk Groups
```rust
let at_risk_groups = rating_engine.get_groups_at_risk(
    env, 
    GroupHealthRating::Poor, 
    5
);
```

## Benefits

### For Prospective Members
- **Transparency**: Clear visibility into group health before joining
- **Risk Assessment**: Data-driven decision making
- **Due Diligence**: Comprehensive group performance metrics

### For Group Organizers
- **Self-Moderation**: Incentive to maintain healthy group dynamics
- **Reputation Building**: High health scores attract quality members
- **Performance Tracking**: Identify and address issues proactively

### For Platform Health
- **Quality Control**: Automatic filtering of low-quality groups
- **Risk Management**: Early identification of problematic groups
- **Trust Building**: Transparent reputation system

## Technical Implementation

### Storage Structure
- `GroupHealthMetrics`: Primary health score storage
- `PaymentHistory`: Payment consistency data
- `MemberStabilityMetrics`: Member turnover tracking
- `HistoricalPerformance`: Long-term performance data
- `HealthScoreHistory`: Time-series health score tracking

### Calculation Logic
The health score calculation uses a weighted average approach with basis points (bps) for precision:

```rust
health_score = (
    (aggregate_reputation * 4000) / 10000 +
    (payment_consistency * 3500) / 10000 +
    (member_stability * 1500) / 10000 +
    (historical_performance * 1000) / 10000
).min(10000)
```

### Update Triggers
Health scores are updated when:
- New members join or leave
- Payments are made (on-time or late)
- Members default or are ejected
- Milestones are achieved
- Manual recalculation is requested

## Security Considerations

- **Read-Only Public Access**: Health scores are publicly readable but only authorized updates
- **Data Integrity**: All calculations use verified data from the main contract
- **Manipulation Resistance**: Multiple data points make score manipulation difficult
- **Transparent Logic**: Open calculation methodology builds trust

## Future Enhancements

1. **Machine Learning**: Advanced pattern recognition for health prediction
2. **Social Graph Analysis**: Network effects on group health
3. **Economic Factors**: Market conditions impact on group performance
4. **Dynamic Weights**: Adaptive weighting based on group characteristics
5. **Predictive Analytics**: Early warning system for potential issues

## Testing

Comprehensive test suite covering:
- Health score calculation accuracy
- Rating determination logic
- Search functionality
- Data integrity
- Edge cases and error handling

Run tests with:
```bash
cargo test group_resilience_rating_engine_tests
```

## Deployment

The rating engine is deployed as a separate contract that interacts with the main SoroSusu contract through contract clients. This modular approach allows for independent updates and maintenance.

## Conclusion

The Group Resilience Rating Engine provides a robust, transparent, and data-driven system for evaluating group health in the SoroSusu platform. It creates incentives for healthy group behavior while giving prospective members the information they need to make informed decisions about joining groups.
