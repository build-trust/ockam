use crate::vault::Vault;
use crate::VaultError;
use arrayref::array_ref;
use ockam_core::compat::vec::Vec;
use ockam_core::vault::{
    Hasher, Secret, SecretAttributes, SecretType, SecretVault, AES128_SECRET_LENGTH,
    AES256_SECRET_LENGTH,
};
use ockam_core::Result;
use ockam_core::{async_trait, compat::boxed::Box};
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
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Secret>> {
        let entries = self.entries.read().await;

        let ikm: Result<&[u8]> = match ikm {
            Some(ikm) => {
                let ikm = entries.get(&ikm.index()).ok_or(VaultError::EntryNotFound)?;
                if ikm.key_attributes().stype() == SecretType::Buffer {
                    Ok(ikm.key().as_ref())
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            None => Ok(&[0u8; 0]),
        };

        let ikm = ikm?;

        let salt = entries
            .get(&salt.index())
            .ok_or(VaultError::EntryNotFound)?;

        if salt.key_attributes().stype() != SecretType::Buffer {
            return Err(VaultError::InvalidKeyType.into());
        }

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.key().as_ref()), ikm);

            prk.expand(info, okm.as_mut_slice())
                .map_err(|_| Into::<ockam_core::Error>::into(VaultError::HkdfExpandError))?;
            okm
        };

        // Prevent dead-lock by freeing entries lock, since we don't need it
        drop(entries);

        let mut secrets = Vec::<Secret>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let length = attributes.length();
            if attributes.stype() == SecretType::Aes {
                if length != AES256_SECRET_LENGTH && length != AES128_SECRET_LENGTH {
                    return Err(VaultError::InvalidAesKeyLength.into());
                }
            } else if attributes.stype() != SecretType::Buffer {
                return Err(VaultError::InvalidHkdfOutputType.into());
            }
            let secret = &okm[index..index + length];
            let secret = self.secret_import(secret, attributes).await?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }
}

#[cfg(test)]
mod tests {
    use crate::Vault;

    fn new_vault() -> Vault {
        Vault::default()
    }

    #[ockam_macros::vault_test]
    fn sha256() {}

    #[ockam_macros::vault_test]
    fn hkdf() {}
}
