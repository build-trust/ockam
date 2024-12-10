use crate::{
    AeadSecret, AeadSecretKeyHandle, SigningSecret, SigningSecretKeyHandle, X25519SecretKey,
    X25519SecretKeyHandle,
};
use ockam_core::async_trait;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::vec::Vec;
use ockam_core::Result;

#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;

#[cfg(feature = "std")]
use ockam_node::retry;

/// A secrets repository supports the persistence of signing and X25519 secrets
#[async_trait]
pub trait SecretsRepository: Send + Sync + 'static {
    /// Store a signing secret
    async fn store_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
        secret: SigningSecret,
    ) -> Result<()>;

    /// Delete a signing secret
    async fn delete_signing_secret(&self, handle: &SigningSecretKeyHandle) -> Result<bool>;

    /// Get a signing secret
    async fn get_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> Result<Option<SigningSecret>>;

    /// Get the list of all signing secret handles
    async fn get_signing_secret_handles(&self) -> Result<Vec<SigningSecretKeyHandle>>;

    /// Get a X25519 secret
    async fn store_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
        secret: X25519SecretKey,
    ) -> Result<()>;

    /// Get a X25519 secret
    async fn delete_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<bool>;

    /// Get a X25519 secret
    async fn get_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> Result<Option<X25519SecretKey>>;

    /// Get the list of all X25519 secret handles
    async fn get_x25519_secret_handles(&self) -> Result<Vec<X25519SecretKeyHandle>>;

    /// Store AEAD secret.
    async fn store_aead_secret(
        &self,
        handle: &AeadSecretKeyHandle,
        secret: AeadSecret,
    ) -> Result<()>;

    /// Delete AEAD secret.
    async fn delete_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<bool>;

    /// Get AEAD secret.
    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<Option<AeadSecret>>;

    /// Delete all secrets
    async fn delete_all(&self) -> Result<()>;
}

#[cfg(feature = "std")]
#[async_trait]
impl<T: SecretsRepository> SecretsRepository for AutoRetry<T> {
    async fn store_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
        secret: SigningSecret,
    ) -> Result<()> {
        retry!(self.wrapped.store_signing_secret(handle, secret.clone()))
    }

    async fn delete_signing_secret(&self, handle: &SigningSecretKeyHandle) -> Result<bool> {
        retry!(self.wrapped.delete_signing_secret(handle))
    }

    async fn get_signing_secret(
        &self,
        handle: &SigningSecretKeyHandle,
    ) -> Result<Option<SigningSecret>> {
        retry!(self.wrapped.get_signing_secret(handle))
    }

    async fn get_signing_secret_handles(&self) -> Result<Vec<SigningSecretKeyHandle>> {
        retry!(self.wrapped.get_signing_secret_handles())
    }

    async fn store_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
        secret: X25519SecretKey,
    ) -> Result<()> {
        retry!(self.wrapped.store_x25519_secret(handle, secret.clone()))
    }

    async fn delete_x25519_secret(&self, handle: &X25519SecretKeyHandle) -> Result<bool> {
        retry!(self.wrapped.delete_x25519_secret(handle))
    }

    async fn get_x25519_secret(
        &self,
        handle: &X25519SecretKeyHandle,
    ) -> Result<Option<X25519SecretKey>> {
        retry!(self.wrapped.get_x25519_secret(handle))
    }

    async fn get_x25519_secret_handles(&self) -> Result<Vec<X25519SecretKeyHandle>> {
        retry!(self.wrapped.get_x25519_secret_handles())
    }

    async fn store_aead_secret(
        &self,
        handle: &AeadSecretKeyHandle,
        secret: AeadSecret,
    ) -> Result<()> {
        retry!(self.wrapped.store_aead_secret(handle, secret.clone()))
    }

    async fn delete_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<bool> {
        retry!(self.wrapped.delete_aead_secret(handle))
    }

    async fn get_aead_secret(&self, handle: &AeadSecretKeyHandle) -> Result<Option<AeadSecret>> {
        retry!(self.wrapped.get_aead_secret(handle))
    }

    async fn delete_all(&self) -> Result<()> {
        retry!(self.wrapped.delete_all())
    }
}
