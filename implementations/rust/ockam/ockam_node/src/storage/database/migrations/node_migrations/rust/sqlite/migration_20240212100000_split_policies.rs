use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, SqlxDatabase, ToVoid, Version};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration moves policies attached to resource types from
/// table "resource_policy" to "resource_type_policy"
#[derive(Debug)]
pub struct SplitPolicies;

#[async_trait]
impl RustMigration for SplitPolicies {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> Version {
        Self::version()
    }

    async fn migrate(
        &self,
        _legacy_sqlite_database: Option<SqlxDatabase>,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        Self::migrate_policies(connection).await
    }
}

impl SplitPolicies {
    /// Migration version
    pub fn version() -> Version {
        Version(20240212100000)
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20240212100000_migrate_policies"
    }

    pub(crate) async fn migrate_policies(connection: &mut AnyConnection) -> Result<()> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        let query_policies =
            query_as("SELECT resource_name, action, expression, node_name FROM resource_policy");
        let rows: Vec<ResourcePolicyRow> = query_policies
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;
        // Copy resource type policies to table "resource_type_policy"
        for row in rows {
            if row.resource_name == "tcp-outlet" || row.resource_name == "tcp-inlet" {
                query("INSERT INTO resource_type_policy (resource_type, action, expression, node_name) VALUES ($1, $2, $3, $4)")
                    .bind(row.resource_name)
                    .bind(row.action)
                    .bind(row.expression)
                    .bind(row.node_name)
                    .execute(&mut *transaction)
                    .await
                    .void()?;
            }
        }
        // Remove policies from table "resource_policy" where resource is "tcp-outlet" or "tcp-inlet"
        query(
            "DELETE FROM resource_policy WHERE resource_name = 'tcp-outlet' OR resource_name = 'tcp-inlet'",
        )
        .execute(&mut *transaction)
        .await
        .void()?;

        // Commit
        transaction.commit().await.void()?;

        Ok(())
    }
}

#[derive(FromRow)]
struct ResourcePolicyRow {
    resource_name: String,
    action: String,
    expression: String,
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
            .migrate_up_to_skip_last_rust_migration(&pool, SplitPolicies::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // insert some policies
        let policy1 = insert_policy("tcp-outlet");
        let policy2 = insert_policy("tcp-inlet");
        let policy3 = insert_policy("my_outlet_1");
        let policy4 = insert_policy("my_outlet_2");
        let policy5 = insert_policy("my_inlet_1");

        policy1.execute(&mut *connection).await.void()?;
        policy2.execute(&mut *connection).await.void()?;
        policy3.execute(&mut *connection).await.void()?;
        policy4.execute(&mut *connection).await.void()?;
        policy5.execute(&mut *connection).await.void()?;

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, SplitPolicies::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check that the "tcp-inlet" and "tcp-outlet" policies are moved to the new table
        let rows: Vec<ResourceTypePolicyRow> = query_as(
            "SELECT resource_type, action, expression, node_name FROM resource_type_policy",
        )
        .fetch_all(&mut *connection)
        .await
        .into_core()?;
        assert_eq!(rows.len(), 2);
        rows.iter()
            .find(|r| r.resource_type == "tcp-outlet")
            .unwrap();
        rows.iter()
            .find(|r| r.resource_type == "tcp-inlet")
            .unwrap();

        // check that they are not in the resource_policy table and that we kept the other policies
        let rows: Vec<ResourcePolicyRow> =
            query_as("SELECT resource_name, action, expression, node_name FROM resource_policy")
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows.len(), 3);
        rows.iter()
            .find(|r| r.resource_name == "my_outlet_1")
            .unwrap();
        rows.iter()
            .find(|r| r.resource_name == "my_outlet_2")
            .unwrap();
        rows.iter()
            .find(|r| r.resource_name == "my_inlet_1")
            .unwrap();

        Ok(())
    }

    #[derive(FromRow)]
    #[allow(dead_code)]
    struct ResourceTypePolicyRow {
        resource_type: String,
        action: String,
        expression: String,
        node_name: String,
    }

    /// HELPERS
    fn insert_policy(resource: &str) -> Query<Any, AnyArguments> {
        let action = "handle_message";
        let expression = random_string();
        let node_name = random_string();
        query("INSERT INTO resource_policy (resource_name, action, expression, node_name) VALUES ($1, $2, $3, $4)")
            .bind(resource)
            .bind(action)
            .bind(expression)
            .bind(node_name)
    }
}
