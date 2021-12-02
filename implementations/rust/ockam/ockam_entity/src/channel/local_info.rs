use crate::{EntityError, ProfileIdentifier};
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};
use serde::{Deserialize, Serialize};

/// Entity SecureChannel LocalInfo unique Identifier
pub const ENTITY_SECURE_CHANNEL_IDENTIFIER: &str = "ENTITY_SECURE_CHANNEL_IDENTIFIER";

/// Entity SecureChannel LocalInfo used for LocalMessage
#[derive(Serialize, Deserialize)]
pub struct EntitySecureChannelLocalInfo {
    their_profile_id: ProfileIdentifier,
}

impl EntitySecureChannelLocalInfo {
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != ENTITY_SECURE_CHANNEL_IDENTIFIER {
            return Err(EntityError::InvalidLocalInfoType.into());
        }

        if let Ok(info) = EntitySecureChannelLocalInfo::decode(value.data()) {
            return Ok(info);
        }

        Err(EntityError::InvalidLocalInfoType.into())
    }

    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            ENTITY_SECURE_CHANNEL_IDENTIFIER.into(),
            self.encode()?,
        ))
    }

    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == ENTITY_SECURE_CHANNEL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(EntityError::InvalidLocalInfoType.into())
        }
    }
}

impl EntitySecureChannelLocalInfo {
    /// Key exchange name
    pub fn their_profile_id(&self) -> &ProfileIdentifier {
        &self.their_profile_id
    }
}

impl EntitySecureChannelLocalInfo {
    /// Constructor
    pub fn new(their_profile_id: ProfileIdentifier) -> Self {
        Self { their_profile_id }
    }
}
