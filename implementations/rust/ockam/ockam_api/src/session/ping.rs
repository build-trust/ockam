use core::fmt;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::{Decodable, Encodable, Error, Message, Result};
use rand::random;

#[derive(Debug, Default, Copy, Clone, Encode, Decode, CborLen, PartialEq, Eq)]
#[cbor(transparent)]
pub struct Ping(#[n(0)] u64);

impl Ping {
    pub fn new() -> Self {
        Self(random())
    }
}

impl fmt::Display for Ping {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:x}", self.0)
    }
}

impl Encodable for Ping {
    fn encode(self) -> Result<Vec<u8>> {
        ockam_core::cbor_encode_preallocate(self).map_err(Error::from)
    }
}

impl Decodable for Ping {
    fn decode(m: &[u8]) -> Result<Self> {
        minicbor::decode(m).map_err(Error::from)
    }
}

impl Message for Ping {}
