use crate::database::{FromSqlxError, RustMigration, ToSqlxType, ToVoid};
use ockam_core::{async_trait, Result};
use sqlx::sqlite::SqliteRow;
use sqlx::*;

/// This struct adds a node name column to the identity attributes table
#[derive(Debug)]
pub struct NodeNameIdentityAttributes;

#[async_trait]
impl RustMigration for NodeNameIdentityAttributes {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> i64 {
        Self::version()
    }

    async fn migrate(&self, connection: &mut SqliteConnection) -> Result<bool> {
        Self::migrate_attributes_node_name(connection).await
    }
}

impl NodeNameIdentityAttributes {
    /// Migration version
    pub fn version() -> i64 {
        20231231100000
    }

    /// Migration name
    pub fn name() -> &'static str {
        "migration_20231231100000_node_name_identity_attributes"
    }

    fn table_exists(table_name: &str) -> String {
        format!("SELECT EXISTS(SELECT name FROM sqlite_schema WHERE type = 'table' AND name = '{table_name}')")
    }

    /// Duplicate all attributes entry for every known node
    pub(crate) async fn migrate_attributes_node_name(
        connection: &mut SqliteConnection,
    ) -> Result<bool> {
        // don't run the migration twice
        let data_migration_needed: Option<SqliteRow> =
            query(&Self::table_exists("identity_attributes_old"))
                .fetch_optional(&mut *connection)
                .await
                .into_core()?;
        let data_migration_needed = data_migration_needed.map(|r| r.get(0)).unwrap_or(false);

        if !data_migration_needed {
            // Trigger marking as migrated
            return Ok(true);
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
                let insert = query("INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(row.identifier.to_sql())
                    .bind(row.attributes.to_sql())
                    .bind((row.added as u64).to_sql())
                    .bind(row.expires.map(|e| (e as u64).to_sql()))
                    .bind(row.attested_by.clone().map(|e| e.to_sql()))
                    .bind(node_name.name.to_sql());

                insert.execute(&mut *transaction).await.void()?;
            }
        }

        // finally drop the old table
        query("DROP TABLE identity_attributes_old")
            .execute(&mut *transaction)
            .await
            .void()?;

        transaction.commit().await.void()?;

        Ok(true)
    }
}

// Low-level representation of a table row before data migration
#[derive(FromRow)]
struct IdentityAttributesRow {
    identifier: String,
    attributes: Vec<u8>,
    added: i64,
    expires: Option<i64>,
    attested_by: Option<String>,
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
}

#[cfg(test)]
mod test {
    use crate::database::migrations::node_migration_set::NodeMigrationSet;
    use crate::database::{MigrationSet, SqlxDatabase};
    use sqlx::query::Query;
    use sqlx::sqlite::SqliteArguments;
    use std::collections::BTreeMap;
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
            .migrate_up_to_skip_last_rust_migration(&pool, NodeNameIdentityAttributes::version())
            .await?;

        // insert attribute rows in the previous table
        let attributes = create_attributes("identifier1")?;
        let insert = insert_query("identifier1", attributes.clone());
        insert.execute(&mut *connection).await.void()?;

        let insert_node1 = insert_node("node1".to_string());
        insert_node1.execute(&mut *connection).await.void()?;

        let insert_node2 = insert_node("node2".to_string());
        insert_node2.execute(&mut *connection).await.void()?;

        // apply migrations
        NodeMigrationSet
            .create_migrator()?
            .migrate_up_to(&pool, NodeNameIdentityAttributes::version())
            .await?;

        // check data
        let rows1: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes WHERE node_name = ?")
                .bind("node1".to_string().to_sql())
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        let rows2: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes WHERE node_name = ?")
                .bind("node2".to_string().to_sql())
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
        assert_eq!(row1.expires, Some(2));
        assert_eq!(row1.attested_by, Some("authority".to_string()));

        Ok(())
    }

    /// HELPERS
    fn create_attributes(identifier: &str) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(BTreeMap::from([
            ("name".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
            ("age".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
        ]))
    }

    fn insert_query(identifier: &str, attributes: Vec<u8>) -> Query<Sqlite, SqliteArguments> {
        query("INSERT INTO identity_attributes_old VALUES (?, ?, ?, ?, ?)")
            .bind(identifier.to_sql())
            .bind(attributes.to_sql())
            .bind(1.to_sql())
            .bind(Some(2).map(|e| e.to_sql()))
            .bind(Some("authority").map(|e| e.to_sql()))
    }

    fn insert_node(name: String) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES (?, ?, ?, ?, ?)")
            .bind(name.to_sql())
            .bind("I_TEST".to_string().to_sql())
            .bind(1.to_sql())
            .bind(0.to_sql())
            .bind(0.to_sql())
    }
}
