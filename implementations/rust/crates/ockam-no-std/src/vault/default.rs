use crate::error::OckamResult;
use crate::random::{Seed, SEED_BYTES};
use crate::vault::{Vault, VaultAttributes, ERROR_DEFAULT_RANDOM_REQUIRED, ERROR_INVALID_SIZE};

use core::prelude::v1::Some;
use rand::rngs::StdRng;
use rand::{RngCore, SeedableRng};

// Default Vault Specific Constants
const VAULT_DEFAULT_RANDOM_MAX_SIZE: usize = 0xFFFF;

/// The default software Vault;
///
/// The DefaultVault is a software only implementation of Vault methods.
pub struct DefaultVault<'a> {
    attributes: &'a VaultAttributes<'a>,
    rng: Option<StdRng>,
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
        }
    }

    /// Initializes the Default Vaults internal RNG with seed data from
    /// the Vault Attributes Random.
    fn init(&mut self) -> OckamResult<()> {
        let mut seed: Seed = [0; SEED_BYTES];
        self.attributes.random.get_bytes(&mut seed)?;

        self.rng = Some(StdRng::from_seed(seed));
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
}
