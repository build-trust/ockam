use minicbor::{Decode, Encode};
use ockam_api::{CowBytes, CowStr};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct EncryptRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8899004>,
    #[b(1)] key_id: CowStr<'a>,
    #[b(2)] plaintext: CowBytes<'a>,
    #[b(3)] nonce: CowBytes<'a>,
    #[b(4)] aad: CowBytes<'a>,
}

impl<'a> EncryptRequest<'a> {
    pub fn new(
        key_id: impl Into<CowStr<'a>>,
        plaintext: impl Into<CowBytes<'a>>,
        nonce: impl Into<CowBytes<'a>>,
        aad: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
            plaintext: plaintext.into(),
            nonce: nonce.into(),
            aad: aad.into(),
        }
    }
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn plaintext(&self) -> &[u8] {
        &self.plaintext
    }
    pub fn nonce(&self) -> &[u8] {
        &self.nonce
    }
    pub fn aad(&self) -> &[u8] {
        &self.aad
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct DecryptRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9518326>,
    #[b(1)] key_id: CowStr<'a>,
    #[b(2)] ciphertext: CowBytes<'a>,
    #[b(3)] nonce: CowBytes<'a>,
    #[b(4)] aad: CowBytes<'a>,
}

impl<'a> DecryptRequest<'a> {
    pub fn new(
        key_id: impl Into<CowStr<'a>>,
        ciphertext: impl Into<CowBytes<'a>>,
        nonce: impl Into<CowBytes<'a>>,
        aad: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
            ciphertext: ciphertext.into(),
            nonce: nonce.into(),
            aad: aad.into(),
        }
    }
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
    pub fn nonce(&self) -> &[u8] {
        &self.nonce
    }
    pub fn aad(&self) -> &[u8] {
        &self.aad
    }
}
