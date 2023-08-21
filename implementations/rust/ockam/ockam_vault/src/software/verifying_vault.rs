use crate::constants::{ED25519_PUBLIC_LENGTH_USIZE, NIST_P256_PUBLIC_LENGTH_USIZE, SHA256_LENGTH};
use crate::{PublicKey, SecretType, Signature, VaultError, VerifyingVault};
use arrayref::array_ref;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Error, Result};
use sha2::{Digest, Sha256};

/// [`VerifyingVault`] implementation using software
#[derive(Debug, Default, Clone)]
pub struct SoftwareVerifyingVault {}

impl SoftwareVerifyingVault {
    /// Constructor
    pub fn new() -> Self {
        Self {}
    }
}

impl SoftwareVerifyingVault {
    fn from_pkcs8<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;

        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }

    fn from_ecdsa(e: p256::ecdsa::Error) -> Error {
        Error::new(Origin::Vault, Kind::Unknown, e)
    }

    fn import_p256_key(public_key: &PublicKey) -> Result<p256::ecdsa::VerifyingKey> {
        if public_key.stype() != SecretType::NistP256 {
            return Err(VaultError::InvalidKeyType.into());
        }

        if public_key.data().len() != NIST_P256_PUBLIC_LENGTH_USIZE {
            return Err(VaultError::InvalidPublicLength.into());
        }

        p256::ecdsa::VerifyingKey::from_sec1_bytes(public_key.data()).map_err(Self::from_pkcs8)
    }

    fn import_ed25519_key(public_key: &PublicKey) -> Result<ed25519_dalek::VerifyingKey> {
        if public_key.stype() != SecretType::Ed25519 {
            return Err(VaultError::InvalidKeyType.into());
        }

        if public_key.data().len() != ED25519_PUBLIC_LENGTH_USIZE {
            return Err(VaultError::InvalidPublicLength.into());
        }

        let public_key = array_ref![public_key.data(), 0, ED25519_PUBLIC_LENGTH_USIZE];
        ed25519_dalek::VerifyingKey::from_bytes(public_key)
            .map_err(|_| VaultError::InvalidPublicKey.into())
    }

    /// Compute SHA256
    pub fn compute_sha256(data: &[u8]) -> Result<[u8; 32]> {
        let digest = Sha256::digest(data);
        if digest.len() != SHA256_LENGTH {
            return Err(VaultError::InvalidSha256Len.into());
        }
        let digest = *array_ref![digest, 0, SHA256_LENGTH];
        Ok(digest)
    }
}

#[async_trait]
impl VerifyingVault for SoftwareVerifyingVault {
    async fn sha256(&self, data: &[u8]) -> Result<[u8; 32]> {
        Self::compute_sha256(data)
    }

    async fn verify(
        &self,
        public_key: &PublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        match public_key.stype() {
            SecretType::Ed25519 => {
                let public_key = Self::import_ed25519_key(public_key)?;

                if signature.as_ref().len() != ed25519_dalek::Signature::BYTE_SIZE {
                    return Err(VaultError::InvalidPublicKey.into());
                }
                let signature_bytes =
                    array_ref![signature.as_ref(), 0, ed25519_dalek::Signature::BYTE_SIZE];
                let signature = ed25519_dalek::Signature::from_bytes(signature_bytes);

                use ed25519_dalek::Verifier;
                Ok(public_key.verify(data.as_ref(), &signature).is_ok())
            }
            SecretType::NistP256 => {
                let public_key = Self::import_p256_key(public_key)?;

                let signature = p256::ecdsa::Signature::from_slice(signature.as_ref())
                    .map_err(Self::from_ecdsa)?;

                use p256::ecdsa::signature::Verifier;
                Ok(public_key.verify(data, &signature).is_ok())
            }
            SecretType::Buffer | SecretType::Aes | SecretType::X25519 => {
                Err(VaultError::InvalidPublicKey.into())
            }
        }
    }
}
