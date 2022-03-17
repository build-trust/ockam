use crate::{Vault, VaultEntry, VaultError};
use arrayref::array_ref;
use ockam_core::vault::{
    AsymmetricVault, Buffer, PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType,
    SecretVault, CURVE25519_PUBLIC_LENGTH, CURVE25519_SECRET_LENGTH,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

impl Vault {
    fn ecdh_internal(vault_entry: &VaultEntry, peer_public_key: &PublicKey) -> Result<Buffer<u8>> {
        let key = vault_entry.key();
        match vault_entry.key_attributes().stype() {
            SecretType::X25519 => {
                if peer_public_key.as_ref().len() != CURVE25519_PUBLIC_LENGTH
                    || key.as_ref().len() != CURVE25519_SECRET_LENGTH
                {
                    return Err(VaultError::UnknownEcdhKeyType.into());
                }

                let sk = x25519_dalek::StaticSecret::from(*array_ref!(
                    key.as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH
                ));
                let pk_t = x25519_dalek::PublicKey::from(*array_ref!(
                    peer_public_key.as_ref(),
                    0,
                    CURVE25519_PUBLIC_LENGTH
                ));
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            #[cfg(feature = "bls")]
            SecretType::Bls => Err(VaultError::UnknownEcdhKeyType.into()),
            SecretType::Buffer | SecretType::Aes | SecretType::Ed25519 => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
        }
    }
}

#[async_trait]
impl AsymmetricVault for Vault {
    async fn ec_diffie_hellman(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        let entries = self.entries.read().await;
        let entry = entries
            .get(&context.index())
            .ok_or(VaultError::EntryNotFound)?;

        let dh = Self::ecdh_internal(entry, peer_public_key)?;

        // Prevent dead-lock by freeing entries lock, since we don't need it
        drop(entries);

        let attributes =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, dh.len());
        self.secret_import(&dh, attributes).await
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn ec_diffie_hellman_curve25519() {}
}
