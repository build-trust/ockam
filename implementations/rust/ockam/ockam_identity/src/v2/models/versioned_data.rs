use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VersionedData {
    #[n(1)] pub version: u8,
    #[cbor(with = "minicbor::bytes")]
    #[n(2)] pub data: Vec<u8>,
}
