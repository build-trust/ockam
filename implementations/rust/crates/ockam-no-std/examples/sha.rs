use ockam_no_std::error::OckamResult;
use ockam_no_std::random::external_seed::{ExternalSeededRandom, SeedableRandom};
use ockam_no_std::random::Seed;
use ockam_no_std::vault::{default::DefaultVault, Vault, VaultAttributes};

/// Ockam Vault Random Bytes Generation Example

fn main() -> OckamResult<()> {
    let seed: Seed = [
        0, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 5, 0, 0, 0, 6, 0, 0, 0, 7, 0,
        0, 0,
    ];

    // Default vault initialization.
    let random = ExternalSeededRandom::new(seed);
    let vault_attributes = VaultAttributes { random: &random };
    let mut default_vault = DefaultVault::new(&vault_attributes);

    // Panics on failure.
    default_vault.init()?;

    // Create an array of 32 for ease printing in this example.
    let mut bytes = [0; 32];

    // Generate some random bytes for use as input to the hash function.
    default_vault.random(&mut bytes)?;
    match default_vault.sha256(&mut bytes) {
        Ok(_) => {
            println!("SHA 256: {:02X?}", bytes);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
