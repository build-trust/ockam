use sqlx::*;
use std::sync::Arc;
use tracing::debug;

use crate::secure_channel::Role;
use crate::Identifier;
use ockam_core::{async_trait, Address};
use ockam_core::{Error, Result};
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};
use ockam_vault::{AeadSecretKeyHandle, HandleToSecret};

use crate::secure_channels::storage::secure_channel_repository::{
    PersistedSecureChannel, SecureChannelRepository,
};

/// Implementation of `CredentialRepository` trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct SecureChannelSqlxDatabase {
    database: SqlxDatabase,
}

impl SecureChannelSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for secure channels");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn SecureChannelRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("secure_channel").await?))
    }
}

#[async_trait]
impl SecureChannelRepository for SecureChannelSqlxDatabase {
    async fn get(
        &self,
        decryptor_remote_address: &Address,
    ) -> Result<Option<PersistedSecureChannel>> {
        let query = query_as(
            "SELECT role, my_identifier, their_identifier, decryptor_remote_address, decryptor_api_address, decryption_key_handle FROM secure_channel WHERE decryptor_remote_address = $1"
            )
            .bind(decryptor_remote_address.to_string());
        let secure_channel: Option<SecureChannelRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;

        Ok(secure_channel.map(TryInto::try_into).transpose()?)
    }

    async fn put(&self, secure_channel: PersistedSecureChannel) -> Result<()> {
        let query = query(
            r#"INSERT INTO secure_channel (role, my_identifier, their_identifier, decryptor_remote_address, decryptor_api_address, decryption_key_handle)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (decryptor_remote_address)
            DO UPDATE SET role = $1, my_identifier = $2, their_identifier = $3, decryptor_api_address = $5, decryption_key_handle = $6"#
            )
            .bind(secure_channel.role().str())
            .bind(secure_channel.my_identifier())
            .bind(secure_channel.their_identifier())
            .bind(secure_channel.decryptor_remote().to_string())
            .bind(secure_channel.decryptor_api().to_string())
            .bind(secure_channel.decryption_key_handle());
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete(&self, decryptor_remote_address: &Address) -> Result<()> {
        let query = query("DELETE FROM secure_channel WHERE decryptor_remote_address = $1")
            .bind(decryptor_remote_address.to_string());
        query.execute(&*self.database.pool).await.void()
    }
}

// Low-level representation of a table row
#[derive(FromRow)]
struct SecureChannelRow {
    role: String,
    my_identifier: String,
    their_identifier: String,
    decryptor_remote_address: String,
    decryptor_api_address: String,
    decryption_key_handle: Vec<u8>,
}

impl TryFrom<SecureChannelRow> for PersistedSecureChannel {
    type Error = Error;

    fn try_from(value: SecureChannelRow) -> std::result::Result<Self, Self::Error> {
        let role = Role::try_from(value.role.as_str())?;
        let my_identifier = Identifier::try_from(value.my_identifier)?;
        let their_identifier = Identifier::try_from(value.their_identifier)?;
        let decryptor_remote_address = Address::from_string(value.decryptor_remote_address);
        let decryptor_api_address = Address::from_string(value.decryptor_api_address);
        let decryption_key_handle = HandleToSecret::new(value.decryption_key_handle);
        let decryption_key_handle = AeadSecretKeyHandle::new(decryption_key_handle);

        Ok(PersistedSecureChannel::new(
            role,
            my_identifier,
            their_identifier,
            decryptor_remote_address,
            decryptor_api_address,
            decryption_key_handle,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::secure_channel::Role;
    use ockam_core::compat::rand::RngCore;
    use ockam_core::compat::sync::Arc;
    use rand::thread_rng;

    use super::*;

    #[tokio::test]
    async fn test_secure_channel_repository() -> Result<()> {
        let repository = Arc::new(SecureChannelSqlxDatabase::create().await?);

        let decryptor_remote = Address::random_local();
        let decryptor_api = Address::random_local();

        let mut decryption_key_handle = [0u8; 32];

        let mut rng = thread_rng();
        rng.fill_bytes(&mut decryption_key_handle);

        let decryption_key_handle =
            AeadSecretKeyHandle::new(HandleToSecret::new(decryption_key_handle.to_vec()));

        let my_identifier = Identifier::try_from(
            "Ie70dc5545d64724880257acb32b8851e7dd1dd57076838991bc343165df71bfe",
        )?;
        let their_identifier = Identifier::try_from(
            "Ife42b412ecdb7fda4421bd5046e33c1017671ce7a320c3342814f0b99df9ab60",
        )?;

        let sc = PersistedSecureChannel::new(
            Role::Initiator,
            my_identifier,
            their_identifier,
            decryptor_remote.clone(),
            decryptor_api,
            decryption_key_handle,
        );

        repository.put(sc.clone()).await?;

        let sc2 = repository.get(&decryptor_remote).await?;
        assert_eq!(sc2, Some(sc));

        repository.delete(&decryptor_remote).await?;
        let result = repository.get(&decryptor_remote).await?;
        assert_eq!(result, None);

        Ok(())
    }
}
