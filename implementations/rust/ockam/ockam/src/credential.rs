//! Building block for Attribute-based access control

mod exchange;

pub use exchange::*;

use minicbor::{Decode, Encode};
use ockam_core::CowBytes;

#[cfg(feature = "tag")]
use crate::TypeTag;

/// Credential
#[derive(Debug, Decode, Encode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3796735>,
    #[b(1)] attributes: CowBytes<'a>,
    #[b(2)] signature: CowBytes<'a>,
}

impl<'a> Credential<'a> {
    /// Constructor
    pub fn new(attributes: CowBytes<'a>, signature: CowBytes<'a>) -> Self {
        Credential {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            signature,
        }
    }

    /// Attributes getter
    pub fn attributes(&self) -> &[u8] {
        &self.attributes
    }

    /// Signature getter
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
}
