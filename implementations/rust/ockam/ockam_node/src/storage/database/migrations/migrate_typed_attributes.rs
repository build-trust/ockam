use crate::database::{FromSqlxError, SqlxDatabase, ToSqlxType, ToVoid};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use sqlx::sqlite::SqliteRow;
use sqlx::*;
use std::collections::BTreeMap;

impl SqlxDatabase {
    /// Read attributes as a serialized BTreeMap<Vec<u8>, Vec<u8>> from the previous table
    /// and copy them to the new table with one attribute name / attribute value per row
    pub(crate) async fn migrate_typed_attributes(&self) -> Result<()> {
        // don't run the migration twice
        let data_migration_needed: Option<SqliteRow> =
            query(&Self::table_exists("identity_attributes_old"))
                .fetch_optional(&self.pool)
                .await
                .into_core()?;
        let data_migration_needed = data_migration_needed.map(|r| r.get(0)).unwrap_or(false);

        if !data_migration_needed {
            return Ok(());
        };

        let mut transaction = self.pool.begin().await.into_core()?;
        // read values from the previous table
        let query_all = query_as("SELECT * FROM identity_attributes_old");
        let rows: Vec<IdentityAttributesRow> =
            query_all.fetch_all(&mut *transaction).await.into_core()?;

        for row in rows {
            // then re-insert each attribute name / attribute value pair as a distinct row
            // in the new table
            for (attribute_name, attribute_value) in row.attributes()? {
                let insert = query("INSERT INTO identity_attributes VALUES (?, ?, ?, ?, ?, ?)")
                    .bind(row.identifier.to_sql())
                    .bind(attribute_name.to_sql())
                    .bind(attribute_value.to_sql())
                    .bind(row.added.to_sql())
                    .bind(row.expires.map(|e| e.to_sql()))
                    .bind(row.attested_by.clone().map(|e| e.to_sql()));
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

impl IdentityAttributesRow {
    fn attributes(&self) -> Result<BTreeMap<String, String>> {
        let map: BTreeMap<Vec<u8>, Vec<u8>> =
            minicbor::decode(self.attributes.as_slice()).map_err(SqlxDatabase::map_decode_err)?;
        let mut result = BTreeMap::new();
        for (name, value) in map {
            result.insert(
                Self::deserialize_string(name)?,
                Self::deserialize_string(value)?,
            );
        }
        Ok(result)
    }

    fn deserialize_string(string: Vec<u8>) -> Result<String> {
        String::from_utf8(string)
            .map_err(|e| ockam_core::Error::new(Origin::Core, Kind::Serialization, e.to_string()))
    }
}

#[cfg(test)]
mod test {
    use sqlx::query::Query;
    use sqlx::sqlite::SqliteArguments;
    use tempfile::NamedTempFile;

    use super::*;

    #[tokio::test]
    async fn test_migration() -> Result<()> {
        // create the database pool and migrate the tables
        let db_file = NamedTempFile::new().unwrap();
        let pool = SqlxDatabase::create_connection_pool(db_file.path()).await?;
        SqlxDatabase::migrate_tables(&pool).await?;

        // insert attribute rows in the previous table
        let attributes1 = create_attributes("identifier1")?;
        let insert1 = insert_query("identifier1", attributes1);
        insert1.execute(&pool).await.void()?;

        let insert2 = insert_query("identifier2", create_attributes("identifier2")?);
        insert2.execute(&pool).await.void()?;

        // now create a database and apply the migrations
        let db = SqlxDatabase::create(db_file.path()).await?;
        let rows: Vec<IdentityNewAttributesRow> =
            query_as("SELECT * FROM identity_attributes ORDER BY identifier, attribute_name")
                .fetch_all(&db.pool)
                .await
                .into_core()?;
        assert_eq!(rows.len(), 4);
        assert_eq!(
            rows,
            vec![
                expected_row("identifier1", "age", "identifier1"),
                expected_row("identifier1", "name", "identifier1"),
                expected_row("identifier2", "age", "identifier2"),
                expected_row("identifier2", "name", "identifier2"),
            ]
        );

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

    fn expected_row(
        identifier: &str,
        attribute_name: &str,
        attribute_value: &str,
    ) -> IdentityNewAttributesRow {
        IdentityNewAttributesRow {
            identifier: identifier.to_string(),
            attribute_name: attribute_name.to_string(),
            attribute_value: attribute_value.to_string(),
            added: 1,
            expires: Some(2),
            attested_by: Some("authority".to_string()),
        }
    }

    #[derive(FromRow, PartialEq, Eq, Debug)]
    struct IdentityNewAttributesRow {
        identifier: String,
        attribute_name: String,
        attribute_value: String,
        added: i64,
        expires: Option<i64>,
        attested_by: Option<String>,
    }
}
