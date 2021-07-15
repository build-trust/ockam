// This program creates and then immediately stops a node.

use ockam::{Context, Result, SoftwareVault, Vault};
use ockam_vault::ockam_vault_core::{
    SecretPersistence, SecretType, SymmetricVault, AES256_SECRET_LENGTH, CURVE25519_SECRET_LENGTH,
};
use ockam_vault::{Hasher, SecretAttributes, SecretVault};

#[ockam::node]
async fn main(mut ctx: Context) -> Result<()> {
    let vault = Vault::create(&ctx)?;

    Ok(())
}
