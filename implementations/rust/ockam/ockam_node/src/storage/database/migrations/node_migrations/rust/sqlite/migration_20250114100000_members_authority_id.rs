use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToVoid};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration sets the authority id for existing members
#[derive(Debug)]
pub struct SetAuthorityId;

#[async_trait]
impl RustMigration for SetAuthorityId {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> i64 {
        Self::version()
    }

    async fn migrate(&self, connection: &mut AnyConnection) -> Result<bool> {
        Self::set_authority_id(connection).await
    }
}

impl SetAuthorityId {
    /// Migration version
    pub fn version() -> i64 {
        20250114100000
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20250114100000_members_authority_id"
    }

    pub(crate) async fn set_authority_id(connection: &mut AnyConnection) -> Result<bool> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;
        let authority_id: Option<String> =
            query("SELECT identifier FROM node WHERE name = 'authority'")
                .fetch_optional(&mut *transaction)
                .await
                .into_core()?
                .map(|r| r.get(0));

        if let Some(authority_id) = authority_id {
            query("UPDATE authority_member SET authority_id = $1")
                .bind(authority_id)
                .execute(&mut *transaction)
                .await
                .void()?;
        }

        // Commit
        transaction.commit().await.void()?;

        Ok(true)
    }
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseType, MigrationSet, SqlxDatabase};
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
            .migrate_up_to_skip_last_rust_migration(&pool, SetAuthorityId::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // set an identifier for the authority node
        let authority_id = "authority_id_123";
        insert_node(authority_id)
            .execute(&mut *connection)
            .await
            .void()?;

        // insert some members
        let member1 = insert_member("1");
        let member2 = insert_member("2");
        member1.execute(&mut *connection).await.void()?;
        member2.execute(&mut *connection).await.void()?;

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, SetAuthorityId::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check that the update was successful for members
        let rows: Vec<AnyRow> = query("SELECT authority_id FROM authority_member")
            .fetch_all(&mut *connection)
            .await
            .into_core()?;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|e| {
            let s: String = e.get(0);
            assert_eq!(s, authority_id);
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
    fn insert_member(identifier: &str) -> Query<'_, Any, AnyArguments<'_>> {
        query("INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes) VALUES ($1, $2, $3, $4, $5)")
            .bind(identifier)
            .bind("someone")
            .bind(0)
            .bind(0)
            .bind("")
    }

    fn insert_node(identifier: &str) -> Query<'_, Any, AnyArguments<'_>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES ($1, $2, $3, $4, $5)")
            .bind("authority")
            .bind(identifier)
            .bind(1)
            .bind(0)
            .bind(0)
    }
}
