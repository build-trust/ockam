use core::str::FromStr;

use sqlx::*;

use ockam::identity::Identifier;
use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};

use crate::cli_state::{IdentitiesRepository, NamedIdentity};

/// Implementation of `IdentitiesRepository` trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct IdentitiesSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl IdentitiesSqlxDatabase {
    /// Create a new database
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for identities");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("identities").await?,
        )))
    }
}

#[async_trait]
impl IdentitiesRepository for IdentitiesSqlxDatabase {
    async fn store_named_identity(
        &self,
        identifier: &Identifier,
        name: &str,
        vault_name: &str,
    ) -> Result<NamedIdentity> {
        let mut transaction = self.database.begin().await.into_core()?;

        let query1 = query_scalar(
            "SELECT EXISTS(SELECT 1 FROM named_identity WHERE is_default=$1 AND name=$2)",
        )
        .bind(true.to_sql())
        .bind(name.to_sql());
        let is_already_default: bool = query1.fetch_one(&mut *transaction).await.into_core()?;

        let query2 = query("INSERT OR REPLACE INTO named_identity VALUES (?, ?, ?, ?)")
            .bind(identifier.to_sql())
            .bind(name.to_sql())
            .bind(vault_name.to_sql())
            .bind(is_already_default.to_sql());
        query2.execute(&mut *transaction).await.void()?;

        transaction.commit().await.void()?;

        Ok(NamedIdentity::new(
            identifier.clone(),
            name.to_string(),
            vault_name.to_string(),
            is_already_default,
        ))
    }

    async fn delete_identity(&self, name: &str) -> Result<Option<Identifier>> {
        let mut transaction = self.database.begin().await.into_core()?;

        // get the named identity
        let query1 = query_as(
            "SELECT identifier, name, vault_name, is_default FROM named_identity WHERE name=$1",
        )
        .bind(name.to_sql());
        let row: Option<NamedIdentityRow> =
            query1.fetch_optional(&mut *transaction).await.into_core()?;
        let named_identity = row.map(|r| r.named_identity()).transpose()?;

        let result = match named_identity {
            // return None if it wasn't found
            None => None,

            // otherwise delete it and set another identity as the default
            Some(named_identity) => {
                let query2 = query("DELETE FROM named_identity WHERE name=?").bind(name.to_sql());
                query2.execute(&mut *transaction).await.void()?;

                // if the deleted identity was the default one, select another identity to be the default one
                if named_identity.is_default() {
                    if let Some(other_name) =
                        query_scalar::<_, String>("SELECT name FROM named_identity")
                            .fetch_optional(&mut *transaction)
                            .await
                            .into_core()?
                    {
                        let query3 =
                            query("UPDATE named_identity SET is_default = ? WHERE name = ?")
                                .bind(true.to_sql())
                                .bind(other_name.to_sql());
                        query3.execute(&mut *transaction).await.void()?
                    }
                }
                Some(named_identity.identifier())
            }
        };
        transaction.commit().await.void()?;
        Ok(result)
    }

    async fn delete_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>> {
        if let Some(name) = self.get_identity_name_by_identifier(identifier).await? {
            self.delete_identity(&name).await?;
            Ok(Some(name))
        } else {
            Ok(None)
        }
    }

    async fn get_identifier(&self, name: &str) -> Result<Option<Identifier>> {
        let query = query_as(
            "SELECT identifier, name, vault_name, is_default FROM named_identity WHERE name=$1",
        )
        .bind(name.to_sql());
        let row: Option<NamedIdentityRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.identifier()).transpose()
    }

    async fn get_identity_name_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<String>> {
        let query =
            query_as("SELECT identifier, name, vault_name, is_default FROM named_identity WHERE identifier=$1").bind(identifier.to_sql());
        let row: Option<NamedIdentityRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(row.map(|r| r.name()))
    }

    async fn get_named_identity(&self, name: &str) -> Result<Option<NamedIdentity>> {
        let query = query_as(
            "SELECT identifier, name, vault_name, is_default FROM named_identity WHERE name=$1",
        )
        .bind(name.to_sql());
        let row: Option<NamedIdentityRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_identity()).transpose()
    }

    async fn get_named_identity_by_identifier(
        &self,
        identifier: &Identifier,
    ) -> Result<Option<NamedIdentity>> {
        let query =
            query_as("SELECT identifier, name, vault_name, is_default FROM named_identity WHERE identifier=$1").bind(identifier.to_sql());
        let row: Option<NamedIdentityRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_identity()).transpose()
    }

    async fn get_named_identities(&self) -> Result<Vec<NamedIdentity>> {
        let query = query_as("SELECT identifier, name, vault_name, is_default FROM named_identity");
        let row: Vec<NamedIdentityRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        row.iter().map(|r| r.named_identity()).collect()
    }

    async fn get_named_identities_by_vault_name(
        &self,
        vault_name: &str,
    ) -> Result<Vec<NamedIdentity>> {
        let query = query_as("SELECT identifier, name, vault_name, is_default FROM named_identity WHERE vault_name=?").bind(vault_name.to_sql());
        let row: Vec<NamedIdentityRow> = query.fetch_all(&self.database.pool).await.into_core()?;
        row.iter().map(|r| r.named_identity()).collect()
    }

    async fn set_as_default(&self, name: &str) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the identifier as the default one
        let query1 = query("UPDATE named_identity SET is_default = ? WHERE name = ?")
            .bind(true.to_sql())
            .bind(name.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE named_identity SET is_default = ? WHERE name <> ?")
            .bind(false.to_sql())
            .bind(name.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn set_as_default_by_identifier(&self, identifier: &Identifier) -> Result<()> {
        let mut transaction = self.database.begin().await.into_core()?;
        // set the identifier as the default one
        let query1 = query("UPDATE named_identity SET is_default = ? WHERE identifier = ?")
            .bind(true.to_sql())
            .bind(identifier.to_sql());
        query1.execute(&mut *transaction).await.void()?;

        // set all the others as non-default
        let query2 = query("UPDATE named_identity SET is_default = ? WHERE identifier <> ?")
            .bind(false.to_sql())
            .bind(identifier.to_sql());
        query2.execute(&mut *transaction).await.void()?;
        transaction.commit().await.void()
    }

    async fn get_default_named_identity(&self) -> Result<Option<NamedIdentity>> {
        let query =
            query_as("SELECT identifier, name, vault_name, is_default FROM named_identity WHERE is_default=$1").bind(true.to_sql());
        let row: Option<NamedIdentityRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        row.map(|r| r.named_identity()).transpose()
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct NamedIdentityRow {
    identifier: String,
    name: String,
    vault_name: String,
    is_default: bool,
}

impl NamedIdentityRow {
    pub(crate) fn identifier(&self) -> Result<Identifier> {
        Identifier::from_str(&self.identifier)
    }

    pub(crate) fn name(&self) -> String {
        self.name.clone()
    }

    #[allow(unused)]
    pub(crate) fn vault_name(&self) -> String {
        self.vault_name.clone()
    }

    pub(crate) fn named_identity(&self) -> Result<NamedIdentity> {
        Ok(NamedIdentity::new(
            self.identifier()?,
            self.name.clone(),
            self.vault_name.clone(),
            self.is_default,
        ))
    }
}

#[cfg(test)]
mod tests {
    use ockam::identity::identities;

    use super::*;

    #[tokio::test]
    async fn test_identities_repository_named_identities() -> Result<()> {
        let repository = create_repository().await?;

        // A name can be associated to an identity
        let identifier1 = create_identity().await?;
        repository
            .store_named_identity(&identifier1, "name1", "vault")
            .await?;

        let identifier2 = create_identity().await?;
        repository
            .store_named_identity(&identifier2, "name2", "vault")
            .await?;

        let result = repository.get_identifier("name1").await?;
        assert_eq!(result, Some(identifier1.clone()));

        let result = repository
            .get_identity_name_by_identifier(&identifier1)
            .await?;
        assert_eq!(result, Some("name1".into()));

        let result = repository.get_named_identity("name2").await?;
        assert_eq!(result.map(|n| n.identifier()), Some(identifier2.clone()));

        let result = repository.get_named_identities().await?;
        assert_eq!(
            result.iter().map(|n| n.identifier()).collect::<Vec<_>>(),
            vec![identifier1.clone(), identifier2.clone()]
        );

        repository.delete_identity("name1").await?;
        let result = repository.get_named_identities().await?;
        assert_eq!(
            result.iter().map(|n| n.identifier()).collect::<Vec<_>>(),
            vec![identifier2.clone()]
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_identities_repository_default_identities() -> Result<()> {
        let repository = create_repository().await?;

        // A name can be associated to an identity
        let identifier1 = create_identity().await?;
        let named_identity1 = repository
            .store_named_identity(&identifier1, "name1", "vault")
            .await?;

        let identifier2 = create_identity().await?;
        let named_identity2 = repository
            .store_named_identity(&identifier2, "name2", "vault")
            .await?;

        // An identity can be marked as being the default one
        repository
            .set_as_default_by_identifier(&identifier1)
            .await?;
        let result = repository.get_default_named_identity().await?;
        assert_eq!(result, Some(named_identity1.set_as_default()));

        // An identity can be marked as being the default one by passing its name
        repository.set_as_default("name2").await?;
        let result = repository.get_default_named_identity().await?;
        assert_eq!(result, Some(named_identity2.set_as_default()));

        let result = repository.get_named_identity("name1").await?;
        assert!(!result.unwrap().is_default());

        let result = repository.get_default_named_identity().await?;
        assert_eq!(result.map(|i| i.name()), Some("name2".to_string()));

        Ok(())
    }

    #[tokio::test]
    async fn test_get_identities_by_vault_name() -> Result<()> {
        let repository = create_repository().await?;

        // A name can be associated to an identity
        let identifier1 = create_identity().await?;
        repository
            .store_named_identity(&identifier1, "name1", "vault1")
            .await?;

        let identifier2 = create_identity().await?;
        repository
            .store_named_identity(&identifier2, "name2", "vault2")
            .await?;

        let identifier3 = create_identity().await?;
        repository
            .store_named_identity(&identifier3, "name3", "vault1")
            .await?;

        let result = repository
            .get_named_identities_by_vault_name("vault1")
            .await?;
        let names: Vec<String> = result.iter().map(|i| i.name()).collect();
        assert_eq!(names, vec!["name1", "name3"]);

        Ok(())
    }

    /// HELPERS
    async fn create_repository() -> Result<Arc<dyn IdentitiesRepository>> {
        Ok(IdentitiesSqlxDatabase::create().await?)
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }
}
