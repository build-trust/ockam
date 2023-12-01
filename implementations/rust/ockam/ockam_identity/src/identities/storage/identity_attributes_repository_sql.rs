use core::str::FromStr;
use std::collections::BTreeMap;

use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::collections::HashMap;
use ockam_core::compat::sync::Arc;
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::models::Identifier;
use crate::utils::now;
use crate::{
    AttributeName, AttributeValue, AttributesEntry, IdentityAttributesRepository,
    TimestampInSeconds,
};

/// Implementation of `IdentitiesRepository` trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct IdentityAttributesSqlxDatabase {
    database: Arc<SqlxDatabase>,
}

impl IdentityAttributesSqlxDatabase {
    /// Create a new database
    pub fn new(database: Arc<SqlxDatabase>) -> Self {
        debug!("create a repository for identity attributes");
        Self { database }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            SqlxDatabase::in_memory("identity attributes").await?,
        )))
    }
}

#[async_trait]
impl IdentityAttributesRepository for IdentityAttributesSqlxDatabase {
    async fn get_attributes(&self, identity: &Identifier) -> Result<Option<AttributesEntry>> {
        let query =
            query_as("SELECT * FROM identity_attributes WHERE identifier=$1 ORDER BY added DESC")
                .bind(identity.to_sql());
        let rows: Vec<IdentityAttributesRow> =
            query.fetch_all(&self.database.pool).await.into_core()?;
        let mut attributes_entry: Option<AttributesEntry> = None;
        for row in rows {
            if let Some(entry) = &mut attributes_entry {
                entry.insert(row.attribute_name()?, row.attribute_value()?);
            } else {
                attributes_entry = Some(row.attributes()?)
            }
        }

        Ok(attributes_entry)
    }

    async fn list_attributes_by_identifier(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        let query = query_as("SELECT * FROM identity_attributes ORDER BY added DESC");
        let rows: Vec<IdentityAttributesRow> =
            query.fetch_all(&self.database.pool).await.into_core()?;
        let mut attributes_entries: HashMap<Identifier, AttributesEntry> = HashMap::default();
        for row in rows {
            let identifier = row.identifier()?;
            if let Some(entry) = attributes_entries.get_mut(&identifier) {
                entry.insert(row.attribute_name()?, row.attribute_value()?);
            } else {
                attributes_entries.insert(identifier, row.attributes()?);
            };
        }
        let mut result: Vec<(Identifier, AttributesEntry)> =
            attributes_entries.into_iter().collect();
        result.sort_by_key(|k| k.0.clone());
        Ok(result)
    }

    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        let transaction = self.database.begin().await.into_core()?;
        for (attribute_name, attribute_value) in entry.iter() {
            self.execute_insert_query(
                subject,
                entry.expires(),
                entry.attested_by(),
                entry.added(),
                attribute_name.clone(),
                attribute_value.clone(),
            )
            .await?
        }

        transaction.commit().await.void()
    }

    /// Store an attribute name/value pair for a given identity
    /// The attribute is self-attested
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: AttributeName,
        attribute_value: AttributeValue,
    ) -> Result<()> {
        self.execute_insert_query(
            subject,
            None,
            Some(subject.clone()),
            now()?,
            attribute_name,
            attribute_value,
        )
        .await
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        let query =
            query("DELETE FROM identity_attributes WHERE identifier = ?").bind(identity.to_sql());
        query.execute(&self.database.pool).await.void()
    }
}

impl IdentityAttributesSqlxDatabase {
    async fn execute_insert_query(
        &self,
        subject: &Identifier,
        expires: Option<TimestampInSeconds>,
        attested_by: Option<Identifier>,
        created_at: TimestampInSeconds,
        attribute_name: AttributeName,
        attribute_value: AttributeValue,
    ) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO identity_attributes VALUES (?, ?, ?, ?, ?, ?)")
            .bind(subject.to_sql())
            .bind(attribute_name.to_sql())
            .bind(attribute_value.to_sql())
            .bind(created_at.to_sql())
            .bind(expires.map(|e| e.to_sql()))
            .bind(attested_by.map(|e| e.to_sql()));
        query.execute(&self.database.pool).await.void()
    }
}

// Database serialization / deserialization

impl ToSqlxType for TimestampInSeconds {
    fn to_sql(&self) -> SqlxType {
        self.0.to_sql()
    }
}

impl ToSqlxType for AttributeName {
    fn to_sql(&self) -> SqlxType {
        self.to_string().to_sql()
    }
}

impl ToSqlxType for AttributeValue {
    fn to_sql(&self) -> SqlxType {
        self.encode_to_string().unwrap().to_sql()
    }
}

// Low-level representation of a table row
#[derive(FromRow)]
struct IdentityAttributesRow {
    identifier: String,
    attribute_name: String,
    attribute_value: String,
    added: i64,
    expires: Option<i64>,
    attested_by: Option<String>,
}

impl IdentityAttributesRow {
    fn identifier(&self) -> Result<Identifier> {
        Identifier::from_str(&self.identifier)
    }

    fn attribute_name(&self) -> Result<AttributeName> {
        Ok(AttributeName::Str(self.attribute_name.clone()))
    }

    fn attribute_value(&self) -> Result<AttributeValue> {
        AttributeValue::decode_from_string(self.attribute_value.as_str())
            .map_err(|e| ockam_core::Error::new(Origin::Core, Kind::Serialization, e.to_string()))
    }

    fn attributes(&self) -> Result<AttributesEntry> {
        let mut attributes = BTreeMap::new();
        attributes.insert(self.attribute_name()?, self.attribute_value()?);
        let added = TimestampInSeconds(self.added as u64);
        let expires = self.expires.map(|v| TimestampInSeconds(v as u64));
        let attested_by = self
            .attested_by
            .clone()
            .map(|v| Identifier::from_str(&v))
            .transpose()?;

        Ok(AttributesEntry::new(
            attributes,
            added,
            expires,
            attested_by,
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::identities;
    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn test_identities_attributes_repository() -> Result<()> {
        let repository = create_repository().await?;

        // store and retrieve attributes by identity
        let identifier1 = create_identity().await?;
        let attributes1 = create_attributes_entry(&identifier1).await?;
        let identifier2 = create_identity().await?;
        let attributes2 = create_attributes_entry(&identifier2).await?;

        repository
            .put_attributes(&identifier1, attributes1.clone())
            .await?;
        repository
            .put_attributes(&identifier2, attributes2.clone())
            .await?;

        let result = repository.get_attributes(&identifier1).await?;
        assert_eq!(result, Some(attributes1.clone()));

        let result = repository.list_attributes_by_identifier().await?;
        let mut expected = vec![
            (identifier1.clone(), attributes1.clone()),
            (identifier2.clone(), attributes2.clone()),
        ];
        expected.sort_by_key(|k| k.0.clone());

        assert_eq!(result, expected);

        // delete attributes
        repository.delete(&identifier1).await?;
        let result = repository.get_attributes(&identifier1).await?;
        assert_eq!(result, None);

        // store just one attribute name / value
        let before_adding = now()?;
        repository
            .put_attribute_value(&identifier1, "name".into(), "value".into())
            .await?;

        let result = repository.get_attributes(&identifier1).await?.unwrap();
        // the name/value pair is present
        assert_eq!(result.get("name".into()), Some("value".into()));
        // there is a timestamp showing when the attributes have been added
        assert!(result.added() >= before_adding);

        // the attributes are self-attested
        assert_eq!(result.attested_by(), Some(identifier1.clone()));

        // store one more attribute name / value
        // Let time pass for bit to observe a timestamp update
        // We need to wait at least one second since this is the granularity of the
        // timestamp for tracking attributes
        tokio::time::sleep(Duration::from_millis(1100)).await;
        repository
            .put_attribute_value(&identifier1, "name2".into(), "value2".into())
            .await?;

        let result2 = repository.get_attributes(&identifier1).await?.unwrap();

        // both the new and the old name/value pairs are present
        assert_eq!(result2.get("name".into()), Some("value".into()));
        assert_eq!(result2.get("name2".into()), Some("value2".into()));
        // The original timestamp has been updated
        assert!(
            result2.added() > result.added(),
            "before {:?}, after {:?}",
            result.added(),
            result2.added()
        );

        // the attributes are still self-attested
        assert_eq!(result2.attested_by(), Some(identifier1.clone()));
        Ok(())
    }

    /// HELPERS
    async fn create_attributes_entry(identifier: &Identifier) -> Result<AttributesEntry> {
        Ok(AttributesEntry::new(
            BTreeMap::from([("name".into(), "alice".into()), ("age".into(), "20".into())]),
            TimestampInSeconds(1000),
            Some(TimestampInSeconds(2000)),
            Some(identifier.clone()),
        ))
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }

    async fn create_repository() -> Result<Arc<dyn IdentityAttributesRepository>> {
        // Ok(Arc::new(IdentityAttributesSqlxDatabase::new(Arc::new(
        //     SqlxDatabase::create("/Users/etorreborre/.ockam/testdb.sqlite").await?,
        // ))))
        Ok(IdentityAttributesSqlxDatabase::create().await?)
    }
}
