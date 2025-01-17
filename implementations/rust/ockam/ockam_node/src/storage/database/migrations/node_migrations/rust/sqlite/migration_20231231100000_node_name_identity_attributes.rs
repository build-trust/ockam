use crate::database::{
    Boolean, FromSqlxError, Nullable, RustMigration, SqlxDatabase, ToVoid, Version,
};
use ockam_core::{async_trait, Result};
use sqlx::any::AnyRow;
use sqlx::*;

/// This struct adds a node name column to the identity attributes table
#[derive(Debug)]
pub struct NodeNameIdentityAttributes;

#[async_trait]
impl RustMigration for NodeNameIdentityAttributes {
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
        Self::migrate_attributes_node_name(connection).await
    }
}

impl NodeNameIdentityAttributes {
    /// Migration version
    pub fn version() -> Version {
        Version(20231231100000)
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20231231100000_node_name_identity_attributes"
    }

    fn table_exists(table_name: &str) -> String {
        format!("SELECT EXISTS(SELECT name FROM sqlite_schema WHERE type = 'table' AND name = '{table_name}')")
    }

    /// Duplicate all attributes entry for every known node
    pub(crate) async fn migrate_attributes_node_name(connection: &mut AnyConnection) -> Result<()> {
        // don't run the migration twice
        let data_migration_needed: Option<AnyRow> =
            query(&Self::table_exists("identity_attributes_old"))
                .fetch_optional(&mut *connection)
                .await
                .into_core()?;
        let data_migration_needed = data_migration_needed
            .map(|r| r.get::<Boolean, usize>(0).to_bool())
            .unwrap_or(false);

        if !data_migration_needed {
            // Trigger marking as migrated
            return Ok(());
        };

        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        let query_node_names = query_as("SELECT name FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        // read values from the previous table
        let rows: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes_old").fetch_all(&mut *transaction).await.into_core()?;

        for row in rows {
            for node_name in &node_names {
                let insert = query("INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES ($1, $2, $3, $4, $5, $6)")
                    .bind(&row.identifier)
                    .bind(&row.attributes)
                    .bind(row.added)
                    .bind(row.expires.to_option())
                    .bind(row.attested_by.to_option())
                    .bind(&node_name.name);

                insert.execute(&mut *transaction).await.void()?;
            }
        }

        // finally drop the old table
        query("DROP TABLE identity_attributes_old")
            .execute(&mut *transaction)
            .await
            .void()?;

        transaction.commit().await.void()?;

        Ok(())
    }
}

// Low-level representation of a table row before data migration
#[derive(FromRow)]
struct IdentityAttributesRow {
    identifier: String,
    attributes: Vec<u8>,
    added: i64,
    expires: Nullable<i64>,
    attested_by: Nullable<String>,
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{DatabaseType, MigrationSet, SqlxDatabase};
    use sqlx::any::AnyArguments;
    use sqlx::query::Query;
    use std::collections::BTreeMap;
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
            .migrate_up_to_skip_last_rust_migration(&pool, NodeNameIdentityAttributes::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // insert attribute rows in the previous table
        let attributes = create_attributes("identifier1")?;
        let insert = insert_query("identifier1", attributes.clone());
        insert.execute(&mut *connection).await.void()?;

        let insert_node1 = insert_node("node1".to_string());
        insert_node1.execute(&mut *connection).await.void()?;

        let insert_node2 = insert_node("node2".to_string());
        insert_node2.execute(&mut *connection).await.void()?;

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, NodeNameIdentityAttributes::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check data
        let rows1: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes WHERE node_name = $1")
                .bind("node1".to_string())
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        let rows2: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes WHERE node_name = $1")
                .bind("node2".to_string())
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows1.len(), 1);

        let row1 = &rows1[0];
        let row2 = &rows2[0];

        assert_eq!(row1.identifier, row2.identifier);
        assert_eq!(row1.attributes, row2.attributes);
        assert_eq!(row1.added, row2.added);
        assert_eq!(row1.expires, row2.expires);
        assert_eq!(row1.attested_by, row2.attested_by);

        assert_eq!(row1.identifier, "identifier1");
        assert_eq!(row1.attributes, attributes);
        assert_eq!(row1.added, 1);
        assert_eq!(row1.expires.to_option(), Some(2));
        assert_eq!(row1.attested_by.to_option(), Some("authority".to_string()));

        Ok(())
    }

    /// HELPERS
    fn create_attributes(identifier: &str) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(BTreeMap::from([
            ("name".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
            ("age".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
        ]))
    }

    fn insert_query(identifier: &str, attributes: Vec<u8>) -> Query<Any, AnyArguments> {
        query("INSERT INTO identity_attributes_old VALUES ($1, $2, $3, $4, $5)")
            .bind(identifier)
            .bind(attributes)
            .bind(1)
            .bind(Some(2))
            .bind(Some("authority"))
    }

    fn insert_node(name: String) -> Query<'static, Any, AnyArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES ($1, $2, $3, $4, $5)")
            .bind(name)
            .bind("I_TEST".to_string())
            .bind(1)
            .bind(0)
            .bind(0)
    }
}
