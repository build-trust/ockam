use crate::models::{ChangeHash, Ed25519Signature, Identifier, TimestampInSeconds};
use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, vec::Vec};

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    // CBOR serialized VersionedData
    // where VersionedData::data is CBOR serialized CredentialData
    #[n(1)] pub data: Vec<u8>,

    #[n(2)] pub signature: Ed25519Signature,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData {
    #[n(1)] pub subject: Option<Identifier>,
    #[n(2)] pub subject_latest_change_hash: Option<ChangeHash>,

    #[n(3)] pub subject_attributes: Attributes,

    #[n(4)] pub created_at: TimestampInSeconds,
    #[n(5)] pub expires_at: TimestampInSeconds,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes {
    #[n(1)] pub schema: u64,
    #[n(2)] pub map: BTreeMap<Vec<u8>, Vec<u8>>,
}
