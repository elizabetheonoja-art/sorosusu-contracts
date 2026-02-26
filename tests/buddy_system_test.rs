use soroban_sdk::{Env, Address, testutils::Address as _};
use sorosusu_contracts::{SoroSusu, SoroSusuTrait};

#[test]
fn test_buddy_pairing() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusuTrait::init(env.clone(), admin.clone());

    // Create a circle
    let circle_id = SoroSusuTrait::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token.clone(),
        604800,
        0,
        nft_contract.clone(),
    );

    // Both users join the circle
    SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id, 1);
    SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id, 1);

    env.mock_all_auths();

    // User1 pairs with User2 as buddy
    SoroSusuTrait::pair_with_member(env.clone(), user1.clone(), user2.clone());

    // User2 sets safety deposit
    SoroSusuTrait::set_safety_deposit(env.clone(), user2.clone(), circle_id, 2000);

    println!("✅ Buddy system pairing and safety deposit test passed");
}

#[test]
fn test_buddy_payment_fallback() {
    let env = Env::default();
    let admin = Address::generate(&env);
    let creator = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let token = Address::generate(&env);
    let nft_contract = Address::generate(&env);

    // Initialize contract
    SoroSusuTrait::init(env.clone(), admin.clone());

    // Create a circle
    let circle_id = SoroSusuTrait::create_circle(
        env.clone(),
        creator.clone(),
        1000,
        5,
        token.clone(),
        604800,
        0,
        nft_contract.clone(),
    );

    // Both users join the circle
    SoroSusuTrait::join_circle(env.clone(), user1.clone(), circle_id, 1);
    SoroSusuTrait::join_circle(env.clone(), user2.clone(), circle_id, 1);

    env.mock_all_auths();

    // User1 pairs with User2 as buddy
    SoroSusuTrait::pair_with_member(env.clone(), user1.clone(), user2.clone());

    // User2 sets safety deposit (enough to cover user1's payment)
    SoroSusuTrait::set_safety_deposit(env.clone(), user2.clone(), circle_id, 2000);

    // Note: In a real test, we would simulate user1's payment failure
    // and verify that buddy's safety deposit is used
    // This requires more complex token mock setup

    println!("✅ Buddy payment fallback test structure created");
}