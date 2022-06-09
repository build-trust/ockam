use crate::{CowBytes, CowStr};
use minicbor::{Decode, Encode};
use ockam_core::vault::{PublicKey, SecretAttributes};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

/// Response body when creating a software vault.
#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateSecretResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7282551>,
    #[b(1)] key_id: CowStr<'a>,
}

impl<'a> CreateSecretResponse<'a> {
    pub fn key_id(&self) -> &str {
        &self.key_id
    }
    pub fn new(key_id: impl Into<CowStr<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            key_id: key_id.into(),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ExportSecretResponse<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9094765>,
    #[b(1)] secret: CowBytes<'a>,
}

impl<'a> ExportSecretResponse<'a> {
    pub fn secret(&self) -> &[u8] {
        &self.secret
    }
    pub fn new(secret: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            secret: secret.into(),
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct GetSecretAttributesResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9257276>,
    #[b(1)] attributes: SecretAttributes,
}

impl GetSecretAttributesResponse {
    pub fn attributes(&self) -> &SecretAttributes {
        &self.attributes
    }
    pub fn new(attributes: SecretAttributes) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            attributes,
        }
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PublicKeyResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1690381>,
    #[b(1)] public_key: PublicKey,
}

impl PublicKeyResponse {
    pub fn public_key(&self) -> &PublicKey {
        &self.public_key
    }
    pub fn new(public_key: PublicKey) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            public_key,
        }
    }
}
