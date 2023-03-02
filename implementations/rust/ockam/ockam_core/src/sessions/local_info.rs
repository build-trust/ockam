use crate::errcode::{Kind, Origin};
use crate::sessions::SessionId;
use crate::{Decodable, Encodable, Error, LocalInfo, LocalMessage, Result};
use serde::{Deserialize, Serialize};

/// SessionId LocalInfo unique Identifier
pub const SESSION_ID_IDENTIFIER: &str = "SESSION_ID_IDENTIFIER";

/// Session LocalInfo used for LocalMessage
#[derive(Serialize, Deserialize)]
pub struct SessionIdLocalInfo {
    session_id: SessionId,
}

impl SessionIdLocalInfo {
    /// Try to decode `SessionIdLocalInfo` from general `LocalInfo`
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != SESSION_ID_IDENTIFIER {
            return Err(Error::new(
                Origin::Core,
                Kind::Invalid,
                "LocalInfoId doesn't match",
            ));
        }

        if let Ok(info) = SessionIdLocalInfo::decode(value.data()) {
            return Ok(info);
        }

        Err(Error::new(
            Origin::Core,
            Kind::Invalid,
            "LocalInfoId doesn't match",
        ))
    }

    /// Encode `SessionIdLocalInfo` to general `LocalInfo`
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(SESSION_ID_IDENTIFIER.into(), self.encode()?))
    }

    /// Find `SessionIdLocalInfo` in a list of general `LocalInfo` of that `LocalMessage`
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        Self::find_info_from_list(local_msg.local_info())
    }

    /// Find `SessionIdLocalInfo` in a list of general `LocalInfo`
    pub fn find_info_from_list(local_info: &[LocalInfo]) -> Result<Self> {
        if let Some(local_info) = local_info
            .iter()
            .find(|x| x.type_identifier() == SESSION_ID_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(Error::new(
                Origin::Core,
                Kind::Invalid,
                "LocalInfoId doesn't match",
            ))
        }
    }

    /// Constructor
    pub fn new(session_id: SessionId) -> Self {
        Self { session_id }
    }
}

impl SessionIdLocalInfo {
    /// [`SessionId`]
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}
