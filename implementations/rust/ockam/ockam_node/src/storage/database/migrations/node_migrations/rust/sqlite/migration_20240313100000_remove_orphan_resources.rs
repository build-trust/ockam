use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToVoid, Version};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration removes orphan resources from the resource table
#[derive(Debug)]
pub struct RemoveOrphanResources;

#[async_trait]
impl RustMigration for RemoveOrphanResources {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> Version {
        Self::version()
    }

    async fn migrate(&self, connection: &mut AnyConnection) -> Result<bool> {
        Self::migrate(connection).await
    }
}

impl RemoveOrphanResources {
    /// Migration version
    pub fn version() -> Version {
        Version(20240313100000)
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20240313100000_remove_orphan_resources"
    }

    pub(crate) async fn migrate(connection: &mut AnyConnection) -> Result<bool> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        // Get existing node names
        let node_names: Vec<NodeNameRow> = query_as("SELECT name FROM node")
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        // Get existing resources
        let resources: Vec<ResourceRow> =
            query_as("SELECT resource_name, resource_type, node_name FROM resource")
                .fetch_all(&mut *transaction)
                .await
                .into_core()?;

        // Remove resources that are not associated with a node
        for resource in resources {
            if !node_names.iter().any(|n| n.name == resource.node_name) {
                query("DELETE FROM resource WHERE resource_name = $1 AND resource_type = $2 AND node_name = $3")
                    .bind(resource.resource_name)
                    .bind(resource.resource_type)
                    .bind(resource.node_name)
                    .execute(&mut *transaction)
                    .await
                    .void()?;
            }
        }

        // Commit
        transaction.commit().await.void()?;

        Ok(true)
    }
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
}

#[derive(FromRow)]
struct ResourceRow {
    resource_name: String,
    resource_type: String,
    node_name: String,
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseType, MigrationSet, SqlxDatabase};
    use ockam_core::compat::rand::random_string;
    use sqlx::any::AnyArguments;
    use sqlx::query::Query;
    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        // create the database pool and migrate the tables
        let db_file = NamedTempFile::new().unwrap();
        let db_file = db_file.path();

        let pool = SqlxDatabase::create_sqlite_single_connection_pool(db_file).await?;

        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to_skip_last_rust_migration(&pool, RemoveOrphanResources::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // insert a node
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES ($1, $2, $3, $4, $5)")
            .bind("n1")
            .bind(random_string())
            .bind(0)
            .bind(false)
            .bind(false)
            .execute(&mut *connection)
            .await
            .void()?;

        // insert some resources
        let resource1 = insert_resource("r1", "n1");
        let resource2 = insert_resource("r2", "n1");
        let resource3 = insert_resource("r3", "n2");
        let resource4 = insert_resource("r4", "n3");
        let resource5 = insert_resource("r5", "n1");

        resource1.execute(&mut *connection).await.void()?;
        resource2.execute(&mut *connection).await.void()?;
        resource3.execute(&mut *connection).await.void()?;
        resource4.execute(&mut *connection).await.void()?;
        resource5.execute(&mut *connection).await.void()?;

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, RemoveOrphanResources::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check that the resources of "n1" are still there
        // and that the resources of "n2" and "n3" are not
        let rows: Vec<ResourceRow> =
            query_as("SELECT resource_name, resource_type, node_name FROM resource")
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows.len(), 3);
        rows.iter()
            .find(|r| r.resource_name == "r1" && r.node_name == "n1")
            .unwrap();
        rows.iter()
            .find(|r| r.resource_name == "r2" && r.node_name == "n1")
            .unwrap();
        rows.iter()
            .find(|r| r.resource_name == "r5" && r.node_name == "n1")
            .unwrap();

        Ok(())
    }
    /// HELPERS
    fn insert_resource<'a>(
        resource: &'a str,
        node_name: &'a str,
    ) -> Query<'a, Any, AnyArguments<'a>> {
        let resource_type = random_string();
        query("INSERT INTO resource (resource_name, resource_type, node_name) VALUES ($1, $2, $3)")
            .bind(resource)
            .bind(resource_type)
            .bind(node_name)
    }
}
