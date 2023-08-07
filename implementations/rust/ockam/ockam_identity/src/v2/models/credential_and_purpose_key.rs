use super::super::models::{
    Credential, CredentialData, PurposeKeyAttestation, PurposeKeyAttestationData,
};
use minicbor::{Decode, Encode};

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialAndPurposeKey {
    #[n(1)] pub credential: Credential,
    #[n(2)] pub purpose_key_attestation: PurposeKeyAttestation,
}

#[derive(Clone, Debug, Encode, Decode)]
#[rustfmt::skip]
#[cbor(map)]
pub struct CredentialAndPurposeKeyData {
    #[n(1)] pub credential_data: CredentialData,
    #[n(2)] pub purpose_key_data: PurposeKeyAttestationData,
}
