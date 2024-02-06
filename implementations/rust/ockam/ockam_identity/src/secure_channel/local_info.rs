use ockam_core::compat::vec::Vec;
use ockam_core::{Decodable, Encodable, LocalInfo, LocalMessage, Result};
use serde::{Deserialize, Serialize};

use crate::models::Identifier;
use crate::IdentityError;

/// Identity SecureChannel LocalInfo unique Identifier
pub const IDENTITY_SECURE_CHANNEL_IDENTIFIER: &str = "IDENTITY_SECURE_CHANNEL_IDENTIFIER";

/// Identity SecureChannel LocalInfo used for LocalMessage
#[derive(Serialize, Deserialize)]
pub struct IdentitySecureChannelLocalInfo {
    their_identity_id: Identifier,
}

impl IdentitySecureChannelLocalInfo {
    /// Try to decode `IdentitySecureChannelLocalInfo` from general `LocalInfo`
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != IDENTITY_SECURE_CHANNEL_IDENTIFIER {
            return Err(IdentityError::InvalidLocalInfoType)?;
        }

        if let Ok(info) = IdentitySecureChannelLocalInfo::decode(value.data()) {
            return Ok(info);
        }

        Err(IdentityError::InvalidLocalInfoType)?
    }

    /// Encode `IdentitySecureChannelLocalInfo` to general `LocalInfo`
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            IDENTITY_SECURE_CHANNEL_IDENTIFIER.into(),
            self.encode()?,
        ))
    }

    /// Find `IdentitySecureChannelLocalInfo` in a list of general `LocalInfo` of that `LocalMessage`
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        Self::find_info_from_list(local_msg.local_info_ref())
    }

    /// Find `IdentitySecureChannelLocalInfo` in a list of general `LocalInfo`
    pub fn find_info_from_list(local_info: &[LocalInfo]) -> Result<Self> {
        if let Some(local_info) = local_info
            .iter()
            .find(|x| x.type_identifier() == IDENTITY_SECURE_CHANNEL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(IdentityError::InvalidLocalInfoType)?
        }
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Key exchange name
    pub fn their_identity_id(&self) -> Identifier {
        self.their_identity_id.clone()
    }
}

impl IdentitySecureChannelLocalInfo {
    /// Mark a `LocalInfo` vector with `IdentitySecureChannelLocalInfo`
    /// replacing any pre-existing entries
    pub fn mark(
        mut local_info: Vec<LocalInfo>,
        their_identity_id: Identifier,
    ) -> Result<Vec<LocalInfo>> {
        // strip out any pre-existing IdentitySecureChannelLocalInfo
        local_info.retain(|x| x.type_identifier() != IDENTITY_SECURE_CHANNEL_IDENTIFIER);

        // mark the vector
        local_info.push(Self { their_identity_id }.to_local_info()?);

        Ok(local_info)
    }
}
