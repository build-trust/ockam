use super::super::models::{
    ChangeHash, Ed25519PublicKey, Ed25519Signature, P256ECDSAPublicKey, P256ECDSASignature,
    TimestampInSeconds,
};
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_core::Result;
use ockam_vault::PublicKey;

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct ChangeHistory(#[n(0)] pub Vec<Change>);

impl AsRef<[Change]> for ChangeHistory {
    fn as_ref(&self) -> &[Change] {
        self.0.as_ref()
    }
}

impl ChangeHistory {
    pub fn export(&self) -> Result<Vec<u8>> {
        Ok(minicbor::to_vec(self)?)
    }

    pub fn import(data: &[u8]) -> Result<Self> {
        Ok(minicbor::decode(data)?)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Change {
    // CBOR serialized VersionedData
    // where VersionedData::data is CBOR serialized ChangeData
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,

    #[n(2)] pub signature: ChangeSignature, // over data, new key
    #[n(3)] pub previous_signature: Option<ChangeSignature>, // over data, old key
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum ChangeSignature {
    #[n(1)] Ed25519Signature(#[n(0)] Ed25519Signature),
    #[n(2)] P256ECDSASignature(#[n(0)] P256ECDSASignature),
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ChangeData {
    // If first change use the constant value - sha256 of 'BUILD_TRUST'
    #[n(1)] pub previous_change: Option<ChangeHash>,

    #[n(2)] pub primary_public_key: PrimaryPublicKey,
    #[n(3)] pub revoke_all_purpose_keys: bool,

    #[n(4)] pub created_at: TimestampInSeconds,
    #[n(5)] pub expires_at: TimestampInSeconds,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum PrimaryPublicKey {
    #[n(1)] Ed25519PublicKey(#[n(0)] Ed25519PublicKey),
    #[n(2)] P256ECDSAPublicKey(#[n(0)] P256ECDSAPublicKey),
}

impl From<PrimaryPublicKey> for PublicKey {
    fn from(value: PrimaryPublicKey) -> Self {
        match value {
            PrimaryPublicKey::Ed25519PublicKey(value) => Self::from(value),
            PrimaryPublicKey::P256ECDSAPublicKey(value) => Self::from(value),
        }
    }
}
