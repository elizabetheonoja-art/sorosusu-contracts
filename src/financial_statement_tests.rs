#[cfg(test)]
mod financial_statement_tests {
    use super::*;
    use soroban_sdk::{Address, Env};

    #[test]
    fn test_financial_transaction_tracking() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000, // 1000 tokens with 7 decimals
            &2,
            &token_contract,
            &86400,
            &100, // 1% insurance fee
            &nft_contract,
            &arbitrator,
        );
        
        // Join circle
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Make contributions
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        
        // Finalize round and claim pot
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        
        // Verify financial transactions were tracked
        let creator_transactions = client.get_member_financial_transactions(
            &creator,
            &circle_id,
            0,
            u64::MAX,
        );
        
        assert!(creator_transactions.len() >= 2); // Contribution + Payout
        
        // Check transaction types
        let mut contribution_found = false;
        let mut payout_found = false;
        
        for tx in creator_transactions {
            match tx.transaction_type {
                FinancialTransactionType::Contribution => contribution_found = true,
                FinancialTransactionType::Payout => payout_found = true,
                _ => {}
            }
        }
        
        assert!(contribution_found);
        assert!(payout_found);
    }

    #[test]
    fn test_financial_statement_generation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create and join circle
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Complete a full cycle
        let start_time = env.ledger().timestamp();
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        let end_time = env.ledger().timestamp();
        
        // Generate financial statement
        let statement = client.generate_financial_statement(
            &creator,
            &circle_id,
            &start_time,
            &end_time,
        );
        
        // Verify statement structure
        assert_eq!(statement.circle_id, circle_id);
        assert_eq!(statement.statement_period_start, start_time);
        assert_eq!(statement.statement_period_end, end_time);
        assert!(statement.total_contributions > 0);
        assert!(statement.total_payouts > 0);
        assert!(statement.transaction_count > 0);
        assert!(statement.member_count > 0);
        assert!(!statement.statement_hash.is_empty());
        assert_eq!(statement.verifying_member, creator);
        
        // Verify hash consistency
        let statement2 = client.generate_financial_statement(
            &creator,
            &circle_id,
            &start_time,
            &end_time,
        );
        assert_eq!(statement.statement_hash, statement2.statement_hash);
    }

    #[test]
    fn test_statement_hash_verification() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Complete transactions
        let start_time = env.ledger().timestamp();
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        let end_time = env.ledger().timestamp();
        
        // Generate statement
        let statement = client.generate_financial_statement(
            &creator,
            &circle_id,
            &start_time,
            &end_time,
        );
        
        // Verify hash
        let is_valid = client.verify_statement_hash(
            &circle_id,
            &statement.statement_hash,
            &start_time,
            &end_time,
        );
        
        assert!(is_valid);
        
        // Test invalid hash
        let invalid_hash = vec![&env; 32]; // Different hash
        let is_invalid = client.verify_statement_hash(
            &circle_id,
            &invalid_hash,
            &start_time,
            &end_time,
        );
        
        assert!(!is_invalid);
    }

    #[test]
    fn test_pdf_generation_data() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Complete transactions
        let start_time = env.ledger().timestamp();
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        let end_time = env.ledger().timestamp();
        
        // Get PDF generation data
        let (statement, transactions, metadata) = client.get_pdf_generation_data(
            &creator,
            &circle_id,
            &start_time,
            &end_time,
        );
        
        // Verify all data is present
        assert_eq!(statement.circle_id, circle_id);
        assert!(!transactions.is_empty());
        assert_eq!(metadata.circle_id, circle_id);
        assert_eq!(metadata.creator, creator);
        assert_eq!(metadata.contribution_amount, 1_000_000_000_000);
        assert_eq!(metadata.max_members, 2);
        
        // Verify transaction details
        let mut contribution_count = 0;
        let mut payout_count = 0;
        
        for tx in transactions {
            match tx.transaction_type {
                FinancialTransactionType::Contribution => contribution_count += 1,
                FinancialTransactionType::Payout => payout_count += 1,
                _ => {}
            }
        }
        
        assert!(contribution_count > 0);
        assert!(payout_count > 0);
    }

    #[test]
    fn test_late_payment_tracking() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with short deadline for testing
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &1, // 1 second deadline
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Wait past deadline
        env.ledger().set_timestamp(env.ledger().timestamp() + 10);
        
        // Make late contribution
        client.deposit(&user, &circle_id);
        
        // Check transactions
        let transactions = client.get_member_financial_transactions(
            &user,
            &circle_id,
            0,
            u64::MAX,
        );
        
        // Should have late contribution and penalty
        let mut late_contribution_found = false;
        let mut penalty_found = false;
        
        for tx in transactions {
            if tx.is_late && matches!(tx.transaction_type, FinancialTransactionType::Contribution) {
                late_contribution_found = true;
            }
            if matches!(tx.transaction_type, FinancialTransactionType::Penalty) {
                penalty_found = true;
            }
        }
        
        assert!(late_contribution_found);
        assert!(penalty_found);
    }

    #[test]
    fn test_insurance_fee_tracking() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        // Create circle with insurance fee
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &500, // 5% insurance fee
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Make contribution
        client.deposit(&user, &circle_id);
        
        // Check for insurance fee transaction
        let transactions = client.get_member_financial_transactions(
            &user,
            &circle_id,
            0,
            u64::MAX,
        );
        
        let mut insurance_fee_found = false;
        for tx in transactions {
            if matches!(tx.transaction_type, FinancialTransactionType::InsuranceFee) {
                insurance_fee_found = true;
                assert!(tx.insurance_fee > 0);
                break;
            }
        }
        
        assert!(insurance_fee_found);
    }

    #[test]
    fn test_batch_statement_generation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &3,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user1, &circle_id, &1, &None);
        client.join_circle(&user2, &circle_id, &1, &None);
        
        // Complete transactions
        let start_time = env.ledger().timestamp();
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.claim_pot(&creator, &circle_id);
        let end_time = env.ledger().timestamp();
        
        // Generate batch statements
        let members = vec![&env, creator.clone(), user1.clone(), user2.clone()];
        let statements = client.batch_generate_statements(
            &admin,
            &circle_id,
            members,
            &start_time,
            &end_time,
        );
        
        assert_eq!(statements.len(), 3);
        
        // Verify each statement
        for statement in statements {
            assert_eq!(statement.circle_id, circle_id);
            assert_eq!(statement.statement_period_start, start_time);
            assert_eq!(statement.statement_period_end, end_time);
            assert!(!statement.statement_hash.is_empty());
        }
    }

    #[test]
    fn test_transaction_id_increment() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        client.join_circle(&user, &circle_id, &1, &None);
        
        // Make multiple transactions
        client.deposit(&creator, &circle_id);
        client.deposit(&user, &circle_id);
        
        // Get transactions and check IDs
        let transactions = client.get_member_financial_transactions(
            &creator,
            &circle_id,
            0,
            u64::MAX,
        );
        
        // Should have at least one transaction with ID > 0
        assert!(transactions.len() > 0);
        let first_tx = transactions.get(0).unwrap();
        assert!(first_tx.transaction_id > 0);
    }

    #[test]
    fn test_error_handling() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &1_000_000_000_000,
            &2,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
        );
        
        client.join_circle(&creator, &circle_id, &1, &None);
        
        // Test invalid time range
        let result = env.try_invoke_contract(
            &contract_id,
            &Symbol::new(&env, "generate_financial_statement"),
            (
                creator,
                circle_id,
                100u64, // start
                50u64   // end (before start)
            ),
        );
        assert!(result.is_err());
        
        // Test non-existent circle
        let result = env.try_invoke_contract(
            &contract_id,
            &Symbol::new(&env, "generate_financial_statement"),
            (
                creator,
                999u64, // non-existent circle
                0u64,
                u64::MAX
            ),
        );
        assert!(result.is_err());
        
        // Test no transactions in period
        let future_start = env.ledger().timestamp() + 10000;
        let future_end = future_start + 1000;
        
        let result = env.try_invoke_contract(
            &contract_id,
            &Symbol::new(&env, "generate_financial_statement"),
            (
                creator,
                circle_id,
                future_start,
                future_end
            ),
        );
        assert!(result.is_err());
    }
}
