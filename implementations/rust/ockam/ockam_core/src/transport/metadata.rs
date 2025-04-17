use crate::compat::string::{String, ToString};
use crate::errcode::{Kind, Origin};
use crate::transport::{LocalInfoTransportType, TRANSPORT_IDENTIFIER};
use crate::{AddressMetadata, Error, Result, TransportType};

/// Transport Metadata used for Terminal Address
pub struct TransportMetadata {
    transport_type: TransportType,
}

impl TransportMetadata {
    /// Transport type
    pub fn transport_type(&self) -> TransportType {
        self.transport_type
    }
}

impl TransportMetadata {
    #[track_caller]
    fn error_type_id() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid metadata identifier for transport type",
        )
    }

    #[track_caller]
    fn error_format() -> Error {
        Error::new(
            Origin::Identity,
            Kind::Invalid,
            "invalid format for metadata identifier for transport type",
        )
    }

    /// Get the transport type
    pub fn from_terminal_address_metadata(terminal: &AddressMetadata) -> Result<Self> {
        let transport_type = if let Some(transport_type) =
            terminal.attributes.iter().find_map(|(key, value)| {
                if key == TRANSPORT_IDENTIFIER {
                    Some(value.clone())
                } else {
                    None
                }
            }) {
            transport_type
        } else {
            return Err(Self::error_type_id());
        };

        if let Ok(transport_type) = LocalInfoTransportType::try_from(transport_type.as_str()) {
            Ok(Self {
                transport_type: transport_type.transport_type(),
            })
        } else {
            Err(Self::error_format())
        }
    }

    /// Create an attribute for a transport
    pub fn attribute(transport_type: TransportType) -> (String, String) {
        (TRANSPORT_IDENTIFIER.to_string(), transport_type.to_string())
    }
}
