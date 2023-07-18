use crate::compat::string::String;
use crate::compat::vec::Vec;
use crate::Address;

/// Additional metadata for address
#[derive(Debug, Clone, Eq, PartialEq, Default)]
pub struct AddressMetadata {
    /// Indicates that this Address will forward message to another route, therefore the next
    /// hop after this one belongs to another node
    pub is_terminal: bool,
    /// Arbitrary set of attributes
    pub attributes: Vec<(String, String)>,
}

/// A set of metadata for a particular address
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AddressAndMetadata {
    /// Address
    pub address: Address,
    /// Metadata
    pub metadata: AddressMetadata,
}
