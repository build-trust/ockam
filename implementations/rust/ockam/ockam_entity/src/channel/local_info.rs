use crate::{EntityError, ProfileIdentifier};
use ockam_core::compat::string::{String, ToString};
use ockam_core::{Encoded, Message, Result};
use serde::{Deserialize, Serialize};

/// Entity SecureChannel LocalInfo unique Identifier
pub const LOCAL_INFO_IDENTIFIER: &str = "ENTITY_SECURE_CHANNEL_ID";

#[derive(Serialize, Deserialize)]
struct Internal {
    identifier: String,
    their_profile_id: ProfileIdentifier,
}

/// Entity SecureChannel LocalInfo used for LocalMessage
pub struct LocalInfo {
    internal: Internal,
}

impl Message for LocalInfo {
    fn encode(&self) -> Result<Encoded> {
        self.internal.encode()
    }

    fn decode(e: &Encoded) -> Result<Self> {
        let internal = Internal::decode(e)?;
        if internal.identifier != LOCAL_INFO_IDENTIFIER {
            return Err(EntityError::InvalidLocalInfoType.into());
        }
        Ok(Self { internal })
    }
}

impl LocalInfo {
    /// Key exchange name
    pub fn their_profile_id(&self) -> &ProfileIdentifier {
        &self.internal.their_profile_id
    }
}

impl LocalInfo {
    /// Constructor
    pub fn new(their_profile_id: ProfileIdentifier) -> Self {
        LocalInfo {
            internal: Internal {
                identifier: LOCAL_INFO_IDENTIFIER.to_string(),
                their_profile_id,
            },
        }
    }
}
