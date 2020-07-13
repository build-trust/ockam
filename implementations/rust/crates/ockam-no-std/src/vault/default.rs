use crate::error::OckamResult;
use crate::random::{Seed, SEED_BYTES};
use crate::vault::*;

use core::prelude::v1::Some;

use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

use sha2::Sha256;
use sha2::digest::Digest;
use sha2::digest::generic_array::GenericArray;

// Default Vault Specific Constants
const VAULT_DEFAULT_RANDOM_MAX_SIZE: usize = 0xFFFF;

/// The default software Vault;
///
/// The DefaultVault is a software only implementation of Vault methods.
pub struct DefaultVault<'a> {
    attributes: &'a VaultAttributes<'a>,
    rng: Option<StdRng>,
    sha256: Option<Sha256>
}

impl<'a> Vault<'a> for DefaultVault<'a> {
    // Standard methiod to create a new DefaultVault.
    //
    // Explicit lifetime is need to allow the Default Vault
    // to borrow the VaultAttributes.
    fn new(attributes: &'a VaultAttributes<'a>) -> Self {
        DefaultVault {
            attributes,
            rng: None,
            sha256: None
        }
    }

    /// Initializes the Default Vaults internal RNG with seed data from
    /// the Vault Attributes Random.
    fn init(&mut self) -> OckamResult<()> {
        let mut seed: Seed = [0; SEED_BYTES];
        self.attributes.random.get_bytes(&mut seed)?;

        self.rng = Some(StdRng::from_seed(seed));
        self.sha256 = Some(Sha256::new());

        Ok(())
    }

    /// Generate random bytes
    ///
    /// Length of bytes must not exceed VAULT_DEFAULT_RANDOM_MAX_SIZE.
    fn random(&mut self, bytes: &mut [u8]) -> OckamResult<()> {
        if bytes.len() > VAULT_DEFAULT_RANDOM_MAX_SIZE {
            return Err(ERROR_INVALID_SIZE);
        }

        match self.rng.as_mut() {
            Some(r) => {
                r.fill_bytes(bytes);
                Ok(())
            }
            None => Err(ERROR_DEFAULT_RANDOM_REQUIRED),
        }
    }

    /// Generate a 256 bit SHA2 hash for the given bytes with the implementing Vault.
    fn sha256(&mut self, bytes: &mut [u8]) -> OckamResult<GenericArray<u8, U32>> {
        match self.sha256.as_mut() {
            Some(r) => {
                r.update(bytes);
                let result = r.finalize_reset();
                Ok(result)
            }
            None => Err(ERROR_INVALID_CONTEXT),
        }
    }
}

#[cfg(test)]
mod test {
    use crate::random::external_seed::*;
    use crate::vault::*;

    #[test]
    fn can_generate_random_bytes_success() {
        let seed = [
            0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7,
            0, 0, 0,
        ];

        let random = ExternalSeededRandom::new(seed);
        let vault_attributes = VaultAttributes { random: &random };
        let mut default_vault = default::DefaultVault::new(&vault_attributes);

        match default_vault.init() {
            Ok(_) => assert_eq!(true, true),
            Err(_e) => assert_eq!(true, false),
        }

        let mut bytes = [0; 64];
        match default_vault.random(&mut bytes) {
            Ok(_) => assert_eq!(true, true),
            Err(_e) => assert_eq!(true, false),
        }
    }

    #[test]
    fn can_generate_sha256() {
        let seed = [
            0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7,
            0, 0, 0,
        ];

        let random = ExternalSeededRandom::new(seed);
        let vault_attributes = VaultAttributes { random: &random };
        let mut default_vault = default::DefaultVault::new(&vault_attributes);
        
        match default_vault.init() {
            Ok(_) => assert_eq!(true, true),
            Err(_e) => assert_eq!(true, false),
        }

        let mut bytes = [0; 32];
        match default_vault.random(&mut bytes) {
            Ok(_) => assert_eq!(true, true),
            Err(_e) => assert_eq!(true, false),
        }

        // The random number generator is deterministic with the same seed, which allows testing a known hash
        // against the hash generated with the random bytes.
        let hash: [u8; 32] = [0x9E, 0xE5, 0x9A, 0x12, 0xD1, 0x20, 0xFF, 0xF3, 0x3A, 0x85, 0xF4, 0x80, 0x98, 0x38, 0xAD, 0xD0,
        0x3A, 0x46, 0x74, 0x65, 0x9D, 0x45, 0xE7, 0xAF, 0x6E, 0x0B, 0xD2, 0xD3, 0x08, 0x89, 0xEA, 0x90];

        match default_vault.sha256(&mut bytes){
            Ok(_) => assert_eq!(hash, bytes),
            Err(_e) => assert_eq!(true, false),
        }    
    }
}