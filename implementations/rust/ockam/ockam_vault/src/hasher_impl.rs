use crate::software_vault::SoftwareVault;
use crate::VaultError;
use arrayref::array_ref;
use ockam_core::compat::{boxed::Box, vec::Vec};
use ockam_vault_core::{
    Hasher, Secret, SecretAttributes, SecretType, SecretVault, AES128_SECRET_LENGTH,
    AES256_SECRET_LENGTH,
};
use sha2::{Digest, Sha256};

use ockam_core::async_trait::async_trait;
#[async_trait]
impl Hasher for SoftwareVault {
    fn sha256(&mut self, data: &[u8]) -> ockam_core::Result<[u8; 32]> {
        let digest = Sha256::digest(data);
        Ok(*array_ref![digest, 0, 32])
    }

    async fn async_sha256(&mut self, data: &[u8]) -> ockam_core::Result<[u8; 32]> {
        let digest = Sha256::digest(data);
        Ok(*array_ref![digest, 0, 32])
    }

    /// Compute sha256.
    /// Salt and Ikm should be of Buffer type.
    /// Output secrets should be only of type Buffer or AES
    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> ockam_core::Result<Vec<Secret>> {
        let ikm: ockam_core::Result<&[u8]> = match ikm {
            Some(ikm) => {
                let ikm = self.get_entry(ikm)?;
                if ikm.key_attributes().stype() == SecretType::Buffer {
                    Ok(ikm.key().as_ref())
                } else {
                    Err(VaultError::InvalidKeyType.into())
                }
            }
            None => Ok(&[0u8; 0]),
        };

        let ikm = ikm?;

        let salt = self.get_entry(salt)?;

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
            let secret = self.secret_import(secret, attributes)?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }

    /// Compute sha256.
    /// Salt and Ikm should be of Buffer type.
    /// Output secrets should be only of type Buffer or AES
    async fn async_hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> ockam_core::Result<Vec<Secret>> {
        self.hkdf_sha256(salt, info, ikm, output_attributes)
    }
}

#[cfg(test)]
mod tests {
    use crate::SoftwareVault;
    use ockam_vault_test_attribute::*;

    fn new_vault() -> SoftwareVault {
        SoftwareVault::default()
    }

    #[vault_test]
    fn sha256() {}

    #[vault_test]
    fn hkdf() {}
}
