use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToVoid, Version};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration makes sure that policy expressions that were created as subject.has_credential
/// now start with an operator: (= subject.has_credential "true")
#[derive(Debug)]
pub struct UpdatePolicyExpressions;

#[async_trait]
impl RustMigration for UpdatePolicyExpressions {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> Version {
        Self::version()
    }

    async fn migrate(&self, connection: &mut AnyConnection) -> Result<bool> {
        Self::migrate_policy_expressions(connection).await
    }
}

impl UpdatePolicyExpressions {
    /// Migration version
    pub fn version() -> Version {
        Version(20240503100000)
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20240503100000_update_policy_expressions"
    }

    pub(crate) async fn migrate_policy_expressions(connection: &mut AnyConnection) -> Result<bool> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        query("UPDATE resource_policy SET expression = '(= subject.has_credential \"true\")' WHERE expression = 'subject.has_credential'").execute(&mut *transaction).await.void()?;
        query("UPDATE resource_type_policy SET expression = '(= subject.has_credential \"true\")' WHERE expression = 'subject.has_credential'").execute(&mut *transaction).await.void()?;

        // Commit
        transaction.commit().await.void()?;

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseType, MigrationSet, SqlxDatabase};
    use ockam_core::compat::rand::random_string;
    use sqlx::any::{AnyArguments, AnyRow};
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
            .migrate_up_to_skip_last_rust_migration(&pool, UpdatePolicyExpressions::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // insert some policies
        let policy1 = insert_resource_policy("tcp-outlet");
        let policy2 = insert_resource_policy("tcp-inlet");
        let policy3 = insert_resource_type_policy("tcp-outlet");
        let policy4 = insert_resource_type_policy("tcp-inlet");

        policy1.execute(&mut *connection).await.void()?;
        policy2.execute(&mut *connection).await.void()?;
        policy3.execute(&mut *connection).await.void()?;
        policy4.execute(&mut *connection).await.void()?;

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, UpdatePolicyExpressions::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check that the update was successful for resource policies
        let rows: Vec<AnyRow> = query("SELECT expression FROM resource_policy")
            .fetch_all(&mut *connection)
            .await
            .into_core()?;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|e| {
            let s: String = e.get(0);
            assert_eq!(s, *"(= subject.has_credential \"true\")");
            true
        }));

        // check that the update was successful for resource type policies
        let rows: Vec<AnyRow> = query("SELECT expression FROM resource_type_policy")
            .fetch_all(&mut *connection)
            .await
            .into_core()?;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|e| {
            let s: String = e.get(0);
            assert_eq!(s, *"(= subject.has_credential \"true\")");
            true
        }));
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
    fn insert_resource_policy(resource: &str) -> Query<'_, Any, AnyArguments<'_>> {
        let action = "handle_message";
        let expression = "subject.has_credential";
        let node_name = random_string();
        query("INSERT INTO resource_policy (resource_name, action, expression, node_name) VALUES ($1, $2, $3, $4)")
            .bind(resource)
            .bind(action)
            .bind(expression)
            .bind(node_name)
    }

    fn insert_resource_type_policy(resource: &str) -> Query<'_, Any, AnyArguments<'_>> {
        let action = "handle_message";
        let expression = "subject.has_credential";
        let node_name = random_string();
        query("INSERT INTO resource_type_policy (resource_type, action, expression, node_name) VALUES ($1, $2, $3, $4)")
            .bind(resource)
            .bind(action)
            .bind(expression)
            .bind(node_name)
    }
}
