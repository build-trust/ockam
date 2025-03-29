use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::*;
use sqlx_core::any::AnyArgumentBuffer;
use std::sync::Arc;
use tracing::debug;

use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::{CredentialRepository, TimestampInSeconds};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

/// Implementation of `CredentialRepository` trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct CredentialSqlxDatabase {
    database: SqlxDatabase,
    node_name: String,
}

impl CredentialSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase, node_name: &str) -> Self {
        debug!("create a repository for credentials");
        Self {
            database,
            node_name: node_name.to_string(),
        }
    }

    /// Create a repository
    pub fn make_repository(
        database: SqlxDatabase,
        node_name: &str,
    ) -> Arc<dyn CredentialRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database, node_name)))
        } else {
            Arc::new(Self::new(database, node_name))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("credential").await?,
            "default",
        ))
    }
}

impl CredentialSqlxDatabase {
    /// Return all cached credentials for the given node
    pub async fn get_all(&self) -> Result<Vec<(CredentialAndPurposeKey, String)>> {
        let query = query_as("SELECT credential, scope FROM credential WHERE node_name = $1")
            .bind(self.node_name.clone());

        let cached_credential: Vec<CachedCredentialAndScopeRow> =
            query.fetch_all(&*self.database.pool).await.into_core()?;

        let res = cached_credential
            .into_iter()
            .map(|c| {
                let cred = c.credential()?;
                Ok((cred, c.scope().to_string()))
            })
            .collect::<Result<Vec<_>>>()?;

        Ok(res)
    }
}

#[async_trait]
impl CredentialRepository for CredentialSqlxDatabase {
    async fn get(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
    ) -> Result<Option<CredentialAndPurposeKey>> {
        let query = query_as(
            "SELECT credential FROM credential WHERE subject_identifier = $1 AND issuer_identifier = $2 AND scope = $3 AND node_name = $4"
            )
            .bind(subject)
            .bind(issuer)
            .bind(scope)
            .bind(self.node_name.clone());
        let cached_credential: Option<CachedCredentialRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        cached_credential.map(|c| c.credential()).transpose()
    }

    async fn put(
        &self,
        subject: &Identifier,
        issuer: &Identifier,
        scope: &str,
        expires_at: TimestampInSeconds,
        credential: CredentialAndPurposeKey,
    ) -> Result<()> {
        let query = query(
            r#"INSERT INTO credential (subject_identifier, issuer_identifier, scope, credential, expires_at, node_name)
            VALUES ($1, $2, $3, $4, $5, $6)
            ON CONFLICT (subject_identifier, issuer_identifier, scope)
            DO UPDATE SET credential = $4, expires_at = $5, node_name = $6"#)
            .bind(subject)
            .bind(issuer)
            .bind(scope)
            .bind(credential)
            .bind(expires_at)
            .bind(self.node_name.clone());
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete(&self, subject: &Identifier, issuer: &Identifier, scope: &str) -> Result<()> {
        let query = query("DELETE FROM credential WHERE subject_identifier = $1 AND issuer_identifier = $2 AND scope = $3 AND node_name = $4")
            .bind(subject)
            .bind(issuer)
            .bind(scope)
            .bind(self.node_name.clone());
        query.execute(&*self.database.pool).await.void()
    }
}

// Database serialization / deserialization

impl Type<Any> for CredentialAndPurposeKey {
    fn type_info() -> <Any as Database>::TypeInfo {
        <Vec<u8> as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for CredentialAndPurposeKey {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <Vec<u8> as Encode<'_, Any>>::encode_by_ref(&self.encode_as_cbor_bytes().unwrap(), buf)
    }
}

impl Type<Any> for TimestampInSeconds {
    fn type_info() -> <Any as Database>::TypeInfo {
        <i64 as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for TimestampInSeconds {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <i64 as Encode<'_, Any>>::encode_by_ref(&(self.0 as i64), buf)
    }
}

// Low-level representation of a table row
#[derive(FromRow)]
struct CachedCredentialRow {
    credential: Vec<u8>,
}

impl CachedCredentialRow {
    fn credential(&self) -> Result<CredentialAndPurposeKey> {
        CredentialAndPurposeKey::decode_from_cbor_bytes(&self.credential)
    }
}

#[derive(FromRow)]
struct CachedCredentialAndScopeRow {
    credential: Vec<u8>,
    scope: String,
}

impl CachedCredentialAndScopeRow {
    fn credential(&self) -> Result<CredentialAndPurposeKey> {
        CredentialAndPurposeKey::decode_from_cbor_bytes(&self.credential)
    }
    pub fn scope(&self) -> &str {
        &self.scope
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::sync::Arc;
    use ockam_node::database::with_dbs;
    use std::time::Duration;

    use super::*;
    use crate::identities;
    use crate::models::CredentialSchemaIdentifier;
    use crate::utils::AttributesBuilder;

    #[tokio::test]
    async fn test_cached_credential_repository() -> Result<()> {
        with_dbs(|db| async move {
            let credentials_database = CredentialSqlxDatabase::new(db, "node");
            let repository: Arc<dyn CredentialRepository> = Arc::new(credentials_database.clone());

            let scope = "test".to_string();

            let all = credentials_database.get_all().await?;
            assert_eq!(all.len(), 0);

            let identities = identities().await?;

            let issuer = identities.identities_creation().create_identity().await?;
            let subject = identities.identities_creation().create_identity().await?;

            let attributes1 = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
                .with_attribute("key1", "value1")
                .build();
            let credential1 = identities
                .credentials()
                .credentials_creation()
                .issue_credential(&issuer, &subject, attributes1, Duration::from_secs(60 * 60))
                .await?;

            repository
                .put(
                    &subject,
                    &issuer,
                    &scope,
                    credential1.get_credential_data()?.expires_at,
                    credential1.clone(),
                )
                .await?;

            let all = credentials_database.get_all().await?;
            assert_eq!(all.len(), 1);

            let credential2 = repository.get(&subject, &issuer, &scope).await?;
            assert_eq!(credential2, Some(credential1));

            let attributes2 = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
                .with_attribute("key2", "value2")
                .build();
            let credential3 = identities
                .credentials()
                .credentials_creation()
                .issue_credential(&issuer, &subject, attributes2, Duration::from_secs(60 * 60))
                .await?;
            repository
                .put(
                    &subject,
                    &issuer,
                    &scope,
                    credential3.get_credential_data()?.expires_at,
                    credential3.clone(),
                )
                .await?;
            let all = credentials_database.get_all().await?;
            assert_eq!(all.len(), 1);
            let credential4 = repository.get(&subject, &issuer, &scope).await?;
            assert_eq!(credential4, Some(credential3));

            repository.delete(&subject, &issuer, &scope).await?;
            let result = repository.get(&subject, &issuer, &scope).await?;
            assert_eq!(result, None);

            Ok(())
        })
        .await
    }
}
