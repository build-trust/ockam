use super::super::models::{
    ChangeHash, Ed25519Signature, Identifier, P256ECDSASignature, TimestampInSeconds,
};
use minicbor::{Decode, Encode};
use ockam_core::compat::{collections::BTreeMap, vec::Vec};

/// Credential
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Credential {
    /// CBOR serialized [`super::VersionedData`]
    /// where VersionedData::data is CBOR serialized [`CredentialData`]
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,
    /// Signature over data field using corresponding Credentials [`super::PurposeKeyAttestation`]
    #[n(2)] pub signature: CredentialSignature,
}

/// Signature over [`CredentialData`] using corresponding Credentials [`super::PurposeKeyAttestation`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum CredentialSignature {
    /// Signature using EdDSA Ed25519 key from the corresponding [`super::PurposeKeyAttestation`]
    #[n(1)] Ed25519Signature(#[n(0)] Ed25519Signature),
    /// Signature using ECDSA P256 key from the corresponding [`super::PurposeKeyAttestation`]
    #[n(2)] P256ECDSASignature(#[n(0)] P256ECDSASignature),
}

/// Data inside a [`Credential`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialData {
    /// To whom this Credential was issued
    #[n(1)] pub subject: Option<Identifier>,
    /// Latest Subject's Identity [`ChangeHash`] that was known to the Authority (issuer) at the
    /// moment of issuing of that Credential
    #[n(2)] pub subject_latest_change_hash: Option<ChangeHash>,
    /// [`Attributes`] that Authority (issuer) attests about that Subject
    #[n(3)] pub subject_attributes: Attributes,
    /// Creation [`TimestampInSeconds`] (UTC)
    #[n(4)] pub created_at: TimestampInSeconds,
    /// Expiration [`TimestampInSeconds`] (UTC)
    #[n(5)] pub expires_at: TimestampInSeconds,
}

/// Number that determines which keys&values to expect in the [`Attributes`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct SchemaId(#[n(0)] pub u64);

/// Set a keys&values that an Authority (issuer) attests about the Subject
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Attributes {
    /// [`SchemaId`] that determines which keys&values to expect in the [`Attributes`]
    #[n(1)] pub schema: SchemaId,
    /// Set of keys&values
    #[n(2)] pub map: BTreeMap<Vec<u8>, Vec<u8>>,
}
