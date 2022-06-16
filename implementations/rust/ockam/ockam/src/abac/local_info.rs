use super::{error::AbacError, AbacMetadata, Action, Resource, Subject};
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};

use serde::{Deserialize, Serialize};

/// ABAC [`LocalInfo`] unique identifier.
pub const ABAC_LOCAL_INFO_IDENTIFIER: &str = "ABAC_LOCAL_INFO_IDENTIFIER";

/// ABAC [`LocalInfo`] used for  [`LocalMessage`].
#[derive(Debug, Serialize, Deserialize)]
pub struct AbacLocalInfo {
    /// The [`Subject`] performing the authorization request
    pub(crate) subject: Subject,
    /// The [`Resource`] the action will be performed on
    pub(crate) resource: Resource,
    /// The [`Action`] to request authorization for
    pub(crate) action: Action,
}

impl AbacLocalInfo {
    /// Create a new `AbacLocalInfo`.
    pub fn new(subject: Subject, resource: Resource, action: Action) -> Self {
        Self {
            subject,
            resource,
            action,
        }
    }

    /// Find an `AbacLocalInfo` in a [`LocalMessage`].
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == ABAC_LOCAL_INFO_IDENTIFIER)
        {
            AbacLocalInfo::try_from(local_info.clone())
        } else {
            Err(AbacError::InvalidLocalInfoType.into())
        }
    }
}

impl From<AbacMetadata> for AbacLocalInfo {
    fn from(metadata: AbacMetadata) -> Self {
        Self {
            subject: metadata.subject,
            resource: metadata.resource,
            action: metadata.action,
        }
    }
}

impl TryFrom<LocalInfo> for AbacLocalInfo {
    type Error = crate::Error;
    fn try_from(local_info: LocalInfo) -> Result<Self, Self::Error> {
        if local_info.type_identifier() != ABAC_LOCAL_INFO_IDENTIFIER {
            return Err(AbacError::InvalidLocalInfoType.into());
        }

        match AbacLocalInfo::decode(local_info.data()) {
            Ok(abac_local_info) => Ok(abac_local_info),
            Err(_) => Err(AbacError::InvalidLocalInfoType.into()),
        }
    }
}

impl TryInto<LocalInfo> for AbacLocalInfo {
    type Error = crate::Error;
    fn try_into(self) -> Result<LocalInfo, Self::Error> {
        match self.encode() {
            Ok(data) => Ok(LocalInfo::new(ABAC_LOCAL_INFO_IDENTIFIER.into(), data)),
            Err(_) => Err(AbacError::InvalidLocalInfoType.into()),
        }
    }
}
