use crate::{
    ECDSASHA256CurveP256PublicKey, EdDSACurve25519PublicKey, Sha256Output, Signature, VaultError,
    VaultForVerifyingSignatures, VerifyingPublicKey,
};

use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::{async_trait, compat::boxed::Box, Error, Result};

use sha2::{Digest, Sha256};

/// [`VaultForSigning`] implementation using software
#[derive(Debug, Default, Clone)]
pub struct SoftwareVaultForVerifyingSignatures {}

impl SoftwareVaultForVerifyingSignatures {
    /// Constructor
    pub fn new() -> Self {
        Self {}
    }

    /// Create Software implementation Vault
    pub fn create() -> Arc<SoftwareVaultForVerifyingSignatures> {
        Arc::new(Self::new())
    }
}

#[async_trait]
impl VaultForVerifyingSignatures for SoftwareVaultForVerifyingSignatures {
    async fn sha256(&self, data: &[u8]) -> Result<Sha256Output> {
        Self::compute_sha256(data)
    }

    async fn verify_signature(
        &self,
        verifying_public_key: &VerifyingPublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        self.verify_signature_sync(verifying_public_key, data, signature)
    }
}

impl SoftwareVaultForVerifyingSignatures {
    fn from_pkcs8<T: core::fmt::Display>(e: T) -> Error {
        #[cfg(feature = "no_std")]
        use ockam_core::compat::string::ToString;

        Error::new(Origin::Vault, Kind::Unknown, e.to_string())
    }

    fn from_ecdsa(e: p256::ecdsa::Error) -> Error {
        Error::new(Origin::Vault, Kind::Unknown, e)
    }

    fn import_p256_key(
        public_key: &ECDSASHA256CurveP256PublicKey,
    ) -> Result<p256::ecdsa::VerifyingKey> {
        p256::ecdsa::VerifyingKey::from_sec1_bytes(&public_key.0).map_err(Self::from_pkcs8)
    }

    fn import_ed25519_key(
        public_key: &EdDSACurve25519PublicKey,
    ) -> Result<ed25519_dalek::VerifyingKey> {
        ed25519_dalek::VerifyingKey::from_bytes(&public_key.0)
            .map_err(|_| VaultError::InvalidPublicKey.into())
    }

    /// Compute SHA256
    pub fn compute_sha256(data: &[u8]) -> Result<Sha256Output> {
        let digest = Sha256::digest(data);
        Ok(Sha256Output(digest.into()))
    }
}

impl SoftwareVaultForVerifyingSignatures {
    /// Verify a signature
    fn verify_signature_sync(
        &self,
        verifying_public_key: &VerifyingPublicKey,
        data: &[u8],
        signature: &Signature,
    ) -> Result<bool> {
        match (verifying_public_key, signature) {
            (
                VerifyingPublicKey::EdDSACurve25519(verifying_public_key),
                Signature::EdDSACurve25519(signature),
            ) => {
                let verifying_public_key = Self::import_ed25519_key(verifying_public_key)?;

                let signature = ed25519_dalek::Signature::from_bytes(&signature.0);

                use ed25519_dalek::Verifier;
                Ok(verifying_public_key
                    .verify(data.as_ref(), &signature)
                    .is_ok())
            }
            (
                VerifyingPublicKey::ECDSASHA256CurveP256(verifying_public_key),
                Signature::ECDSASHA256CurveP256(signature),
            ) => {
                let verifying_public_key = Self::import_p256_key(verifying_public_key)?;

                let signature =
                    p256::ecdsa::Signature::from_slice(&signature.0).map_err(Self::from_ecdsa)?;

                use p256::ecdsa::signature::Verifier;
                Ok(verifying_public_key.verify(data, &signature).is_ok())
            }
            _ => Err(VaultError::SignatureAndPublicKeyTypesDontMatch.into()),
        }
    }
}
