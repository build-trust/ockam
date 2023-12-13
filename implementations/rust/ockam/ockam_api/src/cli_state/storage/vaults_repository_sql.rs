use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use sqlx::*;

use crate::cli_state::{NamedVault, VaultsRepository};
use ockam::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::async_trait;
use ockam_core::Result;

pub struct VaultsSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl VaultsSqlxDatabase {
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for vaults");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("vaults").await?,
        )))
    }
}

#[async_trait]
impl VaultsRepository for VaultsSqlxDatabase {
    async fn store_vault(&self, name: &str, path: PathBuf, is_kms: bool) -> Result<NamedVault> {
        let query = query("INSERT OR REPLACE INTO vault VALUES (?1, ?2, ?3, ?4)")
            .bind(name.to_sql())
            .bind(path.to_sql())
            .bind(true.to_sql())
            .bind(is_kms.to_sql());
        query.execute(&self.database.pool).await.void()?;

        Ok(NamedVault::new(name, path.clone(), is_kms))
    }

    /// Delete a vault by name
    async fn delete_named_vault(&self, name: &str) -> Result<()> {
        let query = query("DELETE FROM vault WHERE name=?").bind(name.to_sql());
        query.execute(&self.database.pool).await.void()
    }

    async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        let query = query_as("SELECT name, path, is_kms FROM vault");
        let rows: Vec<VaultRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.named_vault()).collect()
    }

    async fn get_named_vault(&self, name: &str) -> Result<Option<NamedVault>> {
        let query =
            query_as("SELECT name, path, is_kms FROM vault WHERE name = $1").bind(name.to_sql());
        let row: Option<VaultRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_vault()).transpose()
    }
}

// Database serialization / deserialization

#[derive(FromRow)]
pub(crate) struct VaultRow {
    name: String,
    path: String,
    is_kms: bool,
}

impl VaultRow {
    pub(crate) fn named_vault(&self) -> Result<NamedVault> {
        Ok(NamedVault::new(
            &self.name,
            PathBuf::from_str(self.path.as_str()).unwrap(),
            self.is_kms,
        ))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[tokio::test]
    async fn test_repository() -> Result<()> {
        let repository = create_repository().await?;

        // A vault can be defined with a path and stored under a specific name
        let named_vault1 = repository
            .store_vault("vault1", "path".into(), false)
            .await?;
        let expected = NamedVault::new("vault1", "path".into(), false);
        assert_eq!(named_vault1, expected);

        // The vault can then be retrieved with its name
        let result = repository.get_named_vault("vault1").await?;
        assert_eq!(result, Some(named_vault1.clone()));

        // The vault can also be deleted
        repository.delete_named_vault("vault1").await?;
        let result = repository.get_named_vault("vault1").await?;
        assert_eq!(result, None);
        Ok(())
    }

    #[tokio::test]
    async fn test_store_kms_vault() -> Result<()> {
        let repository = create_repository().await?;

        // A KMS vault can be created by setting the kms flag to true
        let kms = repository.store_vault("kms", "path".into(), true).await?;
        let expected = NamedVault::new("kms", "path".into(), true);
        assert_eq!(kms, expected);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn VaultsRepository>> {
        Ok(VaultsSqlxDatabase::create().await?)
    }
}
