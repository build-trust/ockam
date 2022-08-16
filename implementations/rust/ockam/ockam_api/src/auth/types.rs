use minicbor::{Decode, Encode};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attribute<'a> {
    #[cfg(feature = "tag")]
    #[cbor(n(0))]
    tag: TypeTag<6844116>,
    #[cbor(b(1), with = "minicbor::bytes")]
    val: &'a [u8]
}

impl<'a> Attribute<'a> {
    pub fn new(val: &'a [u8]) -> Self {
        Attribute {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            val,
        }
    }

    pub fn value(&self) -> &'a [u8] {
        self.val
    }
}
