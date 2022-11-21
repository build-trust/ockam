use minicbor::{Decode, Encode};
use ockam::vault::Secret;
use ockam_core::vault::{PublicKey, SecretAttributes};
use ockam_core::CowStr;

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
pub struct ExportSecretResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<9094765>,
    #[n(1)] secret: Secret
}

impl ExportSecretResponse {
    pub fn secret(&self) -> &Secret {
        &self.secret
    }

    pub fn new(secret: Secret) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            secret,
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
