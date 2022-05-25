use minicbor::{Decode, Encode};
use ockam_api::CowBytes;
use ockam_core::vault::SecretAttributes;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecretRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8005583>,
    #[n(1)] attributes: SecretAttributes,
    #[b(2)] secret: Option<CowBytes<'a>>,
}

impl<'a> CreateSecretRequest<'a> {
    /// Path to the main storage file
    pub fn attributes(&self) -> &SecretAttributes {
        &self.attributes
    }
    pub fn secret(&self) -> Option<&[u8]> {
        self.secret.as_deref()
    }
    pub fn new_generate(attributes: SecretAttributes) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: None,
        }
    }
    pub fn new_import(attributes: SecretAttributes, secret: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: Some(secret.into()),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetSecretRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4500806>,
    /// 1 - get attributes, 2 - get secret bytes
    #[n(1)] operation: u8,
}

impl GetSecretRequest {
    pub fn new(operation: u8) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            operation,
        }
    }
    /// 0 - get attributes, 1 - get secret bytes
    pub fn operation(&self) -> u8 {
        self.operation
    }
}
