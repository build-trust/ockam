use crate::database::migrations::RustMigration;
use crate::database::{FromSqlxError, ToSqlxType, ToVoid};
use ockam_core::{async_trait, Result};
use sqlx::*;

/// This migration moves attributes from identity_attributes to the authority_member table for authority nodes
#[derive(Debug)]
pub struct AuthorityAttributes;

#[async_trait]
impl RustMigration for AuthorityAttributes {
    fn name(&self) -> &str {
        Self::name()
    }

    fn version(&self) -> i64 {
        Self::version()
    }

    async fn migrate(&self, connection: &mut SqliteConnection) -> Result<bool> {
        Self::migrate_authority_attributes_to_members(connection).await
    }
}

impl AuthorityAttributes {
    /// Migration version
    pub fn version() -> i64 {
        20240111100001
    }

    /// Migration name
    pub fn name() -> &'static str {
        // Incorrect format, but left like this to not break existing nodes
        "20240111100001_add_authority_tables"
    }

    /// Duplicate all attributes entry for every known node
    pub(crate) async fn migrate_authority_attributes_to_members(
        connection: &mut SqliteConnection,
    ) -> Result<bool> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        let query_node_names = query_as("SELECT name, is_authority FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        for node_name in node_names.into_iter().filter(|n| n.is_authority) {
            let rows: Vec<IdentityAttributesRow> =
                query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name=?")
                    .bind(node_name.name.to_sql())
                    .fetch_all(&mut *transaction)
                    .await
                    .into_core()?;

            for row in rows {
                let insert = query("INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes) VALUES (?, ?, ?, ?, ?)")
                        .bind(row.identifier.to_sql())
                        .bind(row.attested_by.clone().map(|e| e.to_sql()))
                        .bind((row.added as u64).to_sql())
                        .bind(0.to_sql())
                        .bind(row.attributes.to_sql());

                insert.execute(&mut *transaction).await.void()?;
            }

            query("DELETE FROM identity_attributes WHERE node_name=?")
                .bind(node_name.name.to_sql())
                .execute(&mut *transaction)
                .await
                .void()?;
        }

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
    attested_by: Option<String>,
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
    is_authority: bool,
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

    #[derive(FromRow)]
    struct MemberRow {
        identifier: String,
        attributes: Vec<u8>,
        added_by: Option<String>,
        added_at: i64,
        is_pre_trusted: bool,
    }

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let pool = SqlxDatabase::create_connection_pool(db_file.path()).await?;

        let mut connection = pool.acquire().await.into_core()?;

        NodeMigrationSet
            .create_migrator()?
            .migrate_up_to_skip_last_rust_migration(&pool, AuthorityAttributes::version())
            .await?;

        let authority_node_name = "authority".to_string();
        let regular_node_name = "node".to_string();

        let insert_node1 = insert_node(authority_node_name.clone(), true);
        insert_node1.execute(&mut *connection).await.void()?;
        let insert_node2 = insert_node(regular_node_name.clone(), false);
        insert_node2.execute(&mut *connection).await.void()?;

        let attributes1 = create_attributes(vec![(
            "name".as_bytes().to_vec(),
            "John".as_bytes().to_vec(),
        )])?;
        let insert = insert_query(
            "identifier1",
            attributes1.clone(),
            regular_node_name.clone(),
        );
        insert.execute(&mut *connection).await.void()?;

        let attributes2 =
            create_attributes(vec![("age".as_bytes().to_vec(), "29".as_bytes().to_vec())])?;
        let insert = insert_query(
            "identifier1",
            attributes2.clone(),
            authority_node_name.clone(),
        );
        insert.execute(&mut *connection).await.void()?;

        // apply migrations
        NodeMigrationSet
            .create_migrator()?
            .migrate_up_to(&pool, AuthorityAttributes::version())
            .await?;

        // check data
        let rows1: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name = ?")
                .bind(regular_node_name.to_sql())
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows1.len(), 1);
        assert_eq!(rows1[0].attributes, attributes1);

        let rows2: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name = ?")
                .bind(authority_node_name.to_sql())
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows2.len(), 0);

        let rows3: Vec<MemberRow> =
            query_as("SELECT identifier, attributes, added_by, added_at, is_pre_trusted FROM authority_member")
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        let member = &rows3[0];

        assert_eq!(member.identifier, "identifier1".to_string());
        assert_eq!(member.added_by, Some("authority_id".to_string()));
        assert_eq!(member.added_at, 1);
        assert!(!member.is_pre_trusted);
        assert_eq!(member.attributes, attributes2);

        Ok(())
    }

    /// HELPERS
    fn create_attributes(attributes: Vec<(Vec<u8>, Vec<u8>)>) -> Result<Vec<u8>> {
        let map: BTreeMap<Vec<u8>, Vec<u8>> = attributes.into_iter().collect();
        ockam_core::cbor_encode_preallocate(map)
    }

    fn insert_query(
        identifier: &str,
        attributes: Vec<u8>,
        node_name: String,
    ) -> Query<Sqlite, SqliteArguments> {
        query("INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES (?, ?, ?, ?, ?, ?)")
            .bind(identifier.to_sql())
            .bind(attributes.to_sql())
            .bind(1.to_sql())
            .bind(Some(2).map(|e| e.to_sql()))
            .bind(Some("authority_id").map(|e| e.to_sql()))
            .bind(node_name.to_sql())
    }

    fn insert_node(
        name: String,
        is_authority: bool,
    ) -> Query<'static, Sqlite, SqliteArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES (?, ?, ?, ?, ?)")
            .bind(name.to_sql())
            .bind("I_TEST".to_string().to_sql())
            .bind(1.to_sql())
            .bind(0.to_sql())
            .bind(is_authority.to_sql())
    }
}
