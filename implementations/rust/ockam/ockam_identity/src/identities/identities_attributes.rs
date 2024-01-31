use crate::utils::now;
use crate::{AttributesEntry, Identifier, IdentityAttributesRepository};
use ockam_core::compat::sync::Arc;
use ockam_core::Result;

/// This struct provides access to the identities attributes stored on a node.
///
/// It is responsible for:
///
/// - Setting the time at which a given attribute is persisted
/// - Deleting expired attributes from storage. This deletion is performed every time the
///   repository is accessed to retrieve attributes
///
#[derive(Clone)]
pub struct IdentitiesAttributes {
    repository: Arc<dyn IdentityAttributesRepository>,
}

impl IdentitiesAttributes {
    /// Return a new IdentitiesAttributes struct
    pub fn new(repository: Arc<dyn IdentityAttributesRepository>) -> IdentitiesAttributes {
        IdentitiesAttributes { repository }
    }

    /// Return the attributes for a given pair subject/attesting authority
    /// If there are expired attributes for any subject, they are deleted before retrieving the attributes for the
    /// current subject.
    pub async fn get_attributes(
        &self,
        subject: &Identifier,
        attested_by: &Identifier,
    ) -> Result<Option<AttributesEntry>> {
        self.repository.delete_expired_attributes(now()?).await?;
        self.repository.get_attributes(subject, attested_by).await
    }

    /// Set the attributes associated with the given identity identifier.
    /// Previous values gets overridden.
    pub async fn put_attributes(&self, subject: &Identifier, entry: AttributesEntry) -> Result<()> {
        self.repository.put_attributes(subject, entry).await
    }
}

#[cfg(test)]
mod tests {
    use ockam_core::compat::collections::BTreeMap;
    use ockam_core::compat::sync::Arc;
    use std::time::Duration;

    use super::*;
    use crate::utils::now;
    use crate::{identities, IdentityAttributesSqlxDatabase, TimestampInSeconds};

    #[tokio::test]
    async fn test_identities_attributes_expiration() -> Result<()> {
        let identities_attributes = create_identities_attributes().await?;

        // store and retrieve attributes by identity
        let identifier1 = create_identity().await?;
        let identifier2 = create_identity().await?;
        let attributes1 = create_attributes_entry(&identifier1, now()?, 2.into()).await?;
        let attributes2 = create_attributes_entry(&identifier2, now()?, 6.into()).await?;

        identities_attributes
            .put_attributes(&identifier1, attributes1.clone())
            .await?;
        identities_attributes
            .put_attributes(&identifier2, attributes2.clone())
            .await?;

        tokio::time::sleep(Duration::from_secs(4)).await;

        let result = identities_attributes
            .get_attributes(&identifier1, &identifier1)
            .await?;
        assert_eq!(result, None);

        let result = identities_attributes
            .get_attributes(&identifier2, &identifier2)
            .await?;
        assert_eq!(result, Some(attributes2.clone()));

        tokio::time::sleep(Duration::from_secs(4)).await;

        let result = identities_attributes
            .get_attributes(&identifier2, &identifier2)
            .await?;
        assert_eq!(result, None);

        Ok(())
    }

    /// HELPERS
    async fn create_attributes_entry(
        identifier: &Identifier,
        now: TimestampInSeconds,
        ttl: TimestampInSeconds,
    ) -> Result<AttributesEntry> {
        Ok(AttributesEntry::new(
            BTreeMap::from([
                ("name".as_bytes().to_vec(), "alice".as_bytes().to_vec()),
                ("age".as_bytes().to_vec(), "20".as_bytes().to_vec()),
            ]),
            now,
            Some(now + ttl),
            Some(identifier.clone()),
        ))
    }

    async fn create_identity() -> Result<Identifier> {
        let identities = identities().await?;
        identities.identities_creation().create_identity().await
    }

    async fn create_identities_attributes() -> Result<IdentitiesAttributes> {
        Ok(IdentitiesAttributes::new(Arc::new(
            IdentityAttributesSqlxDatabase::create().await?,
        )))
    }
}
