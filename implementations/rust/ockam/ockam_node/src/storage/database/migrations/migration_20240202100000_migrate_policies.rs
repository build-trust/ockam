use crate::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::Result;
use sqlx::*;

/// This migration duplicates the existing policies for every known node
pub struct PoliciesByNode;

impl PoliciesByNode {
    /// Duplicate all policies entry for every known node
    pub(crate) async fn migrate_policies(pool: &SqlitePool) -> Result<bool> {
        let migration_name = "20240202100000_add_node_name_to_policies";

        if SqlxDatabase::has_migrated(pool, migration_name).await? {
            return Ok(false);
        }

        let mut conn = pool.acquire().await.into_core()?;

        let mut transaction = conn.begin().await.into_core()?;

        let query_node_names = query_as("SELECT name FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;
        let node_names = node_names.into_iter().map(|r| r.name).collect::<Vec<_>>();

        let legacy_policies: Vec<LegacyPolicyRow> =
            query_as("SELECT resource, action, expression FROM policy_old")
                .fetch_all(&mut *transaction)
                .await
                .into_core()?;

        for policy in legacy_policies {
            for node_name in node_names.iter() {
                let insert = query(
                    "INSERT INTO policy (node, resource, action, expression) VALUES (?, ?, ?, ?)",
                )
                .bind(node_name.to_sql())
                .bind(policy.resource.to_sql())
                .bind(policy.action.to_sql())
                .bind(policy.expression.to_sql());

                insert.execute(&mut *transaction).await.void()?;
            }
        }

        // finally drop the old table
        query("DROP TABLE policy_old")
            .execute(&mut *transaction)
            .await
            .void()?;

        transaction.commit().await.void()?;

        SqlxDatabase::mark_as_migrated(pool, migration_name).await?;

        Ok(true)
    }
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
}

#[derive(FromRow)]
struct LegacyPolicyRow {
    resource: String,
    action: String,
    expression: Vec<u8>,
}

#[cfg(test)]
mod test {
    use crate::database::sqlx_migration::NodesMigration;
    use sqlx::query::Query;
    use sqlx::sqlite::SqliteArguments;
    use tempfile::NamedTempFile;

    use super::*;

    #[derive(FromRow)]
    struct PolicyRow {
        node: String,
        resource: String,
        action: String,
        expression: Vec<u8>,
    }

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let pool = SqlxDatabase::create_connection_pool(db_file.path()).await?;
        NodesMigration.migrate_schema(&pool).await?;

        // Add data to the old table before applying the rust migration
        let insert = insert_node("n1".to_string());
        insert.execute(&pool).await.void()?;
        let insert = insert_node("n2".to_string());
        insert.execute(&pool).await.void()?;
        let insert = insert_legacy_policy("r1", "a1", minicbor::to_vec("e1")?);
        insert.execute(&pool).await.void()?;
        let insert = insert_legacy_policy("r2", "a2", minicbor::to_vec("e2")?);
        insert.execute(&pool).await.void()?;

        // now create a database and apply the migrations
        let db = SqlxDatabase::create(db_file.path()).await?;
        for node in &["n1", "n2"] {
            let rows: Vec<PolicyRow> =
                query_as("SELECT node, resource, action, expression FROM policy WHERE node = ?")
                    .bind(node.to_sql())
                    .fetch_all(&*db.pool)
                    .await
                    .into_core()?;
            assert_eq!(rows.len(), 2);
            assert_eq!(&rows[0].node, node);
            assert_eq!(rows[0].resource, "r1");
            assert_eq!(rows[0].action, "a1");
            assert!(!rows[0].expression.is_empty());
            assert_eq!(&rows[1].node, node);
            assert_eq!(rows[1].resource, "r2");
            assert_eq!(rows[1].action, "a2");
            assert!(!rows[1].expression.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_migration_happens_only_once() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db = SqlxDatabase::create_no_migration(db_file.path()).await?;
        NodesMigration.migrate_schema(&db.pool).await?;

        let migrated = PoliciesByNode::migrate_policies(&db.pool).await?;
        assert!(migrated);

        let migrated = PoliciesByNode::migrate_policies(&db.pool).await?;
        assert!(!migrated);

        Ok(())
    }

    /// HELPERS
    fn insert_node(name: String) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES (?, ?, ?, ?, ?)")
            .bind(name.to_sql())
            .bind("I_TEST".to_string().to_sql())
            .bind(1.to_sql())
            .bind(0.to_sql())
            .bind(false.to_sql())
    }

    fn insert_legacy_policy(
        resource: &str,
        action: &str,
        expression: Vec<u8>,
    ) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO policy_old (resource, action, expression) VALUES (?, ?, ?)")
            .bind(resource.to_sql())
            .bind(action.to_sql())
            .bind(expression.to_sql())
    }
}
