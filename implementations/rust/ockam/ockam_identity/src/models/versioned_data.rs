use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;

/// Binary and a version
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub struct VersionedData {
    /// Version
    #[n(0)] pub version: u8,
    /// Binary
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,
}
