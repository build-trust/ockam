pub mod external_seed;

use crate::error::{OckamResult, ERROR_INTERFACE_RANDOM};

// Error code for operations that limit parameter size to MAX_SIZE.
pub const ERROR_INVALID_SIZE: u32 = (ERROR_INTERFACE_RANDOM | 2u32);

// Maximum size of input data for Random methods.
pub const MAX_SIZE: usize = 0xFFFF;

// Bytes required for random number generator seed data.
pub const SEED_BYTES: usize = 32;

// Convienence type for seed data.
pub type Seed = [u8; SEED_BYTES];

// The Random trait defines functionality required for entropy sources.
// Vaults utilize output data from Random methods to seed random number generators
// used by Vault operations.
/// Trait providing Vault operations for the configured VaultAttributes.
///
/// [`get_bytes`]: Random::get_bytes
pub trait Random {
    // Get random bytes.
    //
    // Implementations are required to manage an entropy source.
    fn get_bytes(&self, bytes: &mut [u8]) -> OckamResult<()>;
}
