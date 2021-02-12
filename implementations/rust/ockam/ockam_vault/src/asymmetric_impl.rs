use crate::{SoftwareVault, VaultEntry, VaultError};
use arrayref::array_ref;
use ockam_vault_core::Buffer;
use ockam_vault_core::{
    AsymmetricVault, Secret, SecretAttributes, SecretPersistence, SecretType, SecretVault,
    CURVE25519_PUBLIC_LENGTH, CURVE25519_SECRET_LENGTH,
};

impl SoftwareVault {
    fn ecdh_internal(
        vault_entry: &VaultEntry,
        peer_public_key: &[u8],
    ) -> ockam_core::Result<Buffer<u8>> {
        let key = vault_entry.key();
        match vault_entry.key_attributes().stype() {
            SecretType::Curve25519 => {
                if peer_public_key.len() != CURVE25519_PUBLIC_LENGTH
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
                    peer_public_key,
                    0,
                    CURVE25519_PUBLIC_LENGTH
                ));
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            SecretType::P256 | SecretType::Buffer | SecretType::Aes => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
        }
    }
}

impl AsymmetricVault for SoftwareVault {
    fn ec_diffie_hellman(
        &mut self,
        context: &Secret,
        peer_public_key: &[u8],
    ) -> ockam_core::Result<Secret> {
        let entry = self.get_entry(context)?;

        let dh = Self::ecdh_internal(entry, peer_public_key)?;

        let attributes =
            SecretAttributes::new(SecretType::Buffer, SecretPersistence::Ephemeral, dh.len());
        self.secret_import(&dh, attributes)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use ockam_vault_core::{
        AsymmetricVault, SecretAttributes, SecretPersistence, SecretType, SecretVault,
        CURVE25519_SECRET_LENGTH,
    };

    #[test]
    fn ec_diffie_hellman_curve25519() {
        let mut vault = SoftwareVault::default();
        let attributes = SecretAttributes::new(
            SecretType::Curve25519,
            SecretPersistence::Ephemeral,
            CURVE25519_SECRET_LENGTH,
        );
        let sk_ctx_1 = vault.secret_generate(attributes).unwrap();
        let sk_ctx_2 = vault.secret_generate(attributes).unwrap();
        let pk_1 = vault.secret_public_key_get(&sk_ctx_1).unwrap();
        let pk_2 = vault.secret_public_key_get(&sk_ctx_2).unwrap();

        let res1 = vault.ec_diffie_hellman(&sk_ctx_1, pk_2.as_ref());
        assert!(res1.is_ok());
        let _ss1 = res1.unwrap();

        let res2 = vault.ec_diffie_hellman(&sk_ctx_2, pk_1.as_ref());
        assert!(res2.is_ok());
        let _ss2 = res2.unwrap();
        // TODO: Check result against test vector
    }
}
