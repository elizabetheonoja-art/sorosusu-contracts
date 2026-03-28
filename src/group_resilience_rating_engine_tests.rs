#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{Address, Env, Symbol};

    #[test]
    fn test_health_rating_determination() {
        assert_eq!(
            GroupResilienceRatingEngine::determine_health_rating(9500),
            GroupHealthRating::Excellent
        );
        assert_eq!(
            GroupResilienceRatingEngine::determine_health_rating(7000),
            GroupHealthRating::Good
        );
        assert_eq!(
            GroupResilienceRatingEngine::determine_health_rating(5000),
            GroupHealthRating::Fair
        );
        assert_eq!(
            GroupResilienceRatingEngine::determine_health_rating(3000),
            GroupHealthRating::Poor
        );
        assert_eq!(
            GroupResilienceRatingEngine::determine_health_rating(1000),
            GroupHealthRating::Critical
        );
    }

    #[test]
    fn test_rating_to_threshold() {
        assert_eq!(
            GroupResilienceRatingEngine::rating_to_threshold(GroupHealthRating::Excellent),
            8000
        );
        assert_eq!(
            GroupResilienceRatingEngine::rating_to_threshold(GroupHealthRating::Good),
            6000
        );
        assert_eq!(
            GroupResilienceRatingEngine::rating_to_threshold(GroupHealthRating::Fair),
            4000
        );
        assert_eq!(
            GroupResilienceRatingEngine::rating_to_threshold(GroupHealthRating::Poor),
            2000
        );
        assert_eq!(
            GroupResilienceRatingEngine::rating_to_threshold(GroupHealthRating::Critical),
            0
        );
    }

    #[test]
    fn test_calculate_group_health_score() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        // Initialize the contract
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        // Test calculation for a sample circle
        let circle_id = 1u64;
        let metrics = GroupResilienceRatingEngine::calculate_group_health_score(env.clone(), circle_id);
        
        // Verify the metrics structure
        assert_eq!(metrics.circle_id, circle_id);
        assert!(metrics.health_score <= 10000);
        assert!(metrics.aggregate_reputation <= 10000);
        assert!(metrics.payment_consistency <= 10000);
        assert!(metrics.member_stability <= 10000);
        assert!(metrics.historical_performance <= 10000);
        
        // Verify the rating is one of the valid options
        match metrics.rating {
            GroupHealthRating::Excellent |
            GroupHealthRating::Good |
            GroupHealthRating::Fair |
            GroupHealthRating::Poor |
            GroupHealthRating::Critical => {},
        }
    }

    #[test]
    fn test_get_group_health_metrics() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let circle_id = 2u64;
        
        // First call should calculate and store
        let metrics1 = GroupResilienceRatingEngine::get_group_health_metrics(env.clone(), circle_id);
        
        // Second call should retrieve from storage
        let metrics2 = GroupResilienceRatingEngine::get_group_health_metrics(env.clone(), circle_id);
        
        // Both should be identical
        assert_eq!(metrics1.circle_id, metrics2.circle_id);
        assert_eq!(metrics1.health_score, metrics2.health_score);
        assert_eq!(metrics1.rating, metrics2.rating);
    }

    #[test]
    fn test_payment_history() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let circle_id = 3u64;
        let payment_history = GroupResilienceRatingEngine::get_payment_history(env.clone(), circle_id);
        
        assert_eq!(payment_history.circle_id, circle_id);
        assert_eq!(payment_history.total_payments, 0);
        assert_eq!(payment_history.on_time_payments, 0);
        assert_eq!(payment_history.late_payments, 0);
        assert_eq!(payment_history.very_late_payments, 0);
    }

    #[test]
    fn test_member_stability_metrics() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let circle_id = 4u64;
        let stability_metrics = GroupResilienceRatingEngine::get_member_stability_metrics(env.clone(), circle_id);
        
        assert_eq!(stability_metrics.circle_id, circle_id);
        assert_eq!(stability_metrics.retention_rate, 10000); // Should start at 100%
    }

    #[test]
    fn test_historical_performance() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let circle_id = 5u64;
        let historical_perf = GroupResilienceRatingEngine::get_historical_performance(env.clone(), circle_id);
        
        assert_eq!(historical_perf.circle_id, circle_id);
        assert_eq!(historical_perf.completed_cycles, 0);
        assert_eq!(historical_perf.successful_cycles, 0);
    }

    #[test]
    fn test_search_healthy_groups() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        // Create some sample groups with different health scores
        for i in 1..=5 {
            GroupResilienceRatingEngine::calculate_group_health_score(env.clone(), i);
        }
        
        // Search for groups with Good rating or higher
        let healthy_groups = GroupResilienceRatingEngine::search_healthy_groups(
            env.clone(), 
            GroupHealthRating::Good, 
            10
        );
        
        // Should return some groups (mock data will vary)
        assert!(healthy_groups.len() <= 10);
    }

    #[test]
    fn test_batch_update_health_scores() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let mut circle_ids = Vec::new(&env);
        for i in 1..=3 {
            circle_ids.push_back(i);
        }
        
        let results = GroupResilienceRatingEngine::batch_update_health_scores(env.clone(), circle_ids);
        
        assert_eq!(results.len(), 3);
        for metrics in results {
            assert!(metrics.health_score <= 10000);
        }
    }

    #[test]
    fn test_groups_at_risk() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        // Create some sample groups
        for i in 1..=5 {
            GroupResilienceRatingEngine::calculate_group_health_score(env.clone(), i);
        }
        
        // Search for groups at risk (Poor rating or lower)
        let at_risk_groups = GroupResilienceRatingEngine::get_groups_at_risk(
            env.clone(), 
            GroupHealthRating::Poor, 
            10
        );
        
        // Should return some groups (mock data will vary)
        assert!(at_risk_groups.len() <= 10);
    }

    #[test]
    fn test_health_score_calculation_weights() {
        let env = Env::default();
        let admin = Address::generate(&env);
        
        GroupResilienceRatingEngine::init(env.clone(), admin);
        
        let circle_id = 10u64;
        let metrics = GroupResilienceRatingEngine::calculate_group_health_score(env.clone(), circle_id);
        
        // Verify that the health score is calculated using the correct weights
        let expected_score = (
            (metrics.aggregate_reputation * REPUTATION_WEIGHT) / TOTAL_WEIGHT_BPS +
            (metrics.payment_consistency * PAYMENT_CONSISTENCY_WEIGHT) / TOTAL_WEIGHT_BPS +
            (metrics.member_stability * MEMBER_STABILITY_WEIGHT) / TOTAL_WEIGHT_BPS +
            (metrics.historical_performance * HISTORICAL_PERFORMANCE_WEIGHT) / TOTAL_WEIGHT_BPS
        ).min(10000);
        
        // Allow for small rounding differences
        assert!((metrics.health_score as i32 - expected_score as i32).abs() <= 1);
    }

    #[test]
    fn test_member_metrics_calculation() {
        let env = Env::default();
        
        let (total, active, defaulted, avg_trust) = 
            GroupResilienceRatingEngine::get_member_metrics(&env, 1);
        
        assert!(total >= 5 && total <= 14);
        assert!(active <= total);
        assert!(defaulted <= total);
        assert!(avg_trust >= 75 && avg_trust <= 95);
    }

    #[test]
    fn test_payment_metrics() {
        let env = Env::default();
        
        let on_time_rate = GroupResilienceRatingEngine::get_on_time_payment_rate(&env, 1);
        assert!(on_time_rate >= 8000 && on_time_rate <= 9500);
        
        let completed_cycles = GroupResilienceRatingEngine::get_completed_cycles(&env, 1);
        assert!(completed_cycles <= 12);
    }

    #[test]
    fn test_individual_component_calculations() {
        let env = Env::default();
        
        // Test aggregate reputation calculation
        let reputation = GroupResilienceRatingEngine::calculate_aggregate_reputation(&env, 1);
        assert!(reputation <= 10000);
        
        // Test payment consistency calculation
        let consistency = GroupResilienceRatingEngine::calculate_payment_consistency(&env, 1);
        assert!(consistency <= 10000);
        
        // Test member stability calculation
        let stability = GroupResilienceRatingEngine::calculate_member_stability(&env, 1);
        assert!(stability <= 10000);
        
        // Test historical performance calculation
        let performance = GroupResilienceRatingEngine::calculate_historical_performance(&env, 1);
        assert!(performance <= 10000);
    }
}
