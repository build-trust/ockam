use minicbor::{Decode, Encode};
use ockam_api::{CowBytes, CowStr};

use ockam_core::vault::SecretAttributes;
#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Sha256Request<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4658216>,
    #[b(1)] data: CowBytes<'a>,
}

impl<'a> Sha256Request<'a> {
    pub fn new(data: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            data: data.into(),
        }
    }
    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct HkdfSha256Request<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4101721>,
    #[b(1)] salt: CowStr<'a>,
    #[b(2)] info: CowBytes<'a>,
    #[b(3)] ikm: Option<CowStr<'a>>,
    // TODO: Can be tinyvec
    #[n(4)] output_attributes: Vec<SecretAttributes>,
}

impl<'a> HkdfSha256Request<'a> {
    pub fn new(
        salt: impl Into<CowStr<'a>>,
        info: impl Into<CowBytes<'a>>,
        ikm: Option<impl Into<CowStr<'a>>>,
        output_attributes: impl Into<Vec<SecretAttributes>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            salt: salt.into(),
            info: info.into(),
            ikm: ikm.map(|i| i.into()),
            output_attributes: output_attributes.into(),
        }
    }
    pub fn salt(&self) -> &str {
        &self.salt
    }
    pub fn info(&self) -> &[u8] {
        &self.info
    }
    pub fn ikm(&self) -> Option<&str> {
        self.ikm.as_deref()
    }
    pub fn output_attributes(&self) -> &[SecretAttributes] {
        &self.output_attributes
    }
}
