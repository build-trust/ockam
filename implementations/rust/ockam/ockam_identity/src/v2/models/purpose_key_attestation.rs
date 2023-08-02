use super::super::models::{
    ChangeHash, Ed25519PublicKey, Ed25519Signature, Identifier, P256ECDSAPublicKey,
    P256ECDSASignature, TimestampInSeconds, X25519PublicKey,
};
use super::super::IdentityError;
use minicbor::{Decode, Encode};
use ockam_core::compat::vec::Vec;
use ockam_core::{Error, Result};
use ockam_vault::{PublicKey, SecretType};

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyAttestation {
    // CBOR serialized VersionedData
    // where VersionedData::data is CBOR serialized PurposeKeyAttestationData
    #[cbor(with = "minicbor::bytes")]
    #[n(1)] pub data: Vec<u8>,

    #[n(2)] pub signature: PurposeKeyAttestationSignature,
}

#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
#[rustfmt::skip]
pub enum PurposeKeyAttestationSignature {
    #[n(1)] Ed25519Signature(#[n(0)] Ed25519Signature),
    #[n(2)] P256ECDSASignature(#[n(0)] P256ECDSASignature),
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct PurposeKeyAttestationData {
    #[n(1)] pub subject: Identifier,
    #[n(2)] pub subject_latest_change_hash: ChangeHash,

    #[n(3)] pub public_key: PurposePublicKey,

    #[n(4)] pub created_at: TimestampInSeconds,
    #[n(5)] pub expires_at: TimestampInSeconds,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
pub enum PurposePublicKey {
    #[n(1)] SecureChannelAuthenticationKey(#[n(0)] X25519PublicKey),
    #[n(2)] CredentialSigningKey(#[n(0)] CredentialSigningKey),
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
pub enum CredentialSigningKey {
    #[n(1)] Ed25519PublicKey(#[n(0)] Ed25519PublicKey),
    #[n(2)] P256ECDSAPublicKey(#[n(0)] P256ECDSAPublicKey),
}

impl TryFrom<PublicKey> for CredentialSigningKey {
    type Error = Error;

    fn try_from(value: PublicKey) -> Result<Self> {
        match value.stype() {
            SecretType::Ed25519 => Ok(Self::Ed25519PublicKey(value.try_into()?)),
            SecretType::NistP256 => Ok(Self::P256ECDSAPublicKey(value.try_into()?)),
            _ => Err(IdentityError::InvalidKeyType.into()),
        }
    }
}
