use crate::{IdentityError, IdentityIdentifier};
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};
use serde::{Deserialize, Serialize};

/// Identity SecureChannel LocalInfo unique Identifier
pub const IDENTITY_SECURE_CHANNEL_IDENTIFIER: &str = "IDENTITY_SECURE_CHANNEL_IDENTIFIER";

/// Identity SecureChannel LocalInfo used for LocalMessage
#[derive(Serialize, Deserialize)]
pub struct IdentitySecureChannelLocalInfo {
    their_identity_id: IdentityIdentifier,
}

impl IdentitySecureChannelLocalInfo {
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != IDENTITY_SECURE_CHANNEL_IDENTIFIER {
            return Err(IdentityError::InvalidLocalInfoType.into());
        }

        if let Ok(info) = IdentitySecureChannelLocalInfo::decode(value.data()) {
            return Ok(info);
        }

        Err(IdentityError::InvalidLocalInfoType.into())
    }

    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            IDENTITY_SECURE_CHANNEL_IDENTIFIER.into(),
            self.encode()?,
        ))
    }

    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == IDENTITY_SECURE_CHANNEL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(IdentityError::InvalidLocalInfoType.into())
        }
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Key exchange name
    pub fn their_identity_id(&self) -> &IdentityIdentifier {
        &self.their_identity_id
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Constructor
    pub fn new(their_identity_id: IdentityIdentifier) -> Self {
        Self { their_identity_id }
    }
}
