use core::str::FromStr;
use sqlx::any::AnyArguments;
use sqlx::encode::IsNull;
use sqlx::error::BoxDynError;
use sqlx::query::Query;
use sqlx::*;
use sqlx_core::any::AnyArgumentBuffer;
use std::sync::Arc;
use tracing::debug;

use crate::models::{ChangeHistory, Identifier};
use crate::{ChangeHistoryRepository, Identity, IdentityError, IdentityHistoryComparison, Vault};
use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::AutoRetry;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToVoid};

/// Implementation of [`ChangeHistoryRepository`] trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct ChangeHistorySqlxDatabase {
    database: SqlxDatabase,
}

impl ChangeHistorySqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase) -> Self {
        debug!("create a repository for change history");
        Self { database }
    }

    /// Create a repository
    pub fn make_repository(database: SqlxDatabase) -> Arc<dyn ChangeHistoryRepository> {
        if database.needs_retry() {
            Arc::new(AutoRetry::new(Self::new(database)))
        } else {
            Arc::new(Self::new(database))
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(SqlxDatabase::in_memory("change history").await?))
    }
}

#[async_trait]
impl ChangeHistoryRepository for ChangeHistorySqlxDatabase {
    async fn update_identity(&self, identity: &Identity, ignore_older: bool) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 =
            query_as("SELECT identifier, change_history FROM identity WHERE identifier = $1")
                .bind(identity.identifier());
        let row: Option<ChangeHistoryRow> =
            query1.fetch_optional(&mut *transaction).await.into_core()?;

        let do_insert = match row {
            Some(row) => {
                let known_identity = Identity::import_from_change_history(
                    Some(identity.identifier()),
                    row.change_history()?,
                    Vault::create_verifying_vault(),
                )
                .await?;

                match identity.compare(&known_identity) {
                    IdentityHistoryComparison::Conflict => {
                        return Err(IdentityError::ConsistencyError)?;
                    }
                    IdentityHistoryComparison::Older => {
                        if ignore_older {
                            false
                        } else {
                            return Err(IdentityError::ConsistencyError)?;
                        }
                    }

                    IdentityHistoryComparison::Newer => true,
                    IdentityHistoryComparison::Equal => false,
                }
            }
            None => true,
        };
        if do_insert {
            Self::insert_query(identity.identifier(), identity.change_history())
                .execute(&mut *transaction)
                .await
                .void()?
        };
        transaction.commit().await.void()
    }

    async fn store_change_history(
        &self,
        identifier: &Identifier,
        change_history: ChangeHistory,
    ) -> Result<()> {
        Self::insert_query(identifier, &change_history)
            .execute(&*self.database.pool)
            .await
            .void()
    }

    async fn delete_change_history(&self, identifier: &Identifier) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query("DELETE FROM identity where identifier = $1").bind(identifier);
        query1.execute(&mut *transaction).await.void()?;

        let query2 =
            query("DELETE FROM identity_attributes where identifier = $1").bind(identifier);
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn get_change_history(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        let query =
            query_as("SELECT identifier, change_history FROM identity WHERE identifier = $1")
                .bind(identifier);
        let row: Option<ChangeHistoryRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.change_history()).transpose()
    }

    async fn get_change_histories(&self) -> Result<Vec<ChangeHistory>> {
        let query = query_as("SELECT identifier, change_history FROM identity");
        let row: Vec<ChangeHistoryRow> = query.fetch_all(&*self.database.pool).await.into_core()?;
        row.iter().map(|r| r.change_history()).collect()
    }
}

impl ChangeHistorySqlxDatabase {
    fn insert_query<'a>(
        identifier: &'a Identifier,
        change_history: &'a ChangeHistory,
    ) -> Query<'a, Any, AnyArguments<'a>> {
        query(
            r#"
            INSERT INTO identity (identifier, change_history)
            VALUES ($1, $2)
            ON CONFLICT (identifier)
            DO UPDATE SET change_history = $2"#,
        )
        .bind(identifier)
        .bind(change_history)
    }
}

// Database serialization / deserialization

impl Type<Any> for Identifier {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for Identifier {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.to_string(), buf)
    }
}

impl Type<Any> for ChangeHistory {
    fn type_info() -> <Any as Database>::TypeInfo {
        <String as Type<Any>>::type_info()
    }
}

impl Encode<'_, Any> for ChangeHistory {
    fn encode_by_ref(&self, buf: &mut AnyArgumentBuffer) -> Result<IsNull, BoxDynError> {
        <String as Encode<'_, Any>>::encode_by_ref(&self.export_as_string().unwrap(), buf)
    }
}

// Low-level representation of a table row
#[derive(sqlx::FromRow)]
pub(crate) struct ChangeHistoryRow {
    identifier: String,
    change_history: String,
}

impl ChangeHistoryRow {
    #[allow(dead_code)]
    pub(crate) fn identifier(&self) -> Result<Identifier> {
        Identifier::from_str(&self.identifier)
    }

    pub(crate) fn change_history(&self) -> Result<ChangeHistory> {
        ChangeHistory::import_from_string(&self.change_history)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{identities, Identity};

    use ockam_core::compat::sync::Arc;
    use ockam_node::database::{with_dbs, with_sqlite_dbs};

    #[tokio::test]
    async fn test_identities_repository_has_orchestrator_history() -> Result<()> {
        with_sqlite_dbs(|db| async move {
            // Clean repository should already have the Orchestrator change history
            let repository: Arc<dyn ChangeHistoryRepository> =
                Arc::new(ChangeHistorySqlxDatabase::new(db));

            let (orchestrator_identifier, orchestrator_change_history) = orchestrator_identity();

            // the change history can be retrieved
            let result = repository
                .get_change_history(&orchestrator_identifier)
                .await?;
            assert_eq!(result.as_ref(), Some(&orchestrator_change_history));

            let result = repository.get_change_histories().await?;
            assert_eq!(result, vec![orchestrator_change_history]);

            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_identities_repository() -> Result<()> {
        with_dbs(|db| async move {
            let repository: Arc<dyn ChangeHistoryRepository> =
                Arc::new(ChangeHistorySqlxDatabase::new(db));
            let identity1 = create_identity().await?;
            let identity2 = create_identity().await?;

            // store and retrieve an identity
            repository
                .store_change_history(identity1.identifier(), identity1.change_history().clone())
                .await?;

            // the change history can be retrieved
            let result = repository
                .get_change_history(identity1.identifier())
                .await?;
            assert_eq!(result, Some(identity1.change_history().clone()));

            // trying to retrieve a missing identity returns None
            let result = repository
                .get_change_history(identity2.identifier())
                .await?;
            assert_eq!(result, None);

            // the repository can return the list of all change histories
            repository
                .store_change_history(identity2.identifier(), identity2.change_history().clone())
                .await?;
            let result = repository.get_change_histories().await?;
            assert!(result.contains(&identity1.change_history().clone()));
            assert!(result.contains(&identity2.change_history().clone()));
            // a change history can also be deleted from the repository
            repository
                .delete_change_history(identity2.identifier())
                .await?;
            let result = repository
                .get_change_history(identity2.identifier())
                .await?;
            assert_eq!(result, None);
            Ok(())
        })
        .await
    }

    #[tokio::test]
    async fn test_update_identity() -> Result<()> {
        let identities = identities().await?;
        let identities_creation = identities.identities_creation();
        let identities_verification = identities.identities_verification();
        let identifier = identities_creation.create_identity().await?;

        // rotating the identity twice
        identities_creation.rotate_identity(&identifier).await?;
        let rotated = identities.get_identity(&identifier).await?;
        identities_creation.rotate_identity(&identifier).await?;

        let change_history_latest = identities_verification
            .get_change_history(&identifier)
            .await?;

        // try to update the identity with an old rotated version
        let result = identities_verification.update_identity(&rotated).await;
        assert!(result.is_err());

        assert_eq!(
            identities_verification
                .get_change_history(&identifier)
                .await?,
            change_history_latest
        );

        let result = identities_verification
            .update_identity_ignore_older(&rotated)
            .await;
        assert!(result.is_ok());

        assert_eq!(
            identities_verification
                .get_change_history(&identifier)
                .await?,
            change_history_latest
        );

        Ok(())
    }

    /// HELPERS
    async fn create_identity() -> Result<Identity> {
        let identities = identities().await?;
        let identifier = identities.identities_creation().create_identity().await?;
        identities.get_identity(&identifier).await
    }

    fn orchestrator_identity() -> (Identifier, ChangeHistory) {
        let identifier = Identifier::from_str(
            "I84502ce0d9a0a91bae29026b84e19be69fb4203a6bdd1424c85a43c812772a00",
        )
        .unwrap();
        let change_history = ChangeHistory::import_from_string("81825858830101585385f6820181584104ebf9d78281a04f180029c12a74e994386c7c9fee24903f3bfe351497a9952758ee5f4b57d7ed6236ab5082ed85e1ae8c07d5600e0587f652d36727904b3e310df41a656a365d1a7836395d820181584050bf79071ecaf08a966228c712295a17da53994dc781a22103602afe656276ef83ba83a1004845b1e979e0944abff3cd8c7ceef834a8f5eeeca0e8f720fa38f4").unwrap();

        (identifier, change_history)
    }
}
