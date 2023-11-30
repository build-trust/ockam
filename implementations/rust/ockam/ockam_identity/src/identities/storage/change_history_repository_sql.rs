use core::str::FromStr;

use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::models::{ChangeHistory, Identifier};
use crate::ChangeHistoryRepository;

/// Implementation of `IdentitiesRepository` trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct ChangeHistorySqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl ChangeHistorySqlxDatabase {
    /// Create a new database
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for change history");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("change history").await?,
        )))
    }
}

#[async_trait]
impl ChangeHistoryRepository for ChangeHistorySqlxDatabase {
    async fn store_change_history(
        &self,
        identifier: &Identifier,
        change_history: ChangeHistory,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO identity VALUES (?, ?)")
            .bind(identifier.to_sql())
            .bind(change_history.to_sql());
        query.execute(&self.database.pool).await.void()
    }

    async fn delete_change_history(&self, identifier: &Identifier) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        let query1 = query("DELETE FROM identity where identifier=?").bind(identifier.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        let query2 =
            query("DELETE FROM identity_attributes where identifier=?").bind(identifier.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()?;
        Ok(())
    }

    async fn get_change_history(&self, identifier: &Identifier) -> Result<Option<ChangeHistory>> {
        let query =
            query_as("SELECT * FROM identity WHERE identifier=$1").bind(identifier.to_sql());
        let row: Option<ChangeHistoryRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.change_history()).transpose()
    }

    async fn get_change_histories(&self) -> Result<Vec<ChangeHistory>> {
        let query = query_as("SELECT * FROM identity");
        let row: Vec<ChangeHistoryRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        row.iter().map(|r| r.change_history()).collect()
    }
}

// Database serialization / deserialization

impl ToSqlxType for Identifier {
    fn to_sql(&self) -> SqlxType {
        self.to_string().to_sql()
    }
}

impl ToSqlxType for ChangeHistory {
    fn to_sql(&self) -> SqlxType {
        self.export_as_string().unwrap().to_sql()
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
    use crate::{identities, Identity};

    use super::*;

    #[tokio::test]
    async fn test_identities_repository() -> Result<()> {
        let identity1 = create_identity().await?;
        let identity2 = create_identity().await?;
        let repository = create_repository().await?;

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
        assert_eq!(
            result,
            vec![
                identity1.change_history().clone(),
                identity2.change_history().clone(),
            ]
        );
        // a change history can also be deleted from the repository
        repository
            .delete_change_history(identity2.identifier())
            .await?;
        let result = repository
            .get_change_history(identity2.identifier())
            .await?;
        assert_eq!(result, None);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn ChangeHistoryRepository>> {
        Ok(ChangeHistorySqlxDatabase::create().await?)
    }

    async fn create_identity() -> Result<Identity> {
        let identities = identities().await?;
        let identifier = identities.identities_creation().create_identity().await?;
        identities.get_identity(&identifier).await
    }
}
