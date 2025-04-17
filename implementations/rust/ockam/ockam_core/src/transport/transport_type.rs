use crate::alloc::string::ToString;
use crate::TransportType;
use alloc::string::String;
use core::fmt::{Display, Formatter};
use minicbor::{CborLen, Decode, Encode};

/// Transport identifier in LocalInfo and Metadata
pub const TRANSPORT_IDENTIFIER: &str = "TRANSPORT_IDENTIFIER";

/// LocalInfo TransportType
#[derive(Clone, Eq, PartialEq, Hash, Encode, Decode, CborLen, Debug)]
#[cbor(transparent)]
pub struct LocalInfoTransportType(TransportType);

impl LocalInfoTransportType {
    /// Constructor
    pub fn new(transport_type: TransportType) -> Self {
        Self(transport_type)
    }

    /// Getter
    pub fn transport_type(&self) -> TransportType {
        self.0
    }
}

impl TryFrom<&str> for LocalInfoTransportType {
    type Error = core::num::ParseIntError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value
            .parse::<u8>()
            .map(TransportType::new)
            .map(LocalInfoTransportType::new)
    }
}

impl From<&LocalInfoTransportType> for String {
    fn from(id: &LocalInfoTransportType) -> Self {
        id.0.value().to_string()
    }
}

impl From<LocalInfoTransportType> for String {
    fn from(id: LocalInfoTransportType) -> Self {
        String::from(&id)
    }
}

impl Display for LocalInfoTransportType {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.write_str(&String::from(self))
    }
}
