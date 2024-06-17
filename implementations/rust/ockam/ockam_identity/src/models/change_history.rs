use crate::models::{ChangeHash, TimestampInSeconds};
use minicbor::{CborLen, Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_vault::{
    ECDSASHA256CurveP256PublicKey, ECDSASHA256CurveP256Signature, EdDSACurve25519PublicKey,
    EdDSACurve25519Signature,
};

/// Identity Change History
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct ChangeHistory(#[n(0)] pub Vec<Change>);

/// `data_type` value in [`VersionedData`] struct when used with [`Change`]
pub const CHANGE_DATA_TYPE: u8 = 1;

/// Individual Identity change which implies replacing the old key
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct Change {
    /// CBOR serialized [`super::VersionedData`]
    /// where VersionedData::data is CBOR serialized [`ChangeData`]
    /// and VersionedData::data_type is [`CHANGE_DATA_TYPE`]
    #[cbor(with = "minicbor::bytes")]
    #[n(0)] pub data: Vec<u8>,
    /// Self-signature over the data using the key from this same [`Change`]
    #[n(1)] pub signature: ChangeSignature,
    /// Self-signature over the data using the key
    /// from the previous [`Change`] in the [`ChangeHistory`]
    #[n(2)] pub previous_signature: Option<ChangeSignature>,
}

/// [`Change`] signature
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum ChangeSignature {
    /// Signature using EdDSA Ed25519
    #[n(0)] EdDSACurve25519(#[n(0)] EdDSACurve25519Signature),
    /// Signature using ECDSA P256
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256Signature),
}

/// Data inside a [`Change`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub struct ChangeData {
    /// [`ChangeHash`] linking this [`Change`] to a previous
    /// It's mandatory unless this is the very first [`Change`] in the [`ChangeHistory`]
    #[n(0)] pub previous_change: Option<ChangeHash>,
    /// Public Key from that [`Change`]
    #[n(1)] pub primary_public_key: PrimaryPublicKey,
    /// Indicates that all [`super::PurposeKeyAttestation`]s signed by previous Primary Public Key should not
    /// be considered valid anymore.
    /// This is usually a desired behaviour if a Purpose Key was compromised.
    #[n(2)] pub revoke_all_purpose_keys: bool,
    /// Timestamp (UTC) after which this key can either:
    ///  1. Sign a [`super::PurposeKeyAttestation`] that is tied to this Identifier
    ///  2. Sign [`ChangeData`] that belongs to the same [`ChangeHistory`] and goes straight after this one
    #[n(3)] pub attestations_valid_from: TimestampInSeconds,
    /// Timestamp (UTC) before which this key can either:
    ///  1. Sign a [`super::PurposeKeyAttestation`] that is tied to this Identifier
    ///  2. Sign [`ChangeData`] that belongs to the same [`ChangeHistory`] and goes straight after this one
    #[n(4)] pub attestations_valid_until: TimestampInSeconds,
}

/// [`Change`]'s public key
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode, CborLen)]
#[rustfmt::skip]
pub enum PrimaryPublicKey {
    /// EdDSA Ed25519 Public Key
    #[n(0)] EdDSACurve25519(#[n(0)] EdDSACurve25519PublicKey),
    /// ECDSA P256 Public Key
    #[n(1)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256PublicKey),
}
