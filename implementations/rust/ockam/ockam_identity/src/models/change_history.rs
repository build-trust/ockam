use crate::models::{ChangeHash, TimestampInSeconds};
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_vault::{
    ECDSASHA256CurveP256PublicKey, ECDSASHA256CurveP256Signature, EdDSACurve25519PublicKey,
    EdDSACurve25519Signature,
};

/// Identity Change History
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(transparent)]
pub struct ChangeHistory(#[n(0)] pub Vec<Change>);

/// Individual Identity change which implies replacing the old key
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct Change {
    /// CBOR serialized [`super::VersionedData`]
    /// where VersionedData::data is CBOR serialized [`ChangeData`]
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,
    /// Self-signature over the data using the key from this same [`Change`]
    #[n(2)] pub signature: ChangeSignature,
    /// Self-signature over the data using the key
    /// from the previous [`Change`] in the [`ChangeHistory`]
    #[n(3)] pub previous_signature: Option<ChangeSignature>,
}

/// [`Change`] signature
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum ChangeSignature {
    /// Signature using EdDSA Ed25519
    #[n(1)] EdDSACurve25519(#[n(0)] EdDSACurve25519Signature),
    /// Signature using ECDSA P256
    #[n(2)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256Signature),
}

/// Data inside a [`Change`]
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct ChangeData {
    /// [`ChangeHash`] linking this [`Change`] to a previous
    /// It's mandatory unless this is the very first [`Change`] in the [`ChangeHistory`]
    #[n(1)] pub previous_change: Option<ChangeHash>,
    /// Public Key from that [`Change`]
    #[n(2)] pub primary_public_key: PrimaryPublicKey,
    /// Indicates that all [`super::PurposeKeyAttestation`]s signed by previous Primary Public Key should not
    /// be considered valid anymore.
    /// This is usually a desired behaviour if a Purpose Key was compromised.
    #[n(3)] pub revoke_all_purpose_keys: bool,
    /// Creation [`TimestampInSeconds`] (UTC)
    #[n(4)] pub created_at: TimestampInSeconds,
    /// Expiration [`TimestampInSeconds`] (UTC)
    #[n(5)] pub expires_at: TimestampInSeconds,
}

/// [`Change`]'s public key
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum PrimaryPublicKey {
    /// EdDSA Ed25519 Public Key
    #[n(1)] EdDSACurve25519(#[n(0)] EdDSACurve25519PublicKey),
    /// ECDSA P256 Public Key
    #[n(2)] ECDSASHA256CurveP256(#[n(0)] ECDSASHA256CurveP256PublicKey),
}
