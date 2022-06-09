use crate::CowStr;
use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Sha256Response {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<6962278>,
    #[cbor(n(1), with = "minicbor::bytes")]
    hash: [u8; 32],
}

impl Sha256Response {
    pub fn new(hash: [u8; 32]) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            hash,
        }
    }
    pub fn hash(&self) -> [u8; 32] {
        self.hash
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct HkdfSha256Response<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2616593>,
    // TODO: Can be tinyvec
    #[b(1)] output: Vec<CowStr<'a>>,
}

impl<'a> HkdfSha256Response<'a> {
    pub fn new(output: Vec<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            output,
        }
    }
    pub fn output(&self) -> &[CowStr<'a>] {
        &self.output
    }
}
