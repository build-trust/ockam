use minicbor::{Decode, Encode};
use ockam_api::CowBytes;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct EncryptResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8406980>,
    #[b(1)] ciphertext: CowBytes<'a>,
}

impl<'a> EncryptResponse<'a> {
    pub fn new(ciphertext: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            ciphertext: ciphertext.into(),
        }
    }
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DecryptResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3016559>,
    #[b(1)] plaintext: CowBytes<'a>,
}

impl<'a> DecryptResponse<'a> {
    pub fn new(plaintext: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            plaintext: plaintext.into(),
        }
    }
    pub fn plaintext(&self) -> &[u8] {
        &self.plaintext
    }
}
