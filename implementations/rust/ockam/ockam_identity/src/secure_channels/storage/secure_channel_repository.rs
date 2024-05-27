use crate::secure_channel::Role;
use crate::Identifier;
use async_trait::async_trait;
use core::fmt::{Debug, Formatter};
use ockam_core::compat::boxed::Box;
use ockam_core::{Address, Result};
use ockam_vault::AeadSecret;

/// Secure Channel that was saved to a storage
#[derive(Clone, Eq, PartialEq)]
pub struct PersistedSecureChannel {
    role: Role,
    my_identifier: Identifier,
    their_identifier: Identifier,
    decryptor_remote: Address,
    decryptor_api: Address,
    decryption_key: AeadSecret,
}

impl Debug for PersistedSecureChannel {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("PersistedSecureChannel")
            .field("role", &self.role)
            .field("my_identifier", &self.my_identifier)
            .field("their_identifier", &self.their_identifier)
            .field("decryptor_remote", &self.decryptor_remote)
            .field("decryptor_api", &self.decryptor_api)
            .finish()
    }
}

impl PersistedSecureChannel {
    pub(crate) fn new(
        role: Role,
        my_identifier: Identifier,
        their_identifier: Identifier,
        decryptor_remote: Address,
        decryptor_api: Address,
        decryption_key: AeadSecret,
    ) -> Self {
        Self {
            role,
            my_identifier,
            their_identifier,
            decryptor_remote,
            decryptor_api,
            decryption_key,
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
    pub fn decryption_key(&self) -> &AeadSecret {
        &self.decryption_key
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
