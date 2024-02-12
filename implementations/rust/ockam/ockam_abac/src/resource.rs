use crate::ResourceName;
use core::fmt::{Display, Formatter};
use minicbor::{Decode, Encode};
use ockam_core::compat::string::{String, ToString};
use ockam_core::compat::vec::Vec;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};

#[derive(Clone, Debug, Encode, Decode, PartialEq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Resource {
    #[n(1)] pub resource_name: ResourceName,
    #[n(2)] pub resource_type: ResourceType,
}

impl Resource {
    pub fn new(resource_name: impl Into<ResourceName>, resource_type: ResourceType) -> Self {
        Self {
            resource_name: resource_name.into(),
            resource_type,
        }
    }
}

impl Display for Resource {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "(name: {}, type: {})",
            self.resource_name, self.resource_type
        )
    }
}

#[derive(Clone, Debug, Decode, Encode, PartialEq, Eq, EnumString, Display, EnumIter)]
#[cbor(index_only)]
pub enum ResourceType {
    #[n(1)]
    #[strum(serialize = "tcp-inlet")]
    TcpInlet,
    #[n(2)]
    #[strum(serialize = "tcp-outlet")]
    TcpOutlet,
    #[n(3)]
    #[strum(serialize = "echoer")]
    Echoer,
}

impl ResourceType {
    /// Return a string with all valid values joined by a commas
    pub fn join_enum_values_as_string() -> String {
        Self::iter()
            .map(|v| v.to_string())
            .collect::<Vec<String>>()
            .join(", ")
    }
}
