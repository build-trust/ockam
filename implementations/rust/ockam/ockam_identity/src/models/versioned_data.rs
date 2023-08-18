use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;

/// Binary and a version
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VersionedData {
    /// Version
    #[n(1)] pub version: u8,
    /// Binary
    #[cbor(with = "minicbor::bytes")]
    #[n(2)] pub data: Vec<u8>,
}
