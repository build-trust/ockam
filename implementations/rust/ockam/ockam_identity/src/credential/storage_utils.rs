use crate::authenticated_storage::AuthenticatedStorage;
use crate::credential::{Attributes, Timestamp};
use crate::{IdentityIdentifier, IdentityStateConst};
use minicbor::{Decode, Encode};
use ockam_core::errcode::{Kind, Origin};
use ockam_core::Result;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct AttributesEntry<'a> {
    #[b(1)] attrs: Attributes<'a>,
    #[n(2)] expires: Timestamp,
}

impl<'a> AttributesEntry<'a> {
    pub fn new(attrs: Attributes<'a>, expires: Timestamp) -> Self {
        Self { attrs, expires }
    }
    pub fn attrs(&self) -> &Attributes<'a> {
        &self.attrs
    }
    pub fn expires(&self) -> Timestamp {
        self.expires
    }
}

pub struct AttributesStorageUtils;

impl AttributesStorageUtils {
    /// Return authenticated non-expired attributes attached to that Identity
    pub async fn get_attributes(
        identity_id: &IdentityIdentifier,
        authenticated_storage: &impl AuthenticatedStorage,
    ) -> Result<Option<BTreeMap<String, Vec<u8>>>> {
        let id = identity_id.to_string();
        let entry = match authenticated_storage
            .get(&id, IdentityStateConst::ATTRIBUTES_KEY)
            .await?
        {
            Some(e) => e,
            None => return Ok(None),
        };

        let entry: AttributesEntry = minicbor::decode(&entry)?;

        let now = Timestamp::now().ok_or_else(|| {
            ockam_core::Error::new(Origin::Core, Kind::Internal, "invalid system time")
        })?;
        if entry.expires() <= now {
            authenticated_storage
                .del(&id, IdentityStateConst::ATTRIBUTES_KEY)
                .await?;
            return Ok(None);
        }

        let attrs = entry.attrs().to_owned();

        Ok(Some(attrs))
    }

    pub(crate) async fn put_attributes(
        sender: &IdentityIdentifier,
        entry: AttributesEntry<'_>,
        authenticated_storage: &impl AuthenticatedStorage,
    ) -> Result<()> {
        // TODO: Implement expiration mechanism in Storage
        let entry = minicbor::to_vec(&entry)?;

        authenticated_storage
            .set(
                &sender.to_string(),
                IdentityStateConst::ATTRIBUTES_KEY.to_string(),
                entry,
            )
            .await?;

        Ok(())
    }
}
