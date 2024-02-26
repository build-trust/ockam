use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToSqlxType, ToVoid};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration moves policies attached to resource types from
/// table "resource_policy" to "resource_type_policy"
pub struct SplitPolicies;

#[async_trait]
impl RustMigration for SplitPolicies {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> i64 {
        Self::version()
    }

    async fn migrate(&self, connection: &mut SqliteConnection) -> Result<bool> {
        Self::migrate_policies(connection).await
    }
}

impl SplitPolicies {
    /// Migration version
    pub fn version() -> i64 {
        20240212100000
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20240212100000_migrate_policies"
    }

    pub(crate) async fn migrate_policies(connection: &mut SqliteConnection) -> Result<bool> {
        let mut transaction = sqlx::Connection::begin(&mut *connection)
            .await
            .into_core()?;

        let query_policies =
            query_as("SELECT resource_name, action, expression, node_name FROM resource_policy");
        let rows: Vec<ResourcePolicyRow> = query_policies
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;
        // Copy resource type policies to table "resource_type_policy"
        for row in rows {
            if row.resource_name == "tcp-outlet" || row.resource_name == "tcp-inlet" {
                query("INSERT INTO resource_type_policy (resource_type, action, expression, node_name) VALUES (?, ?, ?, ?)")
                    .bind(row.resource_name.to_sql())
                    .bind(row.action.to_sql())
                    .bind(row.expression.to_sql())
                    .bind(row.node_name.to_sql())
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

        Ok(true)
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
    use crate::database::{MigrationSet, SqlxDatabase};
    use ockam_core::compat::rand::random_string;
    use sqlx::query::Query;
    use sqlx::sqlite::SqliteArguments;
    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        // create the database pool and migrate the tables
        let db_file = NamedTempFile::new().unwrap();

        let pool = SqlxDatabase::create_connection_pool(db_file.path()).await?;

        let mut connection = pool.acquire().await.into_core()?;

        NodeMigrationSet
            .create_migrator()?
            .migrate_before(&pool, SplitPolicies::version(), true)
            .await?;

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

        // apply migrations
        NodeMigrationSet
            .create_migrator()?
            .migrate_before(&pool, SplitPolicies::version() + 1, false)
            .await?;

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
    fn insert_policy(resource: &str) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        let action = "handle_message";
        let expression = random_string();
        let node_name = random_string();
        query("INSERT INTO resource_policy (resource_name, action, expression, node_name) VALUES (?, ?, ?, ?)")
            .bind(resource.to_sql())
            .bind(action.to_sql())
            .bind(expression.to_sql())
            .bind(node_name.to_sql())
    }
}
