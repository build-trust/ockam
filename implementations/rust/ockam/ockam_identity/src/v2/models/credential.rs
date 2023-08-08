use super::super::models::{
    ChangeHash, Ed25519Signature, Identifier, P256ECDSASignature, TimestampInSeconds,
};
use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, vec::Vec};

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    // CBOR serialized VersionedData
    // where VersionedData::data is CBOR serialized CredentialData
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,

    #[n(2)] pub signature: CredentialSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum CredentialSignature {
    #[n(1)] Ed25519Signature(#[n(0)] Ed25519Signature),
    #[n(2)] P256ECDSASignature(#[n(0)] P256ECDSASignature),
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData {
    #[n(1)] pub subject: Option<Identifier>,
    #[n(2)] pub subject_latest_change_hash: Option<ChangeHash>,

    #[n(3)] pub subject_attributes: Attributes,

    #[n(4)] pub created_at: TimestampInSeconds,
    #[n(5)] pub expires_at: TimestampInSeconds,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct SchemaId(#[n(0)] pub u64);

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes {
    #[n(1)] pub schema: SchemaId,
    #[n(2)] pub map: BTreeMap<Vec<u8>, Vec<u8>>,
}
