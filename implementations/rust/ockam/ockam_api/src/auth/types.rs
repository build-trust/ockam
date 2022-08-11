use minicbor::bytes::ByteSlice;
use minicbor::{Decode, Encode};
use ockam_core;
use ockam_core::compat::collections::BTreeMap;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Default, Encode, Decode, PartialEq, Eq)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4724285>,
    #[b(1)] attrs: BTreeMap<&'a str, &'a ByteSlice>
}

impl<'a> Attributes<'a> {
    pub fn new() -> Self {
        Attributes {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attrs: BTreeMap::new(),
        }
    }

    pub fn put(&mut self, k: &'a str, v: &'a [u8]) -> &mut Self {
        self.attrs.insert(k, v.into());
        self
    }

    pub fn get(&self, k: &str) -> Option<&[u8]> {
        self.attrs.get(k).map(|s| &***s)
    }

    pub fn attrs(&self) -> impl Iterator<Item = (&str, &[u8])> {
        self.attrs.iter().map(|(k, v)| (*k, &***v))
    }
}

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
