use core::str::FromStr;

use sqlx::*;
use tracing::debug;

use ockam_core::async_trait;
use ockam_core::Result;
use ockam_node::database::{FromSqlxError, SqlxDatabase, SqlxType, ToSqlxType, ToVoid};

use crate::models::Identifier;
use crate::{AttributesEntry, IdentityAttributesRepository, TimestampInSeconds};

/// Implementation of [`IdentityAttributesRepository`] trait based on an underlying database
/// using sqlx as its API, and Sqlite as its driver
#[derive(Clone)]
pub struct IdentityAttributesSqlxDatabase {
    database: SqlxDatabase,
    node_name: String,
}

impl IdentityAttributesSqlxDatabase {
    /// Create a new database
    pub fn new(database: SqlxDatabase, node_name: &str) -> Self {
        debug!("create a repository for identity attributes");
        Self {
            database,
            node_name: node_name.to_string(),
        }
    }

    /// Create a new in-memory database
    pub async fn create() -> Result<Self> {
        Ok(Self::new(
            SqlxDatabase::in_memory("identity attributes").await?,
            "default",
        ))
    }
}

#[async_trait]
impl IdentityAttributesRepository for IdentityAttributesSqlxDatabase {
    async fn get_attributes(
        &self,
        identity: &Identifier,
        attested_by: &Identifier,
    ) -> Result<Option<AttributesEntry>> {
        let query = query_as(
            "SELECT identifier, attributes, added, expires, attested_by FROM identity_attributes WHERE identifier=$1 AND attested_by=$2 AND node_name=$3"
            )
            .bind(identity.to_sql())
            .bind(attested_by.to_sql())
            .bind(self.node_name.to_sql());
        let identity_attributes: Option<IdentityAttributesRow> = query
            .fetch_optional(&*self.database.pool)
            .await
            .into_core()?;
        Ok(identity_attributes.map(|r| r.attributes()).transpose()?)
    }

    async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        let query = query(
            "INSERT OR REPLACE INTO identity_attributes (identifier, attributes, added, expires, attested_by, node_name) VALUES (?, ?, ?, ?, ?, ?)"
            )
            .bind(subject.to_sql())
            .bind(ockam_core::cbor_encode_preallocate(entry.attrs())?.to_sql())
            .bind(entry.added_at().to_sql())
            .bind(entry.expires_at().map(|e| e.to_sql()))
            .bind(entry.attested_by().map(|e| e.to_sql()))
            .bind(self.node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
    }

    // This query is regularly invoked by IdentitiesAttributes to make sure that we expire attributes regularly
    async fn delete_expired_attributes(&self, now: TimestampInSeconds) -> Result<()> {
        let query = query("DELETE FROM identity_attributes WHERE expires<=? AND node_name=?")
            .bind(now.to_sql())
            .bind(self.node_name.to_sql());
        query.execute(&*self.database.pool).await.void()
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
    #[allow(dead_code)]
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
    use ockam_core::compat::collections::BTreeMap;
    use ockam_core::compat::sync::Arc;
    use std::ops::Add;

    use super::*;
    use crate::identities;
    use crate::utils::now;

    #[tokio::test]
    async fn test_identities_attributes_repository() -> Result<()> {
        let repository = create_repository().await?;
        let now = now()?;

        // store and retrieve attributes by identity
        let identifier1 = create_identity().await?;
        let attributes1 = create_attributes_entry(&identifier1, now, Some(2.into())).await?;
        let identifier2 = create_identity().await?;
        let attributes2 = create_attributes_entry(&identifier2, now, Some(2.into())).await?;

        repository
            .put_attributes(&identifier1, attributes1.clone())
            .await?;
        repository
            .put_attributes(&identifier2, attributes2.clone())
            .await?;

        let result = repository
            .get_attributes(&identifier1, &identifier1)
            .await?;
        assert_eq!(result, Some(attributes1.clone()));

        let result = repository
            .get_attributes(&identifier2, &identifier2)
            .await?;
        assert_eq!(result, Some(attributes2.clone()));

        Ok(())
    }

    #[tokio::test]
    async fn test_delete_expired_attributes() -> Result<()> {
        let repository = create_repository().await?;
        let now = now()?;

        // store some attributes with and without an expiry date
        let identifier1 = create_identity().await?;
        let identifier2 = create_identity().await?;
        let identifier3 = create_identity().await?;
        let identifier4 = create_identity().await?;
        let attributes1 = create_attributes_entry(&identifier1, now, Some(1.into())).await?;
        let attributes2 = create_attributes_entry(&identifier2, now, Some(10.into())).await?;
        let attributes3 = create_attributes_entry(&identifier3, now, Some(100.into())).await?;
        let attributes4 = create_attributes_entry(&identifier4, now, None).await?;

        repository
            .put_attributes(&identifier1, attributes1.clone())
            .await?;
        repository
            .put_attributes(&identifier2, attributes2.clone())
            .await?;
        repository
            .put_attributes(&identifier3, attributes3.clone())
            .await?;
        repository
            .put_attributes(&identifier4, attributes4.clone())
            .await?;

        // delete all the attributes with an expiry date <= now + 10
        // only attributes1 and attributes2 must be deleted
        repository.delete_expired_attributes(now.add(10)).await?;

        let result = repository
            .get_attributes(&identifier1, &identifier1)
            .await?;
        assert_eq!(result, None);

        let result = repository
            .get_attributes(&identifier2, &identifier2)
            .await?;
        assert_eq!(result, None);

        let result = repository
            .get_attributes(&identifier3, &identifier3)
            .await?;
        assert_eq!(
            result,
            Some(attributes3),
            "attributes 3 are not expired yet"
        );

        let result = repository
            .get_attributes(&identifier4, &identifier4)
            .await?;
        assert_eq!(
            result,
            Some(attributes4),
            "attributes 4 have no expiry date"
        );

        Ok(())
    }

    /// HELPERS
    async fn create_attributes_entry(
        identifier: &Identifier,
        now: TimestampInSeconds,
        ttl: Option<TimestampInSeconds>,
    ) -> Result<AttributesEntry> {
        Ok(AttributesEntry::new(
            BTreeMap::from([
                ("name".as_bytes().to_vec(), "alice".as_bytes().to_vec()),
                ("age".as_bytes().to_vec(), "20".as_bytes().to_vec()),
            ]),
            now,
            ttl.map(|ttl| now + ttl),
            Some(identifier.clone()),
        ))
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }

    async fn create_repository() -> Result<Arc<dyn IdentityAttributesRepository>> {
        Ok(Arc::new(IdentityAttributesSqlxDatabase::create().await?))
    }
}
