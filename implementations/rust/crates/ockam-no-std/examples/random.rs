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

    let random = ExternalSeededRandom::new(seed);
    let vault_attributes = VaultAttributes { random: &random };
    let mut default_vault = DefaultVault::new(&vault_attributes);

    // Panics on failure.
    default_vault.init()?;

    // Create an array of 32 for ease printing in this example.
    // Print out the hex values for each generation.
    // Each generation provides a different approach to error handling.
    let mut bytes = [0; 32];
    default_vault.random(&mut bytes)?;
    println!("Generation 1: {:02X?}", bytes);

    // Allow the application to handle the result as needed.
    match default_vault.random(&mut bytes) {
        Ok(_) => println!("Generation 2: {:02X?}", bytes),
        Err(_e) => {}
    }

    // Propogate the result. Application would likely provide a custom result type.
    match default_vault.random(&mut bytes) {
        Ok(_) => {
            println!("Generation 3: {:02X?}", bytes);
            Ok(())
        }
        Err(e) => Err(e),
    }
}
