use crate::{SoftwareVault, software_vault::VaultEntry, VaultError};
use arrayref::array_ref;
use ockam_core::Result;
use ockam_vault_core::Buffer;
use ockam_vault_core::{
    PublicKey, Secret, SecretAttributes, SecretPersistence, SecretType,
    CURVE25519_PUBLIC_LENGTH, CURVE25519_SECRET_LENGTH,
};

impl SoftwareVault {
    pub(crate) fn ec_diffie_hellman_sync(
        &self,
        context: &Secret,
        peer_public_key: &PublicKey,
    ) -> Result<Secret> {
        let dh = {
            let storage = self.inner.read();
            let entry = storage.get_entry(context)?;
            Self::ecdh_internal(entry, peer_public_key)?
        };

        let attributes =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, dh.len());
        self.secret_import_sync(&dh, attributes)
    }

    fn ecdh_internal(vault_entry: &VaultEntry, peer_public_key: &PublicKey) -> Result<Buffer<u8>> {
        let key = vault_entry.key();
        match vault_entry.key_attributes().stype() {
            SecretType::Curve25519 => {
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
            SecretType::P256 | SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
        }
    }
}
