use crate::storage::SecretStorage;
use crate::{
    Hasher, KeyId, Secret, SecretAttributes, SecretKey, SecretType, SecretVault, Vault, VaultError,
    AES128_SECRET_LENGTH_USIZE, AES256_SECRET_LENGTH_USIZE,
};
use arrayref::array_ref;
use ockam_core::compat::vec::Vec;
use ockam_core::{async_trait, compat::boxed::Box, Result};
use sha2::{Digest, Sha256};

#[async_trait]
impl Hasher for Vault {
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        let digest = Sha256::digest(data);
        Ok(*array_ref![digest, 0, 32])
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
        let ikm: Result<SecretKey> = match ikm {
            Some(ikm) => {
                let vault_entry = self.get_secret(ikm, "hkdf_sha256").await?;

                if vault_entry.key_attributes().stype() == SecretType::Buffer {
                    let secret_key = vault_entry.secret().try_as_key()?.clone();
                    Ok(secret_key)
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            None => Ok(SecretKey::new(vec![])),
        };

        let vault_entry = self.get_secret(salt, "hkdf_sha256 salt").await?;

        if vault_entry.key_attributes().stype() != SecretType::Buffer {
            return Err(VaultError::InvalidKeyType.into());
        }

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(
                Some(vault_entry.secret().try_as_key()?.as_ref()),
                ikm?.as_ref(),
            );

            prk.expand(info, okm.as_mut_slice())
                .map_err(|_| Into::<ockam_core::Error>::into(VaultError::HkdfExpandError))?;
            okm
        };

        let mut secrets = Vec::<KeyId>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let length = attributes.length() as usize;
            if attributes.stype() == SecretType::Aes {
                if length != AES256_SECRET_LENGTH_USIZE && length != AES128_SECRET_LENGTH_USIZE {
                    return Err(VaultError::InvalidAesKeyLength.into());
                }
            } else if attributes.stype() != SecretType::Buffer {
                return Err(VaultError::InvalidHkdfOutputType.into());
            }
            let secret = Secret::Key(SecretKey::new(okm[index..index + length].to_vec()));
            let secret = self.secret_import(secret, attributes).await?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }
}

#[cfg(test)]
mod tests {
    use crate as ockam_vault;
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn sha256() {}

    #[ockam_macros::vault_test]
    fn hkdf() {}
}
