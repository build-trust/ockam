use sqlx::*;

use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey};
use ockam::identity::{Identifier, Identity};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::cli_state::{CredentialsRepository, NamedCredential};

#[derive(Clone)]
pub struct CredentialsSqlxDatabase {
    database: SqlxDatabase,
}

impl CredentialsSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for credentials");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("credentials").await?))
    }
}

#[async_trait]
impl CredentialsRepository for CredentialsSqlxDatabase {
    async fn store_credential(
        &self,
        name: &str,
        issuer: &Identity,
        credential: CredentialAndPurposeKey,
    ) -> Result<NamedCredential> {
        let query = query("INSERT OR REPLACE INTO credential VALUES (?, ?, ?, ?)")
            .bind(name.to_sql())
            .bind(issuer.identifier().to_sql())
            .bind(issuer.change_history().to_sql())
            .bind(CredentialAndPurposeKeySql(credential.clone()).to_sql());
        query.execute(&*self.database.pool).await.void()?;
        Ok(NamedCredential::new(name, issuer, credential))
    }

    async fn get_credential(&self, name: &str) -> Result<Option<NamedCredential>> {
        let query = query_as("SELECT name, issuer_identifier, issuer_change_history, credential FROM credential WHERE name=$1").bind(name.to_sql());
        let row: Option<CredentialRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_credential()).transpose()
    }

    async fn get_credentials(&self) -> Result<Vec<NamedCredential>> {
        let query = query_as(
            "SELECT name, issuer_identifier, issuer_change_history, credential FROM credential",
        );
        let row: Vec<CredentialRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.iter().map(|r| r.named_credential()).collect()
    }
}

// Database serialization / deserialization

pub struct CredentialAndPurposeKeySql(pub CredentialAndPurposeKey);

impl ToSqlxType for CredentialAndPurposeKeySql {
    fn to_sql(&self) -> SqlxType {
        self.0.encode_as_string().unwrap().to_sql()
    }
}

/// Low-level representation of a row in the credentials table
#[derive(sqlx::FromRow)]
struct CredentialRow {
    name: String,
    issuer_identifier: String,
    issuer_change_history: String,
    credential: String,
}

impl CredentialRow {
    pub(crate) fn named_credential(&self) -> Result<NamedCredential> {
        Ok(NamedCredential::make(
            &self.name,
            self.issuer_identifier()?,
            self.change_history()?,
            self.credential()?,
        ))
    }

    pub(crate) fn issuer_identifier(&self) -> Result<Identifier> {
        self.issuer_identifier.clone().try_into()
    }

    pub(crate) fn change_history(&self) -> Result<ChangeHistory> {
        ChangeHistory::import_from_string(&self.issuer_change_history)
    }

    pub(crate) fn credential(&self) -> Result<CredentialAndPurposeKey> {
        CredentialAndPurposeKey::decode_from_string(&self.credential)
    }
}

#[cfg(test)]
mod tests {
    use ockam::identity::models::CredentialSchemaIdentifier;
    use ockam::identity::utils::AttributesBuilder;
    use ockam::identity::{identities, Identities};
    use std::sync::Arc;
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_credentials_repository() -> Result<()> {
        let repository = create_repository().await?;
        let identities = identities().await?;

        // a credential can be stored under a name
        let issuer_identity = identities.identities_creation().create_identity().await?;
        let issuer = identities.get_identity(&issuer_identity).await?;
        let credential = create_credential(identities.clone(), &issuer_identity).await?;
        let named_credential1 = repository
            .store_credential("name", &issuer, credential.clone())
            .await?;

        // That credential can be retrieved by name
        let result = repository.get_credential("name").await?;
        assert_eq!(
            result,
            Some(NamedCredential::new("name", &issuer, credential))
        );

        // All the credentials can be retrieved at once
        let credential = create_credential(identities, &issuer_identity).await?;
        let named_credential2 = repository
            .store_credential("name2", &issuer, credential.clone())
            .await?;
        let result = repository.get_credentials().await?;
        assert_eq!(result, vec![named_credential1, named_credential2]);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn CredentialsRepository>> {
        Ok(Arc::new(CredentialsSqlxDatabase::create().await?))
    }

    async fn create_credential(
        identities: Arc<Identities>,
        issuer: &Identifier,
    ) -> Result<CredentialAndPurposeKey> {
        let subject = identities.identities_creation().create_identity().await?;

        let attributes = AttributesBuilder::with_schema(CredentialSchemaIdentifier(1))
            .with_attribute("name".as_bytes().to_vec(), b"value".to_vec())
            .build();

        identities
            .credentials()
            .credentials_creation()
            .issue_credential(issuer, &subject, attributes, Duration::from_secs(1))
            .await
    }
}
