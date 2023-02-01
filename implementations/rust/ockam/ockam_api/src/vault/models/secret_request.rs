use minicbor::{Decode, Encode};
use ockam::vault::Key;
use ockam_core::vault::KeyAttributes;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecretRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8005583>,
    #[n(1)] attributes: KeyAttributes,
    #[n(2)] secret: Option<Key>,
}

impl CreateSecretRequest {
    /// Path to the main storage file
    pub fn attributes(&self) -> &KeyAttributes {
        &self.attributes
    }

    pub fn secret(&self) -> Option<&Key> {
        self.secret.as_ref()
    }

    pub fn into_secret(self) -> Option<Key> {
        self.secret
    }

    pub fn new_generate(attributes: KeyAttributes) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: None,
        }
    }

    pub fn new_import(attributes: KeyAttributes, secret: Key) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: Some(secret),
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
