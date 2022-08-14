use minicbor::{Decode, Encode};
use ockam_core::vault::SecretAttributes;
use ockam_core::CowBytes;

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

#[derive(Debug, Copy, Clone, Encode, Decode, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
#[rustfmt::skip]
#[cbor(index_only)]
pub enum GetSecretRequestOperation {
    #[n(1)] GetAttributes,
    #[n(2)] GetSecretBytes,
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetSecretRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4500806>,
    #[n(1)] operation: GetSecretRequestOperation,
}

impl GetSecretRequest {
    pub fn new(operation: GetSecretRequestOperation) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            operation,
        }
    }
    pub fn operation(&self) -> GetSecretRequestOperation {
        self.operation
    }
}
