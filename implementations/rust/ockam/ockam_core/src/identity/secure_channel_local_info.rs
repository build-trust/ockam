use crate::compat::vec::Vec;
use crate::errcode::{Kind, Origin};
use crate::{
    Error, LocalInfo, LocalInfoIdentifier, LocalMessage, Result, SECURE_CHANNEL_IDENTIFIER,
};

/// SecureChannel LocalInfo used for LocalMessage
pub struct SecureChannelLocalInfo {
    their_identifier: LocalInfoIdentifier,
}

impl SecureChannelLocalInfo {
    /// Their Identifier
    pub fn their_identifier(&self) -> LocalInfoIdentifier {
        self.their_identifier.clone()
    }
}

impl SecureChannelLocalInfo {
    #[track_caller]
    fn error_type_id() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid local info identifier for secure channel",
        )
    }

    #[track_caller]
    fn error_format() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid format for local info identifier for secure channel",
        )
    }

    /// Try to decode `IdentitySecureChannelLocalInfo` from general `LocalInfo`
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != SECURE_CHANNEL_IDENTIFIER {
            return Err(Self::error_type_id());
        }

        match minicbor::decode::<LocalInfoIdentifier>(value.data()) {
            Ok(identifier) => Ok(Self {
                their_identifier: identifier,
            }),
            Err(_) => Err(Self::error_format()),
        }
    }

    /// Encode `IdentitySecureChannelLocalInfo` to general `LocalInfo`
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            SECURE_CHANNEL_IDENTIFIER.into(),
            minicbor::to_vec(&self.their_identifier)?,
        ))
    }

    /// Find `IdentitySecureChannelLocalInfo` in a list of general `LocalInfo` of that `LocalMessage`
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        Self::find_info_from_list(local_msg.local_info_ref())
    }

    /// Find `IdentitySecureChannelLocalInfo` in a list of general `LocalInfo`
    pub fn find_info_from_list(local_info: &[LocalInfo]) -> Result<Self> {
        match local_info
            .iter()
            .find(|x| x.type_identifier() == SECURE_CHANNEL_IDENTIFIER)
        {
            Some(local_info) => Self::from_local_info(local_info),
            None => Err(Self::error_type_id()),
        }
    }
}

impl SecureChannelLocalInfo {
    /// Mark a `LocalInfo` vector with `IdentitySecureChannelLocalInfo`
    /// replacing any pre-existing entries
    pub fn mark(
        mut local_info: Vec<LocalInfo>,
        their_identity_id: LocalInfoIdentifier,
    ) -> Result<Vec<LocalInfo>> {
        // strip out any pre-existing IdentitySecureChannelLocalInfo
        local_info.retain(|x| x.type_identifier() != SECURE_CHANNEL_IDENTIFIER);

        // mark the vector
        local_info.push(
            Self {
                their_identifier: their_identity_id,
            }
            .to_local_info()?,
        );

        Ok(local_info)
    }
}
