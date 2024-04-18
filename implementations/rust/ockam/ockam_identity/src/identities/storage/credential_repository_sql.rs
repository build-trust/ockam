use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::models::{CredentialAndPurposeKey, Identifier};
use crate::{CredentialRepository, TimestampInSeconds};

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
        let query = query_as("SELECT credential, scope FROM credential WHERE node_name=?")
            .bind(self.node_name.to_sql());

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
            "SELECT credential FROM credential WHERE subject_identifier=$1 AND issuer_identifier=$2 AND scope=$3 AND node_name=$4"
            )
            .bind(subject.to_sql())
            .bind(issuer.to_sql())
            .bind(scope.to_sql())
            .bind(self.node_name.to_sql());
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
            "INSERT OR REPLACE INTO credential (subject_identifier, issuer_identifier, scope, credential, expires_at, node_name) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(subject.to_sql())
            .bind(issuer.to_sql())
            .bind(scope.to_sql())
            .bind(credential.encode_as_cbor_bytes()?.to_sql())
            .bind(expires_at.to_sql())
            .bind(self.node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    async fn delete(&self, subject: &Identifier, issuer: &Identifier, scope: &str) -> Result<()> {
        let query = query("DELETE FROM credential WHERE subject_identifier=$1 AND issuer_identifier=$2 AND scope=$3 AND node_name=$4")
            .bind(subject.to_sql())
            .bind(issuer.to_sql())
            .bind(scope.to_sql())
            .bind(self.node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
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
    use std::time::Duration;

    use super::*;
    use crate::identities;
    use crate::models::CredentialSchemaIdentifier;
    use crate::utils::AttributesBuilder;

    #[tokio::test]
    async fn test_cached_credential_repository() -> Result<()> {
        let scope = "test".to_string();
        let repository = Arc::new(CredentialSqlxDatabase::create().await?);

        let all = repository.get_all().await?;
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

        let all = repository.get_all().await?;
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
        let all = repository.get_all().await?;
        assert_eq!(all.len(), 1);
        let credential4 = repository.get(&subject, &issuer, &scope).await?;
        assert_eq!(credential4, Some(credential3));

        repository.delete(&subject, &issuer, &scope).await?;
        let result = repository.get(&subject, &issuer, &scope).await?;
        assert_eq!(result, None);

        Ok(())
    }
}
