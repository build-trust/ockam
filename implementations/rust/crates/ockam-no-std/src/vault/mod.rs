pub mod default;

use crate::error::{OckamResult, ERROR_INTERFACE_VAULT};
use crate::random::Random;

use sha2::digest::generic_array::GenericArray;
use sha2::digest::generic_array::typenum::U32;

/// Vault method input is not sized correctly.
pub const ERROR_INVALID_SIZE: u32 = (ERROR_INTERFACE_VAULT | 5u32);

// Vault Random is not available.
pub const ERROR_DEFAULT_RANDOM_REQUIRED: u32 = (ERROR_INTERFACE_VAULT | 13u32);

// Vault Internal Error
pub const ERROR_INVALID_CONTEXT: u32 = (ERROR_INTERFACE_VAULT | 3u32);

/// Vault attributes provide the underlying capabilities require by Vault methods.
///
/// Explicit lifetime is used to allow a VaultAttribute to borrow various trait objects.
pub struct VaultAttributes<'a> {
    pub random: &'a (dyn Random + 'a),
}

/// Trait providing Vault operations for the configured VaultAttributes.
///
/// [`new`]: Vault::new
/// [`init`]: Vault::init
/// [`random`]: Vault::random
pub trait Vault<'a> {
    /// Create a new Vault with the given VaultAttributes.
    ///
    /// Vaults derive their features from configured VaultAttributes.
    fn new(attributes: &'a VaultAttributes<'a>) -> Self;

    /// Initiailze the internals of the Vault.
    ///
    /// This must be peformed to enable the features supplied by VaultAttributes.
    fn init(&mut self) -> OckamResult<()>;

    /// Generate a random number with the implementing Vault.
    fn random(&mut self, bytes: &mut [u8]) -> OckamResult<()>;

    /// Generate a 256 bit SHA2 hash for the given bytes with the implementing Vault.
    fn sha256(&mut self, bytes: &mut[u8]) -> OckamResult<GenericArray<u8, U32>>;
}
