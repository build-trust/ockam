use ockam::identity::{Identifier, TimestampInSeconds};
use ockam_core::compat::collections::BTreeMap;
use ockam_core::compat::str::FromStr;
use ockam_core::{Error, Result};
use ockam_node::database::Boolean;

/// Project member stored on the Authority node
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorityMember {
    identifier: Identifier,
    attributes: BTreeMap<Vec<u8>, Vec<u8>>,
    added_by: Identifier,
    added_at: TimestampInSeconds,
    // Was provided by TrustedIdentities argument during the Authority startup
    // pre-trusted identities can't be deleted using [`MembersStorage::delete_member()`]
    is_pre_trusted: bool,
}

impl AuthorityMember {
    pub fn new(
        identifier: Identifier,
        attributes: BTreeMap<Vec<u8>, Vec<u8>>,
        added_by: Identifier,
        added_at: TimestampInSeconds,
        is_pre_trusted: bool,
    ) -> Self {
        Self {
            identifier,
            attributes,
            added_by,
            added_at,
            is_pre_trusted,
        }
    }
    pub fn identifier(&self) -> &Identifier {
        &self.identifier
    }
    pub fn attributes(&self) -> &BTreeMap<Vec<u8>, Vec<u8>> {
        &self.attributes
    }
    pub fn added_by(&self) -> &Identifier {
        &self.added_by
    }
    pub fn added_at(&self) -> TimestampInSeconds {
        self.added_at
    }
    pub fn is_pre_trusted(&self) -> bool {
        self.is_pre_trusted
    }
}

// Low-level representation of a table row
#[derive(sqlx::FromRow)]
pub(crate) struct AuthorityMemberRow {
    identifier: String,
    added_by: String,
    added_at: i64,
    is_pre_trusted: Boolean,
    attributes: Vec<u8>,
}

impl TryFrom<AuthorityMemberRow> for AuthorityMember {
    type Error = Error;

    fn try_from(value: AuthorityMemberRow) -> Result<Self, Self::Error> {
        let member = AuthorityMember::new(
            Identifier::from_str(&value.identifier)?,
            minicbor::decode(&value.attributes)?,
            Identifier::from_str(&value.added_by)?,
            TimestampInSeconds(value.added_at as u64),
            value.is_pre_trusted.to_bool(),
        );

        Ok(member)
    }
}
