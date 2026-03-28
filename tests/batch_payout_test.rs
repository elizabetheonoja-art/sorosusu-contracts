use soroban_sdk::{contract, contractimpl, Address, Env, token, Symbol, Vec, i128, u64, u32, u16};
use sorosusu::{SoroSusuClient, SoroSusuTrait, CircleInfo, Member, BatchPayoutRecord, IndividualPayoutClaim, Error};

#[contract]
pub struct MockToken;

#[contractimpl]
impl MockToken {
    pub fn initialize(env: Env, admin: Address) {
        // Mock token initialization
    }

    pub fn mint(env: Env, to: Address, amount: i128) {
        // Mock mint function
    }

    pub fn balance(env: Env, addr: Address) -> i128 {
        // Mock balance function - returns a fixed amount for testing
        1_000_000_000 // 100 tokens
    }
}

#[contract]
pub struct MockNft;

#[contractimpl]
impl MockNft {
    pub fn initialize(env: Env, admin: Address) {
        // Mock NFT initialization
    }

    pub fn mint(env: Env, to: Address, id: u128) {
        // Mock mint function
    }

    pub fn burn(env: Env, from: Address, id: u128) {
        // Mock burn function
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn test_configure_batch_payout_single_winner() {
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
            &100_000_000, // 10 tokens contribution
            &10, // 10 members
            &token_contract,
            &86400, // 1 day cycle
            &100, // 1% insurance
            &nft_contract,
            &arbitrator,
            &50, // 0.5% organizer fee
        );
        
        // Add members
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        client.join_circle(&user2, &circle_id);
        
        // Configure batch payout with 1 winner (should work like regular payout)
        client.configure_batch_payout(&creator, &circle_id, &1);
        
        let circle = client.get_circle(&circle_id);
        assert_eq!(circle.winners_per_round, 1);
        assert!(!circle.batch_payout_enabled); // Single winner doesn't enable batch mode
    }

    #[test]
    fn test_configure_batch_payout_multiple_winners() {
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
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &50,
        );
        
        // Add enough members for batch payout
        for i in 0..5 {
            let user = Address::generate(&env);
            client.join_circle(&user, &circle_id);
        }
        
        // Configure batch payout with 5 winners
        client.configure_batch_payout(&creator, &circle_id, &5);
        
        let circle = client.get_circle(&circle_id);
        assert_eq!(circle.winners_per_round, 5);
        assert!(circle.batch_payout_enabled);
    }

    #[test]
    #[should_panic(expected = "Invalid winners per round. Must be 1, 2, 5, or 10")]
    fn test_configure_batch_payout_invalid_winners() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &10,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &50,
        );
        
        // Try to configure with invalid winner count
        client.configure_batch_payout(&creator, &circle_id, &3); // Should panic
    }

    #[test]
    fn test_batch_payout_two_winners_precise_math() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let user2 = Address::generate(&env);
        let user3 = Address::generate(&env);
        let user4 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let contribution_amount = 100_000_000; // 10 tokens
        let circle_id = client.create_circle(
            &creator,
            &contribution_amount,
            &4, // 4 members for precise math testing
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0, // 0% organizer fee for precise math
        );
        
        // Add all members
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        client.join_circle(&user2, &circle_id);
        client.join_circle(&user3, &circle_id);
        
        // Configure for 2 winners per round
        client.configure_batch_payout(&creator, &circle_id, &2);
        
        // All members contribute
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.deposit(&user3, &circle_id);
        
        // Finalize round
        client.finalize_round(&creator, &circle_id);
        
        // Execute batch payout
        client.distribute_batch_payout(&creator, &circle_id);
        
        // Verify precise math: 4 members * 10 tokens = 40 tokens total
        // With 2 winners, each should get exactly 20 tokens
        let expected_payout_per_winner = contribution_amount * 2; // 20 tokens
        
        // Check batch payout record
        let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
        assert_eq!(batch_record.total_winners, 2);
        assert_eq!(batch_record.total_pot, 400_000_000); // 40 tokens
        assert_eq!(batch_record.net_payout_per_winner, expected_payout_per_winner);
        assert_eq!(batch_record.dust_amount, 0); // Should be no dust with even division
        
        // Check individual payout claims
        let winners = batch_record.winners;
        assert_eq!(winners.len(), 2);
        
        for winner in winners.iter() {
            let claim = client.get_individual_payout_claim(&winner, &circle_id, &0).unwrap();
            assert_eq!(claim.amount_claimed, expected_payout_per_winner);
            assert_eq!(claim.round_number, 0);
        }
    }

    #[test]
    fn test_batch_payout_five_winners_dust_handling() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let mut users = Vec::new(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let contribution_amount = 101_000_000; // 10.1 tokens to create dust
        let circle_id = client.create_circle(
            &creator,
            &contribution_amount,
            &10, // 10 members
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0, // 0% organizer fee
        );
        
        // Add 10 members
        client.join_circle(&creator, &circle_id);
        for i in 0..9 {
            let user = Address::generate(&env);
            users.push_back(user.clone());
            client.join_circle(&user, &circle_id);
        }
        
        // Configure for 5 winners per round
        client.configure_batch_payout(&creator, &circle_id, &5);
        
        // All members contribute
        client.deposit(&creator, &circle_id);
        for user in users.iter() {
            client.deposit(&user, &circle_id);
        }
        
        // Finalize round
        client.finalize_round(&creator, &circle_id);
        
        // Execute batch payout
        client.distribute_batch_payout(&creator, &circle_id);
        
        // Verify dust handling: 10 members * 10.1 tokens = 101 tokens total
        // With 5 winners, each should get floor(101/5) = 20 tokens
        // Total distributed: 5 * 20 = 100 tokens
        // Dust remaining: 101 - 100 = 1 token = 0.1 tokens = 10_000_000 stroops
        let expected_payout_per_winner = contribution_amount * 2; // 20.2 tokens, but floor to 20
        let expected_dust = 10_000_000; // 0.1 tokens in stroops
        
        let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
        assert_eq!(batch_record.total_winners, 5);
        assert_eq!(batch_record.total_pot, 1_010_000_000); // 101 tokens
        assert_eq!(batch_record.net_payout_per_winner, 200_000_000); // 20 tokens
        assert_eq!(batch_record.dust_amount, expected_dust);
        
        // Verify all winners got the same amount
        for winner in batch_record.winners.iter() {
            let claim = client.get_individual_payout_claim(&winner, &circle_id, &0).unwrap();
            assert_eq!(claim.amount_claimed, 200_000_000); // All get identical amount
        }
    }

    #[test]
    fn test_batch_payout_ten_winners_large_group() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let mut users = Vec::new(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let contribution_amount = 50_000_000; // 5 tokens
        let circle_id = client.create_circle(
            &creator,
            &contribution_amount,
            &15, // 15 members
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &100, // 1% organizer fee
        );
        
        // Add 15 members
        client.join_circle(&creator, &circle_id);
        for i in 0..14 {
            let user = Address::generate(&env);
            users.push_back(user.clone());
            client.join_circle(&user, &circle_id);
        }
        
        // Configure for 10 winners per round
        client.configure_batch_payout(&creator, &circle_id, &10);
        
        // All members contribute
        client.deposit(&creator, &circle_id);
        for user in users.iter() {
            client.deposit(&user, &circle_id);
        }
        
        // Finalize round
        client.finalize_round(&creator, &circle_id);
        
        // Execute batch payout
        client.distribute_batch_payout(&creator, &circle_id);
        
        // Verify math: 15 members * 5 tokens = 75 tokens total
        // Organizer fee: 75 * 1% = 0.75 tokens
        // Net payout: 75 - 0.75 = 74.25 tokens
        // With 10 winners: floor(74.25/10) = 7 tokens each
        // Total distributed: 10 * 7 = 70 tokens
        // Dust: 74.25 - 70 = 4.25 tokens
        let expected_payout_per_winner = 70_000_000; // 7 tokens
        let expected_dust = 425_000_000; // 4.25 tokens
        
        let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
        assert_eq!(batch_record.total_winners, 10);
        assert_eq!(batch_record.total_pot, 750_000_000); // 75 tokens
        assert_eq!(batch_record.organizer_fee, 7_500_000); // 0.75 tokens
        assert_eq!(batch_record.net_payout_per_winner, expected_payout_per_winner);
        assert_eq!(batch_record.dust_amount, expected_dust);
        
        // Verify all 10 winners got identical amounts
        assert_eq!(batch_record.winners.len(), 10);
        for winner in batch_record.winners.iter() {
            let claim = client.get_individual_payout_claim(&winner, &circle_id, &0).unwrap();
            assert_eq!(claim.amount_claimed, expected_payout_per_winner);
        }
    }

    #[test]
    fn test_batch_payout_fair_rotation() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let mut users = Vec::new(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &6, // 6 members for easier rotation tracking
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0,
        );
        
        // Add 6 members in specific order
        client.join_circle(&creator, &circle_id);
        for i in 0..5 {
            let user = Address::generate(&env);
            users.push_back(user.clone());
            client.join_circle(&user, &circle_id);
        }
        
        // Configure for 3 winners per round
        client.configure_batch_payout(&creator, &circle_id, &3);
        
        // Test rotation across multiple rounds
        for round in 0..3 {
            // All members contribute
            client.deposit(&creator, &circle_id);
            for user in users.iter() {
                client.deposit(&user, &circle_id);
            }
            
            client.finalize_round(&creator, &circle_id);
            client.distribute_batch_payout(&creator, &circle_id);
            
            let batch_record = client.get_batch_payout_record(&circle_id, &round).unwrap();
            let winners = &batch_record.winners;
            
            // Verify fair rotation: winners should be different each round
            if round > 0 {
                let prev_record = client.get_batch_payout_record(&circle_id, &(round - 1)).unwrap();
                let prev_winners = &prev_record.winners;
                
                // Should have minimal overlap in fair rotation
                let mut overlap_count = 0;
                for winner in winners.iter() {
                    if prev_winners.contains(winner) {
                        overlap_count += 1;
                    }
                }
                
                // With 6 members and 3 winners per round, 
                // there should be some rotation but not necessarily zero overlap
                assert!(overlap_count < 3); // At least some rotation
            }
            
            // Advance to next round
            env.ledger().set_sequence(env.ledger().sequence() + 100);
        }
    }

    #[test]
    #[should_panic(expected = "Batch payout is not enabled for this circle")]
    fn test_batch_payout_not_enabled() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &5,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0,
        );
        
        // Add members but don't configure batch payout
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        
        // Try to execute batch payout without enabling it
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.distribute_batch_payout(&creator, &circle_id); // Should panic
    }

    #[test]
    #[should_panic(expected = "Cannot have more winners than members")]
    fn test_batch_payout_too_many_winners() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let user1 = Address::generate(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &100_000_000,
            &3, // Only 3 members
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0,
        );
        
        // Add members
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        
        // Try to configure for 5 winners when only 3 members
        client.configure_batch_payout(&creator, &circle_id, &5); // Should panic
    }

    #[test]
    fn test_batch_payout_audit_trail() {
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
            &100_000_000,
            &4,
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0,
        );
        
        // Add members
        client.join_circle(&creator, &circle_id);
        client.join_circle(&user1, &circle_id);
        client.join_circle(&user2, &circle_id);
        
        // Configure for 2 winners
        client.configure_batch_payout(&creator, &circle_id, &2);
        
        // Execute batch payout
        client.deposit(&creator, &circle_id);
        client.deposit(&user1, &circle_id);
        client.deposit(&user2, &circle_id);
        client.finalize_round(&creator, &circle_id);
        client.distribute_batch_payout(&creator, &circle_id);
        
        // Verify complete audit trail
        let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
        assert!(batch_record.batch_payout_id > 0);
        assert!(batch_record.payout_timestamp > 0);
        
        // Verify each winner has individual claim record
        for winner in batch_record.winners.iter() {
            let claim = client.get_individual_payout_claim(&winner, &circle_id, &0).unwrap();
            assert_eq!(claim.recipient, *winner);
            assert_eq!(claim.circle_id, circle_id);
            assert_eq!(claim.round_number, 0);
            assert_eq!(claim.batch_payout_id, batch_record.batch_payout_id);
            assert!(claim.claim_timestamp > 0);
        }
    }

    #[test]
    fn test_batch_payout_edge_case_maximum_winners() {
        let env = Env::default();
        let admin = Address::generate(&env);
        let creator = Address::generate(&env);
        let mut users = Vec::new(&env);
        let arbitrator = Address::generate(&env);
        
        let token_contract = env.register_contract(None, MockToken);
        let nft_contract = env.register_contract(None, MockNft);
        
        let contract_id = env.register_contract(None, SoroSusu);
        let client = SoroSusuClient::new(&env, &contract_id);
        
        env.mock_all_auths();
        client.init(&admin);
        
        let circle_id = client.create_circle(
            &creator,
            &10_000_000, // 1 token contribution
            &20, // 20 members
            &token_contract,
            &86400,
            &100,
            &nft_contract,
            &arbitrator,
            &0,
        );
        
        // Add 20 members
        client.join_circle(&creator, &circle_id);
        for i in 0..19 {
            let user = Address::generate(&env);
            users.push_back(user.clone());
            client.join_circle(&user, &circle_id);
        }
        
        // Configure for maximum 10 winners
        client.configure_batch_payout(&creator, &circle_id, &10);
        
        // All members contribute
        client.deposit(&creator, &circle_id);
        for user in users.iter() {
            client.deposit(&user, &circle_id);
        }
        
        client.finalize_round(&creator, &circle_id);
        client.distribute_batch_payout(&creator, &circle_id);
        
        // Verify maximum case math
        let batch_record = client.get_batch_payout_record(&circle_id, &0).unwrap();
        assert_eq!(batch_record.total_winners, 10);
        assert_eq!(batch_record.winners.len(), 10);
        
        // Each winner should get identical amount
        let first_amount = client.get_individual_payout_claim(&batch_record.winners.get(0).unwrap(), &circle_id, &0).unwrap().amount_claimed;
        for winner in batch_record.winners.iter() {
            let claim = client.get_individual_payout_claim(&winner, &circle_id, &0).unwrap();
            assert_eq!(claim.amount_claimed, first_amount); // All identical
        }
    }
}
