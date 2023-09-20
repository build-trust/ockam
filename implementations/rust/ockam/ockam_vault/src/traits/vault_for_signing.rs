use crate::{Signature, SigningKeyType, SigningSecretKeyHandle, VerifyingPublicKey};

use ockam_core::{async_trait, compat::boxed::Box, Result};

/// Vault for signing data.
#[async_trait]
pub trait VaultForSigning: Send + Sync + 'static {
    /// Sign data.
    async fn sign(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
        data: &[u8],
    ) -> Result<Signature>;

    /// Generate a fresh random Signing Secret Key and return the Handle to it.
    async fn generate_signing_secret_key(
        &self,
        signing_key_type: SigningKeyType,
    ) -> Result<SigningSecretKeyHandle>;

    /// Get [`VerifyingPublicKey`] corresponding to a Signing Secret Key given its Handle.
    async fn get_verifying_public_key(
        &self,
        signing_secret_key_handle: &SigningSecretKeyHandle,
    ) -> Result<VerifyingPublicKey>;

    /// Get [`SigningSecretKeyHandle`] to a Signing Secret Key given its [`VerifyingPublicKey`].
    async fn get_secret_key_handle(
        &self,
        verifying_public_key: &VerifyingPublicKey,
    ) -> Result<SigningSecretKeyHandle>;

    /// Delete Signing Secret Key given its Handle.
    async fn delete_signing_secret_key(
        &self,
        signing_secret_key_handle: SigningSecretKeyHandle,
    ) -> Result<bool>;
}
