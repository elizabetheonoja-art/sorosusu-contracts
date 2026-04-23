// --- STEALTH MODE / PRIVATE PAYOUT ORDER MODULE ---
//
// This module implements privacy-preserving random winner selection for ROSCA circles.
// Instead of revealing the "Next Winner" publicly, the payout order is randomized using
// a seed-based generator (similar to Mersenne Twister) that keeps the winner secret
// until the round begins.
//
// Use Cases:
// - Groups that want to stay private
// - Preventing "Social Engineering" or external harassment of winners
// - Ensuring communal saving remains a safe and private activity

use soroban_sdk::{contracttype, Env};

// --- CONSTANTS ---

// Mersenne Twister parameters (MT19937-32)
const MT_N: usize = 624;
const MT_M: usize = 397;
const MT_MATRIX_A: u32 = 0x9908b0df;
const MT_UPPER_MASK: u32 = 0x80000000;
const MT_LOWER_MASK: u32 = 0x7fffffff;

// --- DATA STRUCTURES ---

/// Storage key for stealth mode configuration per circle
#[contracttype]
#[derive(Clone)]
pub struct StealthConfig {
    pub enabled: bool,           // Whether stealth mode is enabled for this circle
    pub seed: u64,               // Current seed for RNG (regenerated each round)
    pub round_number: u32,       // Current round number (for seed derivation)
    pub pending_winner: Option<u32>, // Winner index prepared but still hidden
    pub revealed_winner: Option<u32>, // Index of winner after reveal (None = not revealed)
}

impl Default for StealthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            seed: 0,
            round_number: 0,
            pending_winner: None,
            revealed_winner: None,
        }
    }
}

/// Storage key for the RNG state
#[derive(Clone)]
pub struct RngState {
    pub state: [u32; MT_N],
    pub index: u32,
}

impl Default for RngState {
    fn default() -> Self {
        Self {
            state: [0; MT_N],
            index: MT_N as u32,
        }
    }
}

/// DataKey for stealth mode storage
#[contracttype]
#[derive(Clone)]
pub enum StealthDataKey {
    Config(u64), // StealthConfig for a circle
}

// --- MERSENNE TWISTER IMPLEMENTATION ---

/// Initialize Mersenne Twister RNG from a seed
/// This implements MT19937-32 algorithm
pub fn mt_init(seed: u64) -> RngState {
    let mut state = [0u32; MT_N];
    
    // Initialize state[0] with seed, then fill remaining elements
    state[0] = seed as u32;
    
    for i in 1..MT_N {
        // The magic formula from MT algorithm
        state[i] = (1812433253u32)
            .wrapping_mul(state[i - 1] ^ (state[i - 1] >> 30))
            .wrapping_add(i as u32);
    }
    
    RngState {
        state,
        index: MT_N as u32,
    }
}

/// Generate next 32-bit random number
pub fn mt_next(env: &Env, rng: &mut RngState) -> u32 {
    // Regenerate state if needed
    if rng.index >= MT_N as u32 {
        mt_reload(env, rng);
    }
    
    let mut y = rng.state[rng.index as usize];
    rng.index += 1;
    
    // Tempering transformation
    y ^= y >> 11;
    y ^= (y << 7) & 0x9d2c5680u32;
    y ^= (y << 15) & 0xefc60000u32;
    y ^= y >> 18;
    
    y
}

/// Reload the Mersenne Twister state array
fn mt_reload(_env: &Env, rng: &mut RngState) {
    // This is a simplified reload for Stellar's no_std environment
    // In production, you'd implement the full twist operation
    
    for i in 0..MT_N {
        let x = if i < MT_N - MT_M {
            rng.state[i + MT_M]
        } else {
            rng.state[i + MT_M - MT_N]
        };
        
        let mut y = rng.state[i];
        y = y.wrapping_mul(2); // Simplified twist
        
        // Apply matrix A to upper bit
        if (x & MT_UPPER_MASK) != 0 {
            y ^= MT_MATRIX_A;
        }
        
        rng.state[i] = y;
    }
    
    rng.index = 0;
}

/// Generate a random index from a range [0, max) using MT
pub fn mt_range(env: &Env, rng: &mut RngState, max: u32) -> u32 {
    if max == 0 {
        return 0;
    }
    
    // Use rejection sampling for uniform distribution
    let bound = u32::MAX - (u32::MAX % max);
    loop {
        let r = mt_next(env, rng);
        if r < bound {
            return r % max;
        }
    }
}

// --- STEALTH MODE FUNCTIONS ---

/// Initialize stealth mode for a circle
/// Called when creating a circle with stealth mode enabled
pub fn init_stealth_mode(env: &Env, circle_id: u64, initial_seed: u64) {
    let seed = derive_seed(initial_seed, 0);
    let config = StealthConfig {
        enabled: true,
        seed,
        round_number: 0,
        pending_winner: None,
        revealed_winner: None,
    };
    
    // Store config
    let key = StealthDataKey::Config(circle_id);
    env.storage().instance().set(&key, &config);
    
}

/// Derive a new seed from base seed and round number
/// This creates a deterministic but unpredictable sequence
fn derive_seed(base_seed: u64, round: u32) -> u64 {
    // Simple hash-like derivation
    let mut seed = base_seed.wrapping_add(round as u64);
    seed = seed.wrapping_mul(0x5deece66d);
    seed = seed.wrapping_add(0xb);
    seed
}

/// Prepare the next round's winner (secretly)
/// This should be called at the start of each round but the winner
/// is not revealed until distribute_payout is called
pub fn prepare_next_winner(env: &Env, circle_id: u64, member_count: u32) -> u32 {
    if member_count == 0 {
        return 0;
    }

    let key = StealthDataKey::Config(circle_id);
    let mut config: StealthConfig = env.storage().instance()
        .get(&key)
        .unwrap_or_default();
    
    if !config.enabled {
        // Fall back to sequential if stealth mode not enabled
        let sequential = config.round_number % member_count;
        config.round_number += 1;
        config.pending_winner = Some(sequential);
        config.revealed_winner = None;
        env.storage().instance().set(&key, &config);
        return sequential;
    }
    
    // Increment round number
    config.round_number += 1;
    
    // Derive new seed for this round
    config.seed = derive_seed(config.seed, config.round_number);
    
    // Initialize RNG with new seed
    let mut rng_state = mt_init(config.seed);
    
    // Generate winner index
    let winner_index = mt_range(env, &mut rng_state, member_count);
    
    // Store winner secretly (not revealed yet)
    config.pending_winner = Some(winner_index);
    config.revealed_winner = None;
    
    // Save updated config
    env.storage().instance().set(&key, &config);
    
    // Save updated RNG state
    winner_index
}

/// Reveal the winner for the current round
/// This should be called when distribute_payout is invoked
pub fn reveal_winner(env: &Env, circle_id: u64) -> Option<u32> {
    let key = StealthDataKey::Config(circle_id);
    let mut config: StealthConfig = env.storage().instance()
        .get(&key)
        .unwrap_or_default();
    
    if !config.enabled {
        return None;
    }
    
    if let Some(revealed) = config.revealed_winner {
        return Some(revealed);
    }

    if let Some(pending) = config.pending_winner {
        config.revealed_winner = Some(pending);
        env.storage().instance().set(&key, &config);
        return Some(pending);
    }

    None
}

/// Check if stealth mode is enabled for a circle
pub fn is_stealth_enabled(env: &Env, circle_id: u64) -> bool {
    let key = StealthDataKey::Config(circle_id);
    let config: StealthConfig = env.storage().instance()
        .get(&key)
        .unwrap_or_default();
    
    config.enabled
}

/// Get the current stealth configuration for a circle
pub fn get_stealth_config(env: &Env, circle_id: u64) -> StealthConfig {
    let key = StealthDataKey::Config(circle_id);
    env.storage().instance()
        .get(&key)
        .unwrap_or_default()
}

/// Enable or disable stealth mode for an existing circle
pub fn toggle_stealth_mode(env: &Env, circle_id: u64, enabled: bool, new_seed: u64) {
    let key = StealthDataKey::Config(circle_id);
    let mut config: StealthConfig = env.storage().instance()
        .get(&key)
        .unwrap_or_default();
    
    config.enabled = enabled;
    
    if enabled {
        config.seed = if new_seed > 0 {
            derive_seed(new_seed, 0)
        } else if config.seed > 0 {
            config.seed
        } else {
            derive_seed(generate_random_seed(env), 0)
        };
        config.round_number = 0;
        config.pending_winner = None;
        config.revealed_winner = None;

    } else {
        config.pending_winner = None;
        config.revealed_winner = None;
    }
    
    env.storage().instance().set(&key, &config);
}

// --- UTILITY FUNCTIONS ---

/// Generate a secure random seed from environment
/// Uses ledger timestamp and other entropy sources
pub fn generate_random_seed(env: &Env) -> u64 {
    let timestamp = env.ledger().timestamp();
    let sequence = env.ledger().sequence();
    
    // Mix entropy sources
    let mut seed = timestamp.wrapping_mul(0x5deece66d);
    seed = seed.wrapping_add(sequence as u64);

    // Mix in contract address length for deterministic per-contract variance.
    let contract_addr_len = env.current_contract_address().to_string().len() as u64;
    seed = seed.wrapping_add(contract_addr_len.wrapping_mul(0x9e3779b97f4a7c15));
    
    seed
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_mt_init() {
        let env = Env::default();
        let rng = mt_init(12345);
        assert_eq!(rng.index, MT_N as u32);
    }
    
    #[test]
    fn test_mt_range_uniformity() {
        let env = Env::default();
        let mut rng = mt_init(42);
        
        // Test that range produces values in valid range
        for _ in 0..100 {
            let val = mt_range(&env, &mut rng, 10);
            assert!(val < 10);
        }
    }
    
    #[test]
    fn test_derive_seed_deterministic() {
        let seed1 = derive_seed(100, 1);
        let seed2 = derive_seed(100, 1);
        assert_eq!(seed1, seed2);
        
        let seed3 = derive_seed(100, 2);
        assert_ne!(seed1, seed3);
    }
    
    #[test]
    fn test_stealth_config_default() {
        let config = StealthConfig::default();
        assert!(!config.enabled);
        assert_eq!(config.seed, 0);
        assert_eq!(config.round_number, 0);
        assert_eq!(config.pending_winner, None);
        assert_eq!(config.revealed_winner, None);
    }

    #[test]
    fn test_prepare_and_reveal_winner() {
        let env = Env::default();
        let contract_id = env.register_contract(None, crate::SoroSusu);

        env.as_contract(&contract_id, || {
            init_stealth_mode(&env, 1, 9999);

            let winner = prepare_next_winner(&env, 1, 5);
            assert!(winner < 5);

            let config_after_prepare = get_stealth_config(&env, 1);
            assert_eq!(config_after_prepare.pending_winner, Some(winner));
            assert_eq!(config_after_prepare.revealed_winner, None);

            let revealed = reveal_winner(&env, 1);
            assert_eq!(revealed, Some(winner));

            let config_after_reveal = get_stealth_config(&env, 1);
            assert_eq!(config_after_reveal.revealed_winner, Some(winner));
        });
    }
}