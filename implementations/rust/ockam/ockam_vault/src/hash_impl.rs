use crate::error::Error;
use crate::software_vault::SoftwareVault;
use arrayref::array_ref;
use ockam_vault_core::hash_vault::HashVault;
use ockam_vault_core::secret::Secret;
use ockam_vault_core::secret_vault::SecretVault;
use ockam_vault_core::types::{
    SecretAttributes, SecretType, AES128_SECRET_LENGTH, AES256_SECRET_LENGTH,
};
use sha2::{Digest, Sha256};

impl SoftwareVault {
    fn hkdf_sha256_internal(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: &[u8],
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Secret>, ockam_core::Error> {
        let salt = self.get_entry(salt)?;

        // FIXME: Doesn't work for secrets with size more than 32 bytes
        let okm_len = output_attributes.len() * 32;

        let okm = {
            let mut okm = vec![0u8; okm_len];
            let prk = hkdf::Hkdf::<Sha256>::new(Some(salt.key().as_ref()), ikm);
            prk.expand(info, okm.as_mut_slice())
                .or(Err(Error::HkdfExpandError.into()))?;
            okm
        };

        let mut secrets = Vec::<Secret>::new();
        let mut index = 0;

        for attributes in output_attributes {
            let length = attributes.length;
            if attributes.stype == SecretType::Aes {
                if length != AES256_SECRET_LENGTH && length != AES128_SECRET_LENGTH {
                    return Err(Error::InvalidAesKeyLength.into());
                }
            } else if attributes.stype != SecretType::Buffer {
                return Err(Error::InvalidHkdfOutputType.into());
            }
            let secret = &okm[index..index + length];
            let secret = self.secret_import(&secret, attributes)?;

            secrets.push(secret);
            index += 32;
        }

        Ok(secrets)
    }
}

impl HashVault for SoftwareVault {
    fn sha256(&self, data: &[u8]) -> Result<[u8; 32], ockam_core::Error> {
        let digest = Sha256::digest(data);
        Ok(*array_ref![digest, 0, 32])
    }

    fn hkdf_sha256(
        &mut self,
        salt: &Secret,
        info: &[u8],
        ikm: Option<&Secret>,
        output_attributes: Vec<SecretAttributes>,
    ) -> Result<Vec<Secret>, ockam_core::Error> {
        let ikm_slice = match ikm {
            Some(ikm) => {
                let ikm = self.get_entry(ikm)?;
                if ikm.key_attributes().stype == SecretType::Buffer {
                    Ok(ikm.key().as_ref().to_vec())
                } else {
                    Err(Error::InvalidKeyType.into())
                }
            }
            None => Ok(Vec::new()),
        }?;

        self.hkdf_sha256_internal(salt, info, &ikm_slice, output_attributes)
    }
}
