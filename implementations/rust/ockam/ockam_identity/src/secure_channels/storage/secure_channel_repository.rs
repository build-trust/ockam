use crate::secure_channel::Role;
use crate::Identifier;
use async_trait::async_trait;
use core::fmt::Debug;
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Result};
#[cfg(feature = "std")]
use ockam_node::database::AutoRetry;
#[cfg(feature = "std")]
use ockam_node::retry;
use ockam_vault::AeadSecretKeyHandle;

/// Secure Channel that was saved to a storage
#[derive(Clone, Eq, Debug, PartialEq)]
pub struct PersistedSecureChannel {
    role: Role,
    my_identifier: Identifier,
    their_identifier: Identifier,
    decryptor_remote: Address,
    decryptor_api: Address,
    decryption_key_handle: AeadSecretKeyHandle,
}

impl PersistedSecureChannel {
    pub(crate) fn new(
        role: Role,
        my_identifier: Identifier,
        their_identifier: Identifier,
        decryptor_remote: Address,
        decryptor_api: Address,
        decryption_key_handle: AeadSecretKeyHandle,
    ) -> Self {
        Self {
            role,
            my_identifier,
            their_identifier,
            decryptor_remote,
            decryptor_api,
            decryption_key_handle,
        }
    }

    /// Role
    pub fn role(&self) -> Role {
        self.role
    }

    /// My identifier
    pub fn my_identifier(&self) -> &Identifier {
        &self.my_identifier
    }

    /// Their identifier
    pub fn their_identifier(&self) -> &Identifier {
        &self.their_identifier
    }

    /// Decryptor remote address. See [`Addresses`]
    pub fn decryptor_remote(&self) -> &Address {
        &self.decryptor_remote
    }

    /// Decryptor api address. See [`Addresses`]
    pub fn decryptor_api(&self) -> &Address {
        &self.decryptor_api
    }

    /// Decryption key
    pub fn decryption_key_handle(&self) -> &AeadSecretKeyHandle {
        &self.decryption_key_handle
    }
}

/// Repository for persisted Secure Channels
#[async_trait]
pub trait SecureChannelRepository: Send + Sync + 'static {
    /// Get a previously persisted secure channel with given decryptor remote address
    async fn get(
        &self,
        decryptor_remote_address: &Address,
    ) -> Result<Option<PersistedSecureChannel>>;

    /// Store a secure channel
    async fn put(&self, secure_channel: PersistedSecureChannel) -> Result<()>;

    /// Delete a secure channel
    async fn delete(&self, decryptor_remote_address: &Address) -> Result<()>;
}

#[cfg(feature = "std")]
#[async_trait]
impl<T: SecureChannelRepository> SecureChannelRepository for AutoRetry<T> {
    async fn get(
        &self,
        decryptor_remote_address: &Address,
    ) -> Result<Option<PersistedSecureChannel>> {
        retry!(self.wrapped.get(decryptor_remote_address))
    }

    async fn put(&self, secure_channel: PersistedSecureChannel) -> Result<()> {
        retry!(self.wrapped.put(secure_channel.clone()))
    }

    async fn delete(&self, decryptor_remote_address: &Address) -> Result<()> {
        retry!(self.wrapped.delete(decryptor_remote_address))
    }
}
