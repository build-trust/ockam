use minicbor::{Decode, Encode};
use ockam::vault::Secret;
use ockam_core::vault::SecretAttributes;

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecretRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8005583>,
    #[n(1)] attributes: SecretAttributes,
    #[n(2)] secret: Option<Secret>,
}

impl CreateSecretRequest {
    /// Path to the main storage file
    pub fn attributes(&self) -> &SecretAttributes {
        &self.attributes
    }

    pub fn secret(&self) -> Option<&Secret> {
        self.secret.as_ref()
    }

    pub fn into_secret(self) -> Option<Secret> {
        self.secret
    }

    pub fn new_generate(attributes: SecretAttributes) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: None,
        }
    }

    pub fn new_import(attributes: SecretAttributes, secret: Secret) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
            secret: Some(secret),
        }
    }
}
