use crate::SecureChannelError;
use ockam_core::compat::string::{String, ToString};
use ockam_core::{Encoded, Message, Result};
use serde::{Deserialize, Serialize};

/// SecureChannel LocalInfo unique Identifier
pub const LOCAL_INFO_IDENTIFIER: &str = "SECURE_CHANNEL_ID";

#[derive(Serialize, Deserialize)]
struct Internal {
    identifier: String,
    key_exchange: String,
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
            return Err(SecureChannelError::InvalidLocalInfoType.into());
        }
        Ok(Self { internal })
    }
}

impl LocalInfo {
    /// Key exchange name
    pub fn key_exchange(&self) -> &str {
        &self.internal.key_exchange
    }
}

impl LocalInfo {
    /// Constructor
    pub fn new(key_exchange: String) -> Self {
        Self {
            internal: Internal {
                identifier: LOCAL_INFO_IDENTIFIER.to_string(),
                key_exchange,
            },
        }
    }
}
