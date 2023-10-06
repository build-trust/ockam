use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::{async_trait, Result};
use std::fmt::Debug;

#[async_trait]
pub trait MembersStorage: Debug + Send + Sync + 'static {
    async fn get_member(&self, identifier: &Identifier) -> Result<Option<Member>>;

    async fn get_members(&self) -> Result<Vec<Member>>;

    async fn delete_member(&self, identifier: &Identifier) -> Result<Option<Member>>;

    async fn add_member(&self, member: Member) -> Result<()>;
}

#[derive(Debug, Clone)]
pub struct Member {
    identifier: Identifier,
    attributes: BTreeMap<Vec<u8>, Vec<u8>>,
    added_by: Option<Identifier>,
    added_at: TimestampInSeconds,
    // Was provided by TrustedIdentities argument during the Authority startup
    // permanent identities can't be deleted using [`MembersStorage::delete_member()`]
    is_permanent: bool,
}

impl Member {
    pub fn new(
        identifier: Identifier,
        attributes: BTreeMap<Vec<u8>, Vec<u8>>,
        added_by: Option<Identifier>,
        added_at: TimestampInSeconds,
        is_permanent: bool,
    ) -> Self {
        Self {
            identifier,
            attributes,
            added_by,
            added_at,
            is_permanent,
        }
    }
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }
    pub fn attributes(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attributes
    }
    pub fn added_by(&self) -> &Option<Identifier> {
        &self.added_by
    }
    pub fn added_at(&self) -> TimestampInSeconds {
        self.added_at
    }
    pub fn is_permanent(&self) -> bool {
        self.is_permanent
    }
}
