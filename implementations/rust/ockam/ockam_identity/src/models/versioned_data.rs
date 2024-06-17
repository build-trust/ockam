use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::vec::Vec;

/// Binary and a version
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct VersionedData {
    /// Version
    #[n(0)] pub version: u8,
    /// Numeric tag of type that was serialized into data field
    #[n(1)] pub data_type: u8,
    /// Binary
    #[cbor(with = "minicbor::bytes")]
    #[n(2)] pub data: Vec<u8>,
}
