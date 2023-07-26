use crate::models::{
    ChangeHash, Ed25519PublicKey, Ed25519Signature, Identifier, TimestampInSeconds, X25519PublicKey,
};
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyAttestation {
    // CBOR serialized VersionedData
    // where VersionedData::data is CBOR serialized PurposeKeyAttestationData
    #[n(1)] data: Vec<u8>,

    #[n(2)] signature: Ed25519Signature,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyAttestationData {
    #[n(1)] subject: Identifier,
    #[n(2)] subject_latest_change_hash: ChangeHash,

    #[n(3)] public_key: PurposePublicKey,

    #[n(4)] created_at: TimestampInSeconds,
    #[n(5)] expires_at: TimestampInSeconds,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
pub enum PurposePublicKey {
    #[n(1)] SecureChannelAuthenticationKey(#[n(0)] X25519PublicKey),
    #[n(2)] CredentialSigningKey(#[n(0)] Ed25519PublicKey),
}
