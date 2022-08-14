use minicbor::{Decode, Encode};
use ockam_core::vault::PublicKey;
use ockam_core::{CowBytes, CowStr};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct SignRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5331266>,
    #[b(1)] key_id: CowStr<'a>,
    #[b(2)] data: CowBytes<'a>,
}

impl<'a> SignRequest<'a> {
    pub fn new(key_id: impl Into<CowStr<'a>>, data: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
            data: data.into(),
        }
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct VerifyRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7420437>,
    #[b(1)] signature: CowBytes<'a>,
    #[b(2)] public_key: PublicKey,
    #[b(3)] data: CowBytes<'a>,
}

impl<'a> VerifyRequest<'a> {
    pub fn new(
        signature: impl Into<CowBytes<'a>>,
        public_key: PublicKey,
        data: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            signature: signature.into(),
            public_key,
            data: data.into(),
        }
    }
    pub fn signature(&self) -> &[u8] {
        &self.signature
    }
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}
