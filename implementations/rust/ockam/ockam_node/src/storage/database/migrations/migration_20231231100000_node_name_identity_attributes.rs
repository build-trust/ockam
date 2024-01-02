use crate::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::Result;
use sqlx::sqlite::SqliteRow;
use sqlx::*;

impl SqlxDatabase {
    /// Duplicate all attributes entry for every known node
    pub(crate) async fn migrate_attributes_node_name(&self) -> Result<()> {
        // don't run the migration twice
        let data_migration_needed: Option<SqliteRow> =
            query(&Self::table_exists("identity_attributes_old"))
                .fetch_optional(&*self.pool)
                .await
                .into_core()?;
        let data_migration_needed = data_migration_needed.map(|r| r.get(0)).unwrap_or(false);

        if !data_migration_needed {
            return Ok(());
        };

        let mut transaction = self.pool.begin().await.into_core()?;

        let query_node_names = query_as("SELECT name FROM node");
        let node_names: Vec<NodeNameRow> = query_node_names
            .fetch_all(&mut *transaction)
            .await
            .into_core()?;

        // read values from the previous table
        let query_all = query_as("SELECT * FROM identity_attributes_old");
        let rows: Vec<IdentityAttributesRow> =
            query_all.fetch_all(&mut *transaction).await.into_core()?;

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

        Ok(())
    }

    fn table_exists(table_name: &str) -> String {
        format!("SELECT EXISTS(SELECT name FROM sqlite_schema WHERE type = 'table' AND name = '{table_name}')")
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
        SqlxDatabase::migrate_tables(&pool).await?;

        // insert attribute rows in the previous table
        let attributes = create_attributes("identifier1")?;
        let insert = insert_query("identifier1", attributes.clone());
        insert.execute(&pool).await.void()?;

        let insert_node1 = insert_node("node1".to_string());
        insert_node1.execute(&pool).await.void()?;

        let insert_node2 = insert_node("node2".to_string());
        insert_node2.execute(&pool).await.void()?;

        // now create a database and apply the migrations
        let db = SqlxDatabase::create(db_file.path()).await?;
        let rows1: Vec<IdentityAttributesRow> =
            query_as("SELECT * FROM identity_attributes WHERE node_name = ?")
                .bind("node1".to_string().to_sql())
                .fetch_all(&*db.pool)
                .await
                .into_core()?;
        let rows2: Vec<IdentityAttributesRow> =
            query_as("SELECT * FROM identity_attributes WHERE node_name = ?")
                .bind("node2".to_string().to_sql())
                .fetch_all(&*db.pool)
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
        Ok(minicbor::to_vec(BTreeMap::from([
            ("name".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
            ("age".as_bytes().to_vec(), identifier.as_bytes().to_vec()),
        ]))?)
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
