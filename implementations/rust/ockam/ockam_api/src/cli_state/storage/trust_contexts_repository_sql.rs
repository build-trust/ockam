use std::sync::Arc;

use sqlx::*;

use ockam::identity::models::{ChangeHistory, CredentialAndPurposeKey};
use ockam::identity::SecureChannels;
use ockam::identity::TrustContext;
use ockam_core::async_trait;
use ockam_core::env::FromString;
use ockam_core::Result;
use ockam_multiaddr::MultiAddr;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_transport_tcp::TcpTransport;

use crate::cli_state::storage::CredentialAndPurposeKeySql;
use crate::cli_state::storage::TrustContextsRepository;
use crate::NamedTrustContext;

#[derive(Clone)]
pub struct TrustContextsSqlxDatabase {
    database: SqlxDatabase,
}

impl TrustContextsSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for trust contexts");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("trust contexts").await?))
    }
}

#[async_trait]
impl TrustContextsRepository for TrustContextsSqlxDatabase {
    async fn store_trust_context(&self, trust_context: &NamedTrustContext) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query_scalar(
            "SELECT EXISTS(SELECT 1 FROM trust_context WHERE is_default=$1 AND name=$2)",
        )
        .bind(true.to_sql())
        .bind(trust_context.name().to_sql());
        let is_already_default: bool = query1.fetch_one(&mut *transaction).await.into_core()?;

        let query2 = query("INSERT OR REPLACE INTO trust_context VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(trust_context.name().to_sql())
            .bind(trust_context.trust_context_id().to_sql())
            .bind(is_already_default.to_sql())
            .bind(
                trust_context
                    .credential()
                    .as_ref()
                    .map(|c| CredentialAndPurposeKeySql(c.clone()).to_sql()),
            )
            .bind(
                trust_context
                    .authority_change_history()
                    .as_ref()
                    .map(|c| c.to_sql()),
            )
            .bind(
                trust_context
                    .authority_route()
                    .as_ref()
                    .map(|r| r.to_string().to_sql()),
            );
        query2.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()
    }

    async fn get_default_trust_context(&self) -> Result<Option<NamedTrustContext>> {
        let query = query_as("SELECT name, trust_context_id, is_default, credential, authority_change_history, authority_route FROM trust_context WHERE is_default=$1").bind(true.to_sql());
        let row: Option<NamedTrustContextRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_trust_context()).transpose()
    }

    async fn set_default_trust_context(&self, name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the trust context as the default one
        let query1 = query("UPDATE trust_context SET is_default = ? WHERE name = ?")
            .bind(true.to_sql())
            .bind(name.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE trust_context SET is_default = ? WHERE name <> ?")
            .bind(false.to_sql())
            .bind(name.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn get_trust_context(&self, name: &str) -> Result<Option<NamedTrustContext>> {
        let query = query_as("SELECT name, trust_context_id, is_default, credential, authority_change_history, authority_route FROM trust_context WHERE name=$1").bind(name.to_sql());
        let row: Option<NamedTrustContextRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|u| u.named_trust_context()).transpose()
    }

    async fn get_trust_contexts(&self) -> Result<Vec<NamedTrustContext>> {
        let query = query_as("SELECT name, trust_context_id, is_default, credential, authority_change_history, authority_route FROM trust_context");
        let rows: Vec<NamedTrustContextRow> =
            query.fetch_all(&*self.database.pool).await.into_core()?;
        rows.iter().map(|u| u.named_trust_context()).collect()
    }

    async fn delete_trust_context(&self, name: &str) -> Result<()> {
        let query1 = query("DELETE FROM trust_context WHERE name=?").bind(name.to_sql());
        query1.execute(&*self.database.pool).await.void()
    }
}

// Database serialization / deserialization

#[derive(sqlx::FromRow)]
struct NamedTrustContextRow {
    name: String,
    trust_context_id: String,
    #[allow(unused)]
    is_default: bool,
    credential: Option<String>,
    authority_change_history: Option<String>,
    authority_route: Option<String>,
}

impl NamedTrustContextRow {
    fn named_trust_context(&self) -> Result<NamedTrustContext> {
        let credential: Option<CredentialAndPurposeKey> = self
            .credential
            .as_ref()
            .map(|c| CredentialAndPurposeKey::decode_from_string(c))
            .transpose()?;
        let authority_change_history = self
            .authority_change_history
            .as_ref()
            .map(|i| ChangeHistory::import_from_string(i))
            .transpose()?;
        let authority_route = self
            .authority_route
            .as_ref()
            .map(|r| MultiAddr::from_string(r))
            .transpose()?;
        Ok(NamedTrustContext::new(
            &self.name,
            &self.trust_context_id,
            credential,
            authority_change_history,
            authority_route,
        ))
    }

    #[allow(unused)]
    async fn trust_context(
        &self,
        tcp_transport: &TcpTransport,
        secure_channels: Arc<SecureChannels>,
    ) -> Result<TrustContext> {
        Ok(self
            .named_trust_context()?
            .trust_context(tcp_transport, secure_channels)
            .await?)
    }
}

#[cfg(test)]
mod test {
    use core::time::Duration;

    use ockam::identity::models::CredentialSchemaIdentifier;
    use ockam::identity::utils::AttributesBuilder;
    use ockam::identity::{identities, Identifier, Identities, Identity};

    use super::*;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // create 2 trust contexts
        let identities = identities().await?;
        let issuer_identifier = identities.identities_creation().create_identity().await?;
        let issuer = identities.get_identity(&issuer_identifier).await?;

        let trust_context1 =
            create_trust_context("trust-context-1", identities.clone(), &issuer).await?;
        let trust_context2 = create_trust_context("trust-context-2", identities, &issuer).await?;
        repository.store_trust_context(&trust_context1).await?;
        repository.store_trust_context(&trust_context2).await?;

        // get a trust context by name
        let result = repository.get_trust_context("trust-context-1").await?;
        assert_eq!(result, Some(trust_context1.clone()));

        // get all the trust contexts
        let result = repository.get_trust_contexts().await?;
        assert_eq!(result, vec![trust_context1.clone(), trust_context2.clone()]);

        // set the first trust context as the default trust context
        repository
            .set_default_trust_context("trust-context-1")
            .await?;
        let result = repository.get_default_trust_context().await?;
        assert_eq!(result, Some(trust_context1));

        // then set the second one
        repository
            .set_default_trust_context("trust-context-2")
            .await?;
        let result = repository.get_default_trust_context().await?;
        assert_eq!(result, Some(trust_context2.clone()));

        // a trust context can be deleted
        repository.delete_trust_context("trust-context-1").await?;
        let result = repository.get_trust_contexts().await?;
        assert_eq!(result, vec![trust_context2]);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn TrustContextsRepository>> {
        Ok(Arc::new(TrustContextsSqlxDatabase::create().await?))
    }

    async fn create_trust_context(
        name: &str,
        identities: Arc<Identities>,
        issuer: &Identity,
    ) -> Result<NamedTrustContext> {
        Ok(NamedTrustContext::new(
            name,
            name,
            Some(create_credential(identities, issuer.identifier()).await?),
            Some(issuer.change_history().clone()),
            Some(MultiAddr::from_string("/dnsaddr/k8s-hubdev-hubconso-85b649e0fe-0c482fd0e8117e9d.elb.us-west-1.amazonaws.com/tcp/6252/service/api").unwrap()),
        ))
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
