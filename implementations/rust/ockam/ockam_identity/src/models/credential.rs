use crate::models::{ChangeHash, Identifier, TimestampInSeconds};
use core::fmt::{Display, Formatter};
use minicbor::bytes::ByteVec;
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::string::String;
use ockam_core::compat::{collections::BTreeMap, vec::Vec};
use ockam_vault::{ECDSASHA256CurveP256Signature, EdDSACurve25519Signature};

/// `data_type` value in [`VersionedData`] struct when used with [`Credential`]
pub const CREDENTIAL_DATA_TYPE: u8 = 3;

/// Credential
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct Credential {
    /// CBOR serialized [`super::VersionedData`]
    /// where VersionedData::data is CBOR serialized [`CredentialData`]
    /// and VersionedData::data_type is [`CREDENTIAL_DATA_TYPE`]
    #[cbor(with = "minicbor::bytes")]
    #[n(0)] pub data: Vec<u8>,
    /// Signature over data field using corresponding Credentials [`super::PurposeKeyAttestation`]
    #[n(1)] pub signature: CredentialSignature,
}

/// Signature over [`CredentialData`] using corresponding Credentials [`super::PurposeKeyAttestation`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum CredentialSignature {
    /// An EdDSA signature using Curve 25519.
    #[n(0)] EdDSACurve25519(#[n(0)] EdDSACurve25519Signature),
    /// An ECDSA signature using SHA-256 and Curve P-256.
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256Signature),
}

/// Data inside a [`Credential`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct CredentialData {
    /// To whom this Credential was issued
    #[n(0)] pub subject: Option<Identifier>,
    /// Latest Subject's Identity [`ChangeHash`] that was known to the Authority (issuer) at the
    /// moment of issuing of that Credential
    #[n(1)] pub subject_latest_change_hash: Option<ChangeHash>,
    /// [`Attributes`] that Authority (issuer) attests about that Subject
    #[n(2)] pub subject_attributes: Attributes,
    /// Creation [`TimestampInSeconds`] (UTC)
    #[n(3)] pub created_at: TimestampInSeconds,
    /// Expiration [`TimestampInSeconds`] (UTC)
    #[n(4)] pub expires_at: TimestampInSeconds,
}

/// Number that determines which keys&values to expect in the [`Attributes`]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct CredentialSchemaIdentifier(#[n(0)] pub u64);

/// Set a keys&values that an Authority (issuer) attests about the Subject
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct Attributes {
    /// [`CredentialSchemaIdentifier`] that determines which keys&values to expect in the [`Attributes`]
    #[n(0)] pub schema: CredentialSchemaIdentifier,
    /// Set of keys&values
    #[n(1)] pub map: BTreeMap<ByteVec, ByteVec>,
}

impl Display for Attributes {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let mut attributes = vec![];
        for (key, value) in self.map.clone() {
            let key = Vec::<u8>::from(key);
            let value = Vec::<u8>::from(value);
            let key =
                String::from_utf8(key.clone()).unwrap_or(format!("HEX:{}", hex::encode(&key)));
            let value =
                String::from_utf8(value.clone()).unwrap_or(format!("HEX:{}", hex::encode(&value)));

            attributes.push(format!("{key}={value}"))
        }
        f.debug_struct("Attributes")
            .field("attrs", &attributes.join(","))
            .finish_non_exhaustive()
    }
}
