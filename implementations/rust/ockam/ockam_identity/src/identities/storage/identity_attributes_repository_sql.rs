use core::str::FromStr;
use std::collections::BTreeMap;

use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::compat::sync::Arc;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::models::Identifier;
use crate::utils::now;
use crate::{AttributesEntry, IdentityAttributesRepository, TimestampInSeconds};

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
        let query = query_as("SELECT * FROM identity_attributes WHERE identifier=$1")
            .bind(identity.to_sql());
        let identity_attributes: Option<IdentityAttributesRow> = query
            .fetch_optional(&self.database.pool)
            .await
            .into_core()?;
        Ok(identity_attributes.map(|r| r.attributes()).transpose()?)
    }

    async fn list_attributes_by_identifier(&self) -> Result<Vec<(Identifier, AttributesEntry)>> {
        let query = query_as("SELECT * FROM identity_attributes");
        let result: Vec<IdentityAttributesRow> =
            query.fetch_all(&self.database.pool).await.into_core()?;
        result
            .into_iter()
            .map(|r| r.identifier().and_then(|i| r.attributes().map(|a| (i, a))))
            .collect::<Result<Vec<_>>>()
    }

    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        let query = query("INSERT OR REPLACE INTO identity_attributes VALUES (?, ?, ?, ?, ?)")
            .bind(subject.to_sql())
            .bind(minicbor::to_vec(entry.attrs())?.to_sql())
            .bind(entry.added().to_sql())
            .bind(entry.expires().map(|e| e.to_sql()))
            .bind(entry.attested_by().map(|e| e.to_sql()));
        query.execute(&self.database.pool).await.void()
    }

    /// Store an attribute name/value pair for a given identity
    async fn put_attribute_value(
        &self,
        subject: &Identifier,
        attribute_name: Vec<u8>,
        attribute_value: Vec<u8>,
    ) -> Result<()> {
        let mut attributes = match self.get_attributes(subject).await? {
            Some(entry) => (*entry.attrs()).clone(),
            None => BTreeMap::new(),
        };
        attributes.insert(attribute_name, attribute_value);
        let entry = AttributesEntry::new(attributes, now()?, None, Some(subject.clone()));
        self.put_attributes(subject, entry).await
    }

    async fn delete(&self, identity: &Identifier) -> Result<()> {
        let query =
            query("DELETE FROM identity_attributes WHERE identifier = ?").bind(identity.to_sql());
        query.execute(&self.database.pool).await.void()
    }
}

// Database serialization / deserialization

impl ToSqlxType for TimestampInSeconds {
    fn to_sql(&self) -> SqlxType {
        self.0.to_sql()
    }
}

// Low-level representation of a table row
#[derive(FromRow)]
struct IdentityAttributesRow {
    identifier: String,
    attributes: Vec<u8>,
    added: i64,
    expires: Option<i64>,
    attested_by: Option<String>,
}

impl IdentityAttributesRow {
    fn identifier(&self) -> Result<Identifier> {
        Identifier::from_str(&self.identifier)
    }

    fn attributes(&self) -> Result<AttributesEntry> {
        let attributes =
            minicbor::decode(self.attributes.as_slice()).map_err(SqlxDatabase::map_decode_err)?;
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
        assert_eq!(
            result,
            vec![
                (identifier1.clone(), attributes1.clone()),
                (identifier2.clone(), attributes2.clone())
            ]
        );

        // delete attributes
        repository.delete(&identifier1).await?;
        let result = repository.get_attributes(&identifier1).await?;
        assert_eq!(result, None);

        // store just one attribute name / value
        let before_adding = now()?;
        repository
            .put_attribute_value(
                &identifier1,
                "name".as_bytes().to_vec(),
                "value".as_bytes().to_vec(),
            )
            .await?;

        let result = repository.get_attributes(&identifier1).await?.unwrap();
        // the name/value pair is present
        assert_eq!(
            result.attrs().get("name".as_bytes()),
            Some(&"value".as_bytes().to_vec())
        );
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
            .put_attribute_value(
                &identifier1,
                "name2".as_bytes().to_vec(),
                "value2".as_bytes().to_vec(),
            )
            .await?;

        let result2 = repository.get_attributes(&identifier1).await?.unwrap();

        // both the new and the old name/value pairs are present
        assert_eq!(
            result2.attrs().get("name".as_bytes()),
            Some(&"value".as_bytes().to_vec())
        );
        assert_eq!(
            result2.attrs().get("name2".as_bytes()),
            Some(&"value2".as_bytes().to_vec())
        );
        // The original timestamp has been updated
        assert!(result2.added() > result.added());

        // the attributes are still self-attested
        assert_eq!(result2.attested_by(), Some(identifier1.clone()));
        Ok(())
    }

    /// HELPERS
    async fn create_attributes_entry(identifier: &Identifier) -> Result<AttributesEntry> {
        Ok(AttributesEntry::new(
            BTreeMap::from([
                ("name".as_bytes().to_vec(), "alice".as_bytes().to_vec()),
                ("age".as_bytes().to_vec(), "20".as_bytes().to_vec()),
            ]),
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
        Ok(IdentityAttributesSqlxDatabase::create().await?)
    }
}
