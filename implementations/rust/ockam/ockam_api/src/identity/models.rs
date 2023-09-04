#![allow(missing_docs)]

use bytes::Bytes;
use ockam_core::CowBytes;

use minicbor::{Decode, Encode, bytes::ByteVec};
use ockam::identity::{Identifier, models::{PurposeKeyAttestation, PurposePublicKey}, PurposeKey};

#[cfg(feature = "tag")]
use ockam_core::TypeTag;

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreateResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<3500430>,
    #[n(1)] identity: ByteVec, //Vec<u8>,
    #[n(2)] identity_id: Identifier,
}

impl CreateResponse {
    pub fn new(identity: Vec<u8>, identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
            identity_id,
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreatePurposeKeyRequest {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<8474240>,
    #[n(1)] identity_id: Identifier,
    //TODO: key type
}

impl CreatePurposeKeyRequest {
    pub fn new(identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id,
        }
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CreatePurposeKeyResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<1573564>,
    #[n(1)] public_key: ByteVec, //Vec<u8>,
    #[n(2)] attestation: ByteVec,
}

impl CreatePurposeKeyResponse {
    pub fn new(key : PurposeKey) -> Self {
        let purpose_key_attestation_binary = minicbor::to_vec(&key.attestation()).unwrap(); //FIXME
        let key = match &key.data().public_key {
            PurposePublicKey::SecureChannelStaticKey(key) => key.0.to_vec(),
            PurposePublicKey::CredentialSigningKey(_) => vec![]
        };
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            public_key: key.into(), 
            attestation: purpose_key_attestation_binary.into(),
        }
    }
    pub fn public_key(&self) -> &[u8] {
        &self.public_key
    }
    pub fn attestation(&self) -> &[u8]{
        &self.attestation
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ValidateIdentityChangeHistoryRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<2556809>,
    #[b(1)] identity: CowBytes<'a>,
}

impl<'a> ValidateIdentityChangeHistoryRequest<'a> {
    pub fn new(identity: impl Into<CowBytes<'a>>) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity: identity.into(),
        }
    }
    pub fn identity(&self) -> &[u8] {
        &self.identity
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ValidateIdentityChangeHistoryResponse {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<4245404>,
    #[n(1)] identity_id: Identifier,
}

impl ValidateIdentityChangeHistoryResponse {
    pub fn new(identity_id: Identifier) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            identity_id,
        }
    }
    pub fn identity_id(&self) -> &Identifier {
        &self.identity_id
    }
}

#[derive(Debug, Clone, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CompareIdentityChangeHistoryRequest<'a> {
    #[cfg(feature = "tag")]
    #[n(0)] tag: TypeTag<7300740>,
    #[b(1)] current_identity: CowBytes<'a>,
    #[b(2)] known_identity: CowBytes<'a>,
}

impl<'a> CompareIdentityChangeHistoryRequest<'a> {
    pub fn new(
        current_identity: impl Into<CowBytes<'a>>,
        known_identity: impl Into<CowBytes<'a>>,
    ) -> Self {
        Self {
            #[cfg(feature = "tag")]
            tag: TypeTag,
            current_identity: current_identity.into(),
            known_identity: known_identity.into(),
        }
    }
    pub fn current_identity(&self) -> &[u8] {
        &self.current_identity
    }
    pub fn known_identity(&self) -> &[u8] {
        &self.known_identity
    }
}
