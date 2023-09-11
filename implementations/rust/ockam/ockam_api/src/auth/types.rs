use minicbor::{Decode, Encode};

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attribute<'a> {
    #[cbor(b(1), with = "minicbor::bytes")]
    val: &'a [u8]
}

impl<'a> Attribute<'a> {
    pub fn new(val: &'a [u8]) -> Self {
        Attribute { val }
    }

    pub fn value(&self) -> &'a [u8] {
        self.val
    }
}
