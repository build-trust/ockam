use ockam_core::{
    compat::vec::Vec,
    errcode::{Kind, Origin},
    Decodable, Encodable, Error, LocalInfo, LocalMessage, Result, TransportType,
};
use serde::{Deserialize, Serialize};

/// Node LocalInfo unique Identifier
pub const EXTERNAL_IDENTIFIER: &str = "EXTERNAL_IDENTIFIER";

/// Used for LocalMessage that originates from outside the node (e.g. received from transport)
#[derive(Serialize, Deserialize)]
pub struct ExternalLocalInfo {
    transport_type: TransportType,
}

impl ExternalLocalInfo {
    /// Convert from LocalInfo
    pub fn from_local_info(value: &LocalInfo) -> Result<Self> {
        if value.type_identifier() != EXTERNAL_IDENTIFIER {
            return Err(Error::new_without_cause(Origin::Node, Kind::Invalid));
        }

        if let Ok(info) = ExternalLocalInfo::decode(value.data()) {
            return Ok(info);
        }

        Err(Error::new_without_cause(Origin::Node, Kind::Invalid))
    }

    /// Convert to LocalInfo
    pub fn to_local_info(&self) -> Result<LocalInfo> {
        Ok(LocalInfo::new(EXTERNAL_IDENTIFIER.into(), self.encode()?))
    }

    /// Find first such instance in LocalMessage
    pub fn find_info(local_msg: &LocalMessage) -> Result<Self> {
        if let Some(local_info) = local_msg
            .local_info()
            .iter()
            .find(|x| x.type_identifier() == EXTERNAL_IDENTIFIER)
        {
            Self::from_local_info(local_info)
        } else {
            Err(Error::new_without_cause(Origin::Node, Kind::Invalid))
        }
    }

    /// Find all such instances in LocalMessage
    pub fn find_all(local_msg: &LocalMessage) -> Result<Vec<Self>> {
        Ok(local_msg
            .local_info()
            .iter()
            .filter_map(|x| {
                if x.type_identifier() == EXTERNAL_IDENTIFIER {
                    Self::from_local_info(x).ok()
                } else {
                    None
                }
            })
            .collect())
    }
}

impl ExternalLocalInfo {
    /// Constructor
    pub fn new(transport_type: TransportType) -> Self {
        Self { transport_type }
    }

    /// Transport type
    pub fn transport_type(&self) -> TransportType {
        self.transport_type
    }
}
