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
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 =
            query_scalar("SELECT EXISTS(SELECT 1 FROM vault WHERE is_default=$1 AND name=$2)")
                .bind(true.to_sql())
                .bind(name.to_sql());
        let is_already_default: bool = query1.fetch_one(&mut *transaction).await.into_core()?;

        let query2 = query("INSERT OR REPLACE INTO vault VALUES (?1, ?2, ?3, ?4)")
            .bind(name.to_sql())
            .bind(path.to_sql())
            .bind(is_already_default.to_sql())
            .bind(is_kms.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()?;

        Ok(NamedVault::new(
            name,
            path.clone(),
            is_already_default,
            is_kms,
        ))
    }

    /// Delete a vault by name
    async fn delete_vault(&self, name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;

        // get the named vault
        let query1 = query_as("SELECT name, path, is_default, is_kms FROM vault WHERE name=$1")
            .bind(name.to_sql());
        let row: Option<VaultRow> = query1.fetch_optional(&mut *transaction).await.into_core()?;
        let named_vault = row.map(|r| r.named_vault()).transpose()?;

        match named_vault {
            // return if it wasn't found
            None => (),

            // otherwise delete it and set another vault as the default
            Some(named_vault) => {
                let query2 = query("DELETE FROM vault WHERE name=?").bind(name.to_sql());
                query2.execute(&mut *transaction).await.void()?;

                // if the deleted vault was the default one, select another vault to be the default one
                if named_vault.is_default() {
                    if let Some(other_name) = query_scalar::<_, String>("SELECT name FROM vault")
                        .fetch_optional(&mut *transaction)
                        .await
                        .into_core()?
                    {
                        let query3 = query("UPDATE vault SET is_default = ? WHERE name = ?")
                            .bind(true.to_sql())
                            .bind(other_name.to_sql());
                        query3.execute(&mut *transaction).await.void()?
                    }
                }
            }
        };
        transaction.commit().await.void()
    }

    async fn set_as_default(&self, name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the identifier as the default one
        let query1 = query("UPDATE vault SET is_default = ? WHERE name = ?")
            .bind(true.to_sql())
            .bind(name.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE vault SET is_default = ? WHERE name <> ?")
            .bind(false.to_sql())
            .bind(name.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn is_default(&self, name: &str) -> Result<bool> {
        let query = query_as("SELECT name, path, is_default, is_kms FROM vault WHERE name = $1")
            .bind(name.to_sql());
        let row: Option<VaultRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.is_default()).unwrap_or(false))
    }

    async fn get_named_vaults(&self) -> Result<Vec<NamedVault>> {
        let query = query_as("SELECT name, path, is_default, is_kms FROM vault");
        let rows: Vec<VaultRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        rows.iter().map(|r| r.named_vault()).collect()
    }

    async fn get_named_vault(&self, name: &str) -> Result<Option<NamedVault>> {
        let query = query_as("SELECT name, path, is_default, is_kms FROM vault WHERE name = $1")
            .bind(name.to_sql());
        let row: Option<VaultRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_vault()).transpose()
    }

    async fn get_default_vault(&self) -> Result<Option<NamedVault>> {
        let query =
            query_as("SELECT name, path, is_default, is_kms FROM vault WHERE is_default = $1")
                .bind(true.to_sql());
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
    is_default: bool,
    is_kms: bool,
}

impl VaultRow {
    pub(crate) fn named_vault(&self) -> Result<NamedVault> {
        Ok(NamedVault::new(
            &self.name,
            PathBuf::from_str(self.path.as_str()).unwrap(),
            self.is_default,
            self.is_kms,
        ))
    }

    pub(crate) fn is_default(&self) -> bool {
        self.is_default
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
        let expected = NamedVault::new("vault1", "path".into(), false, false);
        assert_eq!(named_vault1, expected);

        // The vault can then be retrieved with its name
        let result = repository.get_named_vault("vault1").await?;
        assert_eq!(result, Some(named_vault1.clone()));

        // A default vault can be set
        repository.set_as_default("vault1").await?;
        let result = repository.get_default_vault().await?;
        assert_eq!(result, Some(named_vault1.set_as_default()));

        let named_vault2 = repository
            .store_vault("vault2", "path2".into(), false)
            .await?;
        repository.set_as_default("vault2").await?;
        let result = repository.get_default_vault().await?;
        assert_eq!(result, Some(named_vault2.set_as_default()));

        Ok(())
    }

    #[tokio::test]
    async fn test_store_kms_vault() -> Result<()> {
        let repository = create_repository().await?;

        // A KMS vault can be created by setting the kms flag to true
        let kms = repository.store_vault("kms", "path".into(), true).await?;
        let expected = NamedVault::new("kms", "path".into(), false, true);
        assert_eq!(kms, expected);
        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn VaultsRepository>> {
        Ok(VaultsSqlxDatabase::create().await?)
    }
}
