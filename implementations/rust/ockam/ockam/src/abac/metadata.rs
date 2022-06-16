use super::{error::AbacError, AbacLocalInfo, Action, Resource, Subject};
use crate::OckamMessage;
use ockam_core::{Decodable, Encodable, Message, Result};

use serde::{Deserialize, Serialize};

/// ABAC [`Metadata`] unique identifier.
pub const ABAC_METADATA_IDENTIFIER: &str = "ABAC_METADATA_IDENTIFIER";

/// ABAC [`Metadata`] used for [`OckamMessage`]
#[derive(Debug, Serialize, Deserialize)]
pub struct AbacMetadata {
    /// The [`Subject`] performing the authorization request
    pub(crate) subject: Subject,
    /// The [`Resource`] the action will be performed on
    pub(crate) resource: Resource,
    /// The [`Action`] to request authorization for
    pub(crate) action: Action,
}

impl AbacMetadata {
    /// Create an `OckamMessage` from the given [`Message`] and [`AbacMetadata`]
    pub fn into_ockam_message<M>(self, msg: M) -> Result<OckamMessage>
    where
        M: Message,
    {
        match self.encode() {
            Ok(data) => Ok(OckamMessage::new(msg)?.generic_data(ABAC_METADATA_IDENTIFIER, data)),
            Err(_) => Err(AbacError::InvalidMetadataType.into()),
        }
    }

    /// Find abac metadata in an [`OckamMessage`]
    pub fn find_metadata(ockam_msg: &OckamMessage) -> Result<Self> {
        if let Some(generic) = ockam_msg.generic.as_ref() {
            if let Some(metadata) = generic.get(ABAC_METADATA_IDENTIFIER) {
                if let Ok(abac_metadata) = AbacMetadata::decode(metadata) {
                    return Ok(abac_metadata);
                }
            }
        }

        Err(AbacError::InvalidMetadataType.into())
    }
}

impl From<AbacLocalInfo> for AbacMetadata {
    fn from(local_info: AbacLocalInfo) -> Self {
        Self {
            subject: local_info.subject,
            resource: local_info.resource,
            action: local_info.action,
        }
    }
}
