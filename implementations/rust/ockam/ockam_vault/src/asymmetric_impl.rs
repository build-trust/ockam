use crate::{Vault, VaultError};
use arrayref::array_ref;
use ockam_core::vault::{
    AsymmetricVault, Buffer, Hasher, KeyId, PublicKey, Secret, SecretAttributes, SecretKey,
    SecretPersistence, SecretType, SecretVault, VaultEntry, CURVE25519_PUBLIC_LENGTH_USIZE,
    CURVE25519_SECRET_LENGTH_USIZE,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};

impl Vault {
    fn ecdh_internal(vault_entry: &VaultEntry, peer_public_key: &PublicKey) -> Result<Buffer<u8>> {
        match vault_entry.key_attributes().stype() {
            SecretType::X25519 => {
                let key = vault_entry.secret().try_as_key()?;
                if peer_public_key.data().len() != CURVE25519_PUBLIC_LENGTH_USIZE
                    || key.as_ref().len() != CURVE25519_SECRET_LENGTH_USIZE
                {
                    return Err(VaultError::UnknownEcdhKeyType.into());
                }

                let sk = x25519_dalek::StaticSecret::from(*array_ref!(
                    key.as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH_USIZE
                ));
                let pk_t = x25519_dalek::PublicKey::from(*array_ref!(
                    peer_public_key.data(),
                    0,
                    CURVE25519_PUBLIC_LENGTH_USIZE
                ));
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            SecretType::NistP256 | SecretType::Buffer | SecretType::Aes | SecretType::Ed25519 => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
        }
    }
}

#[async_trait]
impl AsymmetricVault for Vault {
    async fn ec_diffie_hellman(
        &self,
        secret: &KeyId,
        peer_public_key: &PublicKey,
    ) -> Result<KeyId> {
        let entries = self.data.entries.read().await;
        let entry = entries
            .get(secret)
            .ok_or(VaultError::EntryNotFound(format!(
                "diffie hellman secret {secret:?}"
            )))?;

        let dh = Self::ecdh_internal(entry, peer_public_key)?;

        // Prevent dead-lock by freeing entries lock, since we don't need it
        drop(entries);

        let attributes = SecretAttributes::new(
            SecretType::Buffer,
            SecretPersistence::Ephemeral,
            dh.len() as u32,
        );
        self.secret_import(Secret::Key(SecretKey::new(dh)), attributes)
            .await
    }

    async fn compute_key_id_for_public_key(&self, public_key: &PublicKey) -> Result<KeyId> {
        let key_id = self.sha256(public_key.data()).await?;
        Ok(hex::encode(key_id))
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
