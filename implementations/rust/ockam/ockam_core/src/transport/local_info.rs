use crate::compat::vec::Vec;
use crate::errcode::{Kind, Origin};
use crate::transport::{LocalInfoTransportType, TRANSPORT_IDENTIFIER};
use crate::{Error, LocalInfo, LocalMessage, Result, TransportType};

/// Transport LocalInfo used for LocalMessage
pub struct TransportLocalInfo {
    transport_type: TransportType,
}

impl TransportLocalInfo {
    /// Transport type
    pub fn transport_type(&self) -> TransportType {
        self.transport_type
    }
}

impl TransportLocalInfo {
    #[track_caller]
    fn error_type_id() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid metadata identifier for transport",
        )
    }

    #[track_caller]
    fn error_format() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid format for transport local info",
        )
    }

    /// Try to decode `LocalInfoTransportType` from general `LocalInfo`
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != TRANSPORT_IDENTIFIER {
            return Err(Self::error_type_id());
        }

        match minicbor::decode::<LocalInfoTransportType>(value.data()) {
            Ok(transport_type) => Ok(Self {
                transport_type: transport_type.transport_type(),
            }),
            Err(_) => Err(Self::error_format()),
        }
    }

    /// Encode `LocalInfoTransportType` to general `LocalInfo`
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(
            TRANSPORT_IDENTIFIER.into(),
            crate::cbor_encode_preallocate(self.transport_type)?,
        ))
    }

    /// Find `LocalInfoTransportType` in a list of general `LocalInfo` of that `LocalMessage`
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        Self::find_info_from_list(local_msg.local_info())
    }

    /// Find `LocalInfoTransportType` in a list of general `LocalInfo`
    pub fn find_info_from_list(local_info: &[LocalInfo]) -> Result<Self> {
        match local_info
            .iter()
            .find(|x| x.type_identifier() == TRANSPORT_IDENTIFIER)
        {
            Some(local_info) => Self::from_local_info(local_info),
            None => Err(Self::error_type_id()),
        }
    }
}

impl TransportLocalInfo {
    /// Mark a `LocalInfo` vector with `IdentitySecureChannelLocalInfo`
    /// replacing any pre-existing entries
    pub fn mark(
        mut local_info: Vec<LocalInfo>,
        transport_type: TransportType,
    ) -> Result<Vec<LocalInfo>> {
        // strip out any pre-existing IdentitySecureChannelLocalInfo
        local_info.retain(|x| x.type_identifier() != TRANSPORT_IDENTIFIER);

        // mark the vector
        local_info.push(Self { transport_type }.to_local_info()?);

        Ok(local_info)
    }
}
