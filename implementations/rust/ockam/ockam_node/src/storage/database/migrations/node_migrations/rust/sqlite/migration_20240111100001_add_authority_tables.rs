use crate::database::migrations::RustMigration;
use crate::database::{Boolean, FromSqlxError, Nullable, SqlxDatabase, ToVoid, Version};
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

    fn version(&self) -> Version {
        Self::version()
    }

    async fn migrate(
        &self,
        _legacy_sqlite_database: Option<SqlxDatabase>,
        connection: &mut AnyConnection,
    ) -> Result<()> {
        Self::migrate_authority_attributes_to_members(connection).await
    }
}

impl AuthorityAttributes {
    /// Migration version
    pub fn version() -> Version {
        Version(20240111100001)
    }

    /// Migration name
    pub fn name() -> &'static str {
        // Incorrect format, but left like this to not break existing nodes
        "20240111100001_add_authority_tables"
    }

    /// Duplicate all attributes entry for every known node
    pub(crate) async fn migrate_authority_attributes_to_members(
        connection: &mut AnyConnection,
    ) -> Result<()> {
        let mut transaction = Connection::begin(&mut *connection).await.into_core()?;

        let query_node_names = query_as("SELECT name, is_authority FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        for node_name in node_names.into_iter().filter(|n| n.is_authority.to_bool()) {
            let rows: Vec<IdentityAttributesRow> =
                query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name = $1")
                    .bind(node_name.name.clone())
                    .fetch_all(&mut *transaction)
                    .await
                    .into_core()?;

            for row in rows {
                let insert = query("INSERT INTO authority_member (identifier, added_by, added_at, is_pre_trusted, attributes) VALUES ($1, $2, $3, $4, $5)")
                        .bind(row.identifier)
                        .bind(row.attested_by.to_option())
                        .bind(row.added)
                        .bind(0)
                        .bind(row.attributes);

                insert.execute(&mut *transaction).await.void()?;
            }

            query("DELETE FROM identity_attributes WHERE node_name = $1")
                .bind(node_name.name.clone())
                .execute(&mut *transaction)
                .await
                .void()?;
        }

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
    attested_by: Nullable<String>,
}

#[derive(FromRow)]
struct NodeNameRow {
    name: String,
    is_authority: Boolean,
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

    #[derive(FromRow)]
    struct MemberRow {
        identifier: String,
        attributes: Vec<u8>,
        added_by: Nullable<String>,
        added_at: i64,
        is_pre_trusted: Boolean,
    }

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        let db_file = NamedTempFile::new().unwrap();
        let db_file = db_file.path();
        let pool = SqlxDatabase::create_sqlite_single_connection_pool(db_file).await?;
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to_skip_last_rust_migration(&pool, AuthorityAttributes::version())
            .await?;

        let authority_node_name = "authority".to_string();
        let regular_node_name = "node".to_string();

        let mut connection = pool.acquire().await.into_core()?;
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

        // SQLite EXCLUSIVE lock needs exactly one connection during the migration
        drop(connection);

        // apply migrations
        NodeMigrationSet::new(DatabaseType::Sqlite)
            .create_migrator()?
            .migrate_up_to(&pool, AuthorityAttributes::version())
            .await?;

        let mut connection = pool.acquire().await.into_core()?;

        // check data
        let rows1: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name = $1")
                .bind(regular_node_name)
                .fetch_all(&mut *connection)
                .await
                .into_core()?;
        assert_eq!(rows1.len(), 1);
        assert_eq!(rows1[0].attributes, attributes1);

        let rows2: Vec<IdentityAttributesRow> =
            query_as("SELECT identifier, attributes, added, attested_by FROM identity_attributes WHERE node_name = $1")
                .bind(authority_node_name)
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
        assert_eq!(
            member.added_by.to_option(),
            Some("authority_id".to_string())
        );
        assert_eq!(member.added_at, 1);
        assert!(!member.is_pre_trusted.to_bool());
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
    ) -> Query<Any, AnyArguments> {
        query("INSERT INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(identifier)
            .bind(attributes)
            .bind(1)
            .bind(Some(2))
            .bind(Some("authority_id"))
            .bind(node_name)
    }

    fn insert_node(name: String, is_authority: bool) -> Query<'static, Any, AnyArguments<'static>> {
        query("INSERT INTO node (name, identifier, verbosity, is_default, is_authority) VALUES ($1, $2, $3, $4, $5)")
            .bind(name)
            .bind("I_TEST".to_string())
            .bind(1)
            .bind(0)
            .bind(is_authority)
    }
}
