use minicbor::{Decode, Encode};
use ockam_core::vault::PublicKey;
use ockam_core::CowStr;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct EcdhRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<5767078>,
    #[b(1)] secret_key_id: CowStr<'a>,
    #[b(2)] public_key: PublicKey,
}

impl<'a> EcdhRequest<'a> {
    pub fn new(secret_key_id: impl Into<CowStr<'a>>, public_key: PublicKey) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            secret_key_id: secret_key_id.into(),
            public_key,
        }
    }
    pub fn secret_key_id(&self) -> &str {
        &self.secret_key_id
    }
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    pub fn into_parts(self) -> (CowStr<'a>, PublicKey) {
        (self.secret_key_id, self.public_key)
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ComputeKeyIdRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9446354>,
    #[b(1)] public_key: PublicKey,
}

impl ComputeKeyIdRequest {
    pub fn new(public_key: PublicKey) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            public_key,
        }
    }
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
}
