use crate::constants::{
    AES128_SECRET_LENGTH_USIZE, AES256_SECRET_LENGTH_USIZE, CURVE25519_SECRET_LENGTH_U32,
};
use crate::{
    AsymmetricVault, Buffer, Implementation, PublicKey, Secret, SecretAttributes, SecretType,
    SecretsStore, StoredSecret, Vault, VaultError,
};
use arrayref::array_ref;
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, compat::boxed::Box, KeyId, Result};
use sha2::Sha256;

#[async_trait]
impl<T: SecretsStore + Implementation> AsymmetricVault for T {
    async fn ec_diffie_hellman(
        &self,
        secret: &KeyId,
        peer_public_key: &PublicKey,
    ) -> Result<KeyId> {
        let stored_secret = self
            .get_ephemeral_secret(secret, "diffie hellman secret")
            .await?;
        let dh = Vault::ecdh_internal(&stored_secret, peer_public_key)?;

        let attributes = SecretAttributes::Buffer(dh.len() as u32);
        self.import_ephemeral_secret(Secret::new(dh), attributes)
            .await
    }

    /// Compute sha256.
    /// Salt and Ikm should be of Buffer type.
    /// Output secrets should be only of type Buffer or AES
    async fn hkdf_sha256(
        &self,
        salt: &KeyId,
        info: &[u8],
        ikm: Option<&KeyId>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<KeyId>> {
        let ikm: Result<Secret> = match ikm {
            Some(ikm) => {
                let vault_entry = self.get_ephemeral_secret(ikm, "hkdf_sha256").await?;

                if vault_entry.attributes().secret_type() == SecretType::Buffer {
                    let secret_key = vault_entry.secret().clone();
                    Ok(secret_key)
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            None => Ok(Secret::new(vec![])),
        };

        let vault_entry = self.get_ephemeral_secret(salt, "hkdf_sha256 salt").await?;

        if vault_entry.attributes().secret_type() != SecretType::Buffer {
            return Err(VaultError::InvalidKeyType.into());
        }

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(vault_entry.secret().as_ref()), ikm?.as_ref());

            prk.expand(info, okm.as_mut_slice())
                .map_err(|_| Into::<ockam_core::Error>::into(VaultError::HkdfExpandError))?;
            okm
        };

        let mut secrets = Vec::<KeyId>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let length = attributes.length() as usize;
            if attributes.secret_type() == SecretType::Aes {
                if length != AES256_SECRET_LENGTH_USIZE && length != AES128_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidAesKeyLength.into());
                }
            } else if attributes.secret_type() != SecretType::Buffer {
                return Err(VaultError::InvalidHkdfOutputType.into());
            }
            let secret = Secret::new(okm[index..index + length].to_vec());
            let secret = self.import_ephemeral_secret(secret, attributes).await?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }
}

impl Vault {
    fn ecdh_internal(
        stored_secret: &StoredSecret,
        peer_public_key: &PublicKey,
    ) -> Result<Buffer<u8>> {
        let attributes = stored_secret.attributes();
        match attributes.secret_type() {
            SecretType::X25519 => {
                let key = stored_secret.secret();
                if peer_public_key.data().len() != attributes.length() as usize {
                    return Err(VaultError::UnknownEcdhKeyType.into());
                }

                let sk = x25519_dalek::StaticSecret::from(*array_ref!(
                    key.as_ref(),
                    0,
                    CURVE25519_SECRET_LENGTH_U32 as usize
                ));
                let pk_t = x25519_dalek::PublicKey::from(*array_ref!(
                    peer_public_key.data(),
                    0,
                    CURVE25519_SECRET_LENGTH_U32 as usize
                ));
                let secret = sk.diffie_hellman(&pk_t);
                Ok(secret.as_bytes().to_vec())
            }
            SecretType::Buffer | SecretType::Aes | SecretType::Ed25519 => {
                Err(VaultError::UnknownEcdhKeyType.into())
            }
            #[cfg(feature = "rustcrypto")]
            SecretType::NistP256 => Err(VaultError::UnknownEcdhKeyType.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::new()
    }

    #[ockam_macros::vault_test]
    fn test_ec_diffie_hellman_curve25519() {}

    #[ockam_macros::vault_test]
    fn test_hkdf_sha256() {}
}
